//! 结果：作品时间线、会话点名册、反馈流（DESIGN §3.4）。
//!
//! 认证和 `sites` 那组一样——开发者的令牌，只能看自己的作品，别人的和不存在的一律 404。
//! 玩家那一侧不带身份地写进来（`routes::events` / `routes::feedback`），只有这里读得出去。
//!
//! 这一层**只做计数、去重和中位数**，不算比例也不算平均值：5–50 个人身上的百分比
//! 既不稳定也没法行动（DESIGN §3.4）。聚合在 Rust 里做而不是写成一条大 SQL——
//! 一次私测的量是几十行，读得懂比省一次遍历重要，换 Postgres 时也不用重写。

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use axum::extract::{Path, Query, State};
use axum::Json;
use playtest_common::ingest::{edge_kind, kind};
use playtest_common::results::{
    median_seconds, ErrorSummary, ErrorTally, FeedbackItem, FeedbackList, FeedbackStatus,
    RosterSort, SessionEvent, SessionRow, SiteResults, UpdateFeedbackRequest, VersionResults,
    VersionSessions, LONG_PLAY_SECONDS, MAX_EVENTS_PER_SESSION, TOP_ERRORS,
};
use rusqlite::{params, Connection, OptionalExtension};
use serde::Deserialize;

use crate::auth::Caller;
use crate::clock;
use crate::db::{self, SiteRow};
use crate::error::{ApiError, ApiResult};
use crate::routes::sites::NO_SUCH_SITE;
use crate::routes::JsonBody;
use crate::state::AppState;

const NO_SUCH_FEEDBACK: &str = "没有这条反馈，或者它不是你的作品的。";

/// 错误的 `name` 是空的时候拿它当 fingerprint。归成一堆总比一条条散着好看。
const UNNAMED_ERROR: &str = "没带名字的错误";

#[derive(Debug, Default, Deserialize)]
pub struct RosterQuery {
    /// `dwell`（默认，停留最短在前）或 `time`。
    #[serde(default)]
    sort: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub struct FeedbackQuery {
    #[serde(default)]
    version: Option<String>,
    #[serde(default)]
    status: Option<String>,
}

/// `GET /v1/sites/{slug}/results`：作品时间线，每版一条摘要，版本倒序。
pub async fn timeline(
    State(state): State<AppState>,
    caller: Caller,
    Path(slug): Path<String>,
) -> ApiResult<Json<SiteResults>> {
    let conn = state.db().lock().await;
    let site = owned_site(&conn, &slug, &caller)?;

    let notes = version_notes(&conn, &slug)?;
    let sessions = session_facts(&conn, &slug, None)?;
    let sdk_loaded = sessions_with_sdk_load(&conn, &slug)?;
    let mut errors = error_tallies(&conn, &slug)?;
    let load_failures = counts_by_version(
        &conn,
        "SELECT version, COUNT(*) FROM session_events
          WHERE slug = ?1 AND source = 'edge' AND kind = ?2 GROUP BY version",
        params![&slug, edge_kind::RESOURCE_FAIL],
    )?;
    let feedback_counts = counts_by_version(
        &conn,
        "SELECT version, COUNT(*) FROM feedback WHERE slug = ?1 GROUP BY version",
        params![&slug],
    )?;
    drop(conn);

    // 版本表里有的，加上只在会话里见过的。两边都列出来：没人打开的新版本要显示
    // 「还没有人打开」，而不是从时间线上消失。
    let mut wanted: BTreeSet<u32> = notes.keys().copied().collect();
    wanted.extend(sessions.iter().map(|s| s.version));

    let mut by_version: HashMap<u32, Vec<&SessionFacts>> = HashMap::new();
    for session in &sessions {
        by_version.entry(session.version).or_default().push(session);
    }

    let mut versions: Vec<VersionResults> = wanted
        .into_iter()
        .rev()
        .map(|version| {
            let rows = by_version.remove(&version).unwrap_or_default();
            let (created_at, note) = notes.get(&version).cloned().unwrap_or((None, None));
            summarize(
                version,
                created_at,
                note,
                &rows,
                &sdk_loaded,
                errors.remove(&version).unwrap_or_default(),
                load_failures.get(&version).copied().unwrap_or(0),
                feedback_counts.get(&version).copied().unwrap_or(0),
            )
        })
        .collect();
    versions.sort_by(|a, b| b.version.cmp(&a.version));

    Ok(Json(SiteResults {
        slug: site.slug,
        title: site.title,
        current_version: site.current_version,
        versions,
    }))
}

/// `GET /v1/sites/{slug}/versions/{version}/sessions`：点名册。
pub async fn sessions(
    State(state): State<AppState>,
    caller: Caller,
    Path((slug, version)): Path<(String, u32)>,
    Query(query): Query<RosterQuery>,
) -> ApiResult<Json<VersionSessions>> {
    let sort = match query.sort.as_deref() {
        None | Some("") => RosterSort::default(),
        Some(value) => value.parse().map_err(ApiError::invalid)?,
    };

    let conn = state.db().lock().await;
    owned_site(&conn, &slug, &caller)?;

    let facts = session_facts(&conn, &slug, Some(version))?;
    let mut events = events_by_session(&conn, &slug, version)?;
    let feedback_counts = feedback_by_session(&conn, &slug, version)?;
    drop(conn);

    let mut rows: Vec<SessionRow> = facts
        .into_iter()
        .map(|facts| {
            let mut timeline = events.remove(&facts.id).unwrap_or_default();
            let more_events = timeline.len() > MAX_EVENTS_PER_SESSION;
            let errors = timeline.iter().filter(|e| e.kind == kind::ERROR).count() as u32;
            // 玩到哪：最后一个开发者自己打的点。
            let reached = timeline
                .iter()
                .rev()
                .find(|e| e.kind == kind::EVENT)
                .and_then(|e| e.name.clone());
            timeline.truncate(MAX_EVENTS_PER_SESSION);

            let entered_at = facts
                .start_at
                .as_deref()
                .or(facts.first_frame_at.as_deref())
                .unwrap_or(&facts.first_seen_at);

            SessionRow {
                at: facts.first_seen_at.clone(),
                device: facts.device,
                browser: facts.browser,
                os: facts.os,
                wechat: facts.wechat,
                referrer_kind: facts.referrer_kind,
                started: facts.start_at.is_some(),
                first_frame: facts.first_frame_at.is_some(),
                entered: facts.start_at.is_some()
                    || facts.first_frame_at.is_some()
                    || timeline.iter().any(|e| e.kind == kind::LOAD),
                dwell_s: facts.dwell_s,
                last_input_after_s: facts
                    .last_input_at
                    .as_deref()
                    .map(|input| seconds_between(entered_at, input)),
                reached,
                errors,
                feedback: feedback_counts.get(&facts.id).copied().unwrap_or(0),
                is_return: facts.is_return,
                events: timeline,
                more_events,
                id: facts.id,
            }
        })
        .collect();

    match sort {
        // 停留最短的排最前面，同样短的先来的在前——排在最前面的人就是你要看的人。
        RosterSort::Dwell => rows.sort_by(|a, b| a.dwell_s.cmp(&b.dwell_s).then(a.at.cmp(&b.at))),
        RosterSort::Time => rows.sort_by(|a, b| b.at.cmp(&a.at)),
    }

    Ok(Json(VersionSessions {
        slug,
        version,
        sort,
        sessions: rows,
    }))
}

/// `GET /v1/sites/{slug}/feedback`：反馈流，时间倒序。
pub async fn feedback(
    State(state): State<AppState>,
    caller: Caller,
    Path(slug): Path<String>,
    Query(query): Query<FeedbackQuery>,
) -> ApiResult<Json<FeedbackList>> {
    let version = match query.version.as_deref() {
        None | Some("") => None,
        Some(value) => Some(value.parse::<u32>().map_err(|_| {
            ApiError::invalid(format!("版本号只能是数字，不认识「{value}」。"))
        })?),
    };
    let status = match query.status.as_deref() {
        None | Some("") => None,
        Some(value) => Some(value.parse::<FeedbackStatus>().map_err(ApiError::invalid)?),
    };

    let conn = state.db().lock().await;
    owned_site(&conn, &slug, &caller)?;

    let mut stmt = conn.prepare(
        "SELECT id, session_id, version, ts, text, seconds_in, device, browser, screenshot_hash, status
           FROM feedback
          WHERE slug = ?1
            AND (?2 IS NULL OR version = ?2)
            AND (?3 IS NULL OR status = ?3)
          ORDER BY ts DESC, id DESC",
    )?;
    let items = stmt
        .query_map(
            params![&slug, version, status.map(|s| s.as_db())],
            feedback_from_row,
        )?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    drop(stmt);
    drop(conn);

    Ok(Json(FeedbackList { slug, items }))
}

/// `PATCH /v1/sites/{slug}/feedback/{id}`：标记已看 / 已处理。
pub async fn update_feedback(
    State(state): State<AppState>,
    caller: Caller,
    Path((slug, id)): Path<(String, i64)>,
    JsonBody(request): JsonBody<UpdateFeedbackRequest>,
) -> ApiResult<Json<FeedbackItem>> {
    let conn = state.db().lock().await;
    owned_site(&conn, &slug, &caller)?;

    let changed = conn.execute(
        "UPDATE feedback SET status = ?3 WHERE id = ?1 AND slug = ?2",
        params![id, &slug, request.status.as_db()],
    )?;
    if changed == 0 {
        return Err(ApiError::not_found(NO_SUCH_FEEDBACK));
    }

    let item = conn
        .query_row(
            "SELECT id, session_id, version, ts, text, seconds_in, device, browser, screenshot_hash, status
               FROM feedback WHERE id = ?1",
            params![id],
            feedback_from_row,
        )
        .optional()?
        .ok_or_else(|| ApiError::not_found(NO_SUCH_FEEDBACK))?;
    Ok(Json(item))
}

/// 我的、没删的作品。别人的和不存在的说同一句话，不告诉外面这个 slug 存不存在。
fn owned_site(conn: &Connection, slug: &str, caller: &Caller) -> ApiResult<SiteRow> {
    db::find_live_site(conn, slug, &caller.user_id)?.ok_or_else(|| ApiError::not_found(NO_SUCH_SITE))
}

/// 一个会话里聚合要用到的几列。整行读出来在 Rust 里分组，比每版一条 SQL 好读。
struct SessionFacts {
    id: String,
    version: u32,
    first_seen_at: String,
    last_seen_at: String,
    start_at: Option<String>,
    first_frame_at: Option<String>,
    last_input_at: Option<String>,
    device: Option<String>,
    browser: Option<String>,
    os: Option<String>,
    referrer_kind: Option<String>,
    wechat: bool,
    is_return: bool,
    dwell_s: u32,
}

fn session_facts(
    conn: &Connection,
    slug: &str,
    version: Option<u32>,
) -> rusqlite::Result<Vec<SessionFacts>> {
    let mut stmt = conn.prepare(
        "SELECT id, version, first_seen_at, last_seen_at, start_at, first_frame_at, last_input_at,
                device, browser, os, referrer_kind, wechat, is_return
           FROM sessions
          WHERE slug = ?1 AND (?2 IS NULL OR version = ?2)
          ORDER BY first_seen_at, id",
    )?;
    let rows = stmt.query_map(params![slug, version], |row| {
        let first_seen_at: String = row.get(2)?;
        let last_seen_at: String = row.get(3)?;
        Ok(SessionFacts {
            id: row.get(0)?,
            version: row.get(1)?,
            dwell_s: seconds_between(&first_seen_at, &last_seen_at),
            first_seen_at,
            last_seen_at,
            start_at: row.get(4)?,
            first_frame_at: row.get(5)?,
            last_input_at: row.get(6)?,
            device: row.get(7)?,
            browser: row.get(8)?,
            os: row.get(9)?,
            referrer_kind: row.get(10)?,
            wechat: row.get::<_, i64>(11)? != 0,
            is_return: row.get::<_, i64>(12)? != 0,
        })
    })?;
    rows.collect()
}

/// 报过 SDK `load` 的会话。没有 `start_at`（边缘还没补送）但 SDK 出过声的，也算进到游戏了。
fn sessions_with_sdk_load(conn: &Connection, slug: &str) -> rusqlite::Result<HashSet<String>> {
    let mut stmt = conn.prepare(
        "SELECT DISTINCT session_id FROM session_events
          WHERE slug = ?1 AND source = 'sdk' AND kind = ?2",
    )?;
    let rows = stmt.query_map(params![slug, kind::LOAD], |row| row.get::<_, String>(0))?;
    rows.collect()
}

fn version_notes(
    conn: &Connection,
    slug: &str,
) -> rusqlite::Result<BTreeMap<u32, (Option<String>, Option<String>)>> {
    let mut stmt =
        conn.prepare("SELECT version, created_at, note FROM versions WHERE slug = ?1")?;
    let rows = stmt.query_map(params![slug], |row| {
        Ok((
            row.get::<_, u32>(0)?,
            (row.get::<_, Option<String>>(1)?, row.get(2)?),
        ))
    })?;
    rows.collect()
}

fn counts_by_version(
    conn: &Connection,
    sql: &str,
    args: impl rusqlite::Params,
) -> rusqlite::Result<HashMap<u32, u32>> {
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map(args, |row| Ok((row.get::<_, u32>(0)?, row.get::<_, u32>(1)?)))?;
    rows.collect()
}

/// 每版按 fingerprint 归堆的错误，撞得多的在前。
fn error_tallies(conn: &Connection, slug: &str) -> rusqlite::Result<HashMap<u32, Vec<ErrorTally>>> {
    let mut stmt = conn.prepare(
        "SELECT version, COALESCE(NULLIF(name, ''), ?2) AS fingerprint, COUNT(*)
           FROM session_events
          WHERE slug = ?1 AND kind = ?3
          GROUP BY version, fingerprint
          ORDER BY version, COUNT(*) DESC, fingerprint",
    )?;
    let rows = stmt.query_map(params![slug, UNNAMED_ERROR, kind::ERROR], |row| {
        Ok((
            row.get::<_, u32>(0)?,
            ErrorTally {
                fingerprint: row.get(1)?,
                count: row.get(2)?,
            },
        ))
    })?;

    let mut by_version: HashMap<u32, Vec<ErrorTally>> = HashMap::new();
    for row in rows {
        let (version, tally) = row?;
        by_version.entry(version).or_default().push(tally);
    }
    Ok(by_version)
}

fn events_by_session(
    conn: &Connection,
    slug: &str,
    version: u32,
) -> rusqlite::Result<HashMap<String, Vec<SessionEvent>>> {
    let mut stmt = conn.prepare(
        "SELECT session_id, ts, source, kind, name, data
           FROM session_events
          WHERE slug = ?1 AND version = ?2
          ORDER BY ts, id",
    )?;
    let rows = stmt.query_map(params![slug, version], |row| {
        let data: Option<String> = row.get(5)?;
        Ok((
            row.get::<_, String>(0)?,
            SessionEvent {
                ts: row.get(1)?,
                source: row.get(2)?,
                kind: row.get(3)?,
                name: row.get(4)?,
                // 存进去的时候是 JSON。真存进了别的东西也照样带出去，
                // 展开一行是为了看现场，不是为了看我们的解析器。
                data: data.map(|raw| {
                    serde_json::from_str(&raw).unwrap_or(serde_json::Value::String(raw))
                }),
            },
        ))
    })?;

    let mut by_session: HashMap<String, Vec<SessionEvent>> = HashMap::new();
    for row in rows {
        let (session_id, event) = row?;
        by_session.entry(session_id).or_default().push(event);
    }
    Ok(by_session)
}

fn feedback_by_session(
    conn: &Connection,
    slug: &str,
    version: u32,
) -> rusqlite::Result<HashMap<String, u32>> {
    let mut stmt = conn.prepare(
        "SELECT session_id, COUNT(*) FROM feedback
          WHERE slug = ?1 AND version = ?2 GROUP BY session_id",
    )?;
    let rows = stmt.query_map(params![slug, version], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, u32>(1)?))
    })?;
    rows.collect()
}

fn feedback_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<FeedbackItem> {
    Ok(FeedbackItem {
        id: row.get(0)?,
        session_id: row.get(1)?,
        version: row.get(2)?,
        ts: row.get(3)?,
        text: row.get(4)?,
        seconds_in: row.get(5)?,
        device: row.get(6)?,
        browser: row.get(7)?,
        screenshot_hash: row.get(8)?,
        status: FeedbackStatus::from_db(&row.get::<_, String>(9)?),
    })
}

#[allow(clippy::too_many_arguments)]
fn summarize(
    version: u32,
    created_at: Option<String>,
    note: Option<String>,
    rows: &[&SessionFacts],
    sdk_loaded: &HashSet<String>,
    tallies: Vec<ErrorTally>,
    load_failures: u32,
    feedback_count: u32,
) -> VersionResults {
    let mut dwells: Vec<u32> = rows.iter().map(|s| s.dwell_s).collect();

    let entered = rows
        .iter()
        .filter(|s| {
            s.start_at.is_some() || s.first_frame_at.is_some() || sdk_loaded.contains(&s.id)
        })
        .count() as u32;

    // 首帧要 SDK 才报得出来。这一版一个人都没报过，就是「不知道」而不是「一个都没掉」。
    let any_first_frame = rows.iter().any(|s| s.first_frame_at.is_some());
    let dropped_before_first_frame = any_first_frame.then(|| {
        rows.iter()
            .filter(|s| s.start_at.is_some() && s.first_frame_at.is_none())
            .count() as u32
    });

    let total: u32 = tallies.iter().map(|t| t.count).sum();
    let mut top = tallies;
    let distinct = top.len() as u32;
    top.truncate(TOP_ERRORS);

    VersionResults {
        version,
        created_at,
        note,
        opened: rows.len() as u32,
        entered,
        dropped_before_first_frame,
        returned: rows.iter().filter(|s| s.is_return).count() as u32,
        dwell_median_s: median_seconds(&mut dwells),
        played_5min_plus: rows
            .iter()
            .filter(|s| s.dwell_s >= LONG_PLAY_SECONDS)
            .count() as u32,
        errors: ErrorSummary {
            distinct,
            total,
            top,
        },
        load_failures,
        feedback_count,
        first_at: rows.iter().map(|s| s.first_seen_at.clone()).min(),
        last_at: rows.iter().map(|s| s.last_seen_at.clone()).max(),
    }
}

/// 两个 RFC 3339 之间差几秒。倒着的和解析不出来的都算 0——
/// 玩家的钟和我们的钟不是一个，负数的停留时间没有意义。
fn seconds_between(from: &str, to: &str) -> u32 {
    match (clock::parse(from), clock::parse(to)) {
        (Some(from), Some(to)) => (to - from).whole_seconds().max(0) as u32,
        _ => 0,
    }
}
