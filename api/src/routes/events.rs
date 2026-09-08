//! 玩家浏览器写进来的事件：SDK 的一批，边缘补送的一批（DESIGN §3.4）。
//!
//! 这两个端点和这个进程里其它端点最大的不同是**没有令牌**——玩家不登录、不装东西，
//! 浏览器直连（DESIGN §3.1 玩家侧零门槛）。所以这里做的每一件事都假设请求是伪造的：
//!
//! - 版本号不看客户端说什么，用 `sites` 表里的当前版本；
//! - 类型是白名单，条数、字数、字节数都有上限；
//! - `Origin` 必须是玩家域下的一页，否则连一行都不写；
//! - 按会话和按 slug 各一个令牌桶。
//!
//! CORS 挡的是「哪一页能往这里写」，不是「谁能往这里写」——不带浏览器的客户端随时能伪造
//! 一个 Origin。真正的防线是上面那几条形状约束和令牌桶：最坏情况是有人往自己的 slug 里
//! 灌垃圾事件，而不是别人的作品被污染或者库被撑爆。
//!
//! **不记 IP**，和边缘一样（DESIGN §3.4，`edge/src/events.rs` 的注释说了为什么）。

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use axum::body::Bytes;
use axum::extract::State;
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::{Extension, Json};
use playtest_common::api::ErrorCode;
use playtest_common::ingest::{self, Accepted, EdgeBatch, EventBatch};
use rusqlite::{params, Connection, OptionalExtension};
use serde::de::DeserializeOwned;
use time::{Duration, OffsetDateTime};

use crate::clock;
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

pub const NO_SUCH_SITE: &str = "没有这个作品，或者它的链接已经失效了。";
const BAD_SESSION: &str = "会话 id 的形态不对。它应该是门禁页种下的那一个。";
const TOO_MANY: &str = "这个会话发得太快了，先歇一会儿。";

/// 客户端的钟不可信，但也不能一律不信——退出时补发的「最后一次输入」本来就是几分钟前的事。
/// 落在这个窗口里就采信客户端的时间，否则用服务端收到的时间。
const PAST_WINDOW_HOURS: i64 = 24;
const FUTURE_WINDOW_MINUTES: i64 = 5;

/// 隔这么久再出现，就算「回来过第二次」（DESIGN §3.4 的点名册里那一行）。
/// 这是个实验参数不是结论：私测里看会话的真实间隔分布再调。
const RETURN_AFTER_MINUTES: i64 = 30;

/// `data.ms` 的上限，超过就不要了。十分钟还没出首帧的不是加载慢，是这个数不对。
const MAX_LOAD_MS: i64 = 10 * 60 * 1000;

// ---------------------------------------------------------------- 端点

pub async fn from_sdk(
    State(state): State<AppState>,
    Extension(limiter): Extension<Limiter>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let origin = match check_origin(&state, &headers) {
        Origin::Denied => return denied_origin(),
        other => other.allowed(),
    };
    let done = sdk_batch(&state, &limiter, &headers, &body).await;
    with_cors(done.into_response(), origin.as_deref())
}

async fn sdk_batch(
    state: &AppState,
    limiter: &Limiter,
    headers: &HeaderMap,
    body: &Bytes,
) -> ApiResult<Json<Accepted>> {
    let batch: EventBatch = parse(body)?;
    if !ingest::is_session_id(&batch.session) {
        return Err(ApiError::invalid(BAD_SESSION));
    }
    if batch.events.len() > ingest::MAX_EVENTS_PER_BATCH {
        return Err(ApiError::invalid(format!(
            "一批最多 {} 条事件，这批有 {} 条。",
            ingest::MAX_EVENTS_PER_BATCH,
            batch.events.len()
        )));
    }
    let slug = clean_slug(&batch.slug)?;
    if !limiter.take(&batch.session, &slug) {
        return Err(too_many());
    }

    let now = clock::now();
    let ua = header_str(headers, header::USER_AGENT.as_str());
    let mut conn = state.db().lock().await;
    let version =
        current_version(&conn, &slug)?.ok_or_else(|| ApiError::not_found(NO_SUCH_SITE))?;

    // 「最后一次见到这个人」按批里最晚的那条事件算，不按请求到达的时刻：退出时补发的那一批
    // 本来就晚于它记录的事情，用到达时刻会把停留时长算长。
    let stamped: Vec<OffsetDateTime> = batch
        .events
        .iter()
        .filter(|event| ingest::kind::known(&event.kind))
        .map(|event| stamp(&event.ts, now))
        .collect();
    let at = stamped.iter().max().copied().unwrap_or(now);

    let tx = conn.transaction()?;
    touch_session(
        &tx,
        &Seen {
            id: &batch.session,
            slug: &slug,
            version,
            at: &clock::format(at),
            ua,
            // SDK 的请求头里那个 Referer 是作品自己的地址，不是玩家从哪来的，
            // 拿它填 referrer_kind 只会填出一个假答案。这一列留给边缘。
            referer: None,
            return_before: &clock::format(at - Duration::minutes(RETURN_AFTER_MINUTES)),
        },
    )?;

    let mut accepted = 0usize;
    for event in &batch.events {
        if !ingest::kind::known(&event.kind) {
            continue;
        }
        let ts = clock::format(stamp(&event.ts, now));
        insert_event(
            &tx,
            &NewEvent {
                session_id: &batch.session,
                slug: &slug,
                version,
                ts: &ts,
                source: "sdk",
                kind: &event.kind,
                name: clip(event.name.as_deref(), ingest::MAX_NAME_CHARS).as_deref(),
                data: data_json(event.data.as_ref()).as_deref(),
            },
        )?;
        match event.kind.as_str() {
            ingest::kind::LOAD => {
                first_frame(&tx, &batch.session, &ts, load_ms(event.data.as_ref()))?
            }
            ingest::kind::INPUT => last_input(&tx, &batch.session, &ts)?,
            _ => {}
        }
        accepted += 1;
    }
    tx.commit()?;

    Ok(Json(Accepted { accepted }))
}

/// 边缘那边现在把这些行写在自己盘上的 JSONL 里（`edge/src/events.rs`）。
/// **谁把它们送过来还没定**（第三周的事），这里只是先把收的一侧准备好；
/// 那一步落地时这个端点要换成边缘的签名令牌，现在它和 SDK 的端点一样是敞开的。
pub async fn from_edge(
    State(state): State<AppState>,
    Extension(limiter): Extension<Limiter>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let origin = match check_origin(&state, &headers) {
        Origin::Denied => return denied_origin(),
        other => other.allowed(),
    };
    let done = edge_batch(&state, &limiter, &body).await;
    with_cors(done.into_response(), origin.as_deref())
}

async fn edge_batch(
    state: &AppState,
    limiter: &Limiter,
    body: &Bytes,
) -> ApiResult<Json<Accepted>> {
    let batch: EdgeBatch = parse(body)?;
    if batch.events.len() > ingest::MAX_EVENTS_PER_BATCH {
        return Err(ApiError::invalid(format!(
            "一批最多 {} 条事件，这批有 {} 条。",
            ingest::MAX_EVENTS_PER_BATCH,
            batch.events.len()
        )));
    }

    let now = clock::now();
    let mut conn = state.db().lock().await;
    let tx = conn.transaction()?;

    let mut accepted = 0usize;
    let mut versions: HashMap<String, u32> = HashMap::new();
    for line in &batch.events {
        if !ingest::edge_kind::known(&line.kind) || !ingest::is_session_id(&line.sid) {
            continue;
        }
        let Ok(slug) = clean_slug(&line.slug) else {
            continue;
        };
        if !limiter.take_slug(&slug) {
            continue;
        }
        let current = match versions.get(&slug) {
            Some(version) => *version,
            None => {
                let Some(version) = current_version(&tx, &slug)? else {
                    continue;
                };
                versions.insert(slug.clone(), version);
                version
            }
        };
        // 边缘服务的可能是一个还没被新版本顶掉的旧版本，那个版本号是真的；
        // 比当前版本还新的只能是伪造，按当前版本记。
        let version = if line.version >= 1 && line.version <= current {
            line.version
        } else {
            current
        };
        let at = stamp(&line.ts, now);
        let ts = clock::format(at);

        touch_session(
            &tx,
            &Seen {
                id: &line.sid,
                slug: &slug,
                version,
                at: &ts,
                ua: Some(line.ua.as_str()).filter(|ua| !ua.is_empty()),
                referer: Some(line.referer.as_str()),
                return_before: &clock::format(at - Duration::minutes(RETURN_AFTER_MINUTES)),
            },
        )?;
        insert_event(
            &tx,
            &NewEvent {
                session_id: &line.sid,
                slug: &slug,
                version,
                ts: &ts,
                source: "edge",
                kind: &line.kind,
                name: clip(line.reason.as_deref(), ingest::MAX_NAME_CHARS).as_deref(),
                data: detail_json(line.detail.as_deref()).as_deref(),
            },
        )?;
        match line.kind.as_str() {
            ingest::edge_kind::GATE_VIEW => stage(&tx, &line.sid, "gate_view_at", &ts)?,
            ingest::edge_kind::START => stage(&tx, &line.sid, "start_at", &ts)?,
            _ => {}
        }
        accepted += 1;
    }
    tx.commit()?;

    Ok(Json(Accepted { accepted }))
}

/// 预检。SDK 发的是 `text/plain` 的体，正常路径上不会走到这里；
/// 开发者自己用 `fetch` 发 JSON 时会。
pub async fn preflight(State(state): State<AppState>, headers: HeaderMap) -> Response {
    match check_origin(&state, &headers) {
        Origin::Denied => denied_origin(),
        other => {
            let mut response = StatusCode::NO_CONTENT.into_response();
            let out = response.headers_mut();
            put(out, "access-control-allow-methods", "POST, OPTIONS");
            put(out, "access-control-allow-headers", "content-type");
            put(out, "access-control-max-age", "86400");
            with_cors(response, other.allowed().as_deref())
        }
    }
}

// ---------------------------------------------------------------- 来源

pub enum Origin {
    /// 不是浏览器发的（没有 `Origin` 头）。不加 CORS 头，也不拦——拦了也没有意义。
    Absent,
    Allowed(String),
    Denied,
}

impl Origin {
    fn allowed(self) -> Option<String> {
        match self {
            Origin::Allowed(origin) => Some(origin),
            _ => None,
        }
    }
}

/// 只有玩家域下的一页能写进来：`Origin` 的主机要以 `.<内容域后缀>` 结尾。
///
/// 后缀不另开一个环境变量，从已有的 `PLAYTEST_SITE_URL_TEMPLATE` 推出来——
/// 那个模板本来就是「玩家链接长什么样」，两处配置对不上比少一个配置更容易出事。
pub fn check_origin(state: &AppState, headers: &HeaderMap) -> Origin {
    let Some(origin) = header_str(headers, header::ORIGIN.as_str()) else {
        return Origin::Absent;
    };
    let host = ingest::host_of(origin);
    let suffix = content_host_suffix(state);
    if !host.is_empty() && host.len() > suffix.len() + 1 && host.ends_with(&format!(".{suffix}")) {
        Origin::Allowed(origin.to_string())
    } else {
        Origin::Denied
    }
}

/// 链接模板里放一个假 slug，剩下的就是内容域后缀。
fn content_host_suffix(state: &AppState) -> String {
    const PROBE: &str = "sluglugslug";
    let url = state.site_url(PROBE);
    let host = ingest::host_of(&url);
    host.strip_prefix(&format!("{PROBE}."))
        .unwrap_or(host)
        .to_ascii_lowercase()
}

pub fn with_cors(mut response: Response, origin: Option<&str>) -> Response {
    if let Some(origin) = origin {
        let out = response.headers_mut();
        put(out, "access-control-allow-origin", origin);
        put(out, "vary", "Origin");
    }
    response
}

fn denied_origin() -> Response {
    ApiError::public(
        StatusCode::FORBIDDEN,
        ErrorCode::Invalid,
        "这个地址只收玩家页面发来的事件。",
    )
    .into_response()
}

// ---------------------------------------------------------------- 限流

/// 按会话和按 slug 各一个令牌桶。
///
/// 挡的是「一个页面在循环里发」和「一个 slug 被刷」，不是精确计量：桶在进程内存里，
/// 多进程时每个进程各一份。v0.1 单进程（DESIGN §4.5），这一层够用；换 Postgres 那天
/// 这里要跟着挪。
#[derive(Clone)]
pub struct Limiter {
    inner: Arc<Mutex<Buckets>>,
}

/// 一个会话的容量与恢复速度。SDK 是 1 秒合并或 20 条一发，正常一分钟也就几次。
const SESSION_BURST: f64 = 30.0;
const SESSION_PER_SEC: f64 = 0.5;
/// 一个 slug 上所有人加起来。20 个人同时在玩、每人每秒一发也用不到这个数。
const SLUG_BURST: f64 = 600.0;
const SLUG_PER_SEC: f64 = 20.0;
/// 桶多到这个数就清一遍空闲的。一个满桶等于「这个 id 最近没发过东西」。
const MAX_BUCKETS: usize = 20_000;

#[derive(Default)]
struct Buckets {
    by_session: HashMap<String, Bucket>,
    by_slug: HashMap<String, Bucket>,
}

struct Bucket {
    tokens: f64,
    updated: Instant,
}

impl Bucket {
    fn refill(&mut self, now: Instant, per_sec: f64, burst: f64) {
        let elapsed = now.duration_since(self.updated).as_secs_f64();
        self.tokens = (self.tokens + elapsed * per_sec).min(burst);
        self.updated = now;
    }
}

impl Limiter {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(Buckets::default())),
        }
    }

    /// 两个桶都还有票才放行，两个都扣一张。
    pub fn take(&self, session: &str, slug: &str) -> bool {
        let now = Instant::now();
        let mut guard = self.locked();
        let buckets = &mut *guard;
        prune(&mut buckets.by_session, SESSION_BURST);
        prune(&mut buckets.by_slug, SLUG_BURST);

        let session_ok = ready(
            &mut buckets.by_session,
            session,
            now,
            SESSION_PER_SEC,
            SESSION_BURST,
        );
        let slug_ok = ready(&mut buckets.by_slug, slug, now, SLUG_PER_SEC, SLUG_BURST);
        if !session_ok || !slug_ok {
            return false;
        }
        spend(&mut buckets.by_session, session);
        spend(&mut buckets.by_slug, slug);
        true
    }

    /// 只按 slug 扣。给边缘补送用：那一批里的会话是历史，不是此刻在发请求的人，
    /// 拿会话桶去挡它只会把补送的行丢掉。
    pub fn take_slug(&self, slug: &str) -> bool {
        let now = Instant::now();
        let mut guard = self.locked();
        let buckets = &mut *guard;
        prune(&mut buckets.by_slug, SLUG_BURST);
        if !ready(&mut buckets.by_slug, slug, now, SLUG_PER_SEC, SLUG_BURST) {
            return false;
        }
        spend(&mut buckets.by_slug, slug);
        true
    }

    fn locked(&self) -> std::sync::MutexGuard<'_, Buckets> {
        // 锁被毒化说明别处 panic 过。限流不是正确性的一部分，这时照常放行，
        // 让请求自己去撞真正的约束。
        self.inner.lock().unwrap_or_else(|err| err.into_inner())
    }
}

fn ready(
    buckets: &mut HashMap<String, Bucket>,
    key: &str,
    now: Instant,
    per_sec: f64,
    burst: f64,
) -> bool {
    let bucket = buckets.entry(key.to_string()).or_insert(Bucket {
        tokens: burst,
        updated: now,
    });
    bucket.refill(now, per_sec, burst);
    bucket.tokens >= 1.0
}

fn spend(buckets: &mut HashMap<String, Bucket>, key: &str) {
    if let Some(bucket) = buckets.get_mut(key) {
        bucket.tokens -= 1.0;
    }
}

impl Default for Limiter {
    fn default() -> Self {
        Self::new()
    }
}

fn prune(buckets: &mut HashMap<String, Bucket>, burst: f64) {
    if buckets.len() <= MAX_BUCKETS {
        return;
    }
    let now = Instant::now();
    buckets.retain(|_, bucket| {
        bucket.refill(now, 0.0, burst);
        bucket.tokens < burst
    });
}

fn too_many() -> ApiError {
    ApiError::public(
        StatusCode::TOO_MANY_REQUESTS,
        ErrorCode::QuotaExceeded,
        TOO_MANY,
    )
}

// ---------------------------------------------------------------- 库

/// 一次「这个会话又出现了」。
pub struct Seen<'a> {
    pub id: &'a str,
    pub slug: &'a str,
    pub version: u32,
    /// 这一下发生的时间。
    pub at: &'a str,
    /// 只有拿得到 UA 时才填，空的会把已经认出来的设备覆盖掉。
    pub ua: Option<&'a str>,
    /// `None` 是「不知道从哪来」，和「直接打开」不是一回事。
    pub referer: Option<&'a str>,
    /// 上一次活动早于这个时间就算回头客。
    pub return_before: &'a str,
}

/// 会话行：没有就建，有就把「最后一次见到」往后挪。
///
/// 先读再写而不是一条 upsert：几列的取舍规则（第一次的 UA 说了算、微信只加不减、
/// 隔久了才算回头客）写成 SQL 会变成一串看不懂的 CASE，而这个连接本来就是串行的。
pub fn touch_session(conn: &Connection, seen: &Seen<'_>) -> rusqlite::Result<()> {
    let existing: Option<(String, Option<String>)> = conn
        .query_row(
            "SELECT last_seen_at, ua FROM sessions WHERE id = ?1",
            params![seen.id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;

    let wechat = seen.ua.is_some_and(playtest_common::ingest::is_wechat_ua);
    let client = seen.ua.map(ingest::classify_ua);
    // 第一个带会话 id 的事件是「点了开始」，它的 Referer 是作品自己的门禁页，不是玩家从哪来的。
    // 把自己当来源会让所有人都变成「其它」；自己引用自己按「不知道」处理，留给真的外部来源。
    let referrer_kind = if wechat {
        Some("wechat")
    } else {
        seen.referer
            .filter(|r| !ingest::is_self_referral(r, seen.slug))
            .map(|r| ingest::referrer_kind(r, false))
    };

    let Some((last_seen_at, known_ua)) = existing else {
        conn.execute(
            "INSERT INTO sessions
                 (id, slug, version, first_seen_at, last_seen_at, ua, device, browser, os,
                  referrer_kind, wechat, is_return)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, 0)",
            params![
                seen.id,
                seen.slug,
                seen.version,
                seen.at,
                seen.at,
                seen.ua,
                client.map(|c| c.device),
                client.map(|c| c.browser),
                client.map(|c| c.os),
                referrer_kind,
                wechat as i64,
            ],
        )?;
        return Ok(());
    };

    let is_return = last_seen_at.as_str() < seen.return_before;
    conn.execute(
        "UPDATE sessions SET
             last_seen_at  = MAX(last_seen_at, ?2),
             first_seen_at = MIN(first_seen_at, ?2),
             is_return     = CASE WHEN ?3 THEN 1 ELSE is_return END,
             wechat        = CASE WHEN ?4 THEN 1 ELSE wechat END,
             referrer_kind = COALESCE(referrer_kind, ?5)
         WHERE id = ?1",
        params![
            seen.id,
            seen.at,
            is_return as i64,
            wechat as i64,
            referrer_kind,
        ],
    )?;

    // 第一个报上来的 UA 说了算。边缘和 SDK 报的是同一个浏览器，先到的那个已经够用，
    // 后到的只会在两边截断长度不同时制造一行假的「换了设备」。
    if known_ua.is_none_or(|ua| ua.is_empty()) {
        if let (Some(ua), Some(client)) = (seen.ua, client) {
            conn.execute(
                "UPDATE sessions SET ua = ?2, device = ?3, browser = ?4, os = ?5 WHERE id = ?1",
                params![seen.id, ua, client.device, client.browser, client.os],
            )?;
        }
    }
    Ok(())
}

pub struct NewEvent<'a> {
    pub session_id: &'a str,
    pub slug: &'a str,
    pub version: u32,
    pub ts: &'a str,
    pub source: &'a str,
    pub kind: &'a str,
    pub name: Option<&'a str>,
    pub data: Option<&'a str>,
}

pub fn insert_event(conn: &Connection, event: &NewEvent<'_>) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO session_events (session_id, slug, version, ts, source, kind, name, data)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            event.session_id,
            event.slug,
            event.version,
            event.ts,
            event.source,
            event.kind,
            event.name,
            event.data,
        ],
    )?;
    Ok(())
}

/// 这个 slug 现在是第几版。作品不存在或已删返回 `None`；还没上传过版本的是 0。
pub fn current_version(conn: &Connection, slug: &str) -> rusqlite::Result<Option<u32>> {
    conn.query_row(
        "SELECT COALESCE(current_version, 0) FROM sites WHERE slug = ?1 AND deleted_at IS NULL",
        params![slug],
        |row| row.get(0),
    )
    .optional()
}

/// 首帧只认第一次：同一个会话里刷新一次不该把「门禁到首帧」这段重算。
fn first_frame(conn: &Connection, id: &str, ts: &str, ms: Option<i64>) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE sessions SET
             first_frame_at = COALESCE(first_frame_at, ?2),
             load_ms        = COALESCE(load_ms, ?3)
         WHERE id = ?1",
        params![id, ts, ms],
    )?;
    Ok(())
}

fn last_input(conn: &Connection, id: &str, ts: &str) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE sessions SET last_input_at = MAX(COALESCE(last_input_at, ''), ?2) WHERE id = ?1",
        params![id, ts],
    )?;
    Ok(())
}

/// 门禁页出现、点了开始：一个会话里只记第一次。
fn stage(conn: &Connection, id: &str, column: &str, ts: &str) -> rusqlite::Result<()> {
    // 列名是上面两个字面量之一，不来自请求。
    let sql = format!("UPDATE sessions SET {column} = COALESCE({column}, ?2) WHERE id = ?1");
    conn.execute(&sql, params![id, ts])?;
    Ok(())
}

// ---------------------------------------------------------------- 零件

/// 不看 `Content-Type` 直接解析。`navigator.sendBeacon` 只能发 `text/plain`
/// 这类不触发预检的类型，为它挡一个 415 没有任何好处。
pub fn parse<T: DeserializeOwned>(body: &Bytes) -> ApiResult<T> {
    serde_json::from_slice(body).map_err(|err| {
        tracing::debug!(%err, "写入端点收到了解析不了的体");
        ApiError::invalid("请求内容不是我们认识的格式。")
    })
}

pub fn clean_slug(raw: &str) -> ApiResult<String> {
    let slug = raw.trim().to_ascii_lowercase();
    playtest_common::slug::validate(&slug).map_err(|_| ApiError::not_found(NO_SUCH_SITE))?;
    Ok(slug)
}

pub fn header_str<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers.get(name).and_then(|value| value.to_str().ok())
}

fn put(headers: &mut HeaderMap, name: &'static str, value: &str) {
    match HeaderValue::from_str(value) {
        Ok(value) => {
            headers.insert(name, value);
        }
        Err(err) => tracing::warn!(name, %err, "响应头的值不合法，丢掉"),
    }
}

/// 客户端报的时间落在窗口里就用它，否则用服务端的。
fn stamp(raw: &str, now: OffsetDateTime) -> OffsetDateTime {
    match clock::parse(raw) {
        Some(t)
            if t <= now + Duration::minutes(FUTURE_WINDOW_MINUTES)
                && t >= now - Duration::hours(PAST_WINDOW_HOURS) =>
        {
            t
        }
        _ => now,
    }
}

pub fn clip(value: Option<&str>, max_chars: usize) -> Option<String> {
    let text = value.map(str::trim).filter(|t| !t.is_empty())?;
    Some(match text.char_indices().nth(max_chars) {
        Some((idx, _)) => text[..idx].to_string(),
        None => text.to_string(),
    })
}

/// 事件的 `data` 存成一段 JSON 文本。太大的整条丢掉——错误堆栈是最大的那一个，
/// SDK 那边已经截到 2 KB，还超说明这不是我们的 SDK 发的。
fn data_json(value: Option<&serde_json::Value>) -> Option<String> {
    let text = serde_json::to_string(value?).ok()?;
    (text.len() <= ingest::MAX_DATA_BYTES).then_some(text)
}

fn detail_json(detail: Option<&str>) -> Option<String> {
    let detail = clip(detail, ingest::MAX_FEEDBACK_CHARS)?;
    serde_json::to_string(&serde_json::json!({ "detail": detail })).ok()
}

/// `load` 事件里的毫秒数。负数、离谱的大数、不是数的都当没报。
fn load_ms(data: Option<&serde_json::Value>) -> Option<i64> {
    let ms = data?.get("ms")?.as_f64()?;
    (ms.is_finite() && ms >= 0.0 && ms <= MAX_LOAD_MS as f64).then_some(ms.round() as i64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_clocks_are_believed_only_within_a_window() {
        let now = clock::now();
        let recent = now - Duration::minutes(3);
        assert_eq!(
            stamp(&clock::format(recent), now),
            recent,
            "几分钟前的补报要保留原时间"
        );
        assert_eq!(stamp("下午三点", now), now);
        assert_eq!(
            stamp(&clock::format(now + Duration::hours(3)), now),
            now,
            "钟快了三小时的机器不能把事件写到未来"
        );
        assert_eq!(stamp(&clock::format(now - Duration::hours(48)), now), now);
    }

    #[test]
    fn load_ms_ignores_nonsense() {
        assert_eq!(
            load_ms(Some(&serde_json::json!({"ms": 1234.6}))),
            Some(1235)
        );
        assert_eq!(load_ms(Some(&serde_json::json!({"ms": -1}))), None);
        assert_eq!(load_ms(Some(&serde_json::json!({"ms": 1e12}))), None);
        assert_eq!(load_ms(Some(&serde_json::json!({"ms": "快"}))), None);
        assert_eq!(load_ms(None), None);
    }

    #[test]
    fn oversized_data_is_dropped_not_truncated() {
        let big = serde_json::json!({ "stack": "x".repeat(ingest::MAX_DATA_BYTES) });
        assert_eq!(data_json(Some(&big)), None);
        let small = serde_json::json!({ "level": 3 });
        assert_eq!(data_json(Some(&small)).as_deref(), Some(r#"{"level":3}"#));
    }

    #[test]
    fn buckets_run_out_and_come_back() {
        let limiter = Limiter::new();
        let session = "a".repeat(32);
        let mut allowed = 0;
        for _ in 0..100 {
            if limiter.take(&session, "brisk-otter-41") {
                allowed += 1;
            }
        }
        assert_eq!(allowed, SESSION_BURST as i32, "一个会话的桶就这么大");
        // 换一个会话，同一个 slug 还有额度。
        assert!(limiter.take(&"b".repeat(32), "brisk-otter-41"));
    }
}
