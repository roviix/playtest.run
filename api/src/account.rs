use axum::extract::{Path, Query, Request, State};
use axum::http::{header, HeaderMap, HeaderValue, Method, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::{Extension, Json};
use playtest_common::api::{Me, WebLoginExchange};
use playtest_common::hash;
use rusqlite::{params, Connection, OptionalExtension};
use serde::Deserialize;
use serde_json::json;

use crate::auth::{self, Caller, UserKind};
use crate::error::{ApiError, ApiResult};
use crate::routes::{events::Limiter, follow, login, JsonBody};
use crate::{clock, db, notify, AppState};

const SESSION_SECONDS: u64 = 30 * 24 * 60 * 60;

pub fn cookie_name(secure: bool) -> &'static str {
    if secure {
        "__Host-pt_session"
    } else {
        "pt_session"
    }
}

fn secure(state: &AppState) -> bool {
    state.notify().root_url().starts_with("https://")
}

pub fn read_cookie<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    let mut found = None;
    for value in headers.get_all(header::COOKIE) {
        for part in value.to_str().ok()?.split(';') {
            if let Some((key, value)) = part.trim().split_once('=') {
                if key == name {
                    if found.is_some() {
                        return None;
                    }
                    found = Some(value);
                }
            }
        }
    }
    found
}

fn set_cookie(state: &AppState, name: &str, value: &str, seconds: u64) -> HeaderValue {
    HeaderValue::from_str(&format!(
        "{name}={value}; Path=/; HttpOnly; SameSite=Lax; Max-Age={seconds}{}",
        if secure(state) { "; Secure" } else { "" }
    ))
    .expect("server-generated cookie")
}

pub fn session_owner(conn: &Connection, token: &str) -> rusqlite::Result<Option<Caller>> {
    conn.query_row(
        "SELECT u.id,u.kind,u.display_name,u.login,u.avatar_url,u.expires_at FROM browser_sessions s JOIN users u ON u.id=s.user_id WHERE s.token_hash=?1 AND s.expires_at>?2 AND (u.expires_at IS NULL OR u.expires_at>?2)",
        params![hash::hash_bytes(token.as_bytes()), clock::now_string()],
        |row| Ok(Caller { user_id: row.get(0)?, kind: UserKind::from_db(&row.get::<_, String>(1)?), display_name: row.get(2)?, login: row.get(3)?, avatar_url: row.get(4)?, expires_at: row.get(5)? }),
    ).optional()
}

pub async fn caller(state: &AppState, headers: &HeaderMap) -> ApiResult<Option<Caller>> {
    let conn = state.db().read().await;
    if let Some(token) = read_cookie(headers, cookie_name(secure(state))) {
        if let Some(caller) = session_owner(&conn, token)? {
            return Ok(Some(caller));
        }
    }
    if let Some(token) = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|raw| {
            let (scheme, value) = raw.split_once(' ')?;
            if scheme.eq_ignore_ascii_case("bearer") {
                Some(value.trim())
            } else {
                None
            }
        })
    {
        let token_hash = hash::hash_bytes(token.as_bytes());
        if let Some(owner) = db::find_token_owner(&conn, &token_hash)? {
            let expired = owner
                .token_expires_at
                .as_deref()
                .is_some_and(clock::is_expired)
                || owner
                    .user_expires_at
                    .as_deref()
                    .is_some_and(clock::is_expired);
            if !expired {
                return Ok(Some(Caller {
                    user_id: owner.user_id,
                    kind: UserKind::from_db(&owner.kind),
                    display_name: owner.display_name,
                    login: owner.login,
                    avatar_url: owner.avatar_url,
                    expires_at: owner.user_expires_at,
                }));
            }
        }
    }
    Ok(None)
}

pub fn create_session(conn: &Connection, user_id: &str) -> rusqlite::Result<String> {
    let token = auth::new_token();
    let now = clock::now();
    conn.execute(
        "DELETE FROM browser_sessions WHERE expires_at<=?1",
        params![clock::format(now)],
    )?;
    conn.execute("INSERT INTO browser_sessions(token_hash,user_id,created_at,expires_at) VALUES (?1,?2,?3,?4)",
        params![hash::hash_bytes(token.as_bytes()), user_id, clock::format(now), clock::format(clock::plus_hours(now, SESSION_SECONDS / 3600))])?;
    Ok(token)
}

pub fn player_for_user(conn: &Connection, user_id: &str) -> rusqlite::Result<db::PlayerRow> {
    let existing: Option<String> = conn
        .query_row(
            "SELECT id FROM players WHERE user_id=?1",
            params![user_id],
            |row| row.get(0),
        )
        .optional()?;
    let player_id = match existing {
        Some(id) => id,
        None => {
            let id = uuid::Uuid::new_v4().to_string();
            db::insert_player(
                conn,
                &db::NewPlayer {
                    id: &id,
                    email: None,
                    push_subscription: None,
                    push_endpoint: None,
                    unsubscribe_token: &auth::new_token(),
                    created_at: &clock::now_string(),
                },
            )?;
            conn.execute(
                "UPDATE players SET user_id=?1 WHERE id=?2",
                params![user_id, id],
            )?;
            id
        }
    };
    db::find_player(conn, &player_id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)
}

pub fn user_for_player(conn: &Connection, player_id: &str) -> rusqlite::Result<String> {
    let existing: Option<String> = conn.query_row(
        "SELECT user_id FROM players WHERE id=?1",
        params![player_id],
        |row| row.get(0),
    )?;
    if let Some(id) = existing {
        return Ok(id);
    }
    let id = uuid::Uuid::new_v4().to_string();
    db::insert_user(
        conn,
        &db::NewUser {
            id: &id,
            kind: "email",
            display_name: "新朋友",
            created_at: &clock::now_string(),
            expires_at: None,
        },
    )?;
    conn.execute(
        "UPDATE players SET user_id=?1 WHERE id=?2",
        params![id, player_id],
    )?;
    Ok(id)
}

pub async fn protect(State(state): State<AppState>, request: Request, next: Next) -> Response {
    let writes = !matches!(
        *request.method(),
        Method::GET | Method::HEAD | Method::OPTIONS
    );
    let browser_action = request.uri().path().starts_with("/v1/account/")
        || request.uri().path() == playtest_common::api::routes::LOGIN_WEB_EXCHANGE
        || read_cookie(request.headers(), cookie_name(secure(&state))).is_some();
    if writes
        && browser_action
        && request
            .headers()
            .get(header::ORIGIN)
            .and_then(|value| value.to_str().ok())
            != Some(state.notify().root_url())
    {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({"code":"invalid","message":"请在 playtest 页面完成这个操作。"})),
        )
            .into_response();
    }
    let mut response = next.run(request).await;
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response.headers_mut().insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("same-origin"),
    );
    response
}

fn safe_return(value: &str) -> String {
    if value.starts_with('/')
        && !value.starts_with("//")
        && !value.contains('\\')
        && !value.chars().any(char::is_control)
        && value.len() <= 1000
    {
        value.to_string()
    } else {
        "/".to_string()
    }
}

fn me(identity: &Caller) -> Me {
    Me {
        kind: identity.kind.as_db().to_string(),
        display_name: identity.display_name.clone(),
        login: identity.login.clone(),
        avatar_url: identity.avatar_url.clone(),
        expires_at: identity.expires_at.clone(),
    }
}

pub async fn view(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Json<serde_json::Value>> {
    let identity = caller(&state, &headers).await?;
    let profile = if let Some(ref identity) = identity {
        let conn = state.db().read().await;
        let email: Option<String> = conn
            .query_row(
                "SELECT email FROM players WHERE user_id=?1 AND email_verified_at IS NOT NULL",
                params![identity.user_id],
                |row| row.get(0),
            )
            .optional()?
            .flatten();
        Some(json!({"me":me(identity),"email":email}))
    } else {
        None
    };
    Ok(Json(
        json!({"account":profile,"email_available":state.notify().email_on(),"github_available":state.github().is_some_and(|github|github.client_secret.is_some())}),
    ))
}

pub async fn logout(State(state): State<AppState>, headers: HeaderMap) -> ApiResult<Response> {
    if let Some(token) = read_cookie(&headers, cookie_name(secure(&state))) {
        let conn = state.db().lock().await;
        conn.execute(
            "DELETE FROM browser_sessions WHERE token_hash=?1",
            params![hash::hash_bytes(token.as_bytes())],
        )?;
    }
    let mut response = StatusCode::NO_CONTENT.into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        set_cookie(&state, cookie_name(secure(&state)), "", 0),
    );
    Ok(response)
}

#[derive(Deserialize)]
pub struct EmailStart {
    email: String,
    #[serde(default)]
    return_to: String,
    #[serde(default)]
    link: bool,
}

pub async fn email_start(
    State(state): State<AppState>,
    Extension(limiter): Extension<Limiter>,
    headers: HeaderMap,
    JsonBody(request): JsonBody<EmailStart>,
) -> ApiResult<Json<serde_json::Value>> {
    if !state.notify().email_on() {
        return Err(ApiError::login_unavailable(
            "邮箱登录暂不可用，请使用 GitHub 或稍后重试。",
        ));
    }
    let email = request.email.trim().to_ascii_lowercase();
    if !playtest_common::follow::looks_like_email(&email) {
        return Err(ApiError::invalid("请填写有效的邮箱地址。"));
    }
    follow::spend(&limiter, &headers, Some(&email))?;
    let link_user = if request.link {
        Some(
            caller(&state, &headers)
                .await?
                .ok_or_else(|| ApiError::unauthorized("请先登录再关联邮箱。"))?
                .user_id,
        )
    } else {
        None
    };
    let token = auth::new_token();
    let now = clock::now();
    let mut conn = state.db().lock().await;
    let tx = conn.transaction()?;
    let player_id = follow::find_or_create_by_email(&tx, &email, now)?;
    tx.execute(
        "DELETE FROM account_links WHERE expires_at<=?1",
        params![clock::format(now)],
    )?;
    tx.execute("INSERT INTO account_links(token_hash,player_id,link_user_id,return_to,expires_at) VALUES (?1,?2,?3,?4,?5)", params![hash::hash_bytes(token.as_bytes()), player_id, link_user, safe_return(&request.return_to), clock::format(clock::plus_hours(now,1))])?;
    notify::enqueue_send_link(
        &tx,
        &player_id,
        &format!("{}/console/?email_token={token}", state.notify().root_url()),
        now,
    )?;
    tx.commit()?;
    Ok(Json(
        json!({"message":"登录链接已加入发送队列，请查收邮箱。链接一小时内有效。"}),
    ))
}

#[derive(Deserialize)]
pub struct EmailConfirm {
    token: String,
}

pub async fn email_preview(
    State(state): State<AppState>,
    JsonBody(request): JsonBody<EmailConfirm>,
) -> ApiResult<Json<serde_json::Value>> {
    let conn = state.db().read().await;
    let pending: Option<(String,bool)> = conn.query_row("SELECT p.email,a.link_user_id IS NOT NULL FROM account_links a JOIN players p ON p.id=a.player_id WHERE a.token_hash=?1 AND a.expires_at>?2",params![hash::hash_bytes(request.token.as_bytes()),clock::now_string()],|row|Ok((row.get(0)?,row.get(1)?))).optional()?;
    let (email, link) = pending
        .ok_or_else(|| ApiError::login_failed("链接已使用或已过期，请重新发送登录链接。"))?;
    Ok(Json(
        json!({"email":playtest_common::follow::mask_email(&email),"link":link}),
    ))
}

pub async fn email_confirm(
    State(state): State<AppState>,
    Extension(limiter): Extension<Limiter>,
    headers: HeaderMap,
    JsonBody(request): JsonBody<EmailConfirm>,
) -> ApiResult<Response> {
    follow::spend(&limiter, &headers, None)?;
    let current = caller(&state, &headers).await?;
    let mut conn = state.db().lock().await;
    let tx = conn.transaction()?;
    let token_hash = hash::hash_bytes(request.token.as_bytes());
    let pending: Option<(String,Option<String>,String)> = tx.query_row("SELECT player_id,link_user_id,return_to FROM account_links WHERE token_hash=?1 AND expires_at>?2",params![token_hash,clock::now_string()],|row|Ok((row.get(0)?,row.get(1)?,row.get(2)?))).optional()?;
    let (player_id, link_user, return_to) = pending
        .ok_or_else(|| ApiError::login_failed("链接已使用或已过期，请重新发送登录链接。"))?;
    if let Some(ref user_id) = link_user {
        if current.as_ref().map(|identity| &identity.user_id) != Some(user_id) {
            return Err(ApiError::login_failed("请在发起关联的账号中打开这封邮件。"));
        }
        let owner: Option<String> = tx.query_row(
            "SELECT user_id FROM players WHERE id=?1",
            params![player_id],
            |row| row.get(0),
        )?;
        if owner.as_ref().is_some_and(|owner| owner != user_id) {
            return Err(ApiError::invalid(
                "这个邮箱已属于另一个账号，未合并任何作品或关注。请使用该邮箱登录。",
            ));
        }
        let old_player = player_for_user(&tx, user_id)?;
        if old_player.id != player_id {
            if old_player.email_verified_at.is_some() {
                return Err(ApiError::invalid("账号已关联邮箱，暂不支持替换。"));
            }
            tx.execute("INSERT OR IGNORE INTO follows(player_id,target_kind,target_slug,source,created_at) SELECT ?1,target_kind,target_slug,source,created_at FROM follows WHERE player_id=?2",params![player_id,old_player.id])?;
            tx.execute(
                "DELETE FROM follows WHERE player_id=?1",
                params![old_player.id],
            )?;
            tx.execute(
                "UPDATE players SET user_id=NULL WHERE id=?1",
                params![old_player.id],
            )?;
            tx.execute(
                "UPDATE players SET user_id=?1 WHERE id=?2",
                params![user_id, player_id],
            )?;
        }
    }
    db::confirm_player_email(&tx, &player_id, &clock::now_string())?;
    let user_id = user_for_player(&tx, &player_id)?;
    let token = create_session(&tx, &user_id)?;
    tx.execute(
        "DELETE FROM account_links WHERE token_hash=?1",
        params![token_hash],
    )?;
    tx.commit()?;
    let mut response = Json(json!({"return_to":return_to})).into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        set_cookie(&state, cookie_name(secure(&state)), &token, SESSION_SECONDS),
    );
    Ok(response)
}

#[derive(Deserialize, Default)]
pub struct GithubStart {
    #[serde(default)]
    return_to: String,
    #[serde(default)]
    link: bool,
}

pub async fn github_start(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(request): Query<GithubStart>,
) -> ApiResult<Response> {
    let link_user = if request.link {
        Some(
            caller(&state, &headers)
                .await?
                .ok_or_else(|| ApiError::unauthorized("请先登录再关联 GitHub。"))?
                .user_id,
        )
    } else {
        None
    };
    let mut response = login::web_start(State(state.clone())).await;
    let Some(location) = response
        .headers()
        .get(header::LOCATION)
        .and_then(|value| value.to_str().ok())
    else {
        return Ok(response);
    };
    let url =
        reqwest::Url::parse(location).map_err(|_| ApiError::login_failed("登录地址无效。"))?;
    let nonce = url
        .query_pairs()
        .find(|(key, _)| key == "state")
        .map(|(_, value)| value.into_owned())
        .ok_or_else(|| ApiError::login_failed("登录请求无效。"))?;
    let browser = auth::new_token();
    {
        let conn = state.db().lock().await;
        conn.execute(
            "DELETE FROM browser_oauth WHERE expires_at<=?1",
            params![clock::now_string()],
        )?;
        conn.execute("INSERT INTO browser_oauth(state_hash,browser_hash,link_user_id,return_to,expires_at) VALUES (?1,?2,?3,?4,?5)",params![hash::hash_bytes(nonce.as_bytes()),hash::hash_bytes(browser.as_bytes()),link_user,safe_return(&request.return_to),clock::format(clock::plus_hours(clock::now(),1))])?;
    }
    response.headers_mut().insert(
        header::SET_COOKIE,
        set_cookie(
            &state,
            if secure(&state) {
                "__Host-pt_oauth"
            } else {
                "pt_oauth"
            },
            &browser,
            600,
        ),
    );
    Ok(response)
}

pub async fn github_exchange(
    State(state): State<AppState>,
    headers: HeaderMap,
    JsonBody(request): JsonBody<WebLoginExchange>,
) -> ApiResult<Response> {
    let browser = read_cookie(
        &headers,
        if secure(&state) {
            "__Host-pt_oauth"
        } else {
            "pt_oauth"
        },
    )
    .ok_or_else(|| ApiError::login_failed("请在发起登录的浏览器中重试。"))?;
    let (link_user, return_to) = {
        let conn = state.db().lock().await;
        conn.query_row("DELETE FROM browser_oauth WHERE state_hash=?1 AND browser_hash=?2 AND expires_at>?3 RETURNING link_user_id,return_to",params![hash::hash_bytes(request.state.as_bytes()),hash::hash_bytes(browser.as_bytes()),clock::now_string()],|row|Ok((row.get::<_,Option<String>>(0)?,row.get::<_,String>(1)?))).optional()?.ok_or_else(||ApiError::login_failed("这次登录已失效，请重新开始。"))?
    };
    if link_user.is_some()
        && caller(&state, &headers)
            .await?
            .map(|identity| identity.user_id)
            != link_user
    {
        return Err(ApiError::login_failed("账号已改变，请重新发起关联。"));
    }
    let github = login::web_exchange(State(state.clone()), JsonBody(request)).await?;
    let (token, changed_slugs) = {
        let mut conn = state.db().lock().await;
        let tx = conn.transaction()?;
        let owner: Option<String> = tx
            .query_row(
                "SELECT id FROM users WHERE github_id=?1",
                params![github.id],
                |row| row.get(0),
            )
            .optional()?;
        let avatar = github
            .avatar_url
            .as_deref()
            .filter(|url| url.starts_with("https://"));
        let user_id = if let Some(link_user) = link_user {
            if owner.as_ref().is_some_and(|owner| owner != &link_user) {
                return Err(ApiError::invalid(
                    "这个 GitHub 已属于另一个账号，未合并任何作品或关注。",
                ));
            }
            if owner.is_none() {
                let already: Option<i64> = tx.query_row(
                    "SELECT github_id FROM users WHERE id=?1",
                    params![link_user],
                    |row| row.get(0),
                )?;
                if already.is_some() {
                    return Err(ApiError::invalid("账号已关联 GitHub，暂不支持替换。"));
                }
                tx.execute(
                    "UPDATE users SET github_id=?1,login=?2,avatar_url=?3 WHERE id=?4",
                    params![github.id, github.login, avatar, link_user],
                )?;
            }
            link_user
        } else {
            let display_name = github
                .name
                .as_deref()
                .map(str::trim)
                .filter(|name| !name.is_empty())
                .unwrap_or(&github.login);
            db::upsert_github_user(
                &tx,
                github.id,
                &github.login,
                display_name,
                avatar,
                &clock::now_string(),
            )?
            .0
        };
        let changed_slugs = db::live_slugs_of(&tx, &user_id)?;
        player_for_user(&tx, &user_id)?;
        let token = create_session(&tx, &user_id)?;
        tx.commit()?;
        (token, changed_slugs)
    };
    for slug in &changed_slugs {
        crate::live::publish(&state, slug).await;
    }
    if !changed_slugs.is_empty() {
        crate::plaza::publish(&state).await;
    }
    let mut response = Json(json!({"return_to":return_to})).into_response();
    response.headers_mut().append(
        header::SET_COOKIE,
        set_cookie(&state, cookie_name(secure(&state)), &token, SESSION_SECONDS),
    );
    response.headers_mut().append(
        header::SET_COOKIE,
        set_cookie(
            &state,
            if secure(&state) {
                "__Host-pt_oauth"
            } else {
                "pt_oauth"
            },
            "",
            0,
        ),
    );
    Ok(response)
}

pub async fn create_token(
    State(state): State<AppState>,
    identity: Caller,
) -> ApiResult<Json<serde_json::Value>> {
    if identity.kind.is_anon() {
        return Err(ApiError::invalid("请先登录长期账号。"));
    }
    let token = auth::new_token();
    let conn = state.db().lock().await;
    db::insert_token(
        &conn,
        &hash::hash_bytes(token.as_bytes()),
        &identity.user_id,
        &clock::now_string(),
        None,
    )?;
    Ok(Json(json!({"token":token})))
}

pub async fn tokens(
    State(state): State<AppState>,
    identity: Caller,
) -> ApiResult<Json<serde_json::Value>> {
    let conn = state.db().read().await;
    let mut statement = conn.prepare("SELECT token_hash,created_at,expires_at FROM tokens WHERE user_id=?1 ORDER BY created_at DESC")?;
    let rows = statement.query_map(params![identity.user_id],|row|Ok(json!({"id":row.get::<_,String>(0)?,"created_at":row.get::<_,String>(1)?,"expires_at":row.get::<_,Option<String>>(2)?})))?.collect::<Result<Vec<_>,_>>()?;
    Ok(Json(json!(rows)))
}

pub async fn revoke_token(
    State(state): State<AppState>,
    identity: Caller,
    Path(token_id): Path<String>,
) -> ApiResult<StatusCode> {
    let conn = state.db().lock().await;
    conn.execute(
        "DELETE FROM tokens WHERE user_id=?1 AND token_hash=?2",
        params![identity.user_id, token_id],
    )?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
pub struct Profile {
    display_name: String,
}

pub async fn profile(
    State(state): State<AppState>,
    identity: Caller,
    JsonBody(request): JsonBody<Profile>,
) -> ApiResult<StatusCode> {
    let name = request.display_name.trim();
    if name.is_empty() || name.chars().count() > 40 || name.chars().any(char::is_control) {
        return Err(ApiError::invalid("名字请用 1–40 个字，不含控制字符。"));
    }
    let slugs = {
        let conn = state.db().lock().await;
        conn.execute(
            "UPDATE users SET display_name=?1 WHERE id=?2",
            params![name, identity.user_id],
        )?;
        db::live_slugs_of(&conn, &identity.user_id)?
    };
    for slug in slugs {
        crate::live::publish(&state, &slug).await;
    }
    crate::plaza::publish(&state).await;
    Ok(StatusCode::NO_CONTENT)
}
