//! GitHub 登录（DESIGN §3.2）：设备码与网页授权码两条路，以及「匿名作品归入账号」。
//!
//! GitHub 用一个本机的假服务器顶替（`PLAYTEST_GITHUB_BASE_URL` 那个口子就是给它留的），
//! 控制面像线上一样走 HTTP 去问它。真 GitHub 的那一遍记在 `docs/spikes/` 里。

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use axum::body::{Body, Bytes};
use axum::extract::State;
use axum::http::{header, Request, StatusCode};
use axum::routing::{get, post};
use axum::{Json, Router};
use playtest_api::config::GitHubApp;
use playtest_api::{app, AppState, Config};
use playtest_common::api::{
    routes as paths, AnonSessionResponse, CommitUploadResponse, CreateSiteRequest, DeviceLoginPoll,
    DeviceLoginStart, ErrorBody, ErrorCode, LoginPollResponse, LoginResponse, Me,
    PrepareUploadRequest, PrepareUploadResponse, Site, WebLoginExchange,
};
use playtest_common::hash;
use playtest_common::manifest::{FileEntry, GateMode};
use playtest_common::store::FsStore;
use serde::de::DeserializeOwned;
use serde::Serialize;
use tower::ServiceExt;

const CONSOLE_URL: &str = "http://localhost:5273/";

// ---------------------------------------------------------------- 假 GitHub

#[derive(Clone)]
struct FakeGitHub {
    /// 设备码轮询被问了几次：第一次说「还没输完」，第二次给令牌。
    polls: Arc<AtomicUsize>,
}

async fn fake_device_code() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "device_code": "dev-code-1",
        "user_code": "WDJB-MJHT",
        "verification_uri": "https://github.com/login/device",
        "expires_in": 899,
        "interval": 5
    }))
}

async fn fake_access_token(
    State(gh): State<FakeGitHub>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    if body.get("device_code").is_some() {
        let n = gh.polls.fetch_add(1, Ordering::SeqCst);
        return Json(if n == 0 {
            serde_json::json!({ "error": "authorization_pending", "interval": 5 })
        } else {
            serde_json::json!({ "access_token": "gho_fake", "token_type": "bearer" })
        });
    }
    match body.get("code").and_then(|c| c.as_str()) {
        Some("good-code") if body.get("client_secret").is_some() => {
            Json(serde_json::json!({ "access_token": "gho_fake_web", "token_type": "bearer" }))
        }
        _ => Json(serde_json::json!({ "error": "bad_verification_code" })),
    }
}

async fn fake_user() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "id": 4242, "login": "octo", "name": "Octo Cat" }))
}

async fn start_fake_github() -> String {
    let gh = FakeGitHub {
        polls: Arc::new(AtomicUsize::new(0)),
    };
    let router = Router::new()
        .route("/login/device/code", post(fake_device_code))
        .route("/login/oauth/access_token", post(fake_access_token))
        .route("/user", get(fake_user))
        .with_state(gh);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    format!("http://{addr}")
}

// ---------------------------------------------------------------- 控制面

struct Harness {
    router: Router,
    store: FsStore,
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

    fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }
}

impl Harness {
    async fn start(github: Option<GitHubApp>) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let config = Config {
            listen: "127.0.0.1:0".parse().unwrap(),
            data_dir: dir.path().to_path_buf(),
            site_url_template: "http://{slug}.localhost:8443".to_string(),
            github,
        };
        let state = AppState::from_config(&config).await.unwrap();
        Self {
            router: app(state),
            store: FsStore::new(config.store_root()),
            _dir: dir,
        }
    }

    async fn with_fake_github(secret: Option<&str>) -> Self {
        let base = start_fake_github().await;
        Self::start(Some(GitHubApp {
            client_id: "Ov23test".to_string(),
            client_secret: secret.map(str::to_string),
            console_url: CONSOLE_URL.to_string(),
            web_base: base.clone(),
            api_base: base,
        }))
        .await
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

    async fn post<T: Serialize>(&self, path: &str, token: Option<&str>, value: &T) -> Reply {
        let mut builder = Request::builder()
            .method("POST")
            .uri(path)
            .header(header::CONTENT_TYPE, "application/json");
        if let Some(token) = token {
            builder = builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
        }
        self.send(
            builder
                .body(Body::from(serde_json::to_vec(value).unwrap()))
                .unwrap(),
        )
        .await
    }

    async fn get(&self, path: &str, token: Option<&str>) -> Reply {
        let mut builder = Request::builder().method("GET").uri(path);
        if let Some(token) = token {
            builder = builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
        }
        self.send(builder.body(Body::empty()).unwrap()).await
    }

    async fn anon_token(&self) -> String {
        self.post(paths::ANON_SESSIONS, None, &serde_json::json!({}))
            .await
            .json::<AnonSessionResponse>()
            .token
    }

    async fn new_site(&self, token: &str) -> Site {
        self.post(paths::SITES, Some(token), &CreateSiteRequest::default())
            .await
            .json()
    }

    async fn publish(&self, slug: &str, token: &str) -> u32 {
        const INDEX_HTML: &[u8] = b"<!doctype html><title>x</title>";
        let file = FileEntry {
            path: "index.html".to_string(),
            hash: hash::hash_bytes(INDEX_HTML),
            size: INDEX_HTML.len() as u64,
        };
        let prepared: PrepareUploadResponse = self
            .post(
                &paths::site_uploads(slug),
                Some(token),
                &PrepareUploadRequest {
                    files: vec![file.clone()],
                    title: None,
                    note: None,
                    summary: None,
                    cover: None,
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
        self.post(
            &paths::site_upload_commit(slug, &prepared.upload_id),
            Some(token),
            &serde_json::json!({}),
        )
        .await
        .json::<CommitUploadResponse>()
        .version
    }
}

// ---------------------------------------------------------------- 测试

#[tokio::test]
async fn the_device_flow_logs_in_and_adopts_anonymous_sites() {
    let h = Harness::with_fake_github(None).await;

    // 先像第一次用的人那样：匿名传了一版，链接带着 24 小时到期。
    let anon = h.anon_token().await;
    let site = h.new_site(&anon).await;
    let version = h.publish(&site.slug, &anon).await;
    assert!(site.expires_at.is_some());

    let start: DeviceLoginStart = h
        .post(paths::LOGIN_DEVICE_START, None, &serde_json::json!({}))
        .await
        .json();
    assert_eq!(start.user_code, "WDJB-MJHT");
    assert_eq!(start.interval, 5);

    let poll = DeviceLoginPoll {
        device_code: start.device_code,
    };
    // 第一次：人还没输完。
    let first: LoginPollResponse = h
        .post(paths::LOGIN_DEVICE_POLL, Some(&anon), &poll)
        .await
        .json();
    assert_eq!(first, LoginPollResponse::Pending { interval: 5 });

    // 第二次：成了，而且顺带把匿名作品归了进来。
    let second: LoginPollResponse = h
        .post(paths::LOGIN_DEVICE_POLL, Some(&anon), &poll)
        .await
        .json();
    let LoginPollResponse::Ok(login) = second else {
        panic!("第二次轮询应该登录成功：{second:?}");
    };
    assert_eq!(login.login, "octo");
    assert_eq!(login.display_name, "Octo Cat");
    assert_eq!(login.migrated_sites, 1);

    // 新令牌是长期的、是 GitHub 身份。
    let me: Me = h.get(paths::ME, Some(&login.token)).await.json();
    assert_eq!(me.kind, "github");
    assert_eq!(me.login.as_deref(), Some("octo"));
    assert!(me.expires_at.is_none());

    // 作品换了主人、去掉了到期时间；清单里的名字和到期时间也一起改了（门禁页读的是清单）。
    let mine: Vec<Site> = h.get(paths::SITES, Some(&login.token)).await.json();
    assert_eq!(mine.len(), 1);
    assert_eq!(mine[0].slug, site.slug);
    assert!(
        mine[0].expires_at.is_none(),
        "归入账号的作品不该再有到期时间"
    );
    let manifest = h
        .store
        .get_manifest(&site.slug, version)
        .await
        .unwrap()
        .expect("清单还在");
    assert!(manifest.expires_at.is_none());
    assert_eq!(manifest.developer, "Octo Cat");

    // 匿名令牌那边看过去，作品已经不是它的了。
    let left: Vec<Site> = h.get(paths::SITES, Some(&anon)).await.json();
    assert!(left.is_empty());
}

#[tokio::test]
async fn the_web_flow_needs_a_matching_state_and_uses_it_once() {
    let h = Harness::with_fake_github(Some("shh")).await;

    let start = h.get(paths::LOGIN_WEB_START, None).await;
    assert_eq!(start.status, StatusCode::SEE_OTHER, "{}", start.text());
    let location = start.headers[header::LOCATION]
        .to_str()
        .unwrap()
        .to_string();
    assert!(
        location.contains("/login/oauth/authorize?client_id=Ov23test&redirect_uri=http%3A%2F%2Flocalhost%3A5273%2F&state="),
        "{location}"
    );
    let state = location
        .rsplit("state=")
        .next()
        .unwrap()
        .split('&')
        .next()
        .unwrap()
        .to_string();

    // state 编的、不认识的：拒。
    h.post(
        paths::LOGIN_WEB_EXCHANGE,
        None,
        &WebLoginExchange {
            code: "good-code".into(),
            state: "made-up".into(),
        },
    )
    .await
    .error(StatusCode::BAD_REQUEST, ErrorCode::LoginFailed);

    let login: LoginResponse = h
        .post(
            paths::LOGIN_WEB_EXCHANGE,
            None,
            &WebLoginExchange {
                code: "good-code".into(),
                state: state.clone(),
            },
        )
        .await
        .json();
    assert_eq!(login.login, "octo");
    assert_eq!(login.migrated_sites, 0, "没带匿名令牌就没有东西可归");

    // 同一个 state 用第二次：不行。
    h.post(
        paths::LOGIN_WEB_EXCHANGE,
        None,
        &WebLoginExchange {
            code: "good-code".into(),
            state,
        },
    )
    .await
    .error(StatusCode::BAD_REQUEST, ErrorCode::LoginFailed);
}

#[tokio::test]
async fn a_bad_code_from_github_is_a_login_failure_not_a_crash() {
    let h = Harness::with_fake_github(Some("shh")).await;
    let start = h.get(paths::LOGIN_WEB_START, None).await;
    let location = start.headers[header::LOCATION].to_str().unwrap();
    let state = location
        .rsplit("state=")
        .next()
        .unwrap()
        .split('&')
        .next()
        .unwrap()
        .to_string();
    let body = h
        .post(
            paths::LOGIN_WEB_EXCHANGE,
            None,
            &WebLoginExchange {
                code: "used-already".into(),
                state,
            },
        )
        .await
        .error(StatusCode::BAD_REQUEST, ErrorCode::LoginFailed);
    assert!(body.message.contains("再试一次"), "{}", body.message);
}

#[tokio::test]
async fn without_github_configured_login_says_so_and_anonymous_still_works() {
    let h = Harness::start(None).await;
    let body = h
        .post(paths::LOGIN_DEVICE_START, None, &serde_json::json!({}))
        .await
        .error(StatusCode::NOT_IMPLEMENTED, ErrorCode::LoginUnavailable);
    assert!(
        body.message.contains("匿名链接照常能用"),
        "{}",
        body.message
    );
    h.get(paths::LOGIN_WEB_START, None)
        .await
        .error(StatusCode::NOT_IMPLEMENTED, ErrorCode::LoginUnavailable);

    let anon = h.anon_token().await;
    let me: Me = h.get(paths::ME, Some(&anon)).await.json();
    assert_eq!(me.kind, "anon");
    assert!(me.expires_at.is_some());
}

#[tokio::test]
async fn only_device_login_when_there_is_no_client_secret() {
    let h = Harness::with_fake_github(None).await;
    let body = h
        .get(paths::LOGIN_WEB_START, None)
        .await
        .error(StatusCode::NOT_IMPLEMENTED, ErrorCode::LoginUnavailable);
    assert!(body.message.contains("client secret"), "{}", body.message);
}
