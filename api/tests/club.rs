//! 「俱乐部」那几样（DESIGN §3.3、§3.5）在控制面这一侧：名额与留名、群链接、
//! 公开反馈，以及它们怎么落到门禁页读的那份 `live.json`。
//!
//! 和 `plaza.rs` 一样直接喂 `Router`，不起端口。这里问的每一句都是
//! 「开发者改了一下之后，玩家那边下一秒看到的是什么」。

use axum::body::{Body, Bytes};
use axum::http::{header, Request, StatusCode};
use axum::Router;
use playtest_api::{app, live, AppState, Config};
use playtest_common::api::{
    routes as paths, AnonSessionResponse, CommitUploadResponse, CreateSiteRequest, ErrorBody,
    ErrorCode, PrepareUploadRequest, PrepareUploadResponse, Site, UpdateSiteRequest,
};
use playtest_common::hash;
use playtest_common::ingest::{
    self, edge_kind, routes as ingest_paths, EdgeBatch, EdgeEvent, FeedbackAccepted,
    FeedbackRequest,
};
use playtest_common::limits;
use playtest_common::live::SiteLive;
use playtest_common::manifest::{FileEntry, GateMode};
use playtest_common::results::{
    routes as result_paths, FeedbackItem, FeedbackList, SiteResults, UpdateFeedbackRequest,
    VersionSessions,
};
use serde::de::DeserializeOwned;
use serde::Serialize;
use tower::ServiceExt;

const INDEX_HTML: &[u8] = b"<!doctype html><meta charset=utf-8><canvas id=game></canvas>";
const EDGE_TOKEN: &str = playtest_api::config::TEST_EDGE_INGEST_TOKEN;
/// 微信里打开的那种 UA：拿来验证 `from=card` 压得过它。
const WECHAT_UA: &str =
    "Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) AppleWebKit/605.1.15 \
     (KHTML, like Gecko) Mobile/15E148 MicroMessenger/8.0.49(0x18003128)";

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

impl Harness {
    async fn start() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let config = Config {
            listen: "127.0.0.1:0".parse().unwrap(),
            data_dir: dir.path().to_path_buf(),
            site_url_template: "http://{slug}.localhost:8443".to_string(),
            github: None,
            ..Config::default()
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
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        Reply { status, body }
    }

    async fn call<T: Serialize>(
        &self,
        method: &str,
        path: &str,
        token: Option<&str>,
        value: Option<&T>,
    ) -> Reply {
        let mut builder = Request::builder().method(method).uri(path);
        if let Some(token) = token {
            builder = builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
        }
        let body = match value {
            Some(v) => {
                builder = builder.header(header::CONTENT_TYPE, "application/json");
                Body::from(serde_json::to_vec(v).unwrap())
            }
            None => Body::empty(),
        };
        self.send(builder.body(body).unwrap()).await
    }

    /// 匿名令牌 + 一个已经发过一版的作品。
    async fn live_site(&self) -> (String, String) {
        let token: AnonSessionResponse = self
            .call::<()>("POST", paths::ANON_SESSIONS, None, None)
            .await
            .json();
        let site: Site = self
            .call(
                "POST",
                paths::SITES,
                Some(&token.token),
                Some(&CreateSiteRequest::default()),
            )
            .await
            .json();
        self.publish(&token.token, &site.slug).await;
        (token.token, site.slug)
    }

    async fn publish(&self, token: &str, slug: &str) -> u32 {
        let file = FileEntry {
            path: "index.html".to_string(),
            hash: hash::hash_bytes(INDEX_HTML),
            size: INDEX_HTML.len() as u64,
        };
        let prepared: PrepareUploadResponse = self
            .call(
                "POST",
                &paths::site_uploads(slug),
                Some(token),
                Some(&PrepareUploadRequest {
                    files: vec![file.clone()],
                    title: Some("小球".to_string()),
                    note: None,
                    summary: None,
                    cover: None,
                    gate: GateMode::Once,
                    isolated: false,
                    spa: false,
                    engine: None,
                    kind: Default::default(),
                    entry: None,
                    chapters: vec![],
                }),
            )
            .await
            .json();
        for missing in &prepared.missing {
            let put = Request::builder()
                .method("PUT")
                .uri(paths::blob(missing))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::from(INDEX_HTML))
                .unwrap();
            assert!(self.send(put).await.status.is_success());
        }
        let committed: CommitUploadResponse = self
            .call(
                "POST",
                &paths::site_upload_commit(slug, &prepared.upload_id),
                Some(token),
                Some(&serde_json::json!({})),
            )
            .await
            .json();
        committed.version
    }

    async fn patch(&self, token: &str, slug: &str, request: &UpdateSiteRequest) -> Reply {
        self.call("PATCH", &paths::site(slug), Some(token), Some(request))
            .await
    }

    /// 边缘补送一批。服务器对服务器，没有 Origin。
    async fn edge_sends(&self, events: Vec<EdgeEvent>) -> Reply {
        self.send(
            Request::builder()
                .method("POST")
                .uri(ingest_paths::EDGE)
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {EDGE_TOKEN}"))
                .body(Body::from(
                    serde_json::to_vec(&EdgeBatch { events }).unwrap(),
                ))
                .unwrap(),
        )
        .await
    }

    /// 玩家在门禁页上留一句话。
    async fn say(&self, slug: &str, sid: &str, text: &str) -> FeedbackAccepted {
        self.send(
            Request::builder()
                .method("POST")
                .uri(ingest_paths::FEEDBACK)
                .header(header::CONTENT_TYPE, "text/plain;charset=UTF-8")
                .header(header::ORIGIN, format!("http://{slug}.localhost:8443"))
                .body(Body::from(
                    serde_json::to_vec(&FeedbackRequest {
                        session: sid.to_string(),
                        slug: slug.to_string(),
                        text: text.to_string(),
                        seconds_in: Some(30),
                    })
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .json()
    }

    async fn live(&self, slug: &str) -> SiteLive {
        self.state
            .store()
            .get_live(slug)
            .await
            .unwrap()
            .expect("控制面该写过 live.json 了")
    }

    async fn session_name(&self, sid: &str) -> Option<String> {
        let conn = self.state.db().lock().await;
        conn.query_row("SELECT name FROM sessions WHERE id = ?1", [sid], |row| {
            row.get(0)
        })
        .unwrap()
    }
}

/// 门禁页种下的会话 id 是 32 个十六进制字符（`ingest::is_session_id`），
/// 所以夹具也只能给这种形状——种子换成它的码位，一个种子一个 id。
fn session_id(seed: char) -> String {
    format!("{:032x}", seed as u32)
}

/// 服务端只采信最近 24 小时里的客户端时间，所以锚在「现在」附近。
fn at(offset_secs: i64) -> String {
    let base = time::OffsetDateTime::now_utc()
        .replace_nanosecond(0)
        .unwrap()
        - time::Duration::minutes(10);
    (base + time::Duration::seconds(offset_secs))
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap()
}

fn edge_event(kind: &str, slug: &str, sid: &str) -> EdgeEvent {
    EdgeEvent {
        ts: at(0),
        kind: kind.to_string(),
        slug: slug.to_string(),
        version: 1,
        sid: sid.to_string(),
        ua: WECHAT_UA.to_string(),
        referer: String::new(),
        wechat: false,
        reason: None,
        detail: None,
        from: None,
        name: None,
    }
}

/// 点了「开始」，留了个名字。
fn start_with_name(slug: &str, sid: &str, name: &str) -> EdgeEvent {
    EdgeEvent {
        name: Some(name.to_string()),
        ..edge_event(edge_kind::START, slug, sid)
    }
}

// ---------------------------------------------------------------- 设置

#[tokio::test]
async fn settings_say_what_is_wrong_in_plain_words() {
    let h = Harness::start().await;
    let (token, slug) = h.live_site().await;

    // 名额有上限，报错里要带上他写的那个数。
    let body = h
        .patch(
            &token,
            &slug,
            &UpdateSiteRequest {
                seats: Some(limits::MAX_SEATS + 1),
                ..Default::default()
            },
        )
        .await
        .error(StatusCode::BAD_REQUEST, ErrorCode::Invalid);
    assert!(
        body.message.contains(&limits::MAX_SEATS.to_string())
            && body.message.contains(&(limits::MAX_SEATS + 1).to_string()),
        "{}",
        body.message
    );

    // 群链接要能点开。
    let body = h
        .patch(
            &token,
            &slug,
            &UpdateSiteRequest {
                community_url: Some("weixin: 我的微信号".into()),
                ..Default::default()
            },
        )
        .await
        .error(StatusCode::BAD_REQUEST, ErrorCode::Invalid);
    assert!(body.message.contains("http"), "{}", body.message);

    let too_long = format!(
        "https://example.com/{}",
        "群".repeat(limits::MAX_COMMUNITY_URL_CHARS)
    );
    h.patch(
        &token,
        &slug,
        &UpdateSiteRequest {
            community_url: Some(too_long),
            ..Default::default()
        },
    )
    .await
    .error(StatusCode::BAD_REQUEST, ErrorCode::Invalid);
}

#[tokio::test]
async fn asking_for_seats_means_looking_for_players() {
    let h = Harness::start().await;
    let (token, slug) = h.live_site().await;

    let site: Site = h
        .patch(
            &token,
            &slug,
            &UpdateSiteRequest {
                seats: Some(12),
                community_url: Some("https://example.com/qq-group".into()),
                feedback_public: Some(true),
                ..Default::default()
            },
        )
        .await
        .json();
    assert_eq!(site.listing.seats, Some(12));
    assert!(site.listing.seeking, "设了名额就是在找人测");
    assert!(site.listing.public, "找人测蕴含公开");
    assert!(site.listing.feedback_public);
    assert_eq!(
        site.listing.community_url.as_deref(),
        Some("https://example.com/qq-group")
    );

    // 门禁页读的那一份跟着变了。
    let live = h.live(&slug).await;
    assert_eq!(live.seats, Some(12));
    assert!(live.feedback_public);
    assert!(live.listed && live.seeking);
    assert_eq!(
        live.community_url.as_deref(),
        Some("https://example.com/qq-group")
    );

    // 0 和空串是「清掉」。
    let site: Site = h
        .patch(
            &token,
            &slug,
            &UpdateSiteRequest {
                seats: Some(0),
                community_url: Some(String::new()),
                ..Default::default()
            },
        )
        .await
        .json();
    assert_eq!(site.listing.seats, None);
    assert_eq!(site.listing.community_url, None);
    assert!(
        site.listing.seeking,
        "清名额不等于不找人了，那是 --seek 的事"
    );

    let live = h.live(&slug).await;
    assert_eq!(live.seats, None);
    assert_eq!(live.community_url, None);
}

// ---------------------------------------------------------------- 留名与来源

#[tokio::test]
async fn a_name_at_the_start_counts_as_one_who_joined() {
    let h = Harness::start().await;
    let (token, slug) = h.live_site().await;
    h.patch(
        &token,
        &slug,
        &UpdateSiteRequest {
            seats: Some(5),
            ..Default::default()
        },
    )
    .await
    .json::<Site>();

    let named = session_id('a');
    let dirty = session_id('b');
    let quiet = session_id('c');
    h.edge_sends(vec![
        start_with_name(&slug, &named, "  小王\n  "),
        start_with_name(&slug, &dirty, "傻逼玩家"),
        edge_event(edge_kind::START, &slug, &quiet),
    ])
    .await
    .json::<ingest::Accepted>();

    assert_eq!(
        h.session_name(&named).await.as_deref(),
        Some("小王"),
        "两头的空白和换行要去掉"
    );
    assert_eq!(
        h.session_name(&dirty).await,
        None,
        "脏词当没留名，但人照样进得去"
    );
    assert_eq!(h.session_name(&quiet).await, None);

    // 「已加入」只数留了名的那些。
    let live = h.live(&slug).await;
    assert_eq!(live.joined, 1);
    assert_eq!(live.seats, Some(5));
    assert!(!live.seats_full());

    let site: Site = h
        .call::<()>("GET", &paths::site(&slug), Some(&token), None)
        .await
        .json();
    assert_eq!(site.listing.joined, 1);

    // 同一个人再点一次开始，改不了已经留下的名字。
    h.edge_sends(vec![start_with_name(&slug, &named, "另一个名字")])
        .await
        .json::<ingest::Accepted>();
    assert_eq!(h.session_name(&named).await.as_deref(), Some("小王"));
    assert_eq!(h.live(&slug).await.joined, 1);
}

#[tokio::test]
async fn an_overlong_name_is_cut_by_characters_not_bytes() {
    let h = Harness::start().await;
    let (_token, slug) = h.live_site().await;
    let sid = session_id('d');
    // 汉字一个字三个字节：按字节截会把最后一个字切成乱码。
    let long = "汉".repeat(limits::MAX_PLAYER_NAME_CHARS + 10);
    h.edge_sends(vec![start_with_name(&slug, &sid, &long)])
        .await
        .json::<ingest::Accepted>();
    let name = h.session_name(&sid).await.unwrap();
    assert_eq!(name.chars().count(), limits::MAX_PLAYER_NAME_CHARS);
    assert!(name.chars().all(|c| c == '汉'));
}

#[tokio::test]
async fn the_card_beats_the_wechat_ua() {
    let h = Harness::start().await;
    let (token, slug) = h.live_site().await;

    let from_card = session_id('e');
    let just_wechat = session_id('f');
    h.edge_sends(vec![
        EdgeEvent {
            from: Some(ingest::source::CARD.to_string()),
            wechat: true,
            ..edge_event(edge_kind::GATE_VIEW, &slug, &from_card)
        },
        EdgeEvent {
            wechat: true,
            ..edge_event(edge_kind::GATE_VIEW, &slug, &just_wechat)
        },
    ])
    .await
    .json::<ingest::Accepted>();

    let roster: VersionSessions = h
        .call::<()>(
            "GET",
            &result_paths::site_version_sessions(&slug, 1),
            Some(&token),
            None,
        )
        .await
        .json();
    let kind_of = |id: &str| {
        roster
            .sessions
            .iter()
            .find(|s| s.id == id)
            .unwrap_or_else(|| panic!("点名册里没有 {id}"))
            .referrer_kind
            .clone()
    };
    assert_eq!(
        kind_of(&from_card).as_deref(),
        Some(ingest::source::CARD),
        "扫卡的人多半也在微信里，开发者想知道的是那张卡"
    );
    assert_eq!(
        kind_of(&just_wechat).as_deref(),
        Some(ingest::source::WECHAT)
    );
}

// ---------------------------------------------------------------- 结果

#[tokio::test]
async fn the_timeline_counts_sources_and_names() {
    let h = Harness::start().await;
    let (token, slug) = h.live_site().await;

    let one = session_id('1');
    let two = session_id('2');
    let three = session_id('3');
    h.edge_sends(vec![
        EdgeEvent {
            from: Some(ingest::source::CARD.to_string()),
            ..edge_event(edge_kind::GATE_VIEW, &slug, &one)
        },
        start_with_name(&slug, &one, "小王"),
        EdgeEvent {
            from: Some(ingest::source::CARD.to_string()),
            ..edge_event(edge_kind::GATE_VIEW, &slug, &two)
        },
        EdgeEvent {
            from: Some(ingest::source::NOTICE.to_string()),
            ..edge_event(edge_kind::GATE_VIEW, &slug, &three)
        },
        start_with_name(&slug, &three, "小李"),
    ])
    .await
    .json::<ingest::Accepted>();

    let results: SiteResults = h
        .call::<()>(
            "GET",
            &result_paths::site_results(&slug),
            Some(&token),
            None,
        )
        .await
        .json();
    let v1 = results
        .versions
        .iter()
        .find(|v| v.version == 1)
        .expect("时间线里该有 v1");
    assert_eq!(v1.named, 2);
    let kinds: Vec<(&str, u32)> = v1
        .sources
        .iter()
        .map(|s| (s.kind.as_str(), s.count))
        .collect();
    assert_eq!(
        kinds,
        vec![(ingest::source::CARD, 2), (ingest::source::NOTICE, 1)],
        "多的在前，0 的不列"
    );
    assert_eq!(results.followers, 0, "还没有人关注");

    // 点名册里带名字。
    let roster: VersionSessions = h
        .call::<()>(
            "GET",
            &result_paths::site_version_sessions(&slug, 1),
            Some(&token),
            None,
        )
        .await
        .json();
    let mut names: Vec<String> = roster
        .sessions
        .iter()
        .filter_map(|s| s.name.clone())
        .collect();
    names.sort();
    assert_eq!(names, vec!["小李".to_string(), "小王".to_string()]);
}

// ---------------------------------------------------------------- 公开反馈

#[tokio::test]
async fn public_feedback_reaches_the_gate_and_can_be_taken_back() {
    let h = Harness::start().await;
    let (token, slug) = h.live_site().await;
    let sid = session_id('g');

    h.edge_sends(vec![start_with_name(&slug, &sid, "小王")])
        .await
        .json::<ingest::Accepted>();
    h.say(&slug, &sid, "第三关的跳跃判定有点飘").await;

    // 还没打开开关：反馈在开发者那儿看得到，但不是公开的。
    let list: FeedbackList = h
        .call::<()>(
            "GET",
            &result_paths::site_feedback(&slug),
            Some(&token),
            None,
        )
        .await
        .json();
    let item = &list.items[0];
    assert_eq!(item.name.as_deref(), Some("小王"), "署名从会话上来");
    assert!(!item.public);
    assert!(h.live(&slug).await.public_feedback.is_empty());

    // 打开开关，门禁页上就有了。
    h.patch(
        &token,
        &slug,
        &UpdateSiteRequest {
            feedback_public: Some(true),
            ..Default::default()
        },
    )
    .await
    .json::<Site>();
    let live = h.live(&slug).await;
    assert_eq!(live.public_feedback.len(), 1);
    assert_eq!(live.public_feedback[0].name.as_deref(), Some("小王"));
    assert_eq!(live.public_feedback[0].text, "第三关的跳跃判定有点飘");
    assert_eq!(live.public_feedback[0].version, 1);

    // 单独藏一条。
    let id = item.id;
    let hidden: FeedbackItem = h
        .call(
            "PATCH",
            &result_paths::site_feedback_item(&slug, id),
            Some(&token),
            Some(&UpdateFeedbackRequest {
                public: Some(false),
                ..Default::default()
            }),
        )
        .await
        .json();
    assert!(!hidden.public);
    assert!(
        h.live(&slug).await.public_feedback.is_empty(),
        "藏起来之后门禁页上就不该还有"
    );

    // 再放出来。
    let shown: FeedbackItem = h
        .call(
            "PATCH",
            &result_paths::site_feedback_item(&slug, id),
            Some(&token),
            Some(&UpdateFeedbackRequest {
                public: Some(true),
                ..Default::default()
            }),
        )
        .await
        .json();
    assert!(shown.public);
    assert_eq!(h.live(&slug).await.public_feedback.len(), 1);

    // 什么都不带的请求要说清楚。
    let body = h
        .call(
            "PATCH",
            &result_paths::site_feedback_item(&slug, id),
            Some(&token),
            Some(&UpdateFeedbackRequest::default()),
        )
        .await
        .error(StatusCode::BAD_REQUEST, ErrorCode::Invalid);
    assert!(body.message.contains("status"), "{}", body.message);
}

#[tokio::test]
async fn the_gate_shows_the_newest_three_public_notes() {
    let h = Harness::start().await;
    let (token, slug) = h.live_site().await;
    h.patch(
        &token,
        &slug,
        &UpdateSiteRequest {
            feedback_public: Some(true),
            ..Default::default()
        },
    )
    .await
    .json::<Site>();

    for (i, seed) in ['h', 'i', 'j', 'k'].iter().enumerate() {
        let sid = session_id(*seed);
        h.say(&slug, &sid, &format!("第 {i} 句")).await;
    }
    // 新来一条反馈不单独重写 live.json（玩家留话很密），靠那一档 5 分钟的兜底。
    live::publish(&h.state, &slug).await;
    let live = h.live(&slug).await;
    assert_eq!(
        live.public_feedback.len(),
        playtest_common::live::PUBLIC_FEEDBACK_ON_GATE
    );
    assert_eq!(live.public_feedback[0].text, "第 3 句", "新的在前");
}

#[tokio::test]
async fn setting_a_cover_from_a_note_says_there_is_no_screenshot_yet() {
    let h = Harness::start().await;
    let (token, slug) = h.live_site().await;
    let sid = session_id('l');
    h.say(&slug, &sid, "这里有个洞").await;
    let list: FeedbackList = h
        .call::<()>(
            "GET",
            &result_paths::site_feedback(&slug),
            Some(&token),
            None,
        )
        .await
        .json();

    // 玩家留话时还附不了截图（那是 v0.2），所以「把截图设成封面」这条路整条都还不存在：
    // 与其留一个永远拒绝的端点、控制台里一个永远不出现的按钮，不如等截图做出来再一起加。
    assert!(list.items[0].screenshot_hash.is_none(), "截图是 v0.2 的事");
}

// ---------------------------------------------------------------- capabilities

#[tokio::test]
async fn capabilities_tell_the_edge_what_this_machine_can_do() {
    let h = Harness::start().await;
    playtest_api::capabilities::publish(&h.state).await;
    let caps = h
        .state
        .store()
        .get_capabilities()
        .await
        .unwrap()
        .expect("启动时该写过 capabilities.json");
    assert_eq!(caps.schema, playtest_common::capabilities::SCHEMA);
    assert!(caps.email, "默认是 log：信发得出去（只是发进日志）");
    let key = caps.push_public_key.expect("VAPID 公钥该有");
    // 未压缩的 P-256 公钥是 65 字节，base64url 之后 87 个字符。
    assert_eq!(key.len(), 87, "{key}");
    assert!(!key.contains('='), "base64url 不带填充");
}

#[tokio::test]
async fn turning_email_off_shows_up_in_capabilities() {
    let dir = tempfile::tempdir().unwrap();
    let mut config = Config {
        listen: "127.0.0.1:0".parse().unwrap(),
        data_dir: dir.path().to_path_buf(),
        site_url_template: "http://{slug}.localhost:8443".to_string(),
        github: None,
        ..Config::default()
    };
    config.notify.email = playtest_api::config::EmailProvider::Off;
    let state = AppState::from_config(&config).await.unwrap();
    playtest_api::capabilities::publish(&state).await;
    let caps = state.store().get_capabilities().await.unwrap().unwrap();
    assert!(!caps.email, "明确关掉发信，边缘就不显示留邮箱那一栏");
    assert!(caps.push_public_key.is_some(), "推送和发信是两件事");
}
