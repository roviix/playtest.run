//! 每 slug 每小时的流量熔断，从玩家那一侧看（DESIGN §4.8）。
//!
//! 真的把上限跑满要发 3 GiB，测试里不干这种事：熔断计数是个公开的进程内计数器，
//! 直接把它填到上限，再看边缘怎么答。填计数用的是生产代码同一个入口
//! （[`playtest_edge::breaker::Breaker::record`]），不是测试专用的后门。

use std::sync::Arc;

use axum::body::{Body, Bytes};
use axum::http::{HeaderMap, Request, StatusCode};
use http_body_util::BodyExt;
use playtest_common::hash::hash_bytes;
use playtest_common::limits;
use playtest_common::manifest::{FileEntry, GateMode, Manifest, SCHEMA};
use playtest_common::store::{Current, FsStore};
use playtest_edge::breaker;
use playtest_edge::{router, App, Config};
use tower::ServiceExt;

const SLUG: &str = "brisk-otter-41";
const HOST: &str = "brisk-otter-41.localhost:8443";

const INDEX_HTML: &str = "<!doctype html><title>小球大冒险</title><canvas id=c></canvas>";
const GAME_JS: &str = "console.log('game')";

struct Site {
    _dir: tempfile::TempDir,
    app: Arc<App>,
}

impl Site {
    async fn build(shape: impl FnOnce(&mut Manifest)) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let store = FsStore::new(dir.path().join("store"));

        let blobs: Vec<(&str, Vec<u8>)> = vec![
            ("game.js", GAME_JS.as_bytes().to_vec()),
            ("index.html", INDEX_HTML.as_bytes().to_vec()),
        ];
        let mut files = Vec::new();
        for (path, bytes) in &blobs {
            let hash = hash_bytes(bytes);
            store.put_blob(&hash, bytes).await.unwrap();
            files.push(FileEntry {
                path: (*path).to_string(),
                hash,
                size: bytes.len() as u64,
            });
        }

        let mut manifest = Manifest {
            schema: SCHEMA,
            slug: SLUG.into(),
            version: 7,
            title: "小球大冒险".into(),
            developer: "某某".into(),
            note: None,
            created_at: "2026-09-07T00:00:00Z".into(),
            expires_at: None,
            badge: true,
            gate: GateMode::Once,
            isolated: false,
            spa: false,
            engine: Some("godot".into()),
            files,
        };
        shape(&mut manifest);

        store.put_manifest(&manifest).await.unwrap();
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

        Site {
            app: Arc::new(App::new(Config {
                listen: "127.0.0.1:0".parse().unwrap(),
                data_dir: dir.path().to_path_buf(),
                host_suffix: "localhost".into(),
                public_scheme: "http".into(),
            })),
            _dir: dir,
        }
    }

    async fn plain() -> Self {
        Self::build(|_| {}).await
    }

    /// 把这一小时的额度按满。
    fn burn_the_hour(&self, limit: u64) {
        self.app.breaker.record(SLUG, limit);
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

    fn events(&self) -> Vec<serde_json::Value> {
        let raw = std::fs::read_to_string(self.app.events.path()).unwrap_or_default();
        raw.lines().map(|l| serde_json::from_str(l).unwrap()).collect()
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
}

fn base(path: &str) -> axum::http::request::Builder {
    Request::builder().uri(path).header("host", HOST)
}

fn nav(path: &str) -> axum::http::request::Builder {
    base(path)
        .header("accept", "text/html,application/xhtml+xml,*/*;q=0.8")
        .header("sec-fetch-dest", "document")
}

fn asset(path: &str) -> axum::http::request::Builder {
    base(path).header("accept", "*/*").header("sec-fetch-dest", "script")
}

#[tokio::test]
async fn serving_a_file_counts_toward_the_hour() {
    let site = Site::plain().await;
    assert_eq!(site.app.breaker.total(SLUG), 0);

    site.send(asset("/game.js").body(Body::empty()).unwrap()).await;
    assert_eq!(site.app.breaker.total(SLUG), GAME_JS.len() as u64);

    // HEAD 不出体，不记账。
    site.send(asset("/game.js").method("HEAD").body(Body::empty()).unwrap())
        .await;
    assert_eq!(site.app.breaker.total(SLUG), GAME_JS.len() as u64);

    // 304 也不。
    let etag = format!("\"{}\"", hash_bytes(GAME_JS.as_bytes()));
    let reply = site
        .send(
            asset("/game.js")
                .header("if-none-match", &etag)
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(reply.status, StatusCode::NOT_MODIFIED);
    assert_eq!(site.app.breaker.total(SLUG), GAME_JS.len() as u64);
}

#[tokio::test]
async fn a_navigation_gets_a_page_a_subresource_gets_nothing() {
    let site = Site::plain().await;
    site.burn_the_hour(limits::SLUG_HOURLY_BYTES);

    // 顶层导航：一页看得懂的说明。玩家拿到的是别人发给他的一条链接，
    // 浏览器默认的错误页只会让他以为是自己的网络坏了。
    let page = site.send(nav("/").body(Body::empty()).unwrap()).await;
    assert_eq!(page.status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(page.header("content-type"), Some("text/html; charset=utf-8"));
    assert_eq!(page.header("cache-control"), Some("no-store"));
    assert!(page.header("retry-after").is_some());
    assert!(page.text().contains("这一小时的流量用完了"));
    assert!(!page.text().contains("playtest.sh"));

    // 子资源：状态码就是全部。**一个字节的 HTML 都不给**——和门禁页同一条硬线，
    // 加载器拿到 HTML 会崩在一个和真实原因无关的地方。
    for path in ["/game.js", "/index.html", "/nope.png"] {
        let reply = site.send(asset(path).body(Body::empty()).unwrap()).await;
        assert_eq!(reply.status, StatusCode::TOO_MANY_REQUESTS, "{path}");
        assert!(reply.body.is_empty(), "{path}：熔断给子资源发了正文");
        assert_ne!(
            reply.header("content-type").map(|t| t.starts_with("text/html")),
            Some(true),
            "{path}"
        );
        assert!(reply.header("retry-after").is_some(), "{path}");
    }
}

#[tokio::test]
async fn the_report_form_stays_open() {
    // 熔断关的是作品，不是举报入口——一个正在被刷的 slug 恰恰是最需要能举报的。
    let site = Site::plain().await;
    site.burn_the_hour(limits::SLUG_HOURLY_BYTES);

    let form = site.send(nav("/_playtest/report").body(Body::empty()).unwrap()).await;
    assert_eq!(form.status, StatusCode::OK);
    assert!(form.text().contains("<select name=\"reason\">"));

    let done = site
        .send(
            base("/_playtest/report")
                .method("POST")
                .body(Body::from("reason=phishing&detail=%E5%81%87%E7%9A%84"))
                .unwrap(),
        )
        .await;
    assert_eq!(done.status, StatusCode::OK);

    // 健康检查也不受影响，否则一个被刷的 slug 会把整台机器从负载均衡里摘掉。
    let health = site
        .send(
            Request::builder()
                .uri("/_playtest/healthz")
                .header("host", "127.0.0.1:8443")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(health.status, StatusCode::OK);
}

#[tokio::test]
async fn the_trip_is_written_once_not_once_per_request() {
    let site = Site::plain().await;
    site.burn_the_hour(limits::SLUG_HOURLY_BYTES);

    for _ in 0..20 {
        site.send(asset("/game.js").body(Body::empty()).unwrap()).await;
    }
    site.send(nav("/").body(Body::empty()).unwrap()).await;

    let trips: Vec<_> = site
        .events()
        .into_iter()
        .filter(|e| e["type"] == "breaker_trip")
        .collect();
    assert_eq!(
        trips.len(),
        1,
        "被刷的时候每个请求写一行，日志自己就成了放大器"
    );
    assert_eq!(trips[0]["slug"], SLUG);
    assert_eq!(trips[0]["version"], 7);
    assert_eq!(trips[0]["bytes"], limits::SLUG_HOURLY_BYTES);
    assert_eq!(trips[0]["limit"], limits::SLUG_HOURLY_BYTES);
    // 不记 IP，熔断这条也一样（DESIGN §3.4）。
    assert!(trips[0].get("ip").is_none());
}

#[tokio::test]
async fn anonymous_links_trip_earlier() {
    // 匿名链接 24 小时总共才 1 GiB（DESIGN §6），每小时的闸门跟着更紧。
    let site = Site::build(|m| m.expires_at = Some("2099-01-01T00:00:00Z".into())).await;
    assert!(limits::ANON_SLUG_HOURLY_BYTES < limits::SLUG_HOURLY_BYTES);

    site.burn_the_hour(limits::ANON_SLUG_HOURLY_BYTES);
    let reply = site.send(nav("/").body(Body::empty()).unwrap()).await;
    assert_eq!(reply.status, StatusCode::TOO_MANY_REQUESTS);

    // 同样的字节数在登录账号的作品上还远远没到。
    let site = Site::plain().await;
    site.burn_the_hour(limits::ANON_SLUG_HOURLY_BYTES);
    let reply = site.send(nav("/").body(Body::empty()).unwrap()).await;
    assert_eq!(reply.status, StatusCode::OK);
}

#[tokio::test]
async fn one_slug_tripping_does_not_touch_the_others() {
    let site = Site::plain().await;
    site.app.breaker.record("keen-gecko-9", limits::SLUG_HOURLY_BYTES);

    let reply = site.send(nav("/").body(Body::empty()).unwrap()).await;
    assert_eq!(reply.status, StatusCode::OK);
    assert!(reply.text().contains(">开始</button>"));

    // 根域介绍页也不受任何 slug 的影响。
    let root = site
        .send(
            Request::builder()
                .uri("/")
                .header("host", "localhost:8443")
                .header("accept", "text/html")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(root.status, StatusCode::OK);
}

#[tokio::test]
async fn the_limit_matches_what_the_manifest_says() {
    let mut m = Manifest {
        schema: SCHEMA,
        slug: SLUG.into(),
        version: 1,
        title: "t".into(),
        developer: "d".into(),
        note: None,
        created_at: "2026-09-07T00:00:00Z".into(),
        expires_at: None,
        badge: true,
        gate: GateMode::Once,
        isolated: false,
        spa: false,
        engine: None,
        files: vec![],
    };
    assert_eq!(breaker::limit_for(&m), limits::SLUG_HOURLY_BYTES);
    m.expires_at = Some("2026-09-08T00:00:00Z".into());
    assert_eq!(breaker::limit_for(&m), limits::ANON_SLUG_HOURLY_BYTES);
}
