//! 门禁页的三条硬线（DESIGN §3.3）。
//!
//! > 门禁页只在顶层文档导航上渲染；子资源、API、WebSocket upgrade **永不返回
//! > 「200 + HTML」**，要么给资源本身要么给 403 / 404；**不存在绕过它的请求头**。
//!
//! 这三条不是「注意一下」，是把门禁页和 ngrok 那种警告页分开的东西，所以在这里钉死。
//! 反面教材是 pinggy：靠 UA 猜访客是不是浏览器，对每个没带确认 cookie 的路径都回
//! 200 + 15 KB HTML。一个 Phaser 导出物的入口 HTML 只有 843 字节、真东西是 1.2 MB 的 JS——
//! 拿到 HTML 的浏览器不报网络错，最好的结果是一个看不懂的语法错，最坏是白屏加沉默。
//! **在游戏里 200 比 404 坏得多。**
//!
//! 这里打的是真的 Router，断言的是玩家真正会收到的状态码、响应头和字节。

use std::sync::Arc;

use axum::body::{Body, Bytes};
use axum::http::{HeaderMap, Request, StatusCode};
use http_body_util::BodyExt;
use playtest_common::hash::hash_bytes;
use playtest_common::manifest::{FileEntry, GateMode, Manifest, SCHEMA};
use playtest_common::store::{Current, FsStore};
use playtest_edge::{router, App, Config};
use tower::ServiceExt;

const SLUG: &str = "brisk-otter-41";
const HOST: &str = "brisk-otter-41.localhost:8443";

const INDEX_HTML: &str = "<!doctype html><title>小球大冒险</title><canvas id=c></canvas>";
const APP_JS: &str = "console.log('这是作品自己的脚本，不是门禁页')";
const WASM: &[u8] = b"\0asm\x01\0\0\0";
// Unity 的 Decompression Fallback 产物：字节里已经是 brotli，但解压在 JS 里做。
const UNITYWEB: &[u8] = b"<pretend brotli inside a .unityweb>";

/// 门禁页上一定有、作品自己的文件上一定没有的那几个记号。
fn is_gate_page(body: &str) -> bool {
    body.contains("/_playtest/start")
        || body.contains("class=\"start-btn\"")
        || body.contains(">Start</button>")
        || body.contains(">Play</button>")
        || body.contains(">Test</button>")
}

fn root_nav(path: &str) -> axum::http::request::Builder {
    Request::builder()
        .uri(path)
        .header("host", "localhost:8443")
        .header("accept", "text/html,application/xhtml+xml,*/*;q=0.8")
        .header("sec-fetch-dest", "document")
}

struct Site {
    _dir: tempfile::TempDir,
    app: Arc<App>,
}

impl Site {
    async fn build(shape: impl FnOnce(&mut Manifest)) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let store = FsStore::new(dir.path().join("store"));

        let blobs: Vec<(&str, Vec<u8>)> = vec![
            ("Build/game.wasm.unityweb", UNITYWEB.to_vec()),
            ("assets/app.js", APP_JS.as_bytes().to_vec()),
            ("game.wasm", WASM.to_vec()),
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
            summary: None,
            cover: None,
            created_at: "2026-09-07T00:00:00Z".into(),
            expires_at: None,
            badge: true,
            gate: GateMode::Once,
            isolated: false,
            spa: false,
            engine: Some("godot".into()),
            kind: Default::default(),
            entry: None,
            article: None,
            chapters: vec![],
            files,
        };
        shape(&mut manifest);

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

        Site {
            app: Arc::new(App::new(Config {
                listen: "127.0.0.1:0".parse().unwrap(),
                data_dir: dir.path().to_path_buf(),
                host_suffix: "localhost".into(),
                public_scheme: "http".into(),
                api_internal_url: None,
                edge_ingest_token: None,
            })),
            _dir: dir,
        }
    }

    async fn plain() -> Self {
        Self::build(|_| {}).await
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

    fn is_html(&self) -> bool {
        self.header("content-type")
            .is_some_and(|t| t.starts_with("text/html"))
    }

    /// 硬线本身：这一条响应绝不能是「200 + HTML」。
    fn assert_not_200_html(&self, what: &str) {
        assert!(
            !(self.status == StatusCode::OK && self.is_html()),
            "{what}：拿到了 200 + {:?}，这正是 pinggy 的死法",
            self.header("content-type")
        );
        assert!(!is_gate_page(&self.text()), "{what}：正文里出现了门禁页");
    }
}

fn base(path: &str) -> axum::http::request::Builder {
    Request::builder().uri(path).header("host", HOST)
}

/// 一个浏览器点开链接时的样子。
fn nav(path: &str) -> axum::http::request::Builder {
    base(path)
        .header("accept", "text/html,application/xhtml+xml,*/*;q=0.8")
        .header("sec-fetch-dest", "document")
}

// ---------------------------------------------------- 硬线 a · 子资源永不 200 + HTML

#[tokio::test]
async fn subresources_without_a_cookie_get_the_bytes_or_a_404() {
    let site = Site::plain().await;

    // 清单里有的，照出，出的是作品自己的字节。
    let js = site
        .send(
            base("/assets/app.js")
                .header("sec-fetch-dest", "script")
                .header("accept", "*/*")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(js.status, StatusCode::OK);
    assert_eq!(
        js.header("content-type"),
        Some("text/javascript; charset=utf-8")
    );
    assert_eq!(js.text(), APP_JS);
    js.assert_not_200_html("带 Sec-Fetch-Dest: script 的 /assets/app.js");

    // 连 Accept 都不带的客户端（很多引擎的加载器就这样）。
    let wasm = site
        .send(base("/game.wasm").body(Body::empty()).unwrap())
        .await;
    assert_eq!(wasm.status, StatusCode::OK);
    assert_eq!(wasm.header("content-type"), Some("application/wasm"));
    assert_eq!(wasm.body.as_ref(), WASM);
    wasm.assert_not_200_html("不带任何 Accept 的 /game.wasm");

    // 清单里没有的，只能是 404，不能是「200 + 一页 HTML」。
    for path in [
        "/assets/missing.js",
        "/Build/absent.wasm",
        "/textures/x.png",
    ] {
        for dest in [
            "script", "empty", "image", "style", "font", "worker", "audio",
        ] {
            let reply = site
                .send(
                    base(path)
                        .header("sec-fetch-dest", dest)
                        .header("accept", "*/*")
                        .body(Body::empty())
                        .unwrap(),
                )
                .await;
            assert_eq!(reply.status, StatusCode::NOT_FOUND, "{path} / {dest}");
            reply.assert_not_200_html(&format!("{path} / {dest}"));
        }
    }
}

#[tokio::test]
async fn spa_fallback_never_turns_a_missing_asset_into_the_gate_page() {
    // 开了 `spa` 的作品，任何路径都能解析成 index.html——这是最容易漏的一格：
    // 一个 404 的 `.js` 变成 200 + HTML，加载器会崩在一个和真实原因无关的地方。
    let site = Site::build(|m| m.spa = true).await;

    for path in ["/assets/app-4f2c.js", "/Build/game.wasm", "/audio/bgm.mp3"] {
        // 子资源的样子。
        let reply = site
            .send(
                base(path)
                    .header("sec-fetch-dest", "script")
                    .header("accept", "*/*")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await;
        reply.assert_not_200_html(path);
        assert_eq!(reply.status, StatusCode::NOT_FOUND, "{path}");

        // 老客户端的样子：没有 Sec-Fetch-Dest，Accept 里却写着 text/html。
        // 这一条会命中 SPA 回退（拿到 index.html），但**不能拿到门禁页**。
        let reply = site
            .send(
                base(path)
                    .header("accept", "text/html,*/*")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await;
        assert!(
            !is_gate_page(&reply.text()),
            "{path}：SPA 回退把门禁页发给了资源路径"
        );
    }

    // 开了 SPA 的作品，子域上真正的导航拿到的是 SPA 入口（INDEX_HTML），而不是门禁页。
    let reply = site
        .send(nav("/level/3").body(Body::empty()).unwrap())
        .await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.text(), INDEX_HTML);
    assert!(!is_gate_page(&reply.text()));

    // 而作品邀请函在主域 `/p/{slug}`，访问它拿到的是门禁页。
    let reply = site
        .send(root_nav(&format!("/p/{SLUG}")).body(Body::empty()).unwrap())
        .await;
    assert_eq!(reply.status, StatusCode::OK);
    assert!(is_gate_page(&reply.text()));
}

#[tokio::test]
async fn a_websocket_upgrade_is_not_a_navigation() {
    // 隧道路径接上之后这里会是真的 WS；现在它至少不能拿到一页 HTML
    // ——WebSocket 握手收到 200 + HTML 的客户端只会报一个莫名其妙的错。
    let site = Site::plain().await;
    let reply = site
        .send(
            base("/socket.io/")
                .header("connection", "Upgrade")
                .header("upgrade", "websocket")
                .header("sec-fetch-dest", "websocket")
                .header("sec-websocket-version", "13")
                .header("accept", "text/html")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    reply.assert_not_200_html("WebSocket upgrade");
}

// ------------------------------------- 硬线 b · Sec-Fetch-Dest 在场就以它为准

#[tokio::test]
async fn accept_html_does_not_override_a_non_document_fetch_dest() {
    let site = Site::plain().await;

    // `fetch('/')` 带的就是 `Sec-Fetch-Dest: empty`；有的库还顺手写 Accept: text/html。
    // 两个信号打架时信 Accept，就等于把门禁页发给了一个 XHR。
    for dest in [
        "empty",
        "script",
        "iframe",
        "frame",
        "image",
        "style",
        "font",
        "worker",
        "sharedworker",
        "serviceworker",
        "manifest",
        "object",
        "embed",
        "audio",
        "video",
        "websocket",
        "report",
    ] {
        for path in ["/", "/index.html"] {
            let reply = site
                .send(
                    base(path)
                        .header("accept", "text/html,application/xhtml+xml")
                        .header("sec-fetch-dest", dest)
                        .body(Body::empty())
                        .unwrap(),
                )
                .await;
            assert!(
                !is_gate_page(&reply.text()),
                "{path} / Sec-Fetch-Dest: {dest} 拿到了门禁页"
            );
            // 拿到的是作品自己的 HTML，不是我们的一页。
            assert_eq!(reply.status, StatusCode::OK, "{path} / {dest}");
            assert_eq!(reply.text(), INDEX_HTML, "{path} / {dest}");
        }
    }

    // 子域上的导航直接拿到作品自己的 HTML
    let reply = site.send(nav("/").body(Body::empty()).unwrap()).await;
    assert_eq!(reply.text(), INDEX_HTML);
    assert!(!is_gate_page(&reply.text()));

    // 主域上的导航拿到的是作品邀请函（门禁页）
    let reply = site
        .send(root_nav(&format!("/p/{SLUG}")).body(Body::empty()).unwrap())
        .await;
    assert_eq!(reply.status, StatusCode::OK);
    assert!(is_gate_page(&reply.text()));
}

// ------------------------------------------------ 硬线 c · 没有绕过门禁的请求头

#[tokio::test]
async fn no_request_header_can_skip_the_gate() {
    // 门禁页就是作品邀请函（DESIGN §3.3），位于主域 `/p/{slug}`。
    // 它是信任凭证、告示牌和会话起点。没有任何请求头能跳过它。
    let site = Site::plain().await;

    let suspects: &[(&str, &str)] = &[
        ("x-playtest-no-gate", "1"),
        ("x-playtest-skip-gate", "true"),
        ("x-playtest-internal", "1"),
        ("x-playtest-token", "whatever"),
        ("x-playtest-gate", "off"),
        ("x-no-gate", "1"),
        ("x-skip-gate", "1"),
        ("x-bypass", "1"),
        ("x-forwarded-for", "127.0.0.1"),
        ("x-real-ip", "127.0.0.1"),
        ("cf-connecting-ip", "127.0.0.1"),
        ("x-forwarded-host", "brisk-otter-41.localhost"),
        ("x-forwarded-proto", "https"),
        ("x-requested-with", "XMLHttpRequest"),
        ("purpose", "prefetch"),
        ("sec-purpose", "prefetch;prerender"),
        ("x-moz", "prefetch"),
        ("authorization", "Bearer 000"),
        ("origin", "http://brisk-otter-41.localhost:8443"),
        ("referer", "http://brisk-otter-41.localhost:8443/"),
        ("user-agent", "Mozilla/5.0 (compatible; Googlebot/2.1)"),
        ("user-agent", "curl/8.7.1"),
        ("sec-fetch-mode", "cors"),
        ("sec-fetch-site", "same-origin"),
        ("sec-fetch-user", "?1"),
        ("cache-control", "no-cache"),
        ("cookie", "pt_gate2=1"),
        ("cookie", "xpt_gate=1"),
        ("cookie", "pt_gate_=1"),
        ("cookie", "pt_sid=0123456789abcdef0123456789abcdef"),
    ];

    for (name, value) in suspects {
        let reply = site
            .send(
                root_nav(&format!("/p/{SLUG}"))
                    .header(*name, *value)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await;
        assert_eq!(reply.status, StatusCode::OK, "{name}: {value}");
        assert!(
            is_gate_page(&reply.text()),
            "「{name}: {value}」把门禁页绕过去了"
        );
    }

    // 作品子域是彻底纯净的游戏沙盒：直接导航无需任何 cookie 即可直接拿到 index.html
    let passed = site.send(nav("/").body(Body::empty()).unwrap()).await;
    assert_eq!(passed.text(), INDEX_HTML);

    // 带有 ?from= 来源参数的访问会被 303 重定向到主域门禁页
    let redirected = site
        .send(nav("/?from=card").body(Body::empty()).unwrap())
        .await;
    assert_eq!(redirected.status, StatusCode::SEE_OTHER);
    assert_eq!(
        redirected.header("location"),
        Some("http://localhost:8443/p/brisk-otter-41")
    );
}

// ------------------------------------------------------- 同一类的一条：头别配反

#[tokio::test]
async fn unityweb_never_gets_a_content_encoding() {
    // Unity 的两种导出要求相反的服务器配置：`.br` 要 `Content-Encoding`，
    // Decompression Fallback 的 `.unityweb` 要**没有**——解压是加载器在 JS 里做的，
    // 浏览器再解一遍就崩。配反了同样是「200 + 错的头」，加载器崩在看不懂的地方。
    let site = Site::plain().await;
    let reply = site
        .send(
            base("/Build/game.wasm.unityweb")
                .header("accept-encoding", "gzip, deflate, br, zstd")
                .header("sec-fetch-dest", "empty")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.header("content-encoding"), None);
    assert_eq!(
        reply.header("content-type"),
        Some("application/octet-stream")
    );
    assert_eq!(reply.body.as_ref(), UNITYWEB);
}
