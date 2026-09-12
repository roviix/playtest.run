//! 打真的 Router：在临时目录里摆一个和 api 写出来的一模一样的对象存储，
//! 然后用 `tower::ServiceExt::oneshot` 一条条请求过去，断言玩家真正会收到的东西。

use std::sync::Arc;

use axum::body::{Body, Bytes};
use axum::http::{HeaderMap, Request, StatusCode};
use http_body_util::BodyExt;
use playtest_common::hash::hash_bytes;
use playtest_common::manifest::{validate_manifest, FileEntry, GateMode, Manifest, SCHEMA};
use playtest_common::store::{Current, FsStore};
use playtest_edge::{router, App, Config};
use tower::ServiceExt;

const SLUG: &str = "brisk-otter-41";
const HOST: &str = "brisk-otter-41.localhost:8443";
const BIG_LEN: usize = 1024 * 1024;

const INDEX_HTML: &str = "<!doctype html><title>小球大冒险</title><canvas id=c></canvas>";
const GAME_JS: &str = "0123456789 // 前十个字节拿来验 Range";
// 只有 wasm 的魔数，边缘不解析内容，够用了。
const WASM: &[u8] = b"\0asm\x01\0\0\0not-a-real-module";
const WASM_BR: &[u8] = b"<pretend brotli stream for game.wasm>";
const DATA_BR: &[u8] = b"<pretend brotli stream for Unity .data>";
const SUB_HTML: &str = "<!doctype html><title>子目录</title>";
// 只有 PNG 的魔数，边缘不解码图片，够用了。
const COVER_PNG: &[u8] = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR pretend cover";

fn with_cover(m: &mut Manifest) {
    m.cover = Some(playtest_common::manifest::Cover {
        hash: hash_bytes(COVER_PNG),
        size: COVER_PNG.len() as u64,
        mime: "image/png".into(),
    });
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
            ("Build/x.data.br", DATA_BR.to_vec()),
            ("big.bin", vec![7u8; BIG_LEN]),
            ("game.js", GAME_JS.as_bytes().to_vec()),
            ("game.wasm", WASM.to_vec()),
            ("game.wasm.br", WASM_BR.to_vec()),
            ("index.html", INDEX_HTML.as_bytes().to_vec()),
            ("sub/index.html", SUB_HTML.as_bytes().to_vec()),
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
        // 封面的 blob 也放进去，但不在 files 里——它是清单单独的一条引用（DESIGN §3.3）。
        store
            .put_blob(&hash_bytes(COVER_PNG), COVER_PNG)
            .await
            .unwrap();

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
            // 夹具是一个 Phaser 形状的导出物，门禁页上说「邀请你试玩」。
            engine: Some("phaser".into()),
            files,
        };
        shape(&mut manifest);
        // 夹具本身得是一份合法清单，否则测的就不是 api 会写出来的东西。
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
            api_internal_url: None,
        };
        Site {
            app: Arc::new(App::new(config)),
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

    async fn get(&self, path: &str) -> Reply {
        self.send(nav(path).body(Body::empty()).unwrap()).await
    }

    async fn get_root(&self, path: &str) -> Reply {
        self.send(nav_on("localhost:8443", path).body(Body::empty()).unwrap())
            .await
    }

    fn events(&self) -> Vec<serde_json::Value> {
        let raw = std::fs::read_to_string(self.app.events.path()).unwrap_or_default();
        raw.lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect()
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

/// 一个浏览器点开链接时的样子。
fn nav(path: &str) -> axum::http::request::Builder {
    nav_on(HOST, path)
}

fn nav_on(host: &str, path: &str) -> axum::http::request::Builder {
    Request::builder()
        .uri(path)
        .header("host", host)
        .header("accept", "text/html,application/xhtml+xml,*/*;q=0.8")
        .header("sec-fetch-dest", "document")
}

/// 游戏加载器取一个资源时的样子。
fn asset(path: &str) -> axum::http::request::Builder {
    Request::builder()
        .uri(path)
        .header("host", HOST)
        .header("accept", "*/*")
        .header("sec-fetch-dest", "empty")
}

fn passed_gate(builder: axum::http::request::Builder) -> axum::http::request::Builder {
    builder.header(
        "cookie",
        "pt_gate=1; pt_sid=0123456789abcdef0123456789abcdef",
    )
}

// ------------------------------------------------------------------ 门禁页

#[tokio::test]
async fn navigation_without_cookie_gets_the_gate_page() {
    let site = Site::plain().await;
    // 子域直出 index.html 纯净沙盒
    let reply = site.get("/").await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(
        reply.header("content-type"),
        Some("text/html; charset=utf-8")
    );
    assert_eq!(reply.header("cache-control"), Some("no-cache"));
    assert_eq!(reply.text(), INDEX_HTML);

    // 根域 /p/{slug} 承载门面与邀请函
    let root_reply = site.get_root(&format!("/p/{SLUG}")).await;
    assert_eq!(root_reply.status, StatusCode::OK);
    assert_eq!(
        root_reply.header("content-type"),
        Some("text/html; charset=utf-8")
    );
    assert_eq!(root_reply.header("cache-control"), None);

    let html = root_reply.text();
    assert!(html.contains("开始试玩"));
    assert!(html.contains("某某 邀请你试玩"));
    assert!(html.contains("《小球大冒险》"));
    assert!(html.contains("· v7"));
    assert!(html.contains(&format!("action=\"/p/{SLUG}\"")));
    // 门禁页出的不是作品的 index.html。
    assert!(!html.contains("<canvas"));
    // 玩家页面上不出现品牌域名。
    assert!(!html.contains(playtest_common::DEVELOPER_HOST));

    let events = site.events();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0]["type"], "html_view");
    assert_eq!(events[1]["type"], "gate_view");
    assert_eq!(events[1]["slug"], SLUG);
    assert_eq!(events[1]["version"], 7);
}

#[tokio::test]
async fn gate_cookie_lets_the_html_through() {
    let site = Site::plain().await;
    let reply = site
        .send(passed_gate(nav("/")).body(Body::empty()).unwrap())
        .await;

    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.text(), INDEX_HTML);
    assert_eq!(
        reply.header("content-type"),
        Some("text/html; charset=utf-8")
    );
    assert_eq!(reply.header("cache-control"), Some("no-cache"));
    assert_eq!(reply.header("accept-ranges"), Some("bytes"));
    assert_eq!(
        reply.header("etag"),
        Some(format!("\"{}\"", hash_bytes(INDEX_HTML.as_bytes())).as_str())
    );

    let events = site.events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["type"], "html_view");
    assert_eq!(events[0]["sid"], "0123456789abcdef0123456789abcdef");
}

#[tokio::test]
async fn assets_are_never_gated() {
    let site = Site::plain().await;
    // 没有任何 cookie，游戏的 js 照样要能取到——否则加载器会卡在半路。
    let reply = site
        .send(asset("/game.js").body(Body::empty()).unwrap())
        .await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.text(), GAME_JS);
    assert!(site.events().is_empty());
}

#[tokio::test]
async fn gate_never_serves_html_directly() {
    let site = Site::build(|m| m.gate = GateMode::Never).await;
    let reply = site.get("/").await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.text(), INDEX_HTML);
    assert_eq!(site.events()[0]["type"], "html_view");
}

#[tokio::test]
async fn wechat_gets_the_open_in_browser_tip() {
    let site = Site::build(|m| m.isolated = true).await;
    let reply = site
        .send(
            nav_on("localhost:8443", &format!("/p/{SLUG}"))
                .header(
                    "user-agent",
                    "Mozilla/5.0 (iPhone) MicroMessenger/8.0.49 NetType/WIFI",
                )
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    let html = reply.text();
    assert!(html.contains("在浏览器中打开"));
    assert!(html.contains("需要系统浏览器"));
    assert_eq!(site.events()[0]["wechat"], true);
}

// ------------------------------------------------------------------ 响应头

#[tokio::test]
async fn wasm_gets_the_streaming_compilation_mime() {
    let site = Site::plain().await;
    let reply = site
        .send(asset("/game.wasm").body(Body::empty()).unwrap())
        .await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.header("content-type"), Some("application/wasm"));
    assert_eq!(reply.header("content-encoding"), None);
    assert_eq!(reply.body.as_ref(), WASM);
    // 有 .br 兄弟，所以这条路径的响应会随 Accept-Encoding 变。
    assert_eq!(reply.header("vary"), Some("Accept-Encoding"));
    assert_eq!(
        reply.header("cache-control"),
        Some("public, max-age=0, must-revalidate")
    );
}

#[tokio::test]
async fn explicitly_requested_br_keeps_the_inner_type() {
    let site = Site::plain().await;
    // Unity 的加载器就是这样直接请求 Build/x.wasm.br 的。
    let reply = site
        .send(asset("/game.wasm.br").body(Body::empty()).unwrap())
        .await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.header("content-encoding"), Some("br"));
    assert_eq!(reply.header("content-type"), Some("application/wasm"));
    assert_eq!(reply.body.as_ref(), WASM_BR);

    let reply = site
        .send(asset("/Build/x.data.br").body(Body::empty()).unwrap())
        .await;
    assert_eq!(reply.header("content-encoding"), Some("br"));
    assert_eq!(
        reply.header("content-type"),
        Some("application/octet-stream")
    );
    assert_eq!(reply.body.as_ref(), DATA_BR);
}

#[tokio::test]
async fn precompressed_is_preferred_when_accepted() {
    let site = Site::plain().await;
    let reply = site
        .send(
            asset("/game.wasm")
                .header("accept-encoding", "gzip, deflate, br, zstd")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.header("content-encoding"), Some("br"));
    assert_eq!(reply.header("content-type"), Some("application/wasm"));
    assert_eq!(reply.header("vary"), Some("Accept-Encoding"));
    assert_eq!(reply.body.as_ref(), WASM_BR);
    // ETag 认的是真正出去的那份字节。
    assert_eq!(
        reply.header("etag"),
        Some(format!("\"{}\"", hash_bytes(WASM_BR)).as_str())
    );
}

#[tokio::test]
async fn isolated_manifest_adds_the_cross_origin_headers() {
    let site = Site::build(|m| m.isolated = true).await;

    let asset_reply = site
        .send(asset("/game.wasm").body(Body::empty()).unwrap())
        .await;
    assert_eq!(
        asset_reply.header("cross-origin-opener-policy"),
        Some("same-origin")
    );
    assert_eq!(
        asset_reply.header("cross-origin-embedder-policy"),
        Some("require-corp")
    );
    assert_eq!(
        asset_reply.header("cross-origin-resource-policy"),
        Some("same-origin")
    );

    // 门禁页也在这个作品下面，同样要带，否则点开始之后才隔离等于没隔离。
    let gate_reply = site.get("/").await;
    assert_eq!(
        gate_reply.header("cross-origin-opener-policy"),
        Some("same-origin")
    );
    assert_eq!(gate_reply.header("cross-origin-resource-policy"), None);
}

#[tokio::test]
async fn no_csp_header_anywhere() {
    let site = Site::plain().await;
    // CSP 会把别人的游戏弄坏，我们不发。
    assert!(site
        .get("/")
        .await
        .header("content-security-policy")
        .is_none());
    assert!(site
        .send(asset("/game.js").body(Body::empty()).unwrap())
        .await
        .header("content-security-policy")
        .is_none());
}

// -------------------------------------------------------------- Range 与缓存

#[tokio::test]
async fn whole_file_then_single_range() {
    let site = Site::plain().await;

    let whole = site
        .send(asset("/game.js").body(Body::empty()).unwrap())
        .await;
    assert_eq!(whole.status, StatusCode::OK);
    assert_eq!(whole.header("accept-ranges"), Some("bytes"));
    assert_eq!(whole.header("content-range"), None);
    assert_eq!(
        whole.header("content-length"),
        Some(GAME_JS.len().to_string().as_str())
    );

    let part = site
        .send(
            asset("/game.js")
                .header("range", "bytes=0-9")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(part.status, StatusCode::PARTIAL_CONTENT);
    assert_eq!(
        part.header("content-range"),
        Some(format!("bytes 0-9/{}", GAME_JS.len()).as_str())
    );
    assert_eq!(part.header("content-length"), Some("10"));
    assert_eq!(part.text(), "0123456789");
}

#[tokio::test]
async fn suffix_range_on_a_one_mib_file() {
    let site = Site::plain().await;
    let reply = site
        .send(
            asset("/big.bin")
                .header("range", "bytes=-16")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(reply.status, StatusCode::PARTIAL_CONTENT);
    assert_eq!(
        reply.header("content-range"),
        Some(format!("bytes {}-{}/{}", BIG_LEN - 16, BIG_LEN - 1, BIG_LEN).as_str())
    );
    assert_eq!(reply.body.len(), 16);
    assert_eq!(
        reply.header("content-type"),
        Some("application/octet-stream")
    );
}

#[tokio::test]
async fn out_of_range_is_416() {
    let site = Site::plain().await;
    let reply = site
        .send(
            asset("/game.js")
                .header("range", "bytes=999999999-")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(reply.status, StatusCode::RANGE_NOT_SATISFIABLE);
    assert_eq!(
        reply.header("content-range"),
        Some(format!("bytes */{}", GAME_JS.len()).as_str())
    );
}

#[tokio::test]
async fn multi_range_falls_back_to_the_whole_file() {
    let site = Site::plain().await;
    let reply = site
        .send(
            asset("/game.js")
                .header("range", "bytes=0-3,8-9")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.text(), GAME_JS);
}

#[tokio::test]
async fn if_none_match_gets_304() {
    let site = Site::plain().await;
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
    assert_eq!(reply.header("etag"), Some(etag.as_str()));
    assert!(reply.body.is_empty());

    // 换一个 ETag 就要重新拿全文。
    let reply = site
        .send(
            asset("/game.js")
                .header("if-none-match", "\"nope\"")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(reply.status, StatusCode::OK);
}

#[tokio::test]
async fn head_reports_the_length_without_the_body() {
    let site = Site::plain().await;
    let reply = site
        .send(
            asset("/big.bin")
                .method("HEAD")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(
        reply.header("content-length"),
        Some(BIG_LEN.to_string().as_str())
    );
    assert!(reply.body.is_empty());
}

// ------------------------------------------------------------------ 路径

#[tokio::test]
async fn directory_without_trailing_slash_redirects() {
    let site = Site::plain().await;
    let reply = site.get("/sub").await;
    assert_eq!(reply.status, StatusCode::MOVED_PERMANENTLY);
    assert_eq!(reply.header("location"), Some("/sub/"));

    let reply = site
        .send(passed_gate(nav("/sub/")).body(Body::empty()).unwrap())
        .await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.text(), SUB_HTML);
}

#[tokio::test]
async fn traversal_and_missing_files_are_our_own_404_page() {
    let site = Site::plain().await;
    for path in ["/nope.png", "/../etc/passwd", "/%2e%2e/secret", "/a%00b"] {
        let reply = site.get(path).await;
        assert_eq!(reply.status, StatusCode::NOT_FOUND, "{path}");
        assert_eq!(
            reply.header("content-type"),
            Some("text/html; charset=utf-8"),
            "{path}"
        );
        assert!(reply.text().contains("<h1>"), "{path}");
    }
}

#[tokio::test]
async fn spa_fallback_only_for_navigation() {
    let site = Site::build(|m| m.spa = true).await;
    let reply = site
        .send(passed_gate(nav("/level/3")).body(Body::empty()).unwrap())
        .await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.text(), INDEX_HTML);

    let reply = site
        .send(asset("/level/3.png").body(Body::empty()).unwrap())
        .await;
    assert_eq!(reply.status, StatusCode::NOT_FOUND);
}

// ------------------------------------------------------------- 作品状态

#[tokio::test]
async fn unknown_slug_is_a_rendered_404() {
    let site = Site::plain().await;
    let reply = site
        .send(
            nav_on("never-used-42.localhost:8443", "/")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(reply.status, StatusCode::NOT_FOUND);
    assert!(reply.text().contains("这个链接不存在，或者已经失效"));
    assert_eq!(reply.header("x-content-type-options"), Some("nosniff"));
}

#[tokio::test]
async fn the_cover_is_served_from_the_reserved_path() {
    // 有封面：从 /_playtest/cover 出，按清单里记的类型给，不经过门禁，可以缓存。
    let site = Site::build(with_cover).await;
    let reply = site.get("/_playtest/cover").await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.header("content-type"), Some("image/png"));
    assert_eq!(reply.body.as_ref(), COVER_PNG);
    assert!(reply
        .header("cache-control")
        .is_some_and(|c| c.starts_with("public, max-age=")));
    assert_eq!(
        reply.header("etag"),
        Some(format!("\"{}\"", hash_bytes(COVER_PNG)).as_str())
    );
    // 门禁页拿它当第一眼，也拿它当分享卡片的图。
    let gate = site.get_root(&format!("/p/{SLUG}")).await;
    assert!(gate.text().contains(
        "<img class=\"hero\" src=\"http://brisk-otter-41.localhost:8443/_playtest/cover\""
    ));
    assert!(gate.text().contains(
        "<meta property=\"og:image\" content=\"http://brisk-otter-41.localhost:8443/_playtest/cover\">"
    ));

    // 没有封面：裸 404，一个字节的 HTML 都没有——抓卡片的机器人和 <img> 只认状态码。
    let plain = Site::plain().await;
    let reply = plain.get("/_playtest/cover").await;
    assert_eq!(reply.status, StatusCode::NOT_FOUND);
    assert!(reply.body.is_empty());

    // 但分享出去仍然有图：横版邀请卡。它写的是作品名和开发者名，
    // 不是一张假截图，所以可以当 og:image（DESIGN §3.3）。
    let html = plain.get_root(&format!("/p/{SLUG}")).await.text();
    assert!(html.contains(
        "<meta property=\"og:image\" content=\"http://brisk-otter-41.localhost:8443/_playtest/card-wide.png\">"
    ));
    assert!(html.contains("<meta property=\"og:image:width\" content=\"1200\">"));
    assert!(html.contains("<meta property=\"og:image:height\" content=\"630\">"));
    assert!(!html.contains("/_playtest/cover"));
}

#[tokio::test]
async fn expired_anonymous_link_is_410() {
    let site = Site::build(|m| m.expires_at = Some("2026-09-06T00:00:00Z".into())).await;
    let reply = site.get("/").await;
    assert_eq!(reply.status, StatusCode::GONE);
    assert!(reply.text().contains("已过期"));
    assert!(reply.text().contains("24 小时"));
}

#[tokio::test]
async fn host_routing() {
    let site = Site::plain().await;

    // 根域是广场（DESIGN §3.9）：发布说明里那条控制台链接，是玩家路径上唯一去开发者域的出口。
    // 这个测试的对象存储里没有 plaza.json，所以是空广场，但页面照常出、说明照常在。
    for host in ["localhost:8443", "www.localhost"] {
        let reply = site
            .send(nav_on(host, "/").body(Body::empty()).unwrap())
            .await;
        assert_eq!(reply.status, StatusCode::OK, "{host}");
        let text = reply.text();
        assert!(text.contains("广场上还没有作品"), "{host}");
        assert!(text.contains("--public"), "{host}");
        assert!(text.contains(playtest_common::DEVELOPER_API_URL), "{host}");
        // 这一页是我们自己的，能锁死；脚本只放行带 nonce 的那段，图只从作品子域来。
        // 作品页面上则一个 CSP 都不能有（见 no_csp_header_anywhere）。
        let csp = reply.header("content-security-policy").unwrap_or_default();
        assert!(csp.starts_with("default-src 'none'"), "{host}: {csp}");
        assert!(csp.contains("img-src http://*.localhost"), "{host}: {csp}");
        assert!(csp.contains("script-src 'nonce-"), "{host}: {csp}");
    }

    // 根域下别的路径没有内容。
    let reply = site
        .send(
            nav_on("localhost:8443", "/anything")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(reply.status, StatusCode::NOT_FOUND);

    // 多级、保留名都不是作品。
    for host in ["a.b.localhost", "admin.localhost", "example.com"] {
        let reply = site
            .send(nav_on(host, "/").body(Body::empty()).unwrap())
            .await;
        assert_eq!(reply.status, StatusCode::NOT_FOUND, "{host}");
        assert!(
            !reply.text().contains(playtest_common::DEVELOPER_HOST),
            "{host}"
        );
    }
}

// -------------------------------------------------------------- 保留路径

#[tokio::test]
async fn healthz_answers_on_any_host() {
    let site = Site::plain().await;
    let reply = site
        .send(
            Request::builder()
                .uri("/_playtest/healthz")
                .header("host", "127.0.0.1:8443")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.text(), "ok\n");
}

#[tokio::test]
async fn start_sets_cookies_and_redirects_back() {
    let site = Site::plain().await;
    let reply = site
        .send(
            Request::builder()
                .method("POST")
                .uri("/_playtest/start")
                .header("host", HOST)
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from("to=%2Flevel%2F3"))
                .unwrap(),
        )
        .await;

    assert_eq!(reply.status, StatusCode::SEE_OTHER);
    assert_eq!(reply.header("location"), Some("/level/3"));

    let cookies = reply.cookies();
    let gate = cookies.iter().find(|c| c.starts_with("pt_gate=")).unwrap();
    assert!(gate.contains("Max-Age=86400"));
    assert!(gate.contains("Path=/"));
    assert!(gate.contains("SameSite=Lax"));
    assert!(gate.contains("HttpOnly"));
    let sid = cookies.iter().find(|c| c.starts_with("pt_sid=")).unwrap();
    assert!(sid.contains("Max-Age=7776000"));

    let events = site.events();
    assert_eq!(events[0]["type"], "start");
    assert_eq!(events[0]["sid"].as_str().unwrap().len(), 32);
}

/// 「来自哪里」要能穿过门禁页：玩家从 Discord 点进来，门禁页收到的 Referer 是 discord.com，
/// 点「开始」那一下 POST 的 Referer 却是门禁页自己。真正的来源由门禁页放进表单带过去，
/// `start` 事件记的必须是 discord.com，而不是作品自己。
#[tokio::test]
async fn the_real_referrer_survives_the_gate() {
    let site = Site::plain().await;
    let gate = site
        .send(
            nav_on("localhost:8443", &format!("/p/{SLUG}"))
                .header("referer", "https://discord.com/channels/1/2")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(gate.status, StatusCode::OK);
    let html = gate.text();
    assert!(
        html.contains("name=\"ref\" value=\"https://discord.com/channels/1/2\""),
        "门禁页要把来源放进表单：{html}"
    );

    let reply = site
        .send(
            Request::builder()
                .method("POST")
                .uri(&format!("/p/{SLUG}"))
                .header("host", "localhost:8443")
                .header("referer", format!("http://localhost:8443/p/{SLUG}"))
                .header("origin", "http://localhost:8443")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(
                    "to=%2F&ref=https%3A%2F%2Fdiscord.com%2Fchannels%2F1%2F2",
                ))
                .unwrap(),
        )
        .await;
    assert_eq!(reply.status, StatusCode::SEE_OTHER);
    assert_eq!(
        reply.header("location"),
        Some(format!("http://{HOST}/").as_str())
    );

    let events = site.events();
    let start = events.iter().find(|e| e["type"] == "start").unwrap();
    assert_eq!(start["referer"], "https://discord.com/channels/1/2");

    // 表单里塞一个不是 URL 的东西，当没有，不照抄。
    let reply = site
        .send(
            Request::builder()
                .method("POST")
                .uri(&format!("/p/{SLUG}"))
                .header("host", "localhost:8443")
                .header("origin", "http://localhost:8443")
                .header("referer", format!("http://localhost:8443/p/{SLUG}"))
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from("to=%2F&ref=javascript%3Aalert(1)"))
                .unwrap(),
        )
        .await;
    assert_eq!(reply.status, StatusCode::SEE_OTHER);
    let events = site.events();
    assert_eq!(events.last().unwrap()["referer"], "");
}

#[tokio::test]
async fn start_refuses_to_redirect_off_site() {
    let site = Site::plain().await;
    for target in [
        "to=https%3A%2F%2Fevil.example",
        "to=%2F%2Fevil.example",
        "to=%2F%5Cevil",
    ] {
        let reply = site
            .send(
                Request::builder()
                    .method("POST")
                    .uri("/_playtest/start")
                    .header("host", HOST)
                    .body(Body::from(target.to_string()))
                    .unwrap(),
            )
            .await;
        assert_eq!(reply.status, StatusCode::SEE_OTHER, "{target}");
        assert_eq!(reply.header("location"), Some("/"), "{target}");
    }
}

#[tokio::test]
async fn always_mode_uses_a_session_cookie() {
    let site = Site::build(|m| m.gate = GateMode::Always).await;
    let reply = site
        .send(
            Request::builder()
                .method("POST")
                .uri("/_playtest/start")
                .header("host", HOST)
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    let gate = reply
        .cookies()
        .into_iter()
        .find(|c| c.starts_with("pt_gate="))
        .unwrap();
    assert!(
        !gate.contains("Max-Age"),
        "每次都出的作品不该记住一整天：{gate}"
    );
}

#[tokio::test]
async fn report_collects_a_reason_and_says_nothing_more() {
    let site = Site::plain().await;

    let form = site.get("/_playtest/report").await;
    assert_eq!(form.status, StatusCode::OK);
    assert!(form.text().contains("<select name=\"reason\">"));
    assert!(form.text().contains("<textarea name=\"detail\""));

    let done = site
        .send(
            Request::builder()
                .method("POST")
                .uri("/_playtest/report")
                .header("host", HOST)
                .body(Body::from("reason=phishing&detail=%E5%81%87%E7%9A%84"))
                .unwrap(),
        )
        .await;
    assert_eq!(done.status, StatusCode::OK);
    assert!(done.text().contains("已收到。"));

    let events = site.events();
    assert_eq!(events[0]["type"], "report");
    assert_eq!(events[0]["reason"], "phishing");
    assert_eq!(events[0]["detail"], "假的");
    // 不记 IP。
    assert!(events[0].get("ip").is_none());
}

#[tokio::test]
async fn reserved_prefix_shadows_site_files() {
    let site = Site::plain().await;
    let reply = site.get("/_playtest/anything").await;
    assert_eq!(reply.status, StatusCode::NOT_FOUND);

    let reply = site
        .send(
            Request::builder()
                .method("PUT")
                .uri("/_playtest/start")
                .header("host", HOST)
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(reply.status, StatusCode::METHOD_NOT_ALLOWED);
    assert_eq!(reply.header("allow"), Some("POST"));
}
