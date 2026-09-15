//! 作品：建、列、看、改广场状态、删。一个 slug 就是一个作品，永远指向它最新的版本（DESIGN §3.1）。

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use playtest_common::api::{CreateSiteRequest, Site, UpdateSiteRequest};
use playtest_common::limits;
use playtest_common::slug::{self, ADJECTIVES, ANIMALS};
use rand::Rng;
use rusqlite::Connection;

use crate::auth::Caller;
use crate::clock;
use crate::db::{self, ListingRow, SiteRow};
use crate::error::{ApiError, ApiResult};
use crate::plaza;
use crate::routes::uploads::{clean_summary, clean_text, clean_title};
use crate::routes::JsonBody;
use crate::state::AppState;
use crate::words;

pub const ANON_MAX_SITES: u32 = playtest_common::plan::Plan::Anon.limits().active_projects;
pub const LOGGED_IN_MAX_SITES: u32 = playtest_common::plan::Plan::Free.limits().active_projects;

/// 随机名字最多抽几次。抽不到说明词表用完了，那是我们要加词，不是用户的错。
const SLUG_ATTEMPTS: usize = 20;

pub const NO_SUCH_SITE: &str =
    "No such project, or it is not yours. Run playtest ls to see what you have.";

pub async fn create(
    State(state): State<AppState>,
    caller: Caller,
    JsonBody(request): JsonBody<CreateSiteRequest>,
) -> ApiResult<Json<Site>> {
    let title = clean_title(request.title.as_deref())?;
    let created_at = clock::now_string();

    let row = {
        let mut connection = state.db().lock().await;
        let conn = connection.transaction()?;

        let live = db::count_live_sites(&conn, &caller.user_id)?;
        if caller.kind.is_anon() {
            if live >= ANON_MAX_SITES {
                return Err(ApiError::quota(format!(
                    "An anonymous link holds {} at a time and you already have {live}. Remove one with playtest rm, or run playtest login to hold {}.",
                    words::count(u64::from(ANON_MAX_SITES), "project"),
                    words::count(u64::from(LOGGED_IN_MAX_SITES), "project"),
                )));
            }
        } else if live >= LOGGED_IN_MAX_SITES {
            return Err(ApiError::quota(format!(
                "An account holds {} at a time and you already have {live}. Remove one with playtest rm before making another.",
                words::count(u64::from(LOGGED_IN_MAX_SITES), "project"),
            )));
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

        conn.commit()?;
        SiteRow {
            slug,
            title,
            created_at,
            expires_at: caller.expires_at.clone(),
            current_version: None,
            work_kind: "web".into(),
            listing: ListingRow::default(),
        }
    };

    // SQLite 事务不能跨对象存储 await（rusqlite 的事务也不是 Send）。策略发布失败时把刚建的
    // 空作品补偿删除；否则 CLI 重试会看见一个自己从未拿到成功响应的幽灵作品。
    if let Err(error) = state
        .store()
        .put_policy(
            &row.slug,
            &playtest_common::quota::Policy {
                owner: caller.user_id.clone(),
                plan: if caller.kind.is_anon() {
                    playtest_common::plan::Plan::Anon
                } else {
                    playtest_common::plan::Plan::Free
                },
                expires_at: caller.expires_at.clone(),
            },
        )
        .await
    {
        let now = clock::now_string();
        let cleanup = {
            let conn = state.db().lock().await;
            db::mark_site_deleted(&conn, &row.slug, &now)
        };
        if let Err(cleanup) = cleanup {
            tracing::error!(slug = %row.slug, %cleanup, "额度策略发布失败后，空作品也没能补偿删除");
        }
        return Err(error.into());
    }

    tracing::info!(slug = %row.slug, user_id = %caller.user_id, "建了一个作品");
    // 刚建出来的作品什么都还没有：没人留名、没人关注、没有推广。
    Ok(Json(to_site(&state, row, Club::default())))
}

pub async fn list(State(state): State<AppState>, caller: Caller) -> ApiResult<Json<Vec<Site>>> {
    let sites = {
        let conn = state.db().read().await;
        db::list_live_sites(&conn, &caller.user_id)?
            .into_iter()
            .map(|row| {
                let club = club_of(&conn, &row.slug)?;
                Ok(to_site(&state, row, club))
            })
            .collect::<ApiResult<Vec<_>>>()?
    };
    Ok(Json(sites))
}

pub async fn show(
    State(state): State<AppState>,
    caller: Caller,
    Path(slug): Path<String>,
) -> ApiResult<Json<Site>> {
    let row = {
        let conn = state.db().read().await;
        db::find_live_site(&conn, &slug, &caller.user_id)?
    };
    let row = row.ok_or_else(|| ApiError::not_found(NO_SUCH_SITE))?;
    Ok(Json(load_site(&state, row).await?))
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
    // `remove_site` 删的是整个 `sites/<slug>/` 目录，`live.json` 跟着一起没了。
    state.store().remove_site(&slug).await?;
    {
        let now = clock::now_string();
        let conn = state.db().lock().await;
        db::mark_site_deleted(&conn, &slug, &now)?;
        // 作品没了，它在推广位上的那一段也就结束了。
        db::end_boosts_of(&conn, &slug, &now)?;
    }
    // 删掉的作品不能还挂在广场上。
    plaza::publish(&state).await;

    tracing::info!(%slug, user_id = %caller.user_id, "删了一个作品");
    Ok(StatusCode::NO_CONTENT)
}

/// `PATCH /v1/projects/{slug}`：广场上的状态与俱乐部那几项设置（DESIGN §3.8、§3.3、§3.5）。
/// 只改带了的字段；`seats` 传 0、`seek_note` / `community_url` 传空串都是「清掉」。
pub async fn update(
    State(state): State<AppState>,
    caller: Caller,
    Path(slug): Path<String>,
    JsonBody(request): JsonBody<UpdateSiteRequest>,
) -> ApiResult<Json<Site>> {
    // 空字符串是「清掉」；有内容就检查长度。
    let title = clean_title(request.title.as_deref())?;
    let summary: Option<Option<String>> = match request.summary.as_deref() {
        None => None,
        Some(raw) => Some(clean_summary(Some(raw))?),
    };
    let seek_note: Option<Option<String>> = match request.seek_note.as_deref() {
        None => None,
        Some(raw) => Some(clean_text(
            Some(raw),
            limits::MAX_SEEK_NOTE_CHARS,
            "What you want testers to look at",
        )?),
    };
    let seats = clean_seats(request.seats)?;
    let community_url: Option<Option<String>> = match request.community_url.as_deref() {
        None => None,
        Some(raw) => Some(clean_community_url(raw)?),
    };

    let row = {
        let conn = state.db().lock().await;
        let site = db::find_live_site(&conn, &slug, &caller.user_id)?
            .ok_or_else(|| ApiError::not_found(NO_SUCH_SITE))?;

        // 设了名额就是在找人测（DESIGN §3.3 第 4 条：名额那一行只在门禁页上对求测的作品有意义）。
        // 明确写了 seeking 的以他写的为准——他可能就是想「留着名额但先不找人」。
        let asked_seats = matches!(seats, Some(Some(n)) if n > 0);
        let wants_seeking = match (request.seeking, asked_seats, site.listing.seeking) {
            (Some(seeking), _, _) => Some(seeking),
            (None, true, false) => Some(true),
            _ => None,
        };
        // 「正在找人测」蕴含「公开」：没公开的作品谁也看不见它在找人。
        let public = match (request.public, wants_seeking) {
            (None, Some(true)) => Some(true),
            (public, _) => public,
        };
        // 反过来，收回公开就同时不再求测。
        let seeking = match (public, wants_seeking) {
            (Some(false), _) => Some(false),
            (_, seeking) => seeking,
        };
        if public == Some(true) && site.current_version.is_none() {
            return Err(ApiError::invalid(
                "This project has no version yet, so there is nothing on the Plaza to play. Publish a version first.",
            ));
        }
        if title.is_some() || summary.is_some() {
            db::update_site_meta(
                &conn,
                &slug,
                title.as_deref(),
                summary.as_ref().map(|s| s.as_deref()),
            )?;
        }
        db::update_listing(
            &conn,
            &slug,
            public,
            seeking,
            seek_note.as_ref().map(|n| n.as_deref()),
        )?;
        db::update_club_settings(
            &conn,
            &slug,
            seats,
            community_url.as_ref().map(|u| u.as_deref()),
            request.feedback_public,
        )?;
        db::find_live_site(&conn, &slug, &caller.user_id)?
            .ok_or_else(|| ApiError::not_found(NO_SUCH_SITE))?
    };
    plaza::publish(&state).await;
    // 名额、群链接、反馈是否公开都在门禁页上（DESIGN §4.5 的 live.json）。
    crate::live::publish(&state, &slug).await;

    tracing::info!(
        %slug,
        public = row.listing.public,
        seeking = row.listing.seeking,
        seats = row.listing.seats,
        feedback_public = row.listing.feedback_public,
        "改了作品设置"
    );
    let site = load_site(&state, row).await?;
    Ok(Json(site))
}

/// `--seats 0` 是「不找了」。上限挡的是「公开测试」那种量（DESIGN §3.3）。
fn clean_seats(seats: Option<u32>) -> ApiResult<Option<Option<u32>>> {
    match seats {
        None => Ok(None),
        Some(0) => Ok(Some(None)),
        Some(n) if n > limits::MAX_SEATS => Err(ApiError::invalid(format!(
            "You asked for {n} seats; the cap is {}. Looking for more people than that is an open beta, not a playtest.",
            limits::MAX_SEATS
        ))),
        Some(n) => Ok(Some(Some(n))),
    }
}

/// 群链接去哪是开发者的事，我们只确认它是一条能点开的地址（DESIGN §3.3 第 6 条）。
fn clean_community_url(raw: &str) -> ApiResult<Option<String>> {
    let url = raw.trim();
    if url.is_empty() {
        return Ok(None);
    }
    let count = url.chars().count();
    if count > limits::MAX_COMMUNITY_URL_CHARS {
        return Err(ApiError::invalid(format!(
            "The community link is capped at {} characters; this one has {count}.",
            limits::MAX_COMMUNITY_URL_CHARS
        )));
    }
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err(ApiError::invalid(
            "The community link has to start with http:// or https:// — a Discord invite or any page testers can open.",
        ));
    }
    Ok(Some(url.to_string()))
}

/// 一个作品在广场与俱乐部这一侧的现状：留了名字的人、关注的人、推广。
/// 都是查库才知道的，所以 [`to_site`] 要它们从外面传进来。
#[derive(Debug, Clone, Default)]
pub struct Club {
    pub joined: u32,
    pub followers: u32,
    pub boost: Option<playtest_common::boost::Boost>,
}

pub fn club_of(conn: &Connection, slug: &str) -> ApiResult<Club> {
    Ok(Club {
        joined: db::joined_count(conn, slug)?,
        followers: db::followers_count(conn, slug)?,
        boost: db::site_boost(conn, slug)?.map(crate::boosts::to_boost),
    })
}

/// 查一次库把 [`Club`] 补齐再拼响应。
pub async fn load_site(state: &AppState, row: SiteRow) -> ApiResult<Site> {
    let club = {
        let conn = state.db().read().await;
        club_of(&conn, &row.slug)?
    };
    Ok(to_site(state, row, club))
}

/// 控制台与 CLI 看到的那一份。
///
/// 它不再自己从数据库那一行挑字段——先拼成 [`playtest_common::project::Project`]
/// （控制面唯一的拼装点，`crate::project`），再投影。门禁页的 `live.json` 和广场卡
/// 走的是同一个 `Project`，所以三处不会各自漂（REWRITE §2.1）。
///
/// 这里不查库：`club` 已经由调用方取好，列表页才不会变成 N 次查询。
pub fn to_site(state: &AppState, row: SiteRow, club: Club) -> Site {
    let url = state.site_url(&row.slug);
    let extras = crate::project::Extras {
        joined: club.joined,
        followers: club.followers,
        boost: club.boost,
        ..Default::default()
    };
    // 作品是谁的，这条路上已经鉴过权（调用方拿的就是自己的作品），头像与名字这一页用不上。
    crate::project::site_view(&crate::project::compose(row, None, url, extras))
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
            "The name \"{wanted}\" is taken. Pick another one."
        )));
    }
    Ok(wanted.to_string())
}
