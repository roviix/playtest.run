//! 玩家写的一句话（DESIGN §3.4）。
//!
//! 反馈**不设任何必填项**——每加一个必填字段填写率就掉一截。所以这里只有一段文字是必需的，
//! 设备、浏览器、第几版、进来多久全部由服务端从这次请求和会话里补，玩家一个字都不用填。
//!
//! 截图是 v0.2（DESIGN §3.4），表里的 `screenshot_hash` 现在永远是空的。

use axum::body::Bytes;
use axum::extract::State;
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::{Extension, Json};
use playtest_common::api::ErrorCode;
use playtest_common::ingest::{self, FeedbackAccepted, FeedbackRequest};
use rusqlite::{params, Connection};
use time::Duration;

use crate::clock;
use crate::error::{ApiError, ApiResult};
use crate::routes::events::{
    check_origin, clean_slug, clip, current_version, header_str, parse, touch_session, with_cors,
    Limiter, Origin, Seen, BAD_SESSION, NO_SUCH_SITE,
};
use crate::state::AppState;

const EMPTY_TEXT: &str = "This feedback is empty. Write a line, then send it.";

/// 一句话反馈和挑战对话的上限不一样，所以数字现说现算，别在文案里写死。
fn enough(max_allowed: u32) -> String {
    format!(
        "We already have {max_allowed} pieces of feedback from you this visit. Thank you — that is enough for now."
    )
}

/// 和 `events.rs` 里的一致：隔这么久再出现算回头客。
const RETURN_AFTER_MINUTES: i64 = 30;

pub async fn submit(
    State(state): State<AppState>,
    Extension(limiter): Extension<Limiter>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let origin = match check_origin(&state, &headers) {
        Origin::Denied => {
            return ApiError::public(
                StatusCode::FORBIDDEN,
                ErrorCode::Invalid,
                "This endpoint only takes feedback from a player page.",
            )
            .into_response()
        }
        other => match other {
            Origin::Allowed(origin) => Some(origin),
            _ => None,
        },
    };
    let done = save(&state, &limiter, &headers, &body).await;
    with_cors(done.into_response(), origin.as_deref())
}

async fn save(
    state: &AppState,
    limiter: &Limiter,
    headers: &HeaderMap,
    body: &Bytes,
) -> ApiResult<Json<FeedbackAccepted>> {
    let request: FeedbackRequest = parse(body)?;
    if !ingest::is_session_id(&request.session) {
        return Err(ApiError::invalid(BAD_SESSION));
    }
    let slug = clean_slug(&request.slug)?;
    if !limiter.take(&request.session, &slug) {
        return Err(ApiError::public(
            StatusCode::TOO_MANY_REQUESTS,
            ErrorCode::QuotaExceeded,
            "That is too fast. Give it a moment.",
        ));
    }

    let text = clip(Some(&request.text), ingest::MAX_FEEDBACK_CHARS)
        .ok_or_else(|| ApiError::invalid(EMPTY_TEXT))?;
    let ua = header_str(headers, header::USER_AGENT.as_str());
    let client = ua.map(ingest::classify_ua);
    let now = clock::now();
    let at = clock::format(now);

    let is_chat =
        request.source.as_deref() == Some("gate") || request.source.as_deref() == Some("chat");
    let max_allowed = if is_chat {
        ingest::MAX_CHAT_PER_SESSION
    } else {
        ingest::MAX_FEEDBACK_PER_SESSION
    };

    let (version, already) = {
        let mut conn = state.db().lock().await;
        let version =
            current_version(&conn, &slug)?.ok_or_else(|| ApiError::not_found(NO_SUCH_SITE))?;

        let tx = conn.transaction()?;
        let already = count_for_session(&tx, &request.session)?;
        if already >= max_allowed {
            return Err(ApiError::public(
                StatusCode::TOO_MANY_REQUESTS,
                ErrorCode::QuotaExceeded,
                enough(max_allowed),
            ));
        }

        touch_session(
            &tx,
            &Seen {
                id: &request.session,
                slug: &slug,
                version,
                at: &at,
                ua,
                referer: None,
                from: None,
                plaza_host: "",
                return_before: &clock::format(now - Duration::minutes(RETURN_AFTER_MINUTES)),
            },
        )?;
        tx.execute(
            "INSERT INTO feedback
                 (session_id, slug, version, ts, text, seconds_in, device, browser, screenshot_hash, status)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, NULL, 'new')",
            params![
                request.session,
                slug,
                version,
                at,
                text,
                request.seconds_in.and_then(sane_seconds),
                client.map(|c| c.device),
                client.map(|c| c.browser),
            ],
        )?;
        tx.commit()?;
        (version, already)
    };

    tracing::info!(slug = %slug, version, "收到一条反馈");
    crate::live::publish(state, &slug).await;

    Ok(Json(FeedbackAccepted {
        remaining: max_allowed.saturating_sub(already + 1),
    }))
}

fn count_for_session(conn: &Connection, session: &str) -> rusqlite::Result<u32> {
    conn.query_row(
        "SELECT COUNT(*) FROM feedback WHERE session_id = ?1",
        params![session],
        |row| row.get(0),
    )
}

/// 「进来多久」是客户端算的。一个玩家在一页上待过一天已经很离谱，超过就当没报。
fn sane_seconds(seconds: u32) -> Option<u32> {
    (seconds <= 24 * 60 * 60).then_some(seconds)
}
