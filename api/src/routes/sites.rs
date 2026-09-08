//! 作品：建、列、看、删。一个 slug 就是一个作品，永远指向它最新的版本（DESIGN §3.1）。

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use playtest_common::api::{CreateSiteRequest, Site};
use playtest_common::slug::{self, ADJECTIVES, ANIMALS};
use rand::Rng;
use rusqlite::Connection;

use crate::auth::Caller;
use crate::clock;
use crate::db::{self, SiteRow};
use crate::error::{ApiError, ApiResult};
use crate::routes::uploads::clean_title;
use crate::routes::JsonBody;
use crate::state::AppState;

/// 匿名用户同时能有几个作品（DESIGN §6 免费档 3 个活跃 slug）。
pub const ANON_MAX_SITES: u32 = 3;

/// 随机名字最多抽几次。抽不到说明词表用完了，那是我们要加词，不是用户的错。
const SLUG_ATTEMPTS: usize = 20;

pub const NO_SUCH_SITE: &str = "没有这个作品，或者它不是你的。用 playtest ls 看看你有哪些作品。";

pub async fn create(
    State(state): State<AppState>,
    caller: Caller,
    JsonBody(request): JsonBody<CreateSiteRequest>,
) -> ApiResult<Json<Site>> {
    let title = clean_title(request.title.as_deref())?;
    let created_at = clock::now_string();

    let row = {
        let conn = state.db().lock().await;

        if caller.kind.is_anon() {
            let live = db::count_live_sites(&conn, &caller.user_id)?;
            if live >= ANON_MAX_SITES {
                return Err(ApiError::quota(format!(
                    "匿名链接最多同时留 {ANON_MAX_SITES} 个作品，你已经有 {live} 个了。先用 playtest rm 删掉一个再建。"
                )));
            }
        }

        // 匿名用户不能挑名字：好名字是登录之后的事（DESIGN §3.1），
        // 而且随机名字让别人猜不到你正在测的链接。
        let slug = if caller.kind.is_anon() {
            random_slug(&conn)?
        } else {
            match request.slug.as_deref() {
                Some(wanted) => claim_slug(&conn, wanted)?,
                None => random_slug(&conn)?,
            }
        };

        let title = title.unwrap_or_else(|| slug.clone());
        db::insert_site(
            &conn,
            &slug,
            &caller.user_id,
            &title,
            &created_at,
            caller.expires_at.as_deref(),
        )?;

        SiteRow {
            slug,
            title,
            created_at,
            expires_at: caller.expires_at.clone(),
            current_version: None,
        }
    };

    tracing::info!(slug = %row.slug, user_id = %caller.user_id, "建了一个作品");
    Ok(Json(to_site(&state, row)))
}

pub async fn list(
    State(state): State<AppState>,
    caller: Caller,
) -> ApiResult<Json<Vec<Site>>> {
    let rows = {
        let conn = state.db().lock().await;
        db::list_live_sites(&conn, &caller.user_id)?
    };
    Ok(Json(rows.into_iter().map(|r| to_site(&state, r)).collect()))
}

pub async fn show(
    State(state): State<AppState>,
    caller: Caller,
    Path(slug): Path<String>,
) -> ApiResult<Json<Site>> {
    let row = {
        let conn = state.db().lock().await;
        db::find_live_site(&conn, &slug, &caller.user_id)?
    };
    let row = row.ok_or_else(|| ApiError::not_found(NO_SUCH_SITE))?;
    Ok(Json(to_site(&state, row)))
}

pub async fn remove(
    State(state): State<AppState>,
    caller: Caller,
    Path(slug): Path<String>,
) -> ApiResult<StatusCode> {
    {
        let conn = state.db().lock().await;
        if db::find_live_site(&conn, &slug, &caller.user_id)?.is_none() {
            return Err(ApiError::not_found(NO_SUCH_SITE));
        }
    }

    // 先让链接失效，再改库：反过来的话中途失败会留下一个「列表里没有、点开还能玩」的作品。
    state.store().remove_site(&slug).await?;
    {
        let conn = state.db().lock().await;
        db::mark_site_deleted(&conn, &slug, &clock::now_string())?;
    }

    tracing::info!(%slug, user_id = %caller.user_id, "删了一个作品");
    Ok(StatusCode::NO_CONTENT)
}

pub fn to_site(state: &AppState, row: SiteRow) -> Site {
    Site {
        url: state.site_url(&row.slug),
        slug: row.slug,
        title: row.title,
        current_version: row.current_version,
        created_at: row.created_at,
        expires_at: row.expires_at,
    }
}

/// `形容词-动物-两位数`，撞了重抽（DESIGN §3.1）。
fn random_slug(conn: &Connection) -> ApiResult<String> {
    for _ in 0..SLUG_ATTEMPTS {
        let candidate = {
            let mut rng = rand::rng();
            let adjective = ADJECTIVES[rng.random_range(0..ADJECTIVES.len())];
            let animal = ANIMALS[rng.random_range(0..ANIMALS.len())];
            let number = rng.random_range(10..100);
            format!("{adjective}-{animal}-{number}")
        };
        if slug::validate(&candidate).is_err() {
            continue;
        }
        if !db::slug_taken(conn, &candidate)? {
            return Ok(candidate);
        }
    }
    Err(ApiError::Internal(anyhow::anyhow!(
        "连抽 {SLUG_ATTEMPTS} 次随机名字都被占了，词表该扩了"
    )))
}

/// 用户自己挑的名字。v0.1 还没有登录入口，所以这条路暂时走不到。
fn claim_slug(conn: &Connection, wanted: &str) -> ApiResult<String> {
    slug::validate(wanted).map_err(|err| ApiError::invalid(err.to_string()))?;
    if db::slug_taken(conn, wanted)? {
        return Err(ApiError::slug_unavailable(format!(
            "「{wanted}」已经有人用了，换一个。"
        )));
    }
    Ok(wanted.to_string())
}
