//! 管理接口（DESIGN §4.5、§3.11）：推广的赠送与放行、手工撤下、通知队列。
//!
//! 只有一个运营者（就是创始人），所以这里没有角色、没有审计表，只有一个环境变量里的令牌。
//! **令牌没配这一组路由就不注册**——不是「配了才校验」，是根本没有这几个地址，
//! 外面探测到的是 404 而不是 401，也就看不出这台机器有没有管理接口。
//!
//! 这些地址只在开发者域上（api 本身就是开发者域，玩家域是边缘，两个进程）。

use axum::extract::FromRequestParts;
use axum::extract::{Path, State};
use axum::http::{header::AUTHORIZATION, request::Parts, StatusCode};
use axum::Json;
use playtest_common::boost::{
    Boost, BoostStatus, GrantBoostRequest, JobStatus, NotificationQueue, ReviewBoostRequest,
};

use crate::boosts;
use crate::clock;
use crate::db;
use crate::error::{ApiError, ApiResult};
use crate::routes::JsonBody;
use crate::state::AppState;

const BAD_TOKEN: &str = "管理令牌不对。";
const NO_SUCH_BOOST: &str = "没有这一条推广。";
const NO_SUCH_SITE: &str = "没有这个作品。";
const ANON_CANNOT_BOOST: &str =
    "匿名作品不能推广：匿名链接 24 小时就到期，推广位上会留下一个打不开的作品。让作者先用 GitHub 登录。";
/// 手工撤下时写进 `hidden_reason`，控制台照着它告诉开发者。
const HIDDEN_BY_HAND: &str = "我们看过之后从广场撤下了，作品链接照常能开";

/// 带对了管理令牌。提取器放在这里而不是 `auth.rs`：它和开发者令牌不是一套东西，
/// 混在一起早晚有人把 `Caller` 当成管理员。
pub struct Admin;

impl FromRequestParts<AppState> for Admin {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let expected = state
            .admin_token()
            // 走到这里说明路由注册了，也就一定配了令牌；防御性地当成没配。
            .ok_or_else(|| ApiError::unauthorized(BAD_TOKEN))?;
        let given = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .map(str::trim)
            .unwrap_or_default();
        if !given.is_empty() && given == expected {
            Ok(Admin)
        } else {
            Err(ApiError::unauthorized(BAD_TOKEN))
        }
    }
}

/// `GET /admin/boosts`
pub async fn list(State(state): State<AppState>, _: Admin) -> ApiResult<Json<Vec<Boost>>> {
    let conn = state.db().read().await;
    Ok(Json(
        db::list_boosts(&conn)?
            .into_iter()
            .map(boosts::to_boost)
            .collect(),
    ))
}

/// `POST /admin/boosts`
pub async fn grant(
    State(state): State<AppState>,
    _: Admin,
    JsonBody(request): JsonBody<GrantBoostRequest>,
) -> ApiResult<Json<Boost>> {
    let now = clock::now();
    let requested = match request.starts_at.as_deref() {
        Some(raw) => Some(clock::parse(raw).ok_or_else(|| {
            ApiError::invalid(format!(
                "开始时间要写成 2026-09-14T01:00:00Z 这样，现在是「{raw}」。"
            ))
        })?),
        None => None,
    };

    let boost = {
        let conn = state.db().lock().await;
        let owner = db::site_owner(&conn, &request.slug)?
            .ok_or_else(|| ApiError::not_found(NO_SUCH_SITE))?;
        if owner.kind != "github" {
            return Err(ApiError::invalid(ANON_CANNOT_BOOST));
        }
        let window = boosts::window(&conn, request.kind, requested, now)?;
        // 赠送的默认不用再审一遍：运营者自己就是审的那个人（契约里 `review` 的注释）。
        let status = if request.review {
            BoostStatus::Pending
        } else if window.starts_at <= now {
            BoostStatus::Live
        } else {
            BoostStatus::Pending
        };
        let id = db::insert_boost(
            &conn,
            &request.slug,
            boosts::kind_as_db(request.kind),
            boosts::status_as_db(status),
            true,
            &clock::format(window.starts_at),
            Some(&clock::format(window.ends_at)),
            &clock::format(now),
        )?;
        db::find_boost(&conn, id)?
            .map(boosts::to_boost)
            .ok_or_else(|| ApiError::not_found(NO_SUCH_BOOST))?
    };

    tracing::info!(id = boost.id, slug = %boost.slug, "赠送了一段推广");
    crate::plaza::publish(&state).await;
    Ok(Json(boost))
}

/// `POST /admin/boosts/{id}/review`
pub async fn review(
    State(state): State<AppState>,
    _: Admin,
    Path(id): Path<i64>,
    JsonBody(request): JsonBody<ReviewBoostRequest>,
) -> ApiResult<Json<Boost>> {
    let now = clock::now();
    let boost = {
        let conn = state.db().lock().await;
        let row = db::find_boost(&conn, id)?.ok_or_else(|| ApiError::not_found(NO_SUCH_BOOST))?;
        let status = if !request.approve {
            BoostStatus::Rejected
        } else if clock::parse(&row.starts_at).is_some_and(|t| t <= now) {
            BoostStatus::Live
        } else {
            BoostStatus::Pending
        };
        db::update_boost(
            &conn,
            id,
            boosts::status_as_db(status),
            None,
            None,
            request.reason.as_deref(),
        )?;
        db::find_boost(&conn, id)?
            .map(boosts::to_boost)
            .ok_or_else(|| ApiError::not_found(NO_SUCH_BOOST))?
    };
    crate::plaza::publish(&state).await;
    Ok(Json(boost))
}

/// `DELETE /admin/boosts/{id}`：提前结束。
pub async fn end(
    State(state): State<AppState>,
    _: Admin,
    Path(id): Path<i64>,
) -> ApiResult<StatusCode> {
    {
        let conn = state.db().lock().await;
        if db::find_boost(&conn, id)?.is_none() {
            return Err(ApiError::not_found(NO_SUCH_BOOST));
        }
        db::update_boost(
            &conn,
            id,
            boosts::status_as_db(BoostStatus::Ended),
            None,
            Some(&clock::now_string()),
            None,
        )?;
    }
    crate::plaza::publish(&state).await;
    Ok(StatusCode::NO_CONTENT)
}

/// `POST /admin/plaza/{slug}/hide`：手工撤下。作品链接照常能开。
pub async fn hide(
    State(state): State<AppState>,
    _: Admin,
    Path(slug): Path<String>,
) -> ApiResult<StatusCode> {
    {
        let now = clock::now_string();
        let conn = state.db().lock().await;
        if db::find_site(&conn, &slug)?.is_none() {
            return Err(ApiError::not_found(NO_SUCH_SITE));
        }
        db::hide_from_plaza(&conn, &slug, &now, HIDDEN_BY_HAND)?;
        // 撤下了就不该还在卖：收了钱的那一段一起结束（DESIGN §3.11）。
        db::end_boosts_of(&conn, &slug, &now)?;
    }
    crate::plaza::publish(&state).await;
    crate::live::publish(&state, &slug).await;
    Ok(StatusCode::NO_CONTENT)
}

/// `DELETE /admin/plaza/{slug}/hide`：复核之后恢复。
pub async fn unhide(
    State(state): State<AppState>,
    _: Admin,
    Path(slug): Path<String>,
) -> ApiResult<StatusCode> {
    {
        let conn = state.db().lock().await;
        if db::find_site(&conn, &slug)?.is_none() {
            return Err(ApiError::not_found(NO_SUCH_SITE));
        }
        db::unhide_from_plaza(&conn, &slug, &clock::now_string())?;
    }
    crate::plaza::publish(&state).await;
    crate::live::publish(&state, &slug).await;
    Ok(StatusCode::NO_CONTENT)
}

/// `GET /admin/notifications`
/// `GET /admin/jobs`：后台任务板。
pub async fn jobs(State(state): State<AppState>, _: Admin) -> Json<Vec<JobStatus>> {
    Json(state.jobs().snapshot())
}

pub async fn notifications(
    State(state): State<AppState>,
    _: Admin,
) -> ApiResult<Json<NotificationQueue>> {
    let now = clock::now();
    let since = clock::format(now - time::Duration::hours(24));
    let counts = {
        let conn = state.db().read().await;
        db::queue_counts(&conn, &since)?
    };
    Ok(Json(NotificationQueue {
        pending: counts.pending,
        sent_24h: counts.sent_24h,
        failed_24h: counts.failed_24h,
        dead: counts.dead,
        next_digest_at: Some(clock::format(crate::notify::digest::next_run_at(now))),
    }))
}
