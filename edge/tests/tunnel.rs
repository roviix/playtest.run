//! 一整条隧道，三个真的进程角色都在同一个测试里跑（DESIGN §4.3）：
//!
//! - **边缘**：真的 `playtest-edge` Router，`axum::serve` 绑在 `127.0.0.1:0` 上。
//! - **开发者的 dev server**：另一个 axum，也绑 0 端口。故意把 `game.wasm` 说成
//!   `application/octet-stream`（Vite 之外的 dev server 常这样），好看边缘补没补回来。
//! - **CLI**：`common` 的 `handshake_request` + `connect_async` + yamux Acceptor，
//!   收到流就连 dev server 对拷。和真 CLI 的形状一样，只是没有重连退避。
//!
//! 玩家用 hyper 的 HTTP/1.1 客户端一条连接打一次，Host 写成 `brisk-otter-41.localhost`。
//! 每条测试都有超时：隧道上任何一处卡住都表现为「挂着不动」，不设超时就是让 CI 挂死。

// 测试脚手架：为了把一条用例写成一眼能看完的样子，这里放宽 type_complexity。
#![allow(clippy::type_complexity)]

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::{Html, IntoResponse};
use axum::routing::get;
use axum::Router;
use bytes::Bytes;
use futures::{SinkExt, StreamExt};
use http_body_util::{BodyExt, Full};
use hyper::header::{HeaderName, HeaderValue};
use hyper::{HeaderMap, Method, Request, StatusCode};
use hyper_util::rt::TokioIo;
use playtest_common::api::{ErrorBody, ErrorCode};
use playtest_common::manifest::GateMode;
use playtest_common::tunnel::io::{handshake_request, Mux, Role, WsByteStream};
use playtest_common::tunnel::{
    key_files, Claims, SigningKey, HEADER_LOCAL_PORT, TOKEN_TTL_SECS, WS_PATH, WS_PROTOCOL,
};
use playtest_edge::{router, App, Config};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;

const SLUG: &str = "brisk-otter-41";
const HOST: &str = "brisk-otter-41.localhost";
/// 一条测试最多跑这么久。本机全部在毫秒级，留这么宽只是为了让失败长成「断言不过」
/// 而不是「CI 卡了十分钟」。
const LIMIT: Duration = Duration::from_secs(30);

const UPSTREAM_HTML: &str =
    "<!doctype html><title>开发者机器上的那一版</title><canvas id=c></canvas>";
const WASM: &[u8] = b"\0asm\x01\0\0\0not-a-real-module";
/// 2 MiB。远大于 yamux 单流 256 KiB 的初始接收窗口，能把流控真的走一遍。
const BIG_LEN: usize = 2 * 1024 * 1024;

fn big_body() -> Vec<u8> {
    (0..BIG_LEN).map(|i| (i % 251) as u8).collect()
}

// ------------------------------------------------------------------ 三个角色

struct Edge {
    _dir: tempfile::TempDir,
    addr: SocketAddr,
    signing: SigningKey,
    /// 拿着它测试才能直接问「现在谁连着」。
    app: Arc<App>,
}

impl Edge {
    /// 控制面在这里被压缩成一件事：签令牌的私钥，以及它写进对象存储的那把公钥。
    async fn start() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let signing = SigningKey::generate();
        let key_path = dir
            .path()
            .join("store")
            .join(key_files::VERIFYING_KEY_OBJECT);
        std::fs::create_dir_all(key_path.parent().unwrap()).unwrap();
        std::fs::write(&key_path, signing.verifying_key().to_base64()).unwrap();

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let app = Arc::new(App::new(Config {
            listen: addr,
            data_dir: dir.path().to_path_buf(),
            host_suffix: "localhost".into(),
            public_scheme: "http".into(),
        }));
        let service = router(app.clone());
        tokio::spawn(async move {
            let _ = axum::serve(listener, service).await;
        });

        Edge {
            _dir: dir,
            addr,
            signing,
            app,
        }
    }

    fn token(&self, jti: &str, shape: impl FnOnce(&mut Claims)) -> String {
        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        let mut claims = Claims {
            v: 1,
            slug: SLUG.into(),
            sub: "user-1".into(),
            title: "小球大冒险".into(),
            developer: "某某".into(),
            badge: true,
            gate: GateMode::Once,
            isolated: false,
            max_players: 50,
            iat: now,
            exp: now + TOKEN_TTL_SECS,
            jti: jti.into(),
        };
        shape(&mut claims);
        self.signing.sign(&claims)
    }

    /// 101 回来之后，边缘把连接变成 yamux 并登记会话是另一个任务的事。
    async fn wait_online(&self) {
        for _ in 0..600 {
            if self.app.tunnels.get(SLUG).is_some() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        panic!("等了 3 秒隧道还没登记上");
    }

    async fn wait_offline(&self) {
        for _ in 0..600 {
            if self.app.tunnels.get(SLUG).is_none() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        panic!("等了 3 秒隧道还没注销");
    }
}

/// 开发者机器上跑着的那个东西。
async fn upstream() -> SocketAddr {
    let app = Router::new()
        .route("/", get(|| async { Html(UPSTREAM_HTML) }))
        .route(
            "/game.wasm",
            // dev server 最常见的坑：把 wasm 说成字节流，浏览器就没有流式编译。
            get(|| async { ([("content-type", "application/octet-stream")], WASM) }),
        )
        .route("/big", get(|| async { Bytes::from(big_body()) }))
        .route("/echo", get(echo));

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    addr
}

async fn echo(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(|mut socket: WebSocket| async move {
        while let Some(Ok(message)) = socket.recv().await {
            if let Message::Text(text) = message {
                let back = Message::Text(format!("回声：{text}").into());
                if socket.send(back).await.is_err() {
                    return;
                }
            }
        }
    })
}

/// 假的 CLI：出站一条 WebSocket，之后每收到一条流就连开发者的端口对拷。
struct FakeCli {
    accepting: tokio::task::JoinHandle<()>,
}

impl FakeCli {
    async fn connect(edge: &Edge, token: &str, local: SocketAddr) -> Self {
        let url = format!("ws://{}{WS_PATH}", edge.addr);
        let mut request = handshake_request(&url, token, local.port()).unwrap();
        // 边缘按 Host 找作品，而 connect_async 默认写的是 IP。真 CLI 连的是
        // `wss://<slug>.playtest.run`，Host 天然就对。
        request
            .headers_mut()
            .insert("host", HeaderValue::from_static(HOST));

        let (ws, response) = tokio_tungstenite::connect_async(request)
            .await
            .expect("握手该成功");
        assert_eq!(response.status(), StatusCode::SWITCHING_PROTOCOLS);
        assert_eq!(response.headers()["sec-websocket-protocol"], WS_PROTOCOL);

        let mut mux = Mux::spawn(WsByteStream::new(ws), Role::Acceptor);
        let accepting = tokio::spawn(async move {
            while let Some(mut stream) = mux.accept().await {
                tokio::spawn(async move {
                    let Ok(mut local) = TcpStream::connect(local).await else {
                        return;
                    };
                    let _ = tokio::io::copy_bidirectional(&mut stream, &mut local).await;
                });
            }
        });
        FakeCli { accepting }
    }

    /// 拔网线：把驾驭 yamux 的句柄丢掉，对端很快就会看到连接结束。
    fn disconnect(self) {
        self.accepting.abort();
    }

    /// 这条隧道结束时完成——`accept()` 返回 `None`，收流的循环就走到头了。
    async fn wait_closed(self) {
        let _ = tokio::time::timeout(LIMIT, self.accepting)
            .await
            .expect("被挤掉的那条连接该被关掉");
    }
}

// ------------------------------------------------------------------ 玩家这一侧

struct Reply {
    status: StatusCode,
    headers: HeaderMap,
    body: Bytes,
}

impl Reply {
    fn text(&self) -> std::borrow::Cow<'_, str> {
        String::from_utf8_lossy(&self.body)
    }

    fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name).and_then(|v| v.to_str().ok())
    }
}

/// 一次请求一条新连接：连接复用不是这里要测的东西，每次新开最省心。
async fn send(edge: SocketAddr, request: Request<Full<Bytes>>) -> Reply {
    let io = TokioIo::new(TcpStream::connect(edge).await.unwrap());
    let (mut sender, conn) = hyper::client::conn::http1::handshake(io).await.unwrap();
    tokio::spawn(async move {
        let _ = conn.await;
    });
    let response = sender
        .send_request(request)
        .await
        .expect("边缘该给一个响应");
    let status = response.status();
    let headers = response.headers().clone();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    Reply {
        status,
        headers,
        body,
    }
}

fn request(
    method: Method,
    path: &str,
    headers: &[(&str, &str)],
    body: &str,
) -> Request<Full<Bytes>> {
    let mut builder = Request::builder()
        .method(method)
        .uri(path)
        .header("host", HOST);
    for (name, value) in headers {
        builder = builder.header(*name, *value);
    }
    builder
        .body(Full::new(Bytes::from(body.to_string())))
        .unwrap()
}

/// 浏览器点开一条链接时长的样子。
async fn navigate(edge: SocketAddr, path: &str, cookie: Option<&str>) -> Reply {
    let mut headers = vec![
        ("accept", "text/html,application/xhtml+xml"),
        ("sec-fetch-dest", "document"),
    ];
    if let Some(cookie) = cookie {
        headers.push(("cookie", cookie));
    }
    send(edge, request(Method::GET, path, &headers, "")).await
}

/// 游戏加载器取一个子资源时长的样子。
async fn fetch(edge: SocketAddr, path: &str) -> Reply {
    send(
        edge,
        request(Method::GET, path, &[("sec-fetch-dest", "empty")], ""),
    )
    .await
}

/// 一次形状完整的握手请求，用普通 HTTP 客户端发——要看的是被拒时的 JSON。
fn handshake_headers(token: &str, port: u16) -> Vec<(String, String)> {
    [
        ("connection", "Upgrade"),
        ("upgrade", "websocket"),
        ("sec-websocket-version", "13"),
        ("sec-websocket-key", "dGhlIHNhbXBsZSBub25jZQ=="),
        ("sec-websocket-protocol", WS_PROTOCOL),
    ]
    .into_iter()
    .map(|(n, v)| (n.to_string(), v.to_string()))
    .chain([
        ("authorization".to_string(), format!("Bearer {token}")),
        (HEADER_LOCAL_PORT.to_string(), port.to_string()),
    ])
    .collect()
}

async fn probe_handshake(edge: SocketAddr, headers: &[(String, String)]) -> Reply {
    let mut builder = Request::builder()
        .method(Method::GET)
        .uri(WS_PATH)
        .header("host", HOST);
    for (name, value) in headers {
        builder = builder.header(
            HeaderName::from_bytes(name.as_bytes()).unwrap(),
            HeaderValue::from_str(value).unwrap(),
        );
    }
    send(edge, builder.body(Full::new(Bytes::new())).unwrap()).await
}

fn error_body(reply: &Reply) -> ErrorBody {
    serde_json::from_slice(&reply.body)
        .unwrap_or_else(|e| panic!("被拒时该给一份 ErrorBody：{e}：{}", reply.text()))
}

/// `Set-Cookie` 里的门禁 cookie，拼成下一次请求能直接用的 `Cookie` 头。
fn gate_cookie(reply: &Reply) -> String {
    reply
        .headers
        .get_all("set-cookie")
        .iter()
        .filter_map(|v| v.to_str().ok())
        .find_map(|line| line.split(';').next().filter(|c| c.starts_with("pt_gate=")))
        .expect("「开始」那一下该种门禁 cookie")
        .to_string()
}

// ------------------------------------------------------------------ 测试

#[tokio::test]
async fn a_player_plays_what_is_running_on_the_developers_machine() {
    tokio::time::timeout(LIMIT, async {
        let edge = Edge::start().await;
        let dev = upstream().await;
        let cli = FakeCli::connect(&edge, &edge.token("jti-1", |_| {}), dev).await;
        edge.wait_online().await;

        // 1. 第一眼是门禁页，不是游戏。版本那个位置写「在线」——隧道没有版本。
        let gate = navigate(edge.addr, "/", None).await;
        assert_eq!(gate.status, StatusCode::OK);
        assert_eq!(
            gate.header("content-type"),
            Some("text/html; charset=utf-8")
        );
        assert!(gate.text().contains("某某 邀请你体验"));
        assert!(gate.text().contains("《小球大冒险》"));
        assert!(gate.text().contains("· 在线"), "版本位置该是「在线」");
        assert!(!gate.text().contains("v0"), "玩家不该看到合成清单里那个 v0");
        assert!(!gate.text().contains(playtest_common::DEVELOPER_HOST));
        // 门禁页是我们渲染的，开发者的 HTML 一个字节都还没出去。
        assert!(!gate.text().contains("开发者机器上的那一版"));

        // 2. 点「开始」之后才是开发者机器上那一版。
        let started = send(
            edge.addr,
            request(Method::POST, "/_playtest/start", &[], "to=%2F"),
        )
        .await;
        assert_eq!(started.status, StatusCode::SEE_OTHER);
        let cookie = gate_cookie(&started);

        let page = navigate(edge.addr, "/", Some(&cookie)).await;
        assert_eq!(page.status, StatusCode::OK);
        assert_eq!(page.text(), UPSTREAM_HTML, "响应体要一个字节不差");

        // 3. dev server 把 wasm 说成了字节流，边缘换回来——否则没有流式编译。
        let wasm = fetch(edge.addr, "/game.wasm").await;
        assert_eq!(wasm.status, StatusCode::OK);
        assert_eq!(wasm.header("content-type"), Some("application/wasm"));
        assert_eq!(&wasm.body[..], WASM);

        // 4. 2 MiB 过一条 yamux 流，字节要完全一样（流控真的走了一遍）。
        let big = fetch(edge.addr, "/big").await;
        assert_eq!(big.status, StatusCode::OK);
        assert_eq!(big.body.len(), BIG_LEN);
        assert_eq!(&big.body[..], &big_body()[..]);

        // 5. WebSocket 原样过去：101 之后边缘只是两根管子。
        let mut player = format!("ws://{}/echo", edge.addr)
            .into_client_request()
            .unwrap();
        player
            .headers_mut()
            .insert("host", HeaderValue::from_static(HOST));
        let (mut socket, response) = tokio_tungstenite::connect_async(player)
            .await
            .expect("WebSocket 该能穿过隧道");
        assert_eq!(response.status(), StatusCode::SWITCHING_PROTOCOLS);
        socket
            .send(tokio_tungstenite::tungstenite::Message::text("你好"))
            .await
            .unwrap();
        let back = socket.next().await.unwrap().unwrap();
        assert_eq!(back.into_text().unwrap().as_str(), "回声：你好");

        // 记账：请求数和两个方向的字节数都动了。
        let session = edge.app.tunnels.get(SLUG).unwrap();
        use std::sync::atomic::Ordering::Relaxed;
        assert!(session.requests.load(Relaxed) >= 4);
        assert!(session.bytes_out.load(Relaxed) >= BIG_LEN as u64);
        // 长连接开着的时候名额就该占着一个，否则 max_players 形同虚设。
        assert_eq!(session.open_streams.load(Relaxed), 1);

        // 玩家走了，名额要还回来，不能漏。
        socket.close(None).await.unwrap();
        drop(socket);
        for _ in 0..600 {
            if session.open_streams.load(Relaxed) == 0 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        assert_eq!(session.open_streams.load(Relaxed), 0);

        cli.disconnect();
    })
    .await
    .expect("超时：隧道上的某一步卡住了");
}

#[tokio::test]
async fn a_second_process_takes_over_and_the_old_token_is_finished() {
    tokio::time::timeout(LIMIT, async {
        let edge = Edge::start().await;
        let dev = upstream().await;

        let first = FakeCli::connect(&edge, &edge.token("jti-1", |_| {}), dev).await;
        edge.wait_online().await;

        // 同一个作品又开了一个 playtest 进程：新的挤掉旧的（DESIGN §4.3）。
        let second = FakeCli::connect(&edge, &edge.token("jti-2", |_| {}), dev).await;
        first.wait_closed().await;
        assert_eq!(edge.app.tunnels.get(SLUG).unwrap().claims.jti, "jti-2");

        // 旧进程拿旧令牌重连：一律 409，CLI 据此退出而不是和新进程互相挤。
        let refused = probe_handshake(
            edge.addr,
            &handshake_headers(&edge.token("jti-1", |_| {}), dev.port()),
        )
        .await;
        assert_eq!(refused.status, StatusCode::CONFLICT);
        let body = error_body(&refused);
        assert_eq!(body.code, ErrorCode::TunnelReplaced);
        assert!(
            body.message.contains("另一个 playtest 进程"),
            "要说清是被谁接管了：{}",
            body.message
        );
        // 被拒之后新的那条还好好的。
        assert_eq!(navigate(edge.addr, "/", None).await.status, StatusCode::OK);

        second.disconnect();
    })
    .await
    .expect("超时：挤掉旧连接这一步卡住了");
}

#[tokio::test]
async fn the_offline_page_remembers_who_was_here() {
    tokio::time::timeout(LIMIT, async {
        let edge = Edge::start().await;
        let dev = upstream().await;

        // 没连过的 slug 就是不存在，不是「离线」——离线要有人在线过才说得出口。
        let unknown = navigate(edge.addr, "/", None).await;
        assert_eq!(unknown.status, StatusCode::NOT_FOUND);
        assert!(unknown.text().contains("这个链接不存在"));

        let cli = FakeCli::connect(&edge, &edge.token("jti-1", |_| {}), dev).await;
        edge.wait_online().await;
        cli.disconnect();
        edge.wait_offline().await;

        let offline = navigate(edge.addr, "/", None).await;
        assert_eq!(offline.status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(offline.header("cache-control"), Some("no-store"));
        assert_eq!(
            offline.header("content-type"),
            Some("text/html; charset=utf-8")
        );
        let html = offline.text();
        assert!(html.contains("某某 的电脑暂时不在线"));
        assert!(html.contains("《小球大冒险》"));
        assert!(html.contains("上次在线"));
        assert!(html.contains("<time datetime=\""));
        assert!(!html.contains(playtest_common::DEVELOPER_HOST));

        // 落盘了，边缘重启之后还说得出来。
        let saved = std::fs::read_to_string(
            edge._dir
                .path()
                .join("tunnels")
                .join(format!("{SLUG}.json")),
        )
        .expect("上次在线要落到边缘自己的目录里");
        assert!(saved.contains("小球大冒险"));
    })
    .await
    .expect("超时：离线这一步卡住了");
}

#[tokio::test]
async fn an_isolated_site_keeps_its_cross_origin_headers_through_the_tunnel() {
    tokio::time::timeout(LIMIT, async {
        let edge = Edge::start().await;
        let dev = upstream().await;
        let token = edge.token("jti-1", |c| c.isolated = true);
        let cli = FakeCli::connect(&edge, &token, dev).await;
        edge.wait_online().await;

        // dev server 一个隔离头都不发（Vite 就是这样），Godot 4 的线程导出因此白屏。
        let wasm = fetch(edge.addr, "/game.wasm").await;
        assert_eq!(
            wasm.header("cross-origin-opener-policy"),
            Some("same-origin")
        );
        assert_eq!(
            wasm.header("cross-origin-embedder-policy"),
            Some("require-corp")
        );
        // 子资源还要 CORP，不然 COEP 把它自己挡在外面。
        assert_eq!(
            wasm.header("cross-origin-resource-policy"),
            Some("same-origin")
        );

        // 顶层文档不给 CORP，门禁页也一样要隔离——点了开始之后才隔离等于没隔离。
        let gate = navigate(edge.addr, "/", None).await;
        assert_eq!(
            gate.header("cross-origin-opener-policy"),
            Some("same-origin")
        );
        assert_eq!(gate.header("cross-origin-resource-policy"), None);

        cli.disconnect();
    })
    .await
    .expect("超时：跨源隔离这一步卡住了");
}

#[tokio::test]
async fn a_dead_dev_server_is_a_rendered_502() {
    tokio::time::timeout(LIMIT, async {
        let edge = Edge::start().await;
        // 一个刚被放掉的端口：隧道连着，但那头什么都没有。
        let dead = {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            listener.local_addr().unwrap()
        };
        let cli = FakeCli::connect(&edge, &edge.token("jti-1", |_| {}), dead).await;
        edge.wait_online().await;

        let reply = navigate(edge.addr, "/", Some("pt_gate=1")).await;
        assert_eq!(reply.status, StatusCode::BAD_GATEWAY);
        assert!(reply.text().contains("开发者那边没有响应"));
        // 完整的一页，不是裸状态码。
        assert!(reply.text().starts_with("<!doctype html>"));
        assert!(!reply.text().contains(playtest_common::DEVELOPER_HOST));

        // 开不出去的请求也要把名额还回来。
        use std::sync::atomic::Ordering::Relaxed;
        assert_eq!(
            edge.app
                .tunnels
                .get(SLUG)
                .unwrap()
                .open_streams
                .load(Relaxed),
            0
        );

        cli.disconnect();
    })
    .await
    .expect("超时：上游不存在时该很快回 502");
}

#[tokio::test]
async fn the_handshake_says_why_it_said_no() {
    tokio::time::timeout(LIMIT, async {
        let edge = Edge::start().await;
        let good = edge.token("jti-1", |_| {});

        // 拿浏览器打开握手地址：这不是 WebSocket 握手。
        let plain = send(edge.addr, request(Method::GET, WS_PATH, &[], "")).await;
        assert_eq!(plain.status, StatusCode::BAD_REQUEST);
        assert_eq!(error_body(&plain).code, ErrorCode::Invalid);

        let cases: Vec<(&str, Vec<(String, String)>, StatusCode, ErrorCode)> = vec![
            (
                "协议名不对",
                {
                    let mut h = handshake_headers(&good, 5173);
                    h.retain(|(n, _)| n != "sec-websocket-protocol");
                    h.push(("sec-websocket-protocol".into(), "chat".into()));
                    h
                },
                StatusCode::BAD_REQUEST,
                ErrorCode::Invalid,
            ),
            (
                "没带令牌",
                {
                    let mut h = handshake_headers(&good, 5173);
                    h.retain(|(n, _)| n != "authorization");
                    h
                },
                StatusCode::UNAUTHORIZED,
                ErrorCode::Unauthorized,
            ),
            (
                "令牌是别人签的",
                handshake_headers(&SigningKey::generate().sign(&mint(0)), 5173),
                StatusCode::UNAUTHORIZED,
                ErrorCode::Unauthorized,
            ),
            (
                "令牌已过期",
                handshake_headers(&edge.signing.sign(&mint(-TOKEN_TTL_SECS * 2)), 5173),
                StatusCode::UNAUTHORIZED,
                ErrorCode::TokenExpired,
            ),
            (
                "令牌是给别的作品的",
                handshake_headers(
                    &edge.signing.sign(&{
                        let mut c = mint(0);
                        c.slug = "keen-gecko-9".into();
                        c
                    }),
                    5173,
                ),
                StatusCode::UNAUTHORIZED,
                ErrorCode::Unauthorized,
            ),
            (
                "端口不对",
                {
                    let mut h = handshake_headers(&good, 5173);
                    h.retain(|(n, _)| n != HEADER_LOCAL_PORT);
                    h.push((HEADER_LOCAL_PORT.into(), "0".into()));
                    h
                },
                StatusCode::BAD_REQUEST,
                ErrorCode::Invalid,
            ),
            (
                "端口缺了",
                {
                    let mut h = handshake_headers(&good, 5173);
                    h.retain(|(n, _)| n != HEADER_LOCAL_PORT);
                    h
                },
                StatusCode::BAD_REQUEST,
                ErrorCode::Invalid,
            ),
        ];

        for (what, headers, status, code) in cases {
            let reply = probe_handshake(edge.addr, &headers).await;
            assert_eq!(reply.status, status, "{what}");
            let body = error_body(&reply);
            assert_eq!(body.code, code, "{what}：{}", body.message);
            assert!(!body.message.is_empty(), "{what} 要说人话");
        }

        // 一条都没接上，作品仍然是「不存在」。
        assert!(edge.app.tunnels.get(SLUG).is_none());
    })
    .await
    .expect("超时：握手的拒绝分支卡住了");
}

#[tokio::test]
async fn without_a_verifying_key_the_edge_says_so_instead_of_guessing() {
    tokio::time::timeout(LIMIT, async {
        // 边缘先起、api 后起：公钥这一刻还没写进对象存储。
        let dir = tempfile::tempdir().unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let app = Arc::new(App::new(Config {
            listen: addr,
            data_dir: dir.path().to_path_buf(),
            host_suffix: "localhost".into(),
            public_scheme: "http".into(),
        }));
        let service = router(app.clone());
        tokio::spawn(async move {
            let _ = axum::serve(listener, service).await;
        });

        let signing = SigningKey::generate();
        let headers = handshake_headers(&signing.sign(&mint(0)), 5173);
        let reply = probe_handshake(addr, &headers).await;
        // 「我们这边还没准备好」是 503，不是把 CLI 的令牌判成「签名不对」。
        assert_eq!(reply.status, StatusCode::SERVICE_UNAVAILABLE);
        let body = error_body(&reply);
        assert_eq!(body.code, ErrorCode::Internal);
        assert_eq!(body.message, "边缘还没拿到验签公钥");

        // api 起来了，把公钥写进去——不用重启边缘，下一次握手就认。
        let key_path = dir
            .path()
            .join("store")
            .join(key_files::VERIFYING_KEY_OBJECT);
        std::fs::create_dir_all(key_path.parent().unwrap()).unwrap();
        std::fs::write(&key_path, signing.verifying_key().to_base64()).unwrap();

        let dev = upstream().await;
        let mut request = handshake_request(
            &format!("ws://{addr}{WS_PATH}"),
            &signing.sign(&mint(0)),
            dev.port(),
        )
        .unwrap();
        request
            .headers_mut()
            .insert("host", HeaderValue::from_static(HOST));
        let (_ws, response) = tokio_tungstenite::connect_async(request)
            .await
            .expect("公钥出现之后握手就该成功");
        assert_eq!(response.status(), StatusCode::SWITCHING_PROTOCOLS);
    })
    .await
    .expect("超时：没有公钥这一步卡住了");
}

/// `offset` 加在签发时间上，用来造过期的令牌。
fn mint(offset: i64) -> Claims {
    let now = time::OffsetDateTime::now_utc().unix_timestamp() + offset;
    Claims {
        v: 1,
        slug: SLUG.into(),
        sub: "user-1".into(),
        title: "小球大冒险".into(),
        developer: "某某".into(),
        badge: true,
        gate: GateMode::Once,
        isolated: false,
        max_players: 50,
        iat: now,
        exp: now + TOKEN_TTL_SECS,
        jti: "jti-mint".into(),
    }
}
