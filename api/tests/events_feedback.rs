//! 玩家浏览器写进来的那三个端点。
//!
//! 这里关心的和 `upload_flow.rs` 不一样：那边的调用方是开发者的 CLI，带令牌、会看报错；
//! 这边的调用方是玩家的浏览器，不带任何身份、也没人看错误。所以每条断言问的都是
//! 「有人乱发的时候会怎么样」——版本能不能伪造、别人的页面能不能写、发疯了会不会把库撑爆。
//!
//! 真机上用浏览器和 curl 走的那一遍记在 `docs/spikes/2026-09-07-sdk-ingest.md`。

use axum::body::{Body, Bytes};
use axum::http::{header, Request, StatusCode};
use axum::Router;
use playtest_api::{app, AppState, Config};
use playtest_common::api::{
    routes as paths, AnonSessionResponse, CommitUploadResponse, CreateSiteRequest, ErrorBody,
    ErrorCode, PrepareUploadRequest, PrepareUploadResponse, Site,
};
use playtest_common::hash;
use playtest_common::ingest::{
    self, routes as ingest_paths, Accepted, EdgeBatch, EdgeEvent, Event, EventBatch,
    FeedbackAccepted, FeedbackRequest,
};
use playtest_common::manifest::{FileEntry, GateMode};
use serde::de::DeserializeOwned;
use serde::Serialize;
use tower::ServiceExt;

const INDEX_HTML: &[u8] = b"<!doctype html><meta charset=utf-8><canvas id=game></canvas>";
const PLAYER_UA: &str = "Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) AppleWebKit/605.1.15 \
                         (KHTML, like Gecko) Mobile/15E148 MicroMessenger/8.0.49(0x18003128)";

struct Harness {
    router: Router,
    state: AppState,
    _dir: tempfile::TempDir,
}

struct Reply {
    status: StatusCode,
    headers: axum::http::HeaderMap,
    body: Bytes,
}

impl Reply {
    fn json<T: DeserializeOwned>(&self) -> T {
        assert!(
            self.status.is_success(),
            "本来该成功的：{} {}",
            self.status,
            self.text()
        );
        serde_json::from_slice(&self.body)
            .unwrap_or_else(|err| panic!("响应不是预期的 JSON：{err}；原文是 {}", self.text()))
    }

    fn error(&self, status: StatusCode, code: ErrorCode) -> ErrorBody {
        assert_eq!(self.status, status, "状态码不对，响应是 {}", self.text());
        let body: ErrorBody = serde_json::from_slice(&self.body)
            .unwrap_or_else(|err| panic!("错误响应不是 ErrorBody：{err}；原文是 {}", self.text()));
        assert_eq!(body.code, code, "错误码不对：{}", body.message);
        body
    }

    fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name).and_then(|v| v.to_str().ok())
    }

    fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }
}

impl Harness {
    async fn start() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let config = Config {
            listen: "127.0.0.1:0".parse().unwrap(),
            data_dir: dir.path().to_path_buf(),
            site_url_template: "http://{slug}.localhost:8443".to_string(),
        };
        let state = AppState::from_config(&config).await.unwrap();
        Self {
            router: app(state.clone()),
            state,
            _dir: dir,
        }
    }

    async fn send(&self, request: Request<Body>) -> Reply {
        let response = self.router.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let headers = response.headers().clone();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        Reply {
            status,
            headers,
            body,
        }
    }

    /// 玩家的浏览器怎么发，这里就怎么发：`text/plain` 的体（`sendBeacon` 只能这样），
    /// 带作品页面的 Origin 和玩家的 UA。
    async fn from_player<T: Serialize>(&self, path: &str, slug: &str, value: &T) -> Reply {
        self.as_origin(path, &format!("http://{slug}.localhost:8443"), value)
            .await
    }

    async fn as_origin<T: Serialize>(&self, path: &str, origin: &str, value: &T) -> Reply {
        let request = Request::builder()
            .method("POST")
            .uri(path)
            .header(header::CONTENT_TYPE, "text/plain;charset=UTF-8")
            .header(header::ORIGIN, origin)
            .header(header::USER_AGENT, PLAYER_UA)
            .body(Body::from(serde_json::to_vec(value).unwrap()))
            .unwrap();
        self.send(request).await
    }

    /// 边缘补送：服务器对服务器，没有 Origin。
    async fn from_edge<T: Serialize>(&self, value: &T) -> Reply {
        let request = Request::builder()
            .method("POST")
            .uri(ingest_paths::EDGE)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(serde_json::to_vec(value).unwrap()))
            .unwrap();
        self.send(request).await
    }

    /// 一个能玩的作品：匿名令牌 → 建作品 → 传一个文件 → 提交。返回 slug。
    async fn live_site(&self) -> String {
        let token: AnonSessionResponse = self
            .send(
                Request::builder()
                    .method("POST")
                    .uri(paths::ANON_SESSIONS)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .json();
        let site: Site = self
            .developer("POST", paths::SITES, &token.token, &CreateSiteRequest::default())
            .await
            .json();
        self.publish(&site.slug, &token.token).await;
        site.slug
    }

    /// 再发一版。
    async fn publish(&self, slug: &str, token: &str) -> u32 {
        let file = FileEntry {
            path: "index.html".to_string(),
            hash: hash::hash_bytes(INDEX_HTML),
            size: INDEX_HTML.len() as u64,
        };
        let prepared: PrepareUploadResponse = self
            .developer(
                "POST",
                &paths::site_uploads(slug),
                token,
                &PrepareUploadRequest {
                    files: vec![file.clone()],
                    title: Some("小球".to_string()),
                    note: None,
                    gate: GateMode::Once,
                    isolated: false,
                    spa: false,
                    engine: None,
                },
            )
            .await
            .json();
        if !prepared.missing.is_empty() {
            let put = Request::builder()
                .method("PUT")
                .uri(paths::blob(&file.hash))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::from(INDEX_HTML))
                .unwrap();
            self.send(put).await;
        }
        let committed: CommitUploadResponse = self
            .developer(
                "POST",
                &paths::site_upload_commit(slug, &prepared.upload_id),
                token,
                &serde_json::json!({}),
            )
            .await
            .json();
        committed.version
    }

    async fn developer<T: Serialize>(
        &self,
        method: &str,
        path: &str,
        token: &str,
        value: &T,
    ) -> Reply {
        let request = Request::builder()
            .method(method)
            .uri(path)
            .header(header::AUTHORIZATION, format!("Bearer {token}"))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(serde_json::to_vec(value).unwrap()))
            .unwrap();
        self.send(request).await
    }

    async fn one_row<T, F>(&self, sql: &str, params: &[&dyn rusqlite::ToSql], map: F) -> T
    where
        F: FnOnce(&rusqlite::Row<'_>) -> rusqlite::Result<T>,
    {
        let conn = self.state.db().lock().await;
        conn.query_row(sql, params, map).unwrap()
    }

    async fn count(&self, sql: &str) -> i64 {
        self.one_row(sql, &[], |row| row.get(0)).await
    }
}

/// 服务端只采信最近 24 小时内的客户端时间戳（`stamp`），所以测试里的时间要锚在「现在」附近，
/// 不能写死某一天——写死的那一天过了 24 小时整个测试就会静悄悄地失效。
fn at(offset_secs: i64) -> String {
    let base = time::OffsetDateTime::now_utc()
        .replace_nanosecond(0)
        .unwrap()
        - time::Duration::minutes(10);
    (base + time::Duration::seconds(offset_secs))
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap()
}

fn session_id(seed: char) -> String {
    seed.to_string().repeat(32)
}

fn event(kind: &str) -> Event {
    Event {
        ts: at(0),
        kind: kind.to_string(),
        name: None,
        data: None,
    }
}

#[tokio::test]
async fn a_session_gets_stitched_together_from_both_sides() {
    let h = Harness::start().await;
    let slug = h.live_site().await;
    let sid = session_id('a');

    // 一、边缘补送：出了门禁页、点了开始。
    let edge = EdgeBatch {
        events: vec![
            EdgeEvent {
                ts: at(0),
                kind: "gate_view".into(),
                slug: slug.clone(),
                version: 1,
                sid: sid.clone(),
                ua: PLAYER_UA.into(),
                referer: "https://mp.weixin.qq.com/s/abc".into(),
                wechat: true,
                reason: None,
                detail: None,
            },
            EdgeEvent {
                ts: at(9),
                kind: "start".into(),
                slug: slug.clone(),
                version: 1,
                sid: sid.clone(),
                ua: PLAYER_UA.into(),
                referer: String::new(),
                wechat: true,
                reason: None,
                detail: None,
            },
        ],
    };
    let accepted: Accepted = h.from_edge(&edge).await.json();
    assert_eq!(accepted.accepted, 2);

    // 二、SDK：首帧、一个自定义事件、一个错误、退出时补的最后一次输入。
    let batch = EventBatch {
        session: sid.clone(),
        slug: slug.clone(),
        events: vec![
            Event {
                ts: at(12),
                kind: "load".into(),
                name: None,
                data: Some(serde_json::json!({ "ms": 2480, "ttfb": 120 })),
            },
            Event {
                ts: at(40),
                kind: "event".into(),
                name: Some("level_done".into()),
                data: Some(serde_json::json!({ "level": 3 })),
            },
            Event {
                ts: at(60),
                kind: "error".into(),
                name: Some("TypeError: Cannot read 'x' of undefined @ main.js:412".into()),
                data: Some(serde_json::json!({ "stack": "at update (main.js:412:9)" })),
            },
            Event {
                ts: at(150),
                kind: "input".into(),
                name: None,
                data: None,
            },
        ],
    };
    let accepted: Accepted = h
        .from_player(ingest_paths::EVENTS, &slug, &batch)
        .await
        .json();
    assert_eq!(accepted.accepted, 4);

    // 会话这一行就是点名册里的一行人。
    let (device, browser, os, referrer, wechat, gate, start, frame, load_ms, last_input, is_return): (
        String,
        String,
        String,
        String,
        i64,
        String,
        String,
        String,
        i64,
        String,
        i64,
    ) = h
        .one_row(
            "SELECT device, browser, os, referrer_kind, wechat, gate_view_at, start_at,
                    first_frame_at, load_ms, last_input_at, is_return
               FROM sessions WHERE id = ?1",
            &[&sid],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                    row.get(7)?,
                    row.get(8)?,
                    row.get(9)?,
                    row.get(10)?,
                ))
            },
        )
        .await;
    assert_eq!((device.as_str(), browser.as_str(), os.as_str()), ("phone", "wechat", "ios"));
    assert_eq!(referrer, "wechat");
    assert_eq!(wechat, 1);
    assert_eq!(gate, at(0));
    assert_eq!(start, at(9));
    assert_eq!(frame, at(12), "首帧来自 SDK 的 load");
    assert_eq!(load_ms, 2480);
    assert_eq!(last_input, at(150));
    assert_eq!(is_return, 0);

    assert_eq!(h.count("SELECT COUNT(*) FROM session_events").await, 6);
    assert_eq!(
        h.count("SELECT COUNT(*) FROM session_events WHERE source = 'edge'").await,
        2
    );
    let fingerprint: String = h
        .one_row(
            "SELECT name FROM session_events WHERE kind = 'error'",
            &[],
            |row| row.get(0),
        )
        .await;
    assert!(fingerprint.contains("main.js:412"), "{fingerprint}");
}

#[tokio::test]
async fn the_version_comes_from_the_server_not_the_client() {
    let h = Harness::start().await;
    let token: AnonSessionResponse = h
        .send(
            Request::builder()
                .method("POST")
                .uri(paths::ANON_SESSIONS)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .json();
    let site: Site = h
        .developer("POST", paths::SITES, &token.token, &CreateSiteRequest::default())
        .await
        .json();
    h.publish(&site.slug, &token.token).await;
    assert_eq!(h.publish(&site.slug, &token.token).await, 2, "现在是 v2");

    // SDK 的请求体里根本没有版本号这一项，服务端自己填。
    let sid = session_id('b');
    h.from_player(
        ingest_paths::EVENTS,
        &site.slug,
        &EventBatch {
            session: sid.clone(),
            slug: site.slug.clone(),
            events: vec![event("load")],
        },
    )
    .await
    .json::<Accepted>();
    assert_eq!(
        h.count(&format!(
            "SELECT version FROM session_events WHERE session_id = '{sid}'"
        ))
        .await,
        2
    );

    // 边缘报的版本号可以是一个还没被顶掉的旧版本（真的），但不能比当前版本还新（只能是伪造）。
    let older = session_id('c');
    let faked = session_id('d');
    for (sid, claimed) in [(&older, 1u32), (&faked, 999)] {
        h.from_edge(&EdgeBatch {
            events: vec![EdgeEvent {
                ts: at(0),
                kind: "gate_view".into(),
                slug: site.slug.clone(),
                version: claimed,
                sid: sid.clone(),
                ua: PLAYER_UA.into(),
                referer: String::new(),
                wechat: false,
                reason: None,
                detail: None,
            }],
        })
        .await
        .json::<Accepted>();
    }
    assert_eq!(
        h.count(&format!("SELECT version FROM sessions WHERE id = '{older}'"))
            .await,
        1,
        "旧版本是真的，照记"
    );
    assert_eq!(
        h.count(&format!("SELECT version FROM sessions WHERE id = '{faked}'"))
            .await,
        2,
        "比当前版本还新的按当前版本记"
    );
}

#[tokio::test]
async fn only_pages_on_the_content_domain_may_write() {
    let h = Harness::start().await;
    let slug = h.live_site().await;
    let batch = EventBatch {
        session: session_id('e'),
        slug: slug.clone(),
        events: vec![event("load")],
    };

    let allowed = h.from_player(ingest_paths::EVENTS, &slug, &batch).await;
    assert_eq!(allowed.status, StatusCode::OK);
    assert_eq!(
        allowed.header("access-control-allow-origin"),
        Some(format!("http://{slug}.localhost:8443").as_str()),
        "允许的来源要原样回显，浏览器才认"
    );
    assert_eq!(allowed.header("vary"), Some("Origin"));

    for origin in [
        "https://evil.example",
        "http://localhost:8443",
        "https://playtest.run.evil.example",
        "null",
    ] {
        let denied = h.as_origin(ingest_paths::EVENTS, origin, &batch).await;
        denied.error(StatusCode::FORBIDDEN, ErrorCode::Invalid);
        assert!(
            denied.header("access-control-allow-origin").is_none(),
            "{origin} 不该拿到放行头"
        );
    }
    assert_eq!(h.count("SELECT COUNT(*) FROM session_events").await, 1);

    // 预检：开发者自己用 fetch 发 JSON 时浏览器会先问一句。
    let preflight = h
        .send(
            Request::builder()
                .method("OPTIONS")
                .uri(ingest_paths::FEEDBACK)
                .header(header::ORIGIN, format!("http://{slug}.localhost:8443"))
                .header("access-control-request-method", "POST")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(preflight.status, StatusCode::NO_CONTENT);
    assert_eq!(
        preflight.header("access-control-allow-methods"),
        Some("POST, OPTIONS")
    );
}

#[tokio::test]
async fn one_page_cannot_flood_us() {
    let h = Harness::start().await;
    let slug = h.live_site().await;
    let sid = session_id('f');
    let batch = EventBatch {
        session: sid.clone(),
        slug: slug.clone(),
        events: vec![event("event")],
    };

    let mut refused = 0;
    for _ in 0..60 {
        let reply = h.from_player(ingest_paths::EVENTS, &slug, &batch).await;
        if reply.status == StatusCode::TOO_MANY_REQUESTS {
            reply.error(StatusCode::TOO_MANY_REQUESTS, ErrorCode::QuotaExceeded);
            refused += 1;
        }
    }
    assert!(refused > 0, "一个页面连发 60 次不该全收下");

    // 换一个人还能发：桶是按会话的，不是把整个作品关掉。
    let other = EventBatch {
        session: session_id('9'),
        slug: slug.clone(),
        events: vec![event("event")],
    };
    assert_eq!(
        h.from_player(ingest_paths::EVENTS, &slug, &other).await.status,
        StatusCode::OK
    );
}

#[tokio::test]
async fn junk_batches_are_refused_line_by_line() {
    let h = Harness::start().await;
    let slug = h.live_site().await;
    let sid = session_id('7');

    // 认不出来的类型跳过，认得的照收：一条坏的不该把整批退回去。
    let mixed = EventBatch {
        session: sid.clone(),
        slug: slug.clone(),
        events: vec![event("load"), event("pageview"), event("input")],
    };
    let accepted: Accepted = h.from_player(ingest_paths::EVENTS, &slug, &mixed).await.json();
    assert_eq!(accepted.accepted, 2);

    // 一批 51 条：整批退。
    let too_many = EventBatch {
        session: sid.clone(),
        slug: slug.clone(),
        events: (0..=ingest::MAX_EVENTS_PER_BATCH).map(|_| event("event")).collect(),
    };
    let body = h
        .from_player(ingest_paths::EVENTS, &slug, &too_many)
        .await
        .error(StatusCode::BAD_REQUEST, ErrorCode::Invalid);
    assert!(body.message.contains("50"), "{}", body.message);

    // 不是门禁页发的会话 id。
    let forged = EventBatch {
        session: "'; DROP TABLE sessions; --".to_string(),
        slug: slug.clone(),
        events: vec![event("load")],
    };
    h.from_player(ingest_paths::EVENTS, &slug, &forged)
        .await
        .error(StatusCode::BAD_REQUEST, ErrorCode::Invalid);

    // 没有这个作品。
    let nowhere = EventBatch {
        session: sid.clone(),
        slug: "nobody-owns-this-11".to_string(),
        events: vec![event("load")],
    };
    h.as_origin(
        ingest_paths::EVENTS,
        "http://nobody-owns-this-11.localhost:8443",
        &nowhere,
    )
    .await
    .error(StatusCode::NOT_FOUND, ErrorCode::NotFound);

    // 体根本不是 JSON。
    let broken = h
        .send(
            Request::builder()
                .method("POST")
                .uri(ingest_paths::EVENTS)
                .header(header::ORIGIN, format!("http://{slug}.localhost:8443"))
                .body(Body::from("{ 这不是 JSON"))
                .unwrap(),
        )
        .await;
    broken.error(StatusCode::BAD_REQUEST, ErrorCode::Invalid);

    assert_eq!(h.count("SELECT COUNT(*) FROM sessions").await, 1);
}

#[tokio::test]
async fn feedback_is_one_sentence_and_stops_at_three() {
    let h = Harness::start().await;
    let slug = h.live_site().await;
    let sid = session_id('8');

    let first: FeedbackAccepted = h
        .from_player(
            ingest_paths::FEEDBACK,
            &slug,
            &FeedbackRequest {
                session: sid.clone(),
                slug: slug.clone(),
                text: "不知道要按哪个键".to_string(),
                seconds_in: Some(47),
            },
        )
        .await
        .json();
    assert_eq!(first.remaining, 2);

    // 设备、浏览器、第几版、什么时候，玩家一个字都没填。
    let (text, seconds, device, browser, version, status): (
        String,
        i64,
        String,
        String,
        i64,
        String,
    ) = h
        .one_row(
            "SELECT text, seconds_in, device, browser, version, status FROM feedback",
            &[],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                ))
            },
        )
        .await;
    assert_eq!(text, "不知道要按哪个键");
    assert_eq!(seconds, 47);
    assert_eq!((device.as_str(), browser.as_str()), ("phone", "wechat"));
    assert_eq!(version, 1);
    assert_eq!(status, "new");

    let mut last = first;
    for _ in 0..2 {
        last = h
            .from_player(
                ingest_paths::FEEDBACK,
                &slug,
                &FeedbackRequest {
                    session: sid.clone(),
                    slug: slug.clone(),
                    text: "再说一句".to_string(),
                    seconds_in: None,
                },
            )
            .await
            .json();
    }
    assert_eq!(last.remaining, 0);

    let fourth = h
        .from_player(
            ingest_paths::FEEDBACK,
            &slug,
            &FeedbackRequest {
                session: sid.clone(),
                slug: slug.clone(),
                text: "第四句".to_string(),
                seconds_in: None,
            },
        )
        .await
        .error(StatusCode::TOO_MANY_REQUESTS, ErrorCode::QuotaExceeded);
    assert!(fourth.message.contains("谢谢"), "拦也要好好说话：{}", fourth.message);
    assert_eq!(h.count("SELECT COUNT(*) FROM feedback").await, 3);

    // 空的一条：什么都不必填，但总得写点什么。
    let empty = h
        .from_player(
            ingest_paths::FEEDBACK,
            &slug,
            &FeedbackRequest {
                session: session_id('1'),
                slug: slug.clone(),
                text: "   ".to_string(),
                seconds_in: None,
            },
        )
        .await
        .error(StatusCode::BAD_REQUEST, ErrorCode::Invalid);
    assert!(!empty.message.is_empty());

    // 超长的一条：截到上限，不整条丢——玩家写了就是写了。
    h.from_player(
        ingest_paths::FEEDBACK,
        &slug,
        &FeedbackRequest {
            session: session_id('2'),
            slug: slug.clone(),
            text: "长".repeat(ingest::MAX_FEEDBACK_CHARS + 500),
            seconds_in: Some(u32::MAX),
        },
    )
    .await
    .json::<FeedbackAccepted>();
    let (chars, seconds): (i64, Option<i64>) = h
        .one_row(
            "SELECT LENGTH(text), seconds_in FROM feedback ORDER BY id DESC LIMIT 1",
            &[],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .await;
    assert_eq!(chars, ingest::MAX_FEEDBACK_CHARS as i64);
    assert_eq!(seconds, None, "一个人在一页上待一年是不可能的，那个数不要");
}

/// 玩家的浏览器不带 UA、不带 Origin 也能写——`sendBeacon` 在页面被关掉的那一刻发出去，
/// 什么都可能缺。缺的部分留空，不要因为缺一个头就把整条事件丢掉。
#[tokio::test]
async fn a_bare_request_still_counts() {
    let h = Harness::start().await;
    let slug = h.live_site().await;
    let sid = session_id('3');

    let bare = Request::builder()
        .method("POST")
        .uri(ingest_paths::EVENTS)
        .body(Body::from(
            serde_json::to_vec(&EventBatch {
                session: sid.clone(),
                slug: slug.clone(),
                events: vec![event("input")],
            })
            .unwrap(),
        ))
        .unwrap();
    let accepted: Accepted = h.send(bare).await.json();
    assert_eq!(accepted.accepted, 1);

    let (ua, device): (Option<String>, Option<String>) = h
        .one_row(
            "SELECT ua, device FROM sessions WHERE id = ?1",
            &[&sid],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .await;
    assert_eq!(ua, None, "没有 UA 就是没有，不编一个");
    assert_eq!(device, None);
}
