//! 邀请卡、分享页、关注登记、关注页、广场这五样，打真的 Router 走一遍
//! （DESIGN §3.4、§3.6、§3.9、§3.10）。
//!
//! 关注那几条要跟控制面说话，所以这里起一个假控制面：一个真的 axum 服务，
//! 绑随机端口，按脚本回答。它不是 mock 对象——边缘那边走的是真的 TCP、真的 HTTP、
//! 真的 5 秒超时，「控制面不在」这一种失败形态只有这样才验得到。

use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use axum::body::{Body, Bytes};
use axum::extract::{OriginalUri, State};
use axum::http::{HeaderMap, Request, StatusCode};
use axum::response::IntoResponse;
use axum::routing::post;
use axum::{Json, Router};
use http_body_util::BodyExt;
use playtest_common::capabilities::Capabilities;
use playtest_common::follow::{root_paths, routes, ConfirmResponse, FollowResponse, FollowView};
use playtest_common::follow::{FollowTarget, MeView};
use playtest_common::hash::hash_bytes;
use playtest_common::live::{PublicFeedbackItem, SiteLive};
use playtest_common::manifest::{validate_manifest, FileEntry, GateMode, Manifest, SCHEMA};
use playtest_common::plaza::{Plaza, PlazaItem};
use playtest_common::store::{Current, FsStore};
use playtest_edge::{router, App, Config};
use tower::ServiceExt;

const SLUG: &str = "brisk-otter-41";
const HOST: &str = "brisk-otter-41.localhost:8443";
const ROOT: &str = "localhost:8443";
const INDEX_HTML: &str = "<!doctype html><title>小球大冒险</title>";
const ME_TOKEN: &str = "me_0123456789abcdef0123456789abcdef";

// ------------------------------------------------------------------ 假控制面

/// 它下一次要怎么答。每个测试自己摆。
#[derive(Clone)]
enum Answer {
    Follow(FollowResponse),
    /// 控制面在，但说不（401 是「这把钥匙我不认」）。
    Status(u16),
}

#[derive(Clone)]
struct FakeApi {
    answer: Arc<Mutex<Answer>>,
    calls: Arc<Mutex<Vec<(String, serde_json::Value)>>>,
    hits: Arc<AtomicUsize>,
}

impl FakeApi {
    /// 起一个真的 HTTP 服务，返回它的内网地址。
    async fn start(answer: Answer) -> (Self, String) {
        let state = FakeApi {
            answer: Arc::new(Mutex::new(answer)),
            calls: Arc::new(Mutex::new(Vec::new())),
            hits: Arc::new(AtomicUsize::new(0)),
        };
        let app = Router::new()
            .route(routes::FOLLOW, post(Self::follow))
            .route(routes::ME_SEND_LINK, post(Self::follow))
            .route(routes::CONFIRM, post(Self::confirm))
            .route(routes::UNSUBSCRIBE, post(Self::me))
            .route(routes::ME_VIEW, post(Self::me))
            .route(routes::ME_UNFOLLOW, post(Self::me))
            .route(routes::ME_PUSH_OFF, post(Self::me))
            .with_state(state.clone());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr: SocketAddr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });
        (state, format!("http://127.0.0.1:{}", addr.port()))
    }

    fn record(&self, path: &str, body: &serde_json::Value) {
        self.hits.fetch_add(1, Ordering::SeqCst);
        self.calls
            .lock()
            .unwrap()
            .push((path.to_string(), body.clone()));
    }

    fn last(&self) -> (String, serde_json::Value) {
        self.calls.lock().unwrap().last().cloned().unwrap()
    }

    async fn follow(
        State(state): State<FakeApi>,
        Json(body): Json<serde_json::Value>,
    ) -> axum::response::Response {
        state.record(routes::FOLLOW, &body);
        match state.answer.lock().unwrap().clone() {
            Answer::Follow(response) => Json(response).into_response(),
            Answer::Status(code) => StatusCode::from_u16(code).unwrap().into_response(),
        }
    }

    async fn confirm(
        State(state): State<FakeApi>,
        Json(body): Json<serde_json::Value>,
    ) -> axum::response::Response {
        state.record(routes::CONFIRM, &body);
        match state.answer.lock().unwrap().clone() {
            Answer::Status(code) => StatusCode::from_u16(code).unwrap().into_response(),
            Answer::Follow(_) => Json(ConfirmResponse {
                me_token: ME_TOKEN.into(),
                me: me_view(),
            })
            .into_response(),
        }
    }

    async fn me(
        State(state): State<FakeApi>,
        OriginalUri(uri): OriginalUri,
        Json(body): Json<serde_json::Value>,
    ) -> axum::response::Response {
        state.record(uri.path(), &body);
        match state.answer.lock().unwrap().clone() {
            Answer::Status(code) => StatusCode::from_u16(code).unwrap().into_response(),
            Answer::Follow(_) => Json(me_view()).into_response(),
        }
    }
}

fn me_view() -> MeView {
    MeView {
        email_masked: Some("z***@example.com".into()),
        push: false,
        follows: vec![
            FollowView {
                target: FollowTarget::Site { slug: SLUG.into() },
                title: Some("小球大冒险".into()),
                url: Some(format!("http://{HOST}/")),
                since: "2026-09-08T00:00:00Z".into(),
            },
            FollowView {
                target: FollowTarget::Plaza,
                title: None,
                url: None,
                since: "2026-09-08T00:00:00Z".into(),
            },
        ],
    }
}

// ------------------------------------------------------------------ 边缘这一侧

struct Site {
    _dir: tempfile::TempDir,
    app: Arc<App>,
    store: FsStore,
}

impl Site {
    async fn build(api: Option<String>, shape: impl FnOnce(&mut Manifest)) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let store = FsStore::new(dir.path().join("store"));

        let hash = hash_bytes(INDEX_HTML.as_bytes());
        store.put_blob(&hash, INDEX_HTML.as_bytes()).await.unwrap();
        let mut manifest = Manifest {
            schema: SCHEMA,
            slug: SLUG.into(),
            version: 7,
            title: "小球大冒险".into(),
            developer: "某某".into(),
            note: Some("这版改了手感".into()),
            summary: Some("三关，五分钟，手机上也能玩。".into()),
            cover: None,
            created_at: "2026-09-07T00:00:00Z".into(),
            expires_at: None,
            badge: true,
            gate: GateMode::Once,
            isolated: false,
            spa: false,
            engine: Some("phaser".into()),
            kind: Default::default(),
            entry: None,
            article: None,
            chapters: vec![],
            files: vec![FileEntry {
                path: "index.html".into(),
                hash,
                size: INDEX_HTML.len() as u64,
            }],
        };
        shape(&mut manifest);
        validate_manifest(&manifest, 512 * 1024 * 1024).unwrap();
        store.put_manifest(&manifest).await.unwrap();
        store
            .put_policy(
                &manifest.slug,
                &playtest_common::quota::Policy {
                    owner: manifest.slug.clone(),
                    plan: playtest_common::plan::Plan::Free,
                    expires_at: manifest.expires_at.clone(),
                },
            )
            .await
            .unwrap();
        store
            .set_current(
                SLUG,
                &Current {
                    version: manifest.version,
                    updated_at: "2026-09-07T00:00:01Z".into(),
                },
            )
            .await
            .unwrap();

        let config = Config {
            listen: "127.0.0.1:0".parse().unwrap(),
            data_dir: dir.path().to_path_buf(),
            host_suffix: "localhost".into(),
            public_scheme: "http".into(),
            api_internal_url: api,
            edge_ingest_token: None,
        };
        Site {
            app: Arc::new(App::new(config)),
            store,
            _dir: dir,
        }
    }

    async fn plain() -> Self {
        Self::build(None, |_| {}).await
    }

    async fn live(&self, shape: impl FnOnce(&mut SiteLive)) {
        let mut live = SiteLive::empty(SLUG);
        shape(&mut live);
        self.store.put_live(&live).await.unwrap();
    }

    async fn caps(&self, caps: Capabilities) {
        self.store.put_capabilities(&caps).await.unwrap();
    }

    async fn send(&self, request: Request<Body>) -> Reply {
        let response = router(self.app.clone()).oneshot(request).await.unwrap();
        let status = response.status();
        let headers = response.headers().clone();
        let body = response.into_body().collect().await.unwrap().to_bytes();
        Reply {
            status,
            headers,
            body,
        }
    }

    async fn get(&self, path: &str) -> Reply {
        self.send(nav_on(HOST, path).body(Body::empty()).unwrap())
            .await
    }

    async fn get_root(&self, path: &str) -> Reply {
        self.send(nav_on(ROOT, path).body(Body::empty()).unwrap())
            .await
    }

    /// 带着 `pt_me` 打根域，模拟点过确认信的那台设备。
    async fn get_root_as_me(&self, path: &str) -> Reply {
        self.send(
            nav_on(ROOT, path)
                .header("cookie", format!("pt_me={ME_TOKEN}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
    }

    async fn post_form(&self, host: &str, path: &str, form: &str) -> Reply {
        self.send(
            Request::builder()
                .method("POST")
                .uri(path)
                .header("host", host)
                .header("origin", format!("http://{host}"))
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(form.to_string()))
                .unwrap(),
        )
        .await
    }
}

struct Reply {
    status: StatusCode,
    headers: HeaderMap,
    body: Bytes,
}

impl Reply {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name).and_then(|v| v.to_str().ok())
    }

    fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }

    fn cookies(&self) -> Vec<&str> {
        self.headers
            .get_all("set-cookie")
            .iter()
            .filter_map(|v| v.to_str().ok())
            .collect()
    }
}

fn nav_on(host: &str, path: &str) -> axum::http::request::Builder {
    Request::builder()
        .uri(path)
        .header("host", host)
        .header("accept", "text/html,application/xhtml+xml,*/*;q=0.8")
        .header("sec-fetch-dest", "document")
}

fn email_caps() -> Capabilities {
    Capabilities {
        email: true,
        ..Capabilities::default()
    }
}

// ------------------------------------------------------------------ 邀请卡

/// PNG 的宽高写在头 24 个字节里；卡是不是那个尺寸，看它就够，不用把图解码出来。
fn png_size(bytes: &[u8]) -> (u32, u32) {
    assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n", "不是 PNG");
    assert_eq!(&bytes[12..16], b"IHDR");
    let w = u32::from_be_bytes(bytes[16..20].try_into().unwrap());
    let h = u32::from_be_bytes(bytes[20..24].try_into().unwrap());
    (w, h)
}

#[tokio::test]
async fn the_invite_card_is_a_png_of_the_right_size() {
    let site = Site::plain().await;
    site.live(|l| {
        l.seats = Some(10);
        l.joined = 6;
    })
    .await;

    let reply = site.get(playtest_common::CARD_PATH).await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.header("content-type"), Some("image/png"));
    assert_eq!(
        png_size(&reply.body),
        (playtest_common::CARD_WIDTH, playtest_common::CARD_HEIGHT)
    );
    assert_eq!(reply.header("cache-control"), Some("public, max-age=300"));
    // 卡是图，永远不该被当成页面：没有 CSP，但有 nosniff。
    assert_eq!(reply.header("x-content-type-options"), Some("nosniff"));
    assert!(reply.header("content-security-policy").is_none());

    let wide = site.get(playtest_common::CARD_WIDE_PATH).await;
    assert_eq!(
        png_size(&wide.body),
        (
            playtest_common::CARD_WIDE_WIDTH,
            playtest_common::CARD_WIDE_HEIGHT
        )
    );

    // 内容一样就该命中：同一个 ETag，第二次直接 304。
    let etag = reply.header("etag").unwrap().to_string();
    let again = site.get(playtest_common::CARD_PATH).await;
    assert_eq!(again.header("etag"), Some(etag.as_str()));
    let cached = site
        .send(
            Request::builder()
                .uri(playtest_common::CARD_PATH)
                .header("host", HOST)
                .header("if-none-match", etag.as_str())
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(cached.status, StatusCode::NOT_MODIFIED);
    assert!(cached.body.is_empty());

    // HEAD 只要头，不要几十 KB 的图。
    let head = site
        .send(
            Request::builder()
                .method("HEAD")
                .uri(playtest_common::CARD_PATH)
                .header("host", HOST)
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(head.status, StatusCode::OK);
    assert!(head.body.is_empty());
    assert!(head.header("content-length").is_some());

    // 名额变了，卡就是另一张：ETag 由内容决定，不由时间决定。
    // （`live.json` 有 30 秒缓存，所以换一个 Site 来验，不去戳缓存的内部。）
    let other = Site::plain().await;
    other
        .live(|l| {
            l.seats = Some(10);
            l.joined = 9;
        })
        .await;
    let moved = other.get(playtest_common::CARD_PATH).await;
    assert_ne!(moved.header("etag"), Some(etag.as_str()));
}

// ------------------------------------------------------------------ 分享页

#[tokio::test]
async fn sharing_is_only_for_a_work_that_chose_to_be_public() {
    // 没公开：这一页不存在，和任何一个不存在的路径长得一样。
    let hidden = Site::plain().await;
    let reply = hidden.get(playtest_common::SHARE_PATH).await;
    assert_eq!(reply.status, StatusCode::NOT_FOUND);
    assert!(!reply.text().contains("Save image"));
    // 门禁页上也不提「分享」。
    assert!(!hidden
        .get("/")
        .await
        .text()
        .contains(playtest_common::SHARE_PATH));

    let site = Site::plain().await;
    site.live(|l| l.listed = true).await;
    let reply = site.get(playtest_common::SHARE_PATH).await;
    assert_eq!(reply.status, StatusCode::OK);
    let html = reply.text();
    assert!(html.contains(&format!(
        "<img class=\"shot\" src=\"{}\"",
        playtest_common::CARD_PATH
    )));
    assert!(html.contains("download"));
    assert!(html.contains("Copy link"));
    // 作品域上一个 CSP 头都不发（硬线，见 gate_hardlines）。
    assert!(reply.header("content-security-policy").is_none());
    // 保持轻量：没有外部资源。
    assert!(html.len() < 20 * 1024, "分享页 {} 字节", html.len());
}

// ------------------------------------------------------------------ 关注登记

#[tokio::test]
async fn following_a_work_from_its_own_gate() {
    let (api, base) = FakeApi::start(Answer::Follow(FollowResponse::ConfirmSent)).await;
    let site = Site::build(Some(base), |_| {}).await;
    site.caps(email_caps()).await;

    let reply = site
        .post_form(
            HOST,
            playtest_common::follow::edge_paths::FOLLOW,
            "target=site%3Abrisk-otter-41&email=zhong%40example.com&from=gate&to=%2F",
        )
        .await;
    assert_eq!(reply.status, StatusCode::OK);
    let html = reply.text();
    assert!(html.contains("Confirmation email sent to z***@example.com."));
    assert!(html.contains("Click the link in the email to confirm. Check your spam folder if it does not arrive."));
    // 邮箱不回显。
    assert!(!html.contains("zhong@example.com"));

    // 送到控制面的是契约里那个形状。
    let (path, body) = api.last();
    assert_eq!(path, routes::FOLLOW);
    assert_eq!(body["target"]["kind"], "site");
    assert_eq!(body["target"]["slug"], SLUG);
    assert_eq!(body["channel"]["kind"], "email");
    assert_eq!(body["channel"]["email"], "zhong@example.com");
    assert_eq!(body["from"], "gate");

    // 已经生效、早就关注着：各自一句人话。
    *api.answer.lock().unwrap() = Answer::Follow(FollowResponse::Subscribed);
    let reply = site
        .post_form(
            HOST,
            playtest_common::follow::edge_paths::FOLLOW,
            "target=site%3Abrisk-otter-41&email=a%40b.co&to=%2F",
        )
        .await;
    assert!(reply.text().contains("You are all set! We will notify you when a new version is released."));

    *api.answer.lock().unwrap() = Answer::Follow(FollowResponse::AlreadyFollowing);
    let reply = site
        .post_form(
            HOST,
            playtest_common::follow::edge_paths::FOLLOW,
            "target=site%3Abrisk-otter-41&email=a%40b.co&to=%2F",
        )
        .await;
    assert!(reply.text().contains("You are already following this."));
}

#[tokio::test]
async fn a_work_cannot_borrow_a_player_to_follow_another_work() {
    let (_api, base) = FakeApi::start(Answer::Follow(FollowResponse::ConfirmSent)).await;
    let site = Site::build(Some(base), |_| {}).await;
    let reply = site
        .post_form(
            HOST,
            playtest_common::follow::edge_paths::FOLLOW,
            "target=site%3Asomeone-else-99&email=a%40b.co",
        )
        .await;
    assert_eq!(reply.status, StatusCode::BAD_REQUEST);
    assert!(reply.text().contains("This link is incomplete, please go back and try again."));

    // 不像邮箱的东西也是 400，但页面上只有一句人话。
    let reply = site
        .post_form(
            HOST,
            playtest_common::follow::edge_paths::FOLLOW,
            "target=site%3Abrisk-otter-41&email=nope",
        )
        .await;
    assert_eq!(reply.status, StatusCode::BAD_REQUEST);
    let html = reply.text();
    assert!(html.contains("Please check your email address."));
    for word in ["control plane", "error", "failed", "parameter"] {
        assert!(!html.split("</head>").nth(1).unwrap().contains(word));
    }
}

/// 控制面不在的时候不能吓人：一句「现在登记不了，稍后再试。」，503 是给机器看的。
#[tokio::test]
async fn a_control_plane_that_is_not_there_is_one_calm_sentence() {
    // 端口上没有人在听：连都连不上。
    let dead = "http://127.0.0.1:1";
    let site = Site::build(Some(dead.into()), |_| {}).await;
    let reply = site
        .post_form(
            HOST,
            playtest_common::follow::edge_paths::FOLLOW,
            "target=site%3Abrisk-otter-41&email=a%40b.co&to=%2F",
        )
        .await;
    assert_eq!(reply.status, StatusCode::SERVICE_UNAVAILABLE);
    let html = reply.text();
    assert!(html.contains("Unable to process right now. Please try again later."));
    let body = html.split("</head>").nth(1).unwrap();
    for word in ["control plane", "timeout", "error", "500"] {
        assert!(!body.contains(word), "「{word}」不该出现在玩家面前");
    }

    // 控制面在但答 500，也是同一句。
    let (_api, base) = FakeApi::start(Answer::Status(500)).await;
    let site = Site::build(Some(base), |_| {}).await;
    let reply = site
        .post_form(
            HOST,
            playtest_common::follow::edge_paths::FOLLOW,
            "target=site%3Abrisk-otter-41&email=a%40b.co&to=%2F",
        )
        .await;
    assert_eq!(reply.status, StatusCode::SERVICE_UNAVAILABLE);
    assert!(reply.text().contains("Unable to process right now. Please try again later."));
}

// ------------------------------------------------------------------ 根域：关注

#[tokio::test]
async fn me_without_a_key_is_this_device_not_a_login_wall() {
    let site = Site::plain().await;
    site.caps(email_caps()).await;
    let reply = site.get_root(root_paths::ME).await;
    assert_eq!(reply.status, StatusCode::OK);
    let html = reply.text();
    assert!(html.contains("<h1>Following</h1>"));
    assert!(html.contains("Weekly digest"));
    assert!(html.contains("type=\"email\""));
    assert!(html.contains("aria-current=\"page\""));
    assert!(html.contains("Following<span class=\"nav-dot\"></span>"));
    assert!(html.contains("href=\"/\""));
    assert!(html.contains("Plaza</a>"));
    assert!(!html.contains("Explore projects"));
    for word in ["Nickname", "Create account", "Password"] {
        assert!(!html.contains(word), "「{word}」不该出现");
    }
    // 找回与登录入口通过统一账号弹窗提供。
    assert!(html.contains("placeholder=\"name@example.com\""));
    assert!(html.contains(&format!("action=\"{}\"", root_paths::FOLLOW)));
    assert!(html.contains("value=\"plaza\""));
}

#[tokio::test]
async fn me_with_a_key_lists_what_this_person_follows() {
    let (api, base) = FakeApi::start(Answer::Follow(FollowResponse::Subscribed)).await;
    let site = Site::build(Some(base), |_| {}).await;
    site.caps(email_caps()).await;

    let reply = site.get_root_as_me(root_paths::ME).await;
    assert_eq!(reply.status, StatusCode::OK);
    let html = reply.text();
    assert!(html.contains("z***@example.com"));
    assert!(html.contains("小球大冒险"));
    assert!(html.contains("aria-label=\"Unfollow 小球大冒险\">Unfollow</button>"));
    assert!(html.contains("Switch email"));
    // 拿钥匙去问控制面，不是拿邮箱。
    let (_, body) = api.last();
    assert_eq!(body["me_token"], ME_TOKEN);
    // 这一页不缓存：它是这台设备的抽屉。
    assert_eq!(reply.header("cache-control"), Some("no-store"));

    // 控制面不认这把钥匙：清掉 cookie，按没有处理，不解释。
    *api.answer.lock().unwrap() = Answer::Status(401);
    let reply = site.get_root_as_me(root_paths::ME).await;
    assert_eq!(reply.status, StatusCode::OK);
    assert!(reply.text().contains("<h1>Following</h1>"));
    let cleared = reply
        .cookies()
        .iter()
        .find(|c| c.starts_with("pt_me="))
        .unwrap()
        .to_string();
    assert!(cleared.contains("Max-Age=0"));
}

/// 确认信里那条链接：种 `pt_me`，303 到关注页。cookie 的属性是 DESIGN §4.1 的硬要求。
#[tokio::test]
async fn confirming_plants_a_host_only_key() {
    let (api, base) = FakeApi::start(Answer::Follow(FollowResponse::Subscribed)).await;
    let site = Site::build(Some(base), |_| {}).await;

    let reply = site
        .get_root(&format!("{}tok_abc123", root_paths::ME_CONFIRM))
        .await;
    assert_eq!(reply.status, StatusCode::SEE_OTHER);
    assert_eq!(reply.header("location"), Some(root_paths::ME));
    let cookie = reply
        .cookies()
        .iter()
        .find(|c| c.starts_with("pt_me=") && !c.contains("Domain="))
        .unwrap()
        .to_string();
    assert!(cookie.contains(ME_TOKEN));
    assert!(cookie.contains("Path=/"));
    assert!(cookie.contains("HttpOnly"));
    assert!(cookie.contains("SameSite=Lax"));
    assert!(!cookie.contains("Domain="));
    assert!(reply
        .cookies()
        .iter()
        .any(|cookie| cookie.contains("Domain=.localhost") && cookie.contains("Max-Age=0")));
    // 本机是 http，所以不加 Secure；线上 https 会加（见 config.public_scheme）。
    assert!(!cookie.contains("Secure"));
    let (path, body) = api.last();
    assert_eq!(path, routes::CONFIRM);
    assert_eq!(body["token"], "tok_abc123");

    // 用过了 / 过期了 / 我们这头没接上：一页人话加一个再要一条的入口，不是报错页。
    // 只说「现在不管用了」——替它编一个「已经用过了」的理由，那是撒谎。
    *api.answer.lock().unwrap() = Answer::Status(404);
    site.caps(email_caps()).await;
    let reply = site
        .get_root(&format!("{}tok_abc123", root_paths::ME_CONFIRM))
        .await;
    assert_eq!(reply.status, StatusCode::OK);
    let html = reply.text();
    assert!(html.contains("This link is no longer valid."));
    assert!(html.contains("Send another link"));
}

#[tokio::test]
async fn unsubscribing_takes_one_click_and_no_questions() {
    let (_api, base) = FakeApi::start(Answer::Follow(FollowResponse::Subscribed)).await;
    let site = Site::build(Some(base), |_| {}).await;
    let reply = site
        .get_root(&format!("{}tok_bye", root_paths::ME_UNSUBSCRIBE))
        .await;
    assert_eq!(reply.status, StatusCode::OK);
    let html = reply.text();
    assert!(html.contains("Unsubscribed. You will not receive any more notifications."));
    // 不挽留：没有「再想想」「为什么」这类东西。
    for word in ["think again", "why", "are you sure", "pity"] {
        assert!(!html.contains(word), "「{word}」不该出现");
    }
    // 退订之后这台设备上那把钥匙也没意义了。
    assert!(reply
        .cookies()
        .iter()
        .any(|c| c.starts_with("pt_me=") && c.contains("Max-Age=0")));
}

#[tokio::test]
async fn me_actions_redirect_back_and_never_repeat_themselves() {
    let (api, base) = FakeApi::start(Answer::Follow(FollowResponse::Subscribed)).await;
    let site = Site::build(Some(base), |_| {}).await;

    let reply = site
        .send(
            Request::builder()
                .method("POST")
                .uri(root_paths::ME_ACTION)
                .header("host", ROOT)
                .header("origin", format!("http://{ROOT}"))
                .header("cookie", format!("pt_me={ME_TOKEN}"))
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from("action=unfollow&target=site%3Abrisk-otter-41"))
                .unwrap(),
        )
        .await;
    assert_eq!(reply.status, StatusCode::SEE_OTHER);
    assert_eq!(reply.header("location"), Some(root_paths::ME));
    let (_, body) = api.last();
    assert_eq!(body["target"]["slug"], SLUG);

    // 换设备：拿邮箱要一条链接。
    let reply = site
        .post_form(
            ROOT,
            root_paths::ME_ACTION,
            "action=send_link&email=zhong%40example.com",
        )
        .await;
    assert_eq!(reply.status, StatusCode::SEE_OTHER);
    let (_, body) = api.last();
    assert_eq!(body["email"], "zhong@example.com");

    // 关掉浏览器通知：带着 `pt_me` 去控制面清推送订阅。
    let hits = api.hits.load(std::sync::atomic::Ordering::SeqCst);
    let reply = site
        .send(
            Request::builder()
                .method("POST")
                .uri(root_paths::ME_ACTION)
                .header("host", ROOT)
                .header("origin", format!("http://{ROOT}"))
                .header("cookie", format!("pt_me={ME_TOKEN}"))
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from("action=push_off"))
                .unwrap(),
        )
        .await;
    assert_eq!(reply.status, StatusCode::SEE_OTHER);
    let (path, body) = api.last();
    assert_eq!(path, routes::ME_PUSH_OFF);
    assert_eq!(body["me_token"], ME_TOKEN);
    assert_eq!(api.hits.load(std::sync::atomic::Ordering::SeqCst), hits + 1);

    // 没有 `pt_me` 的浏览器点它：不打控制面（没有身份可清）。
    let reply = site
        .post_form(ROOT, root_paths::ME_ACTION, "action=push_off")
        .await;
    assert_eq!(reply.status, StatusCode::SEE_OTHER);
    assert_eq!(api.hits.load(std::sync::atomic::Ordering::SeqCst), hits + 1);
}

#[tokio::test]
async fn the_service_worker_lives_on_the_root_host_only() {
    let site = Site::plain().await;
    let reply = site.get_root("/_playtest/sw.js").await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.header("content-type"), Some("application/javascript"));
    let js = reply.text();
    assert!(js.contains("push"));
    assert!(js.contains("notificationclick"));
    // 它只弹通知、只开链接：不碰 fetch，作品的资源不经过它。
    assert!(!js.contains("addEventListener('fetch'"));
    assert!(js.len() < 2048, "SW {} 字节", js.len());

    // 作品子域上没有这个东西：边缘不往作品域注册 SW（会打坏作品自己的 SW）。
    let on_site = site.get("/_playtest/sw.js").await;
    assert_eq!(on_site.status, StatusCode::NOT_FOUND);
    assert!(!site.get("/").await.text().contains("serviceWorker"));
}

// ------------------------------------------------------------------ 广场

#[tokio::test]
async fn the_wall_is_one_grid_with_a_rail_and_says_which_card_is_paid_for() {
    let site = Site::plain().await;
    site.caps(email_caps()).await;
    let tile = |slug: &str, boosted: bool, seeking: bool| PlazaItem {
        slug: slug.into(),
        url: format!("http://{slug}.localhost:8443"),
        title: "小球大冒险".into(),
        developer: "某某".into(),
        summary: None,
        engine: Some("phaser".into()),
        is_game: true,
        kind: Default::default(),
        version: 7,
        updated_at: "2026-09-08T03:00:00Z".into(),
        expires_at: None,
        cover_hash: None,
        players: 3,
        seeking,
        note: None,
        seats: Some(10),
        joined: 4,
        followers: 12,
        avatar_url: Some("https://avatars.githubusercontent.com/u/1?v=4".into()),
        boosted,
    };
    site.store
        .put_plaza(&Plaza {
            schema: playtest_common::plaza::SCHEMA,
            generated_at: "2026-09-09T00:00:00Z".into(),
            club_followers: 42,
            collections: vec![],
            items: vec![
                tile("paid-one", true, false),
                tile("seeking-one", false, true),
                tile("quiet-one", false, false),
            ],
        })
        .await
        .unwrap();

    let reply = site.get_root("/").await;
    assert_eq!(reply.status, StatusCode::OK);
    let html = reply.text();
    // 顶通栏：字标、广场（当前）、关注、发布。墙上没有门口那句话。
    assert!(html.contains(playtest_edge::html::WORDMARK));
    assert!(html.contains("aria-current=\"page\""));
    assert!(html.contains("Plaza<span class=\"nav-dot\"></span>"));
    assert!(html.contains("Following</a>"));
    assert!(html.contains("href=\"#publish-dialog\""));
    assert!(html.contains("Publish Project"));
    assert!(html.contains(">Publish Project</a>"));
    assert!(!html.contains("brand-icon"));
    assert!(!html.contains("class=\"topbar\""));
    assert!(!html.contains("来玩点，还没定稿的"));
    assert!(!html.contains("class=\"hero\""));
    assert!(!html.contains("SMALL BUILDS"));
    assert!(!html.contains("关于 playtest"));
    assert!(!html.contains("点开就玩，不用注册"));
    assert!(!html.contains("<i>01</i>"));
    assert!(!html.contains("有新东西时告诉我"));
    assert!(!html.contains("关注着这里"));
    // 最新墙保留标明的推广；同一更新时间按 slug 排，招募不覆盖时间顺序。
    let paid = html.find("data-slug=\"paid-one\"").unwrap();
    let seeking = html.find("data-slug=\"seeking-one\"").unwrap();
    let quiet = html.find("data-slug=\"quiet-one\"").unwrap();
    assert!(paid < quiet && quiet < seeking);
    assert!(html.contains("<span class=\"tag ad\">Featured</span>"));
    assert!(html.contains("<span class=\"tag\">Seeking testers</span>"));
    assert!(html.contains("4 / 10 joined"));
    assert!(html.contains("class=\"face\""));
    assert!(!html.contains("推广位永远标出来"));
    assert!(!html.contains("无需登录"));
    // 卡上没有关注、想玩、举报，也没有筛选栏；这一页一行脚本都没有（DESIGN §3.9）。
    assert!(!html.contains("value=\"site:seeking-one\""));
    for word in ["Want to play", "Report", "Most played", "data-band"] {
        assert!(!html.contains(word), "「{word}」不该出现");
    }

    assert_eq!(html.matches("<script nonce=").count(), 1);
    assert!(html.contains("dialog.showModal()"));
    assert!(!html.contains("<script src="));

    // 头像来自 GitHub，所以 CSP 的 img-src 里多这一个来源，且只多这一个。
    let csp = reply.header("content-security-policy").unwrap();
    assert!(csp.starts_with("default-src 'none'"));
    assert!(
        csp.contains("img-src http://*.localhost:8443 https://avatars.githubusercontent.com;"),
        "{csp}"
    );
    assert!(csp.contains("form-action 'self'"));
    assert!(!csp.contains("img-src *"));

    // 有 pt_me 的人看到的是一个按钮，不是又一次要邮箱。
    let (_api, base) = FakeApi::start(Answer::Follow(FollowResponse::Subscribed)).await;
    let signed = Site::build(Some(base), |_| {}).await;
    signed.caps(email_caps()).await;
    signed
        .store
        .put_plaza(&Plaza {
            schema: playtest_common::plaza::SCHEMA,
            generated_at: "2026-09-09T00:00:00Z".into(),
            club_followers: 0,
            collections: vec![],
            items: vec![tile("seeking-one", false, true)],
        })
        .await
        .unwrap();
    let html = signed.get_root_as_me("/").await.text();
    // 有没有钥匙，这一页都一样：关注只在关注页里办。墙上不另写介绍。
    let main = html.split("<main").nth(1).unwrap().split("</main>").next().unwrap();
    assert!(!html.contains("有新东西时告诉我"));
    assert!(!main.contains("type=\"email\""));
    assert!(!html.contains("class=\"intro\""));
    assert!(html.contains("class=\"tile\""));
    assert!(!html.contains("来玩点，还没定稿的"));
}

/// 广场上点「关注」：一下 POST，回到广场。
#[tokio::test]
async fn following_from_the_wall_comes_back_to_the_wall() {
    let (api, base) = FakeApi::start(Answer::Follow(FollowResponse::Subscribed)).await;
    let site = Site::build(Some(base), |_| {}).await;

    let reply = site
        .send(
            Request::builder()
                .method("POST")
                .uri(root_paths::FOLLOW)
                .header("host", ROOT)
                .header("origin", format!("http://{ROOT}"))
                .header("cookie", format!("pt_me={ME_TOKEN}"))
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from("target=plaza&from=plaza&to=%2F"))
                .unwrap(),
        )
        .await;
    assert_eq!(reply.status, StatusCode::OK);
    assert!(reply.text().contains("You are all set! We will notify you when new projects arrive."));
    // 有钥匙就用钥匙，不再问邮箱。
    let (path, body) = api.last();
    assert_eq!(path, routes::FOLLOW);
    assert_eq!(body["channel"]["kind"], "me");
    assert_eq!(body["channel"]["me_token"], ME_TOKEN);
    assert_eq!(body["target"]["kind"], "plaza");
}

/// 根域上就这几条路径，别的一律 404——它是玩家路径里唯一我们说了算的一页。
#[tokio::test]
async fn the_root_host_has_nothing_else_on_it() {
    let site = Site::plain().await;
    for path in ["/anything", "/me/other", "/follow/x", "/_playtest/cover"] {
        assert_eq!(
            site.get_root(path).await.status,
            StatusCode::NOT_FOUND,
            "{path}"
        );
    }
}

#[tokio::test]
async fn secure_identity_is_host_prefixed_and_legacy_identity_is_not_trusted() {
    let (api, base) = FakeApi::start(Answer::Follow(FollowResponse::Subscribed)).await;
    let mut site = Site::build(Some(base), |_| {}).await;
    Arc::get_mut(&mut site.app).unwrap().config.public_scheme = "https".into();
    let reply = site
        .get_root(&format!("{}tok_abc123", root_paths::ME_CONFIRM))
        .await;
    let cookies = reply.cookies();
    let active = cookies
        .iter()
        .find(|cookie| cookie.starts_with("__Host-pt_me="))
        .unwrap();
    assert!(active.contains("Secure"));
    assert!(active.contains("HttpOnly"));
    assert!(active.contains("Path=/"));
    assert!(!active.contains("Domain="));
    assert!(cookies
        .iter()
        .any(|cookie| cookie.starts_with("pt_me=") && cookie.contains("Max-Age=0")));

    let hits = api.hits.load(Ordering::SeqCst);
    let reply = site.get_root_as_me(root_paths::ME).await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(api.hits.load(Ordering::SeqCst), hits);
    assert!(!reply.text().contains("z***@example.com"));
    let reply = site
        .send(
            nav_on(ROOT, root_paths::ME)
                .header("cookie", format!("__Host-pt_me={ME_TOKEN}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(reply.status, StatusCode::OK);
    assert!(reply.text().contains("z***@example.com"));
}

#[tokio::test]
async fn another_work_cannot_act_with_the_root_identity() {
    let (api, base) = FakeApi::start(Answer::Follow(FollowResponse::Subscribed)).await;
    let site = Site::build(Some(base), |_| {}).await;
    for route in [
        root_paths::FOLLOW,
        root_paths::ME_ACTION,
        "/p/brisk-otter-41",
    ] {
        for origin in [
            None,
            Some("null"),
            Some("http://evil.localhost:8443"),
            Some("http://localhost:9999"),
        ] {
            let mut request = Request::builder()
                .method("POST")
                .uri(route)
                .header("host", ROOT)
                .header("cookie", format!("pt_me={ME_TOKEN}"))
                .header("content-type", "application/x-www-form-urlencoded");
            if let Some(origin) = origin {
                request = request.header("origin", origin);
            }
            let reply = site
                .send(
                    request
                        .body(Body::from("action=unfollow&target=site%3Abrisk-otter-41"))
                        .unwrap(),
                )
                .await;
            assert_eq!(reply.status, StatusCode::FORBIDDEN);
            assert!(reply.text().contains("Please open from invitation page"));
        }
    }
    assert_eq!(api.hits.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn a_work_does_not_use_a_legacy_shared_identity() {
    let (api, base) = FakeApi::start(Answer::Follow(FollowResponse::Subscribed)).await;
    let site = Site::build(Some(base), |_| {}).await;
    let reply = site
        .send(
            Request::builder()
                .method("POST")
                .uri("/_playtest/follow")
                .header("host", HOST)
                .header(
                    "cookie",
                    format!("pt_me={ME_TOKEN}; __Host-pt_me={ME_TOKEN}"),
                )
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from("target=site%3Abrisk-otter-41&channel=me"))
                .unwrap(),
        )
        .await;
    assert_eq!(reply.status, StatusCode::BAD_REQUEST);
    assert_eq!(api.hits.load(Ordering::SeqCst), 0);
}

/// 搜到玩家域的助手要能自己找到开发者那一侧，而不是照着首页猜（REWRITE §3.2）。
///
/// 这是玩家域上第二个指向开发者域的出口，和根域介绍页那个链接同性质：给机器，不给玩家。
/// 玩家路径上碰不到它，所以不违反 AGENTS 第 7 条。
#[tokio::test]
async fn an_assistant_landing_on_the_player_host_is_pointed_at_the_docs() {
    let site = Site::plain().await;
    let reply = site.get_root("/llms.txt").await;
    assert_eq!(reply.status, StatusCode::OK);
    let body = reply.text();
    assert!(
        body.contains(playtest_common::DEVELOPER_API_URL),
        "要给出开发者那一侧的地址：{body}"
    );
    assert!(
        body.contains("/skill.md") && body.contains("/openapi.json"),
        "{body}"
    );
    // 这一页不假装自己是 API 入口。
    assert!(body.contains("nothing here for you to call"), "{body}");

    // 它只是一张字条，不是玩家能走进来的门：作品子域上没有这条路径。
    assert_eq!(site.get("/llms.txt").await.status, StatusCode::NOT_FOUND);
}

/// robots.txt 仅在根域提供，返回 text/plain，放行公开路径并声明 Sitemap。
#[tokio::test]
async fn robots_txt_is_served_on_root_with_text_plain_and_allows_public_paths() {
    let site = Site::plain().await;
    let reply = site.get_root(root_paths::ROBOTS_TXT).await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(
        reply.header("content-type"),
        Some("text/plain; charset=utf-8")
    );
    let body = reply.text();
    assert!(body.contains("User-agent: *"));
    assert!(body.contains("Allow: /"));
    assert!(body.contains("Disallow: /_playtest/"));
    assert!(body.contains("Disallow: /v1/"));
    assert!(body.contains("Disallow: /me/"));
    assert!(body.contains("Sitemap: https://playtest.run/sitemap.xml"));
    assert!(body.contains("User-agent: GPTBot"));
    assert!(body.contains("User-agent: PerplexityBot"));

    // 子域没有 robots.txt 这一扇门
    assert_eq!(
        site.get(root_paths::ROBOTS_TXT).await.status,
        StatusCode::NOT_FOUND
    );
}

/// sitemap.xml 仅在根域提供，返回 application/xml，包含核心页面与收录项。
#[tokio::test]
async fn sitemap_xml_is_served_on_root_with_xml_content_type_and_indexes_items() {
    let site = Site::plain().await;
    let plaza: Plaza = serde_json::from_value(serde_json::json!({
        "schema": playtest_common::plaza::SCHEMA,
        "generated_at": "2026-09-09T00:00:00Z",
        "club_followers": 42,
        "collections": [{
            "slug": "pelican",
            "title": "鹈鹕骑单车",
            "summary": "合集摘要",
            "kind": "challenge",
            "prompt": "",
            "rules": "",
            "closes_at": null,
            "public": true,
            "hidden": false,
            "creator": "组织者",
            "created_at": "2026-09-11T00:00:00Z",
            "updated_at": "2026-09-11T00:00:00Z",
            "entries": []
        }],
        "items": [{
            "slug": "first-pelican",
            "url": "https://first-pelican.playtest.run",
            "title": "红色鹈鹕",
            "developer": "小雨",
            "summary": "骑单车",
            "version": 2,
            "updated_at": "2026-09-12T00:00:00Z",
            "players": 1,
            "is_game": false
        }]
    }))
    .unwrap();

    site.store.put_plaza(&plaza).await.unwrap();

    let reply = site.get_root(root_paths::SITEMAP_XML).await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(
        reply.header("content-type"),
        Some("application/xml; charset=utf-8")
    );
    let body = reply.text();
    assert!(body.contains("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
    assert!(body.contains("<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">"));
    assert!(body.contains("<loc>https://playtest.run/</loc>"));
    assert!(body.contains("<loc>https://playtest.run/collections</loc>"));
    assert!(body.contains("<loc>https://playtest.run/c/pelican</loc>"));
    assert!(body.contains("<loc>https://playtest.run/p/first-pelican</loc>"));
    assert!(body.contains("<lastmod>2026-09-12T00:00:00Z</lastmod>"));

    // 子域没有 sitemap.xml
    assert_eq!(
        site.get(root_paths::SITEMAP_XML).await.status,
        StatusCode::NOT_FOUND
    );
}

// ------------------------------------------------------------------ 门禁页那八行

#[tokio::test]
async fn the_gate_shows_only_what_is_really_there() {
    // 什么文件都没有的时候：没有名额、没有群、没有分享、没有关注。
    let bare = Site::plain().await;
    let html = bare.get_root(&format!("/p/{SLUG}")).await.text();
    assert!(!html.contains("is seeking"));
    assert!(!html.contains(">Community</a>"));
    assert!(!html.contains(playtest_common::SHARE_PATH));
    assert!(!html.contains("id=\"notification-settings\""));
    assert!(html.contains(">Play</a>"));

    let site = Site::plain().await;
    site.caps(email_caps()).await;
    site.live(|l| {
        l.seats = Some(10);
        l.joined = 6;
        l.listed = true;
        l.community_url = Some("https://discord.gg/abc".into());
        l.feedback_public = true;
        l.public_feedback = vec![PublicFeedbackItem {
            name: Some("小雨".into()),
            text: "不知道要按哪个键".into(),
            version: 7,
            at: "2026-09-08T00:00:00Z".into(),
        }];
        l.avatar_url = Some("https://avatars.githubusercontent.com/u/1?v=4".into());
    })
    .await;

    let redirect = site.get("/?from=card").await;
    assert_eq!(redirect.status, StatusCode::SEE_OTHER);
    assert_eq!(
        redirect.header("location"),
        Some(format!("http://localhost:8443/p/{SLUG}").as_str())
    );

    let html = site.get_root(&format!("/p/{SLUG}?from=card")).await.text();
    assert!(html.contains("某某 is seeking 10 playtesters · 6 joined"));
    assert!(html.contains(">Community</a>"));
    assert!(html.contains("rel=\"noopener nofollow\""));
    assert!(html.contains(playtest_common::SHARE_PATH));
    assert!(html.contains("id=\"notification-settings\""));
    assert!(html.contains("不知道要按哪个键"));
    assert!(html.contains("avatars.githubusercontent.com"));
    // 扫卡进来的人：来源随「开始」一起带走。
    assert!(html.contains("<input type=\"hidden\" name=\"from\" value=\"card\">"));
    // 留名是可选的。
    assert!(html.contains("<label class=\"holder\"><span>Your name"));
    assert!(html.contains("· optional"));
    // 作品域上永远没有 CSP（硬线）。
    assert!(site
        .get("/")
        .await
        .header("content-security-policy")
        .is_none());
}

/// 「开始」那一下：名字和来源落进事件里（DESIGN §3.3 第 5 条、§3.5）。
#[tokio::test]
async fn a_name_and_a_source_ride_along_with_start() {
    let site = Site::plain().await;
    let reply = site
        .post_form(
            HOST,
            "/_playtest/start",
            "to=%2F&from=card&name=%20%E5%B0%8F%E9%9B%A8%20",
        )
        .await;
    assert_eq!(reply.status, StatusCode::SEE_OTHER);

    let raw = std::fs::read_to_string(site.app.events.path()).unwrap();
    let line: serde_json::Value = serde_json::from_str(raw.lines().last().unwrap()).unwrap();
    assert_eq!(line["type"], "start");
    assert_eq!(line["from"], "card");
    assert_eq!(line["name"], "小雨");

    // 编出来的来源当没有；名字只在 start 上。
    let site = Site::plain().await;
    site.post_form(HOST, "/_playtest/start", "to=%2F&from=whatever")
        .await;
    let raw = std::fs::read_to_string(site.app.events.path()).unwrap();
    let line: serde_json::Value = serde_json::from_str(raw.lines().last().unwrap()).unwrap();
    assert!(line.get("from").is_none());
    assert!(line.get("name").is_none());
}
