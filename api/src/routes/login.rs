//! GitHub 登录（DESIGN §3.2）。两条路，一个账号：
//!
//! - **终端**走设备码流程：`playtest login` 向这里要一个码，人在浏览器里输，CLI 轮询。
//!   不用本机开回调端口，也不用 client secret，所以自托管只配一个 client_id 就能用。
//! - **控制台**走网页授权码流程：浏览器被送到 GitHub，回来时把 `code` 交给这里换令牌。
//!   这一步要 client secret，只在服务器上。
//!
//! 两条路的最后一步都可以顺带带上手里的匿名令牌：那个匿名身份下的作品会归到账号里，
//! 到期时间去掉，门禁页和广场上的「匿名开发者」换成真名。这是 DESIGN §3.2 说的
//! 「第一次不登录也能拿到链接；要保留才登录」——保留的就是这些作品。
//!
//! GitHub 在大陆间歇可用（DESIGN §5）：控制面在香港替 CLI 去问 GitHub，人自己的浏览器
//! 还是得能打开 github.com 才能输码；失败时已经发出去的匿名链接不受影响。

use axum::extract::State;
use axum::http::header::AUTHORIZATION;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Redirect, Response};
use axum::Json;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use playtest_common::api::{
    DeviceLoginPoll, DeviceLoginStart, LoginPollResponse, LoginResponse, Me, WebLoginExchange,
};
use playtest_common::hash;
use rand::RngCore;
use serde::Deserialize;

use crate::auth::{self, Caller};
use crate::clock;
use crate::config::GitHubApp;
use crate::db;
use crate::error::{ApiError, ApiResult};
use crate::plaza;
use crate::routes::JsonBody;
use crate::state::AppState;

const UNAVAILABLE: &str = "这个控制面没有开 GitHub 登录。匿名链接照常能用，只是 24 小时后失效。";

/// GitHub 说 `slow_down` 时要在原间隔上加 5 秒（它的文档这么写）。
const SLOW_DOWN_EXTRA: u32 = 5;

// ---------------------------------------------------------------- 设备码

pub async fn device_start(State(state): State<AppState>) -> ApiResult<Json<DeviceLoginStart>> {
    let gh = state
        .github()
        .ok_or_else(|| ApiError::login_unavailable(UNAVAILABLE))?;

    #[derive(Deserialize)]
    struct Body {
        device_code: String,
        user_code: String,
        verification_uri: String,
        expires_in: u32,
        interval: u32,
    }
    let body: Body = state
        .http()
        .post(format!("{}/login/device/code", gh.web_base))
        .header("Accept", "application/json")
        .json(&serde_json::json!({ "client_id": gh.client_id, "scope": "" }))
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(github_unreachable)?
        .json()
        .await
        .map_err(github_unreachable)?;

    Ok(Json(DeviceLoginStart {
        device_code: body.device_code,
        user_code: body.user_code,
        verification_uri: body.verification_uri,
        expires_in: body.expires_in,
        interval: body.interval,
    }))
}

pub async fn device_poll(
    State(state): State<AppState>,
    headers: HeaderMap,
    JsonBody(request): JsonBody<DeviceLoginPoll>,
) -> ApiResult<Json<LoginPollResponse>> {
    let gh = state
        .github()
        .ok_or_else(|| ApiError::login_unavailable(UNAVAILABLE))?;
    if request.device_code.trim().is_empty() {
        return Err(ApiError::invalid("device_code 是空的。"));
    }

    let outcome = access_token(
        &state,
        gh,
        &[
            ("client_id", gh.client_id.as_str()),
            ("device_code", request.device_code.as_str()),
            ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
        ],
    )
    .await?;

    let access_token = match outcome {
        TokenOutcome::Token(t) => t,
        TokenOutcome::Pending { interval } => {
            return Ok(Json(LoginPollResponse::Pending { interval }));
        }
    };

    let login = finish(&state, gh, &access_token, &headers).await?;
    Ok(Json(LoginPollResponse::Ok(login)))
}

// ---------------------------------------------------------------- 网页授权码

/// 浏览器直接打开这个地址，被送去 GitHub。回来落在控制台上，带着 `code` 和 `state`。
pub async fn web_start(State(state): State<AppState>) -> Response {
    let Some(gh) = state.github() else {
        return ApiError::login_unavailable(UNAVAILABLE).into_response();
    };
    if gh.client_secret.is_none() {
        return ApiError::login_unavailable(
            "这个控制面只配了终端登录（没有 client secret），控制台里请粘贴 playtest login 之后配置文件里的令牌。",
        )
        .into_response();
    }
    let nonce = random_state();
    if !state.remember_login_state(nonce.clone()) {
        return ApiError::public(
            StatusCode::TOO_MANY_REQUESTS,
            playtest_common::api::ErrorCode::LoginFailed,
            "正在登录的人太多，过一分钟再试。",
        )
        .into_response();
    }
    let url = format!(
        "{}/login/oauth/authorize?client_id={}&redirect_uri={}&state={}&scope=",
        gh.web_base,
        urlencode(&gh.client_id),
        urlencode(&gh.console_url),
        urlencode(&nonce)
    );
    Redirect::to(&url).into_response()
}

pub async fn web_exchange(
    State(state): State<AppState>,
    headers: HeaderMap,
    JsonBody(request): JsonBody<WebLoginExchange>,
) -> ApiResult<Json<LoginResponse>> {
    let gh = state
        .github()
        .ok_or_else(|| ApiError::login_unavailable(UNAVAILABLE))?;
    let Some(secret) = gh.client_secret.as_deref() else {
        return Err(ApiError::login_unavailable(
            "这个控制面只配了终端登录（没有 client secret）。",
        ));
    };
    if !state.take_login_state(&request.state) {
        return Err(ApiError::login_failed(
            "这次登录的凭据对不上或已过期（从点「登录」到回来超过了 10 分钟）。回到控制台再点一次。",
        ));
    }
    let outcome = access_token(
        &state,
        gh,
        &[
            ("client_id", gh.client_id.as_str()),
            ("client_secret", secret),
            ("code", request.code.as_str()),
            ("redirect_uri", gh.console_url.as_str()),
        ],
    )
    .await?;
    let access_token = match outcome {
        TokenOutcome::Token(t) => t,
        // 授权码流程没有「等一等」这种状态；GitHub 若真这么回，按失败处理。
        TokenOutcome::Pending { .. } => {
            return Err(ApiError::login_failed("GitHub 没有给出令牌。再试一次。"));
        }
    };
    let login = finish(&state, gh, &access_token, &headers).await?;
    Ok(Json(login))
}

// ---------------------------------------------------------------- 我是谁

pub async fn me(caller: Caller) -> Json<Me> {
    Json(Me {
        kind: caller.kind.as_db().to_string(),
        display_name: caller.display_name,
        login: caller.login,
        expires_at: caller.expires_at,
    })
}

// ---------------------------------------------------------------- 共用

enum TokenOutcome {
    Token(String),
    Pending { interval: u32 },
}

/// `POST /login/oauth/access_token`。GitHub 用 200 + `error` 字段表示各种没成，不用状态码。
/// 请求体用 JSON（GitHub 两种都收），省得再拖一个表单编码的依赖。
async fn access_token(
    state: &AppState,
    gh: &GitHubApp,
    form: &[(&str, &str)],
) -> ApiResult<TokenOutcome> {
    let form: serde_json::Map<String, serde_json::Value> = form
        .iter()
        .map(|(k, v)| (k.to_string(), serde_json::Value::String(v.to_string())))
        .collect();
    #[derive(Deserialize)]
    struct Body {
        access_token: Option<String>,
        error: Option<String>,
        error_description: Option<String>,
        interval: Option<u32>,
    }
    let body: Body = state
        .http()
        .post(format!("{}/login/oauth/access_token", gh.web_base))
        .header("Accept", "application/json")
        .json(&form)
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(github_unreachable)?
        .json()
        .await
        .map_err(github_unreachable)?;

    if let Some(token) = body.access_token.filter(|t| !t.is_empty()) {
        return Ok(TokenOutcome::Token(token));
    }
    let error = body.error.unwrap_or_default();
    match error.as_str() {
        "authorization_pending" => Ok(TokenOutcome::Pending {
            interval: body.interval.unwrap_or(5),
        }),
        "slow_down" => Ok(TokenOutcome::Pending {
            interval: body.interval.unwrap_or(5) + SLOW_DOWN_EXTRA,
        }),
        "expired_token" => Err(ApiError::login_failed(
            "这个码过期了（15 分钟没输完）。重新运行 playtest login 拿一个新的。",
        )),
        "access_denied" => Err(ApiError::login_failed(
            "你在 GitHub 上点了「取消」。想登录的话再运行一次 playtest login。",
        )),
        "incorrect_device_code" | "bad_verification_code" | "incorrect_client_credentials" => {
            Err(ApiError::login_failed(
                "GitHub 不认这次登录的凭据：可能已经用过、或者控制面的 OAuth 配置有误。再试一次；还不行就是我们的问题。",
            ))
        }
        other => {
            tracing::warn!(error = other, description = ?body.error_description, "GitHub 换令牌失败");
            Err(ApiError::login_failed(format!(
                "GitHub 没同意这次登录（{other}）。再试一次。"
            )))
        }
    }
}

/// 拿到 GitHub 的 access token 之后的事：读用户名 → 建/更新用户 → 签我们自己的令牌 →
/// 顺带把请求里那个匿名令牌下的作品归进来。
async fn finish(
    state: &AppState,
    gh: &GitHubApp,
    access_token: &str,
    headers: &HeaderMap,
) -> ApiResult<LoginResponse> {
    #[derive(Deserialize)]
    struct GitHubUser {
        id: i64,
        login: String,
        name: Option<String>,
    }
    let user: GitHubUser = state
        .http()
        .get(format!("{}/user", gh.api_base))
        .bearer_auth(access_token)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(github_unreachable)?
        .json()
        .await
        .map_err(github_unreachable)?;

    let display_name = user
        .name
        .as_deref()
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .unwrap_or(&user.login)
        .to_string();

    let now = clock::now_string();
    let token = auth::new_token();
    let token_hash = hash::hash_bytes(token.as_bytes());
    let anon = anon_caller(state, headers).await;

    let (user_id, adopted) = {
        let mut conn = state.db().lock().await;
        let tx = conn.transaction()?;
        let user_id = db::upsert_github_user(&tx, user.id, &user.login, &display_name, &now)?;
        db::insert_token(&tx, &token_hash, &user_id, &now, None)?;
        let adopted = match &anon {
            Some(anon_id) if *anon_id != user_id => db::adopt_sites(&tx, anon_id, &user_id)?,
            _ => Vec::new(),
        };
        // 清单里的开发者名字和到期时间也要跟着改：门禁页和广场读的是清单，不是库。
        let versions: Vec<(String, Vec<u32>)> = adopted
            .iter()
            .map(|slug| {
                db::list_versions(&tx, slug)
                    .map(|rows| (slug.clone(), rows.into_iter().map(|r| r.version).collect()))
            })
            .collect::<Result<_, _>>()?;
        tx.commit()?;
        (user_id, versions)
    };

    for (slug, versions) in &adopted {
        for version in versions {
            if let Some(mut m) = state.store().get_manifest(slug, *version).await? {
                m.expires_at = None;
                m.developer = display_name.clone();
                state.store().put_manifest(&m).await?;
            }
        }
    }
    if !adopted.is_empty() {
        plaza::publish(state).await;
    }

    tracing::info!(
        user_id = %user_id,
        login = %user.login,
        adopted = adopted.len(),
        "GitHub 登录"
    );
    Ok(LoginResponse {
        token,
        login: user.login,
        display_name,
        migrated_sites: adopted.len() as u32,
    })
}

/// 请求里若带着一个还有效的匿名令牌，返回它的 user_id。带的是别的、或已过期，都当没带。
async fn anon_caller(state: &AppState, headers: &HeaderMap) -> Option<String> {
    let raw = headers.get(AUTHORIZATION)?.to_str().ok()?;
    let (scheme, value) = raw.split_once(' ')?;
    if !scheme.eq_ignore_ascii_case("bearer") || value.trim().is_empty() {
        return None;
    }
    let token_hash = hash::hash_bytes(value.trim().as_bytes());
    let conn = state.db().lock().await;
    let owner = db::find_token_owner(&conn, &token_hash).ok().flatten()?;
    if owner.kind != "anon" {
        return None;
    }
    let expired = owner
        .user_expires_at
        .as_deref()
        .is_some_and(clock::is_expired);
    if expired {
        return None;
    }
    Some(owner.user_id)
}

fn github_unreachable(err: reqwest::Error) -> ApiError {
    tracing::warn!(error = %err, "问 GitHub 失败");
    ApiError::public(
        StatusCode::BAD_GATEWAY,
        playtest_common::api::ErrorCode::LoginFailed,
        "控制面这边连 GitHub 没成功。过一会儿再试；匿名链接不受影响。",
    )
}

fn random_state() -> String {
    let mut bytes = [0u8; 24];
    rand::rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

/// 只给查询参数用的最小编码：字母数字和 `-_.~` 原样，其余按字节转 `%XX`。
fn urlencode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urlencode_keeps_unreserved_and_escapes_the_rest() {
        assert_eq!(urlencode("Ov23abc_-.~"), "Ov23abc_-.~");
        assert_eq!(
            urlencode("https://playtest.roviix.com/console/"),
            "https%3A%2F%2Fplaytest.roviix.com%2Fconsole%2F"
        );
    }

    #[test]
    fn state_nonces_are_url_safe_and_unique() {
        let a = random_state();
        assert_ne!(a, random_state());
        assert!(a
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'));
    }
}
