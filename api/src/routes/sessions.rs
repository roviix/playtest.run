//! 匿名会话：不登录也能拿到一个 24 小时的链接（DESIGN §3.2）。

use axum::extract::State;
use axum::http::{header, HeaderMap, StatusCode};
use axum::Json;
use playtest_common::api::{AnonSessionResponse, ErrorCode};
use playtest_common::hash;
use playtest_common::ANON_LINK_TTL_HOURS;
use uuid::Uuid;

use crate::auth::{self, UserKind, ANON_DISPLAY_NAME};
use crate::clock;
use crate::db::{self, NewUser};
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

/// 当前单节点公开试用的全平台保险丝。它限制匿名身份的总生成速度，不冒充按来源反滥用；
/// 反向代理能提供可信来源信息后，还要在外层增加逐来源限制。
const ANON_SESSIONS_PER_MINUTE: u64 = 60;

pub async fn revoke(
    State(state): State<AppState>,
    caller: crate::auth::Caller,
    headers: HeaderMap,
) -> ApiResult<StatusCode> {
    let token = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split_once(' '))
        .map(|(_, token)| token.trim())
        .ok_or_else(|| crate::error::ApiError::unauthorized("This request carried no token."))?;
    let connection = state.db().lock().await;
    connection.execute(
        "DELETE FROM tokens WHERE token_hash=?1 AND user_id=?2",
        rusqlite::params![hash::hash_bytes(token.as_bytes()), caller.user_id],
    )?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn create(State(state): State<AppState>) -> ApiResult<Json<AnonSessionResponse>> {
    let now = clock::now();
    let created_at = clock::format(now);
    let expires_at = clock::format(clock::plus_hours(now, ANON_LINK_TTL_HOURS));

    let user_id = Uuid::new_v4().to_string();
    let token = auth::new_token();
    let token_hash = hash::hash_bytes(token.as_bytes());

    {
        let mut conn = state.db().lock().await;
        let tx = conn.transaction()?;
        let since = clock::format(now - time::Duration::minutes(1));
        if db::count_anon_users_since(&tx, &since)? >= ANON_SESSIONS_PER_MINUTE {
            return Err(ApiError::public(
                StatusCode::TOO_MANY_REQUESTS,
                ErrorCode::QuotaExceeded,
                "Too many anonymous links were created this minute. Try again shortly.",
            ));
        }
        db::insert_user(
            &tx,
            &NewUser {
                id: &user_id,
                kind: UserKind::Anon.as_db(),
                display_name: ANON_DISPLAY_NAME,
                created_at: &created_at,
                expires_at: Some(&expires_at),
            },
        )?;
        db::insert_token(&tx, &token_hash, &user_id, &created_at, Some(&expires_at))?;
        tx.commit()?;
    }

    tracing::info!(user_id = %user_id, %expires_at, "签发了一个匿名令牌");
    Ok(Json(AnonSessionResponse { token, expires_at }))
}
