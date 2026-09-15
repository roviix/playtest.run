use axum::extract::State;
use axum::http::{header, HeaderMap};
use axum::{Extension, Json};
use playtest_common::api::{DeviceLoginPoll, DeviceLoginStart, LoginPollResponse, LoginResponse};
use playtest_common::hash;
use rand::Rng;
use rusqlite::{params, OptionalExtension};
use serde::Deserialize;
use serde_json::json;

use crate::auth::Caller;
use crate::error::{ApiError, ApiResult};
use crate::routes::{events::Limiter, follow, JsonBody};
use crate::{auth, clock, db, AppState};

pub async fn start(
    State(state): State<AppState>,
    Extension(limiter): Extension<Limiter>,
    headers: HeaderMap,
) -> ApiResult<Json<DeviceLoginStart>> {
    if !state.notify().email_on()
        && state
            .github()
            .is_none_or(|github| github.client_secret.is_none())
    {
        return Err(ApiError::login_unavailable(
            "登录暂不可用，匿名作品仍可分享。请稍后重试。",
        ));
    }
    follow::spend(&limiter, &headers, None)?;
    let device_code = auth::new_token();
    let alphabet = b"BCDFGHJKLMNPQRSTVWXZ";
    let code: String = (0..8)
        .map(|_| alphabet[rand::rng().random_range(0..alphabet.len())] as char)
        .collect();
    let user_code = format!("{}-{}", &code[..4], &code[4..]);
    let now = clock::now();
    let conn = state.db().lock().await;
    let recent: i64 = conn.query_row(
        "SELECT count(*) FROM device_authorizations WHERE created_at>?1",
        params![clock::format(now - time::Duration::minutes(1))],
        |row| row.get(0),
    )?;
    if recent >= 60 {
        return Err(ApiError::quota("正在登录的设备太多，请稍后重试。"));
    }
    conn.execute(
        "DELETE FROM device_authorizations WHERE expires_at<=?1",
        params![clock::format(now)],
    )?;
    let anon = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .and_then(|token| {
            db::find_token_owner(&conn, &hash::hash_bytes(token.as_bytes()))
                .ok()
                .flatten()
        })
        .filter(|owner| {
            owner.kind == "anon"
                && !owner
                    .user_expires_at
                    .as_deref()
                    .is_some_and(clock::is_expired)
                && !owner
                    .token_expires_at
                    .as_deref()
                    .is_some_and(clock::is_expired)
        })
        .map(|owner| owner.user_id);
    conn.execute("INSERT INTO device_authorizations(device_hash,user_code_hash,anon_user_id,created_at,expires_at) VALUES (?1,?2,?3,?4,?5)",params![hash::hash_bytes(device_code.as_bytes()),hash::hash_bytes(code.as_bytes()),anon,clock::format(now),clock::format(now+time::Duration::minutes(10))])?;
    Ok(Json(DeviceLoginStart {
        device_code,
        user_code,
        verification_uri: format!("{}/console/#/device", state.notify().root_url()),
        expires_in: 600,
        interval: 5,
    }))
}

#[derive(Deserialize)]
pub struct Approval {
    user_code: String,
}

fn code_hash(code: &str) -> String {
    hash::hash_bytes(code.trim().replace('-', "").to_ascii_uppercase().as_bytes())
}

pub async fn preview(
    State(state): State<AppState>,
    Extension(limiter): Extension<Limiter>,
    headers: HeaderMap,
    _identity: Caller,
    JsonBody(request): JsonBody<Approval>,
) -> ApiResult<Json<serde_json::Value>> {
    follow::spend(&limiter, &headers, None)?;
    let conn = state.db().read().await;
    let pending: Option<(String,i64)> = conn.query_row("SELECT d.expires_at,(SELECT count(*) FROM sites s WHERE s.user_id=d.anon_user_id AND s.deleted_at IS NULL) FROM device_authorizations d WHERE d.user_code_hash=?1 AND d.user_id IS NULL AND d.expires_at>?2",params![code_hash(&request.user_code),clock::now_string()],|row|Ok((row.get(0)?,row.get(1)?))).optional()?;
    let (expires_at, anonymous_works) = pending.ok_or_else(|| {
        ApiError::invalid("没有找到这次请求，确认终端里的代码，或重新运行 playtest login。")
    })?;
    Ok(Json(
        json!({"expires_at":expires_at,"anonymous_works":anonymous_works}),
    ))
}

pub async fn approve(
    State(state): State<AppState>,
    Extension(limiter): Extension<Limiter>,
    headers: HeaderMap,
    identity: Caller,
    JsonBody(request): JsonBody<Approval>,
) -> ApiResult<Json<serde_json::Value>> {
    if identity.kind.is_anon() {
        return Err(ApiError::unauthorized("请先登录长期账号。"));
    }
    follow::spend(&limiter, &headers, None)?;
    let conn = state.db().lock().await;
    let changed = conn.execute("UPDATE device_authorizations SET user_id=?1 WHERE user_code_hash=?2 AND user_id IS NULL AND expires_at>?3",params![identity.user_id,code_hash(&request.user_code),clock::now_string()])?;
    if changed != 1 {
        return Err(ApiError::invalid("这次请求已处理或过期，请回终端重试。"));
    }
    Ok(Json(json!({"message":"已授权，回到终端继续。"})))
}

pub async fn poll(
    State(state): State<AppState>,
    Extension(limiter): Extension<Limiter>,
    headers: HeaderMap,
    JsonBody(request): JsonBody<DeviceLoginPoll>,
) -> ApiResult<Json<LoginPollResponse>> {
    follow::spend(&limiter, &headers, None)?;
    let (response, adopted) = {
        let mut conn = state.db().lock().await;
        let tx = conn.transaction()?;
        let device_hash = hash::hash_bytes(request.device_code.as_bytes());
        let row: Option<(Option<String>,Option<String>,Option<String>)> = tx.query_row("SELECT user_id,anon_user_id,last_poll_at FROM device_authorizations WHERE device_hash=?1 AND expires_at>?2",params![device_hash,clock::now_string()],|row|Ok((row.get(0)?,row.get(1)?,row.get(2)?))).optional()?;
        let (user_id, anonymous, last_poll) = row.ok_or_else(|| {
            ApiError::login_failed("登录请求已过期或已完成，请重新运行 playtest login。")
        })?;
        let cutoff = clock::format(clock::now() - time::Duration::seconds(5));
        if last_poll.is_some_and(|last| last > cutoff) {
            return Ok(Json(LoginPollResponse::Pending { interval: 5 }));
        }
        tx.execute(
            "UPDATE device_authorizations SET last_poll_at=?1 WHERE device_hash=?2",
            params![clock::now_string(), device_hash],
        )?;
        let Some(user_id) = user_id else {
            tx.commit()?;
            return Ok(Json(LoginPollResponse::Pending { interval: 5 }));
        };
        let (name, login): (String, Option<String>) = tx.query_row(
            "SELECT display_name,login FROM users WHERE id=?1",
            params![user_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        let adopted = if let Some(anonymous) = anonymous {
            db::adopt_sites(&tx, &anonymous, &user_id)?
        } else {
            Vec::new()
        };
        let token = auth::new_token();
        db::insert_token(
            &tx,
            &hash::hash_bytes(token.as_bytes()),
            &user_id,
            &clock::now_string(),
            None,
        )?;
        tx.execute(
            "DELETE FROM device_authorizations WHERE device_hash=?1",
            params![device_hash],
        )?;
        tx.commit()?;
        (
            LoginResponse {
                token,
                login: login.unwrap_or_else(|| name.clone()),
                display_name: name,
                migrated_sites: adopted.len() as u32,
            },
            adopted,
        )
    };
    for slug in &adopted {
        crate::quota::publish(&state, slug).await?;
        let versions = {
            let conn = state.db().read().await;
            db::list_versions(&conn, slug)?
        };
        for version in versions {
            if let Some(mut manifest) = state.store().get_manifest(slug, version.version).await? {
                manifest.expires_at = None;
                manifest.developer = response.display_name.clone();
                state.store().put_manifest(&manifest).await?;
            }
        }
        crate::live::publish(&state, slug).await;
    }
    if !adopted.is_empty() {
        crate::plaza::publish(&state).await;
    }
    Ok(Json(LoginPollResponse::Ok(response)))
}
