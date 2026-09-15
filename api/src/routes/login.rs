use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Redirect, Response};
use axum::Json;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use playtest_common::api::{Me, WebLoginExchange};
use rand::RngCore;
use serde::Deserialize;

use crate::auth::Caller;
use crate::config::GitHubApp;
use crate::error::{ApiError, ApiResult};
use crate::routes::JsonBody;
use crate::state::AppState;

const UNAVAILABLE: &str = "这个控制面没有开 GitHub 登录。匿名链接照常能用，只是 24 小时后失效。";

/// 浏览器直接打开这个地址，被送去 GitHub。回来落在控制台上，带着 `code` 和 `state`。
pub async fn web_start(State(state): State<AppState>) -> Response {
    let Some(gh) = state.github() else {
        return ApiError::login_unavailable(UNAVAILABLE).into_response();
    };
    if gh.client_secret.is_none() {
        return ApiError::login_unavailable(
            "GitHub 登录尚未配置完整，请使用邮箱登录或联系运营者。",
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
    JsonBody(request): JsonBody<WebLoginExchange>,
) -> ApiResult<GitHubUser> {
    let gh = state
        .github()
        .ok_or_else(|| ApiError::login_unavailable(UNAVAILABLE))?;
    let Some(secret) = gh.client_secret.as_deref() else {
        return Err(ApiError::login_unavailable(
            "GitHub 登录尚未配置完整，请使用邮箱登录。",
        ));
    };
    if !state.take_login_state(&request.state) {
        return Err(ApiError::login_failed(
            "这次登录的凭据对不上或已过期（从点「登录」到回来超过了 10 分钟）。回到控制台再点一次。",
        ));
    }
    let access_token = access_token(
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
    state
        .http()
        .get(format!("{}/user", gh.api_base))
        .bearer_auth(access_token)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .and_then(|response| response.error_for_status())
        .map_err(github_unreachable)?
        .json()
        .await
        .map_err(github_unreachable)
}

#[derive(Deserialize)]
pub struct GitHubUser {
    pub id: i64,
    pub login: String,
    pub name: Option<String>,
    pub avatar_url: Option<String>,
}

// ---------------------------------------------------------------- 我是谁

pub async fn me(caller: Caller) -> Json<Me> {
    Json(Me {
        kind: caller.kind.as_db().to_string(),
        display_name: caller.display_name,
        login: caller.login,
        expires_at: caller.expires_at,
        avatar_url: caller.avatar_url,
    })
}

// ---------------------------------------------------------------- 共用

/// `POST /login/oauth/access_token`。GitHub 用 200 + `error` 字段表示各种没成，不用状态码。
/// 请求体用 JSON（GitHub 两种都收），省得再拖一个表单编码的依赖。
async fn access_token(
    state: &AppState,
    gh: &GitHubApp,
    form: &[(&str, &str)],
) -> ApiResult<String> {
    let form: serde_json::Map<String, serde_json::Value> = form
        .iter()
        .map(|(k, v)| (k.to_string(), serde_json::Value::String(v.to_string())))
        .collect();
    #[derive(Deserialize)]
    struct Body {
        access_token: Option<String>,
        error: Option<String>,
        error_description: Option<String>,
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
        return Ok(token);
    }
    tracing::warn!(error = ?body.error, description = ?body.error_description, "GitHub 拒绝授权码");
    Err(ApiError::login_failed(
        "GitHub 授权未完成或已过期，请重新登录。",
    ))
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
            urlencode("https://playtest.run/console/"),
            "https%3A%2F%2Fplaytest.run%2Fconsole%2F"
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
