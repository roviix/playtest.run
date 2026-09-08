//! 结果层：往库里塞几个会话，看点名册和那一版的摘要说得对不对。
//!
//! 写入端（SDK 与边缘）还在另一条线上做，所以这里直接插行——要验的是**聚合**：
//! L7 在没有 SDK 时会不会假装知道、中位数在 0 / 1 / 偶数个会话上取哪一个、
//! 点名册默认是不是把停留最短的人排在最前面。
//!
//! 请求走的是真的 `Router`（`tower::ServiceExt::oneshot`），中间层和鉴权都在。

// 测试脚手架：为了把一条用例写成一眼能看完的样子，这里放宽 too_many_arguments。
#![allow(clippy::too_many_arguments)]

use axum::body::{Body, Bytes};
use axum::http::{header, Request, StatusCode};
use axum::Router;
use playtest_api::{app, AppState, Config};
use playtest_common::api::{
    routes as paths, AnonSessionResponse, CreateSiteRequest, ErrorBody, ErrorCode, Site,
};
use playtest_common::results::{
    routes as result_paths, FeedbackItem, FeedbackList, FeedbackStatus, RosterSort, SiteResults,
    UpdateFeedbackRequest, VersionResults, VersionSessions,
};
use rusqlite::params;
use serde::de::DeserializeOwned;
use serde::Serialize;
use tower::ServiceExt;

struct Harness {
    router: Router,
    state: AppState,
    _dir: tempfile::TempDir,
}

struct Reply {
    status: StatusCode,
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

    fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }
}

/// 一个会话。默认是「打开了、点了开始、报了首帧」的那种人，测试里只改要改的几列。
#[derive(Clone)]
struct Sess {
    id: &'static str,
    version: u32,
    first: &'static str,
    last: &'static str,
    start: Option<&'static str>,
    first_frame: Option<&'static str>,
    last_input: Option<&'static str>,
    device: &'static str,
    browser: &'static str,
    os: &'static str,
    referrer: &'static str,
    wechat: bool,
    is_return: bool,
}

impl Default for Sess {
    fn default() -> Self {
        Self {
            id: "s0",
            version: 7,
            first: "2026-09-05T14:00:00Z",
            last: "2026-09-05T14:00:30Z",
            start: Some("2026-09-05T14:00:05Z"),
            first_frame: Some("2026-09-05T14:00:08Z"),
            last_input: None,
            device: "desktop",
            browser: "chrome",
            os: "macos",
            referrer: "direct",
            wechat: false,
            is_return: false,
        }
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

    async fn request(&self, method: &str, path: &str, token: Option<&str>, body: Body) -> Reply {
        let mut builder = Request::builder().method(method).uri(path);
        if let Some(token) = token {
            builder = builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
        }
        if !matches!(method, "GET" | "DELETE") {
            builder = builder.header(header::CONTENT_TYPE, "application/json");
        }
        let response = self
            .router
            .clone()
            .oneshot(builder.body(body).unwrap())
            .await
            .unwrap();
        let status = response.status();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        Reply { status, body }
    }

    async fn get(&self, path: &str, token: &str) -> Reply {
        self.request("GET", path, Some(token), Body::empty()).await
    }

    async fn patch<T: Serialize>(&self, path: &str, token: &str, value: &T) -> Reply {
        let body = Body::from(serde_json::to_vec(value).unwrap());
        self.request("PATCH", path, Some(token), body).await
    }

    async fn anon_token(&self) -> String {
        self.request("POST", paths::ANON_SESSIONS, None, Body::empty())
            .await
            .json::<AnonSessionResponse>()
            .token
    }

    async fn new_site(&self, token: &str) -> Site {
        let body = Body::from(serde_json::to_vec(&CreateSiteRequest::default()).unwrap());
        self.request("POST", paths::SITES, Some(token), body)
            .await
            .json()
    }

    async fn version(&self, slug: &str, version: u32, created_at: &str, note: &str) {
        let conn = self.state.db().lock().await;
        conn.execute(
            "INSERT INTO versions (slug, version, created_at, note, file_count, total_bytes)
             VALUES (?1, ?2, ?3, ?4, 1, 100)",
            params![slug, version, created_at, note],
        )
        .unwrap();
    }

    async fn session(&self, slug: &str, s: &Sess) {
        let conn = self.state.db().lock().await;
        conn.execute(
            "INSERT INTO sessions (id, slug, version, first_seen_at, last_seen_at, ua, device,
                                   browser, os, referrer_kind, wechat, gate_view_at, start_at,
                                   first_frame_at, load_ms, last_input_at, is_return)
             VALUES (?1, ?2, ?3, ?4, ?5, NULL, ?6, ?7, ?8, ?9, ?10, ?4, ?11, ?12, NULL, ?13, ?14)",
            params![
                s.id,
                slug,
                s.version,
                s.first,
                s.last,
                s.device,
                s.browser,
                s.os,
                s.referrer,
                s.wechat as i64,
                s.start,
                s.first_frame,
                s.last_input,
                s.is_return as i64,
            ],
        )
        .unwrap();
    }

    async fn event(
        &self,
        slug: &str,
        session: &str,
        version: u32,
        ts: &str,
        source: &str,
        kind: &str,
        name: Option<&str>,
        data: Option<&str>,
    ) {
        let conn = self.state.db().lock().await;
        conn.execute(
            "INSERT INTO session_events (session_id, slug, version, ts, source, kind, name, data)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![session, slug, version, ts, source, kind, name, data],
        )
        .unwrap();
    }

    async fn feedback(
        &self,
        slug: &str,
        session: &str,
        version: u32,
        ts: &str,
        text: &str,
        seconds_in: u32,
    ) {
        let conn = self.state.db().lock().await;
        conn.execute(
            "INSERT INTO feedback (session_id, slug, version, ts, text, seconds_in, device, browser, status)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'phone', 'safari', 'new')",
            params![session, slug, version, ts, text, seconds_in],
        )
        .unwrap();
    }
}

fn find(results: &SiteResults, version: u32) -> VersionResults {
    results
        .versions
        .iter()
        .find(|v| v.version == version)
        .unwrap_or_else(|| panic!("时间线里没有 v{version}"))
        .clone()
}

/// DESIGN §3.4 那段话要用到的每一个数，在一份 8 个人的会话上对一遍。
#[tokio::test]
async fn one_version_adds_up_to_the_paragraph() {
    let h = Harness::start().await;
    let token = h.anon_token().await;
    let site = h.new_site(&token).await;
    let slug = &site.slug;
    h.version(slug, 7, "2026-09-05T14:20:00Z", "改了新手引导")
        .await;

    // 六个人进到了游戏，其中三个玩了 5 分钟以上、两个是回头的。
    let long_ones = [
        ("s2", "2026-09-05T14:12:00Z", false),
        ("s3", "2026-09-05T14:13:40Z", true),
        ("s4", "2026-09-05T14:18:00Z", true),
    ];
    for (id, last, is_return) in long_ones {
        h.session(
            slug,
            &Sess {
                id,
                last,
                is_return,
                first: "2026-09-05T14:02:00Z",
                ..Default::default()
            },
        )
        .await;
    }
    for (id, last) in [
        ("s1", "2026-09-05T14:00:30Z"),
        ("s5", "2026-09-05T14:02:00Z"),
        ("s6", "2026-09-05T14:00:45Z"),
    ] {
        h.session(
            slug,
            &Sess {
                id,
                last,
                ..Default::default()
            },
        )
        .await;
    }
    // 两个点了开始但没等到首帧——L7 数的就是这两个人。
    h.session(
        slug,
        &Sess {
            id: "s7",
            last: "2026-09-05T14:00:08Z",
            first_frame: None,
            device: "phone",
            browser: "wechat",
            os: "android",
            referrer: "wechat",
            wechat: true,
            ..Default::default()
        },
    )
    .await;
    h.session(
        slug,
        &Sess {
            id: "s8",
            last: "2026-09-05T14:00:12Z",
            first_frame: None,
            ..Default::default()
        },
    )
    .await;

    // 一个错误撞了三次（两个人身上），外加一条别的。
    for (session, ts) in [
        ("s1", "2026-09-05T14:00:20Z"),
        ("s1", "2026-09-05T14:00:22Z"),
        ("s5", "2026-09-05T14:01:00Z"),
    ] {
        h.event(
            slug,
            session,
            7,
            ts,
            "sdk",
            "error",
            Some("TypeError: Cannot read 'x' of undefined @ main.js:412"),
            None,
        )
        .await;
    }
    h.event(
        slug,
        "s6",
        7,
        "2026-09-05T14:00:40Z",
        "sdk",
        "error",
        Some("RangeError @ boot.js:8"),
        None,
    )
    .await;
    // 边缘报的加载失败。
    h.event(
        slug,
        "s7",
        7,
        "2026-09-05T14:00:07Z",
        "edge",
        "resource_fail",
        Some("game.wasm"),
        None,
    )
    .await;
    h.feedback(
        slug,
        "s2",
        7,
        "2026-09-05T14:03:00Z",
        "不知道要按哪个键",
        47,
    )
    .await;

    let results: SiteResults = h
        .get(&result_paths::site_results(slug), &token)
        .await
        .json();
    let v7 = find(&results, 7);

    assert_eq!(v7.note.as_deref(), Some("改了新手引导"));
    assert_eq!(v7.created_at.as_deref(), Some("2026-09-05T14:20:00Z"));
    assert_eq!(v7.opened, 8);
    assert_eq!(v7.entered, 8, "八个人都点了开始");
    assert_eq!(
        v7.dropped_before_first_frame,
        Some(2),
        "两个人点了开始却没等到首帧"
    );
    // 控制台那句「6 个人进到游戏」就是这么来的。
    assert_eq!(v7.entered - v7.dropped_before_first_frame.unwrap(), 6);
    assert_eq!(v7.played_5min_plus, 3);
    assert_eq!(v7.returned, 2);
    // 停留：8、12、30、45、120、600、700、960 秒，取偏小的那个中位数。
    assert_eq!(v7.dwell_median_s, Some(45));
    assert_eq!(v7.errors.distinct, 2);
    assert_eq!(v7.errors.total, 4);
    assert_eq!(v7.errors.top[0].count, 3, "撞得最多的排最前面");
    assert!(v7.errors.top[0].fingerprint.starts_with("TypeError"));
    assert_eq!(v7.load_failures, 1);
    assert_eq!(v7.feedback_count, 1);
    assert_eq!(v7.first_at.as_deref(), Some("2026-09-05T14:00:00Z"));
    assert_eq!(v7.last_at.as_deref(), Some("2026-09-05T14:18:00Z"));

    // 一个比例、一个平均值都不该出现在响应里（DESIGN §3.4）。
    let raw = serde_json::to_string(&results).unwrap();
    for banned in ["rate", "avg", "percent", "ratio"] {
        assert!(!raw.contains(banned), "结果层里不该有 {banned}：{raw}");
    }
}

/// 没接 SDK 的那一版，L7 是「不知道」，不是「一个都没掉」。
#[tokio::test]
async fn without_the_sdk_l7_says_it_does_not_know() {
    let h = Harness::start().await;
    let token = h.anon_token().await;
    let site = h.new_site(&token).await;
    let slug = &site.slug;

    for id in ["a", "b", "c"] {
        h.session(
            slug,
            &Sess {
                id,
                first_frame: None,
                ..Default::default()
            },
        )
        .await;
    }

    let results: SiteResults = h
        .get(&result_paths::site_results(slug), &token)
        .await
        .json();
    let v7 = find(&results, 7);
    assert_eq!(v7.opened, 3);
    assert_eq!(v7.entered, 3, "点了开始就算进来了");
    assert_eq!(v7.dropped_before_first_frame, None);
    assert_eq!(v7.errors.total, 0);

    // 序列化出去是 null，控制台照这个判断该不该说那半句话。
    let raw: serde_json::Value =
        serde_json::from_slice(&h.get(&result_paths::site_results(slug), &token).await.body)
            .unwrap();
    assert!(raw["versions"][0]["dropped_before_first_frame"].is_null());
}

/// 中位数的三条边界：一个人都没有、只有一个、偶数个。
#[tokio::test]
async fn median_holds_at_zero_one_and_even() {
    let h = Harness::start().await;
    let token = h.anon_token().await;
    let site = h.new_site(&token).await;
    let slug = &site.slug;

    // v1：上传过但没人打开。
    h.version(slug, 1, "2026-09-05T10:00:00Z", "第一版").await;
    // v2：一个人。
    h.session(
        slug,
        &Sess {
            id: "only",
            version: 2,
            first: "2026-09-05T11:00:00Z",
            last: "2026-09-05T11:01:40Z",
            ..Default::default()
        },
    )
    .await;
    // v3：四个人，10 / 20 / 30 / 40 秒。
    for (id, last) in [
        ("q1", "2026-09-05T12:00:10Z"),
        ("q2", "2026-09-05T12:00:20Z"),
        ("q3", "2026-09-05T12:00:30Z"),
        ("q4", "2026-09-05T12:00:40Z"),
    ] {
        h.session(
            slug,
            &Sess {
                id,
                version: 3,
                first: "2026-09-05T12:00:00Z",
                last,
                ..Default::default()
            },
        )
        .await;
    }

    let results: SiteResults = h
        .get(&result_paths::site_results(slug), &token)
        .await
        .json();
    assert_eq!(
        results
            .versions
            .iter()
            .map(|v| v.version)
            .collect::<Vec<_>>(),
        vec![3, 2, 1],
        "版本倒序，没人打开的那一版也要在"
    );

    let v1 = find(&results, 1);
    assert_eq!(v1.opened, 0);
    assert_eq!(v1.dwell_median_s, None, "没有人就没有中位数，不是 0");
    assert_eq!(v1.first_at, None);
    assert_eq!(v1.dropped_before_first_frame, None);

    assert_eq!(find(&results, 2).dwell_median_s, Some(100));
    // 偶数个取偏小的那一个：20 秒是某个真实会话的秒数，25 不是任何人的。
    assert_eq!(find(&results, 3).dwell_median_s, Some(20));
}

/// 点名册默认把停留最短的人排在最前面，`?sort=time` 换成最近在前。
#[tokio::test]
async fn the_roster_puts_the_shortest_stay_first() {
    let h = Harness::start().await;
    let token = h.anon_token().await;
    let site = h.new_site(&token).await;
    let slug = &site.slug;

    h.session(
        slug,
        &Sess {
            id: "早来玩很久",
            first: "2026-09-05T14:00:00Z",
            last: "2026-09-05T14:20:00Z",
            last_input: Some("2026-09-05T14:19:00Z"),
            ..Default::default()
        },
    )
    .await;
    h.session(
        slug,
        &Sess {
            id: "晚来就走",
            first: "2026-09-05T15:00:00Z",
            last: "2026-09-05T15:00:09Z",
            start: Some("2026-09-05T15:00:03Z"),
            first_frame: None,
            device: "phone",
            browser: "wechat",
            os: "android",
            referrer: "wechat",
            wechat: true,
            is_return: true,
            ..Default::default()
        },
    )
    .await;
    h.event(
        slug,
        "早来玩很久",
        7,
        "2026-09-05T14:05:00Z",
        "sdk",
        "event",
        Some("第一关过了"),
        None,
    )
    .await;
    h.event(
        slug,
        "早来玩很久",
        7,
        "2026-09-05T14:12:00Z",
        "sdk",
        "event",
        Some("第二关过了"),
        None,
    )
    .await;
    h.event(
        slug,
        "早来玩很久",
        7,
        "2026-09-05T14:13:00Z",
        "sdk",
        "error",
        Some("TypeError @ main.js:412"),
        Some(r#"{"stack":"main.js:412"}"#),
    )
    .await;
    h.feedback(
        slug,
        "早来玩很久",
        7,
        "2026-09-05T14:14:00Z",
        "不知道要按哪个键",
        840,
    )
    .await;

    let roster: VersionSessions = h
        .get(&result_paths::site_version_sessions(slug, 7), &token)
        .await
        .json();
    assert_eq!(roster.sort, RosterSort::Dwell);
    assert_eq!(
        roster
            .sessions
            .iter()
            .map(|s| s.id.as_str())
            .collect::<Vec<_>>(),
        vec!["晚来就走", "早来玩很久"]
    );

    let quitter = &roster.sessions[0];
    assert_eq!(quitter.dwell_s, 9);
    assert!(
        quitter.started && !quitter.first_frame,
        "点了开始，没等到首帧"
    );
    assert!(quitter.wechat);
    assert_eq!(quitter.browser.as_deref(), Some("wechat"));
    assert_eq!(quitter.referrer_kind.as_deref(), Some("wechat"));
    assert!(quitter.is_return);
    assert_eq!(quitter.reached, None, "一个自定义事件都没打到");
    assert_eq!(quitter.errors, 0);
    assert_eq!(quitter.feedback, 0);

    let stayer = &roster.sessions[1];
    assert_eq!(stayer.dwell_s, 20 * 60);
    assert_eq!(
        stayer.reached.as_deref(),
        Some("第二关过了"),
        "玩到哪 = 最后一个自定义事件"
    );
    assert_eq!(stayer.errors, 1);
    assert_eq!(stayer.feedback, 1);
    // 最后一次输入距「进入」多久：14:00:05 点的开始，14:19:00 最后动的。
    assert_eq!(stayer.last_input_after_s, Some(18 * 60 + 55));
    // 展开这一行看到的是这个会话自己的事件，时间正序。
    assert_eq!(stayer.events.len(), 3);
    assert_eq!(stayer.events[0].name.as_deref(), Some("第一关过了"));
    assert_eq!(stayer.events[2].kind, "error");
    assert_eq!(
        stayer.events[2].data.as_ref().unwrap()["stack"],
        "main.js:412"
    );
    assert!(!stayer.more_events);

    let by_time: VersionSessions = h
        .get(
            &format!("{}?sort=time", result_paths::site_version_sessions(slug, 7)),
            &token,
        )
        .await
        .json();
    assert_eq!(by_time.sort, RosterSort::Time);
    assert_eq!(by_time.sessions[0].id, "晚来就走", "最近打开的排最前面");

    let bad = h
        .get(
            &format!("{}?sort=最短", result_paths::site_version_sessions(slug, 7)),
            &token,
        )
        .await
        .error(StatusCode::BAD_REQUEST, ErrorCode::Invalid);
    assert!(
        bad.message.contains("dwell"),
        "报错要说清能填什么：{}",
        bad.message
    );
}

/// 反馈流：按版本和状态筛，标记已看 / 已处理。
#[tokio::test]
async fn feedback_can_be_filtered_and_marked() {
    let h = Harness::start().await;
    let token = h.anon_token().await;
    let site = h.new_site(&token).await;
    let slug = &site.slug;

    h.session(
        slug,
        &Sess {
            id: "p1",
            ..Default::default()
        },
    )
    .await;
    h.session(
        slug,
        &Sess {
            id: "p2",
            version: 6,
            ..Default::default()
        },
    )
    .await;
    h.feedback(
        slug,
        "p1",
        7,
        "2026-09-05T14:03:00Z",
        "不知道要按哪个键",
        47,
    )
    .await;
    h.feedback(slug, "p1", 7, "2026-09-05T14:09:00Z", "第三关太难了", 380)
        .await;
    h.feedback(slug, "p2", 6, "2026-09-04T10:00:00Z", "上一版的反馈", 12)
        .await;

    let all: FeedbackList = h
        .get(&result_paths::site_feedback(slug), &token)
        .await
        .json();
    assert_eq!(all.items.len(), 3);
    assert_eq!(all.items[0].text, "第三关太难了", "最新的在最前面");
    assert_eq!(all.items[0].seconds_in, Some(380));
    assert_eq!(all.items[0].status, FeedbackStatus::New);
    assert_eq!(all.items[0].device.as_deref(), Some("phone"));
    assert!(all.items[0].screenshot_hash.is_none(), "截图 v0.2 才做");

    let v7: FeedbackList = h
        .get(
            &format!("{}?version=7", result_paths::site_feedback(slug)),
            &token,
        )
        .await
        .json();
    assert_eq!(v7.items.len(), 2);

    let id = all.items[0].id;
    let updated: FeedbackItem = h
        .patch(
            &result_paths::site_feedback_item(slug, id),
            &token,
            &UpdateFeedbackRequest {
                status: FeedbackStatus::Done,
            },
        )
        .await
        .json();
    assert_eq!(updated.status, FeedbackStatus::Done);

    let still_new: FeedbackList = h
        .get(
            &format!("{}?status=new", result_paths::site_feedback(slug)),
            &token,
        )
        .await
        .json();
    assert_eq!(still_new.items.len(), 2);
    assert!(still_new.items.iter().all(|i| i.id != id));

    // 不存在的那条和别人的那条说同一句话。
    h.patch(
        &result_paths::site_feedback_item(slug, 9999),
        &token,
        &UpdateFeedbackRequest {
            status: FeedbackStatus::Seen,
        },
    )
    .await
    .error(StatusCode::NOT_FOUND, ErrorCode::NotFound);

    let bad = h
        .get(
            &format!("{}?status=已读", result_paths::site_feedback(slug)),
            &token,
        )
        .await
        .error(StatusCode::BAD_REQUEST, ErrorCode::Invalid);
    assert!(bad.message.contains("done"), "{}", bad.message);
}

/// 结果只给作品的主人看。
#[tokio::test]
async fn results_are_only_visible_to_the_owner() {
    let h = Harness::start().await;
    let mine = h.anon_token().await;
    let theirs = h.anon_token().await;
    let site = h.new_site(&mine).await;
    let slug = &site.slug;
    h.session(slug, &Sess::default()).await;
    h.feedback(slug, "s0", 7, "2026-09-05T14:03:00Z", "看不到我", 10)
        .await;

    for path in [
        result_paths::site_results(slug),
        result_paths::site_version_sessions(slug, 7),
        result_paths::site_feedback(slug),
    ] {
        h.get(&path, &theirs)
            .await
            .error(StatusCode::NOT_FOUND, ErrorCode::NotFound);
        h.request("GET", &path, None, Body::empty())
            .await
            .error(StatusCode::UNAUTHORIZED, ErrorCode::Unauthorized);
    }

    h.patch(
        &result_paths::site_feedback_item(slug, 1),
        &theirs,
        &UpdateFeedbackRequest {
            status: FeedbackStatus::Done,
        },
    )
    .await
    .error(StatusCode::NOT_FOUND, ErrorCode::NotFound);

    // 主人自己看得见。
    let ok: SiteResults = h.get(&result_paths::site_results(slug), &mine).await.json();
    assert_eq!(ok.slug, *slug);
    assert_eq!(find(&ok, 7).opened, 1);
}
