//! 对着一个假控制面、一个假边缘、一个假 dev server 跑真的 `playtest <端口>`。
//!
//! 三个假东西各管一段：
//!
//! - **假控制面**（axum）：发匿名令牌、建作品、签隧道授权。路由用
//!   `playtest_common::api::routes` 的常量注册，写错了这里就挂。
//! - **假边缘**：一个普通的 TCP 监听 + `tokio_tungstenite::accept_hdr_async`。
//!   不用 axum 的 `ws`——axum 把升级后的连接包成自己的 `WebSocket`，拿不回
//!   `WsByteStream` 要的 `WebSocketStream`；而且握手要按状态码拒绝（409），
//!   tungstenite 的回调正好能干这件事。它扮演 `Role::Opener`：开一条流、按 HTTP/1.1
//!   写一个请求进去，看开发者的进程回什么。
//! - **假 dev server**：手写的极小 HTTP/1.1，`GET /` 回一页带 `/@vite/client` 的 HTML
//!   （顺便验 CLI 认不认得出 Vite），别的路径回一句回显。
//!
//! 每条测试都带超时，卡住就是失败，不会把 CI 挂在那儿。

use std::collections::HashMap;
use std::io;
use std::net::SocketAddr;
use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum::extract::{Path as UrlPath, State};
use axum::routing::post;
use axum::{Json, Router};
use playtest_common::api::{routes, ErrorBody, ErrorCode};
use playtest_common::tunnel::io::{Mux, Role, WsByteStream};
use playtest_common::tunnel::{TunnelGrant, TunnelRequest, HEADER_LOCAL_PORT, WS_PATH, WS_PROTOCOL};
use serde_json::{json, Value};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::accept_hdr_async;
use tokio_tungstenite::tungstenite::handshake::server::{
    ErrorResponse, Request as HandshakeRequest, Response as HandshakeResponse,
};
use tokio_tungstenite::tungstenite::http::header::{
    AUTHORIZATION, CONTENT_TYPE, SEC_WEBSOCKET_PROTOCOL,
};
use tokio_tungstenite::tungstenite::http::{HeaderValue, StatusCode};

const SLUG: &str = "brisk-otter-41";
const PLAYER_URL: &str = "http://brisk-otter-41.localhost:8443";
/// 假边缘写进流里的那个请求，玩家的浏览器发的就长这样。
const PLAYER_REQUEST: &str =
    "GET /echo HTTP/1.1\r\nHost: localhost:0\r\nX-Forwarded-Host: brisk-otter-41.localhost\r\n\r\n";
/// 一条测试最多跑这么久。
const LIMIT: Duration = Duration::from_secs(30);

// ---------------------------------------------------------------- 假边缘

#[derive(Clone, Copy)]
struct EdgeScript {
    /// 第二次握手一律回 409 tunnel_replaced（模拟被别的进程挤掉）。
    replace_on_second: bool,
    /// 记完回显就把这条 WebSocket 关掉，逼 CLI 重连。
    close_after_probe: bool,
}

#[derive(Default)]
struct EdgeLog {
    /// 每次握手 CLI 都带了什么。
    handshakes: Vec<HashMap<String, String>>,
    /// 开发者的进程回给假边缘的那一段。
    echo: Option<String>,
}

struct Edge {
    script: EdgeScript,
    log: Mutex<EdgeLog>,
}

impl Edge {
    fn handshakes(&self) -> usize {
        self.log.lock().unwrap().handshakes.len()
    }

    fn echo(&self) -> Option<String> {
        self.log.lock().unwrap().echo.clone()
    }
}

fn refuse_handshake() -> ErrorResponse {
    let body = serde_json::to_string(&ErrorBody {
        code: ErrorCode::TunnelReplaced,
        message: "这个作品已经有另一条隧道了".into(),
    })
    .unwrap();
    let mut response = ErrorResponse::new(Some(body));
    *response.status_mut() = StatusCode::CONFLICT;
    response
        .headers_mut()
        .insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    response
}

/// 边缘这一侧的握手：记下 CLI 送来的头，决定接还是拒。
fn shake(edge: &Arc<Edge>, request: &HandshakeRequest, mut response: HandshakeResponse) -> Result<HandshakeResponse, ErrorResponse> {
    let mut seen = HashMap::new();
    seen.insert("path".to_string(), request.uri().path().to_string());
    for (name, value) in request.headers() {
        seen.insert(
            name.as_str().to_string(),
            value.to_str().unwrap_or("<非文本>").to_string(),
        );
    }
    let nth = {
        let mut log = edge.log.lock().unwrap();
        log.handshakes.push(seen);
        log.handshakes.len()
    };
    if edge.script.replace_on_second && nth >= 2 {
        return Err(refuse_handshake());
    }
    // 子协议必须原样回：tungstenite 的客户端要求请求里带了就得回，
    // 不回它会把这次握手判成协议错。
    response
        .headers_mut()
        .insert(SEC_WEBSOCKET_PROTOCOL, HeaderValue::from_static(WS_PROTOCOL));
    Ok(response)
}

/// 一条隧道连上之后，假边缘做的事：开一条流、写一个玩家请求、把开发者的回应记下来。
async fn probe_through(edge: Arc<Edge>, socket: TcpStream) {
    let held = {
        let edge = Arc::clone(&edge);
        move |request: &HandshakeRequest, response: HandshakeResponse| shake(&edge, request, response)
    };
    let Ok(ws) = accept_hdr_async(socket, held).await else {
        // 拒了的那次握手会走到这里，正常。
        return;
    };

    let mux = Mux::spawn(WsByteStream::new(ws), Role::Opener);
    let Ok(mut stream) = mux.open().await else {
        return;
    };
    if stream.write_all(PLAYER_REQUEST.as_bytes()).await.is_err() {
        return;
    }
    let mut back = Vec::new();
    let _ = stream.read_to_end(&mut back).await;
    edge.log.lock().unwrap().echo = Some(String::from_utf8_lossy(&back).into_owned());
    // 玩家这次连接到此为止：关掉写的那一半（浏览器关掉页面就是这个样子），
    // CLI 那边的双向拷贝才会收场并把字节数记上。
    let _ = stream.shutdown().await;

    if edge.script.close_after_probe {
        // 把 Mux 丢掉就等于关掉这条隧道，CLI 那边会读到 EOF 然后重连。
        return;
    }
    // 不关就一直连着，等 CLI 自己走（Ctrl-C 的那条测试要的就是这个）。
    let _held_open = (mux, stream);
    std::future::pending::<()>().await;
}

fn start_edge(script: EdgeScript) -> (u16, Arc<Edge>) {
    let edge = Arc::new(Edge {
        script,
        log: Mutex::new(EdgeLog::default()),
    });
    let served = Arc::clone(&edge);
    let port = spawn_server(move |listener| async move {
        loop {
            let Ok((socket, _)) = listener.accept().await else {
                return;
            };
            tokio::spawn(probe_through(Arc::clone(&served), socket));
        }
    });
    (port, edge)
}

// ---------------------------------------------------------------- 假 dev server

/// 开发者自己跑着的那个进程。手写 HTTP/1.1：这里要的就是「一个普通的 TCP 服务」。
fn start_dev_server() -> u16 {
    spawn_server(|listener| async move {
        loop {
            let Ok((socket, _)) = listener.accept().await else {
                return;
            };
            tokio::spawn(async move {
                let _ = answer_one(socket).await;
            });
        }
    })
}

async fn answer_one(mut socket: TcpStream) -> io::Result<()> {
    let mut head = Vec::new();
    let mut byte = [0u8; 1];
    while !head.ends_with(b"\r\n\r\n") {
        if socket.read(&mut byte).await? == 0 {
            return Ok(());
        }
        head.push(byte[0]);
    }
    let request_line = String::from_utf8_lossy(&head)
        .lines()
        .next()
        .unwrap_or_default()
        .to_string();
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or_default().to_string();
    let target = parts.next().unwrap_or_default().to_string();

    let (content_type, body) = if target == "/" {
        (
            "text/html; charset=utf-8",
            "<html><head><script type=\"module\" src=\"/@vite/client\"></script></head>\
             <body>开发中</body></html>"
                .to_string(),
        )
    } else {
        ("text/plain; charset=utf-8", format!("dev server 收到：{request_line}"))
    };

    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    socket.write_all(response.as_bytes()).await?;
    // HEAD 只要头，写了体就是协议错。
    if method != "HEAD" {
        socket.write_all(body.as_bytes()).await?;
    }
    socket.flush().await?;
    socket.shutdown().await
}

// ---------------------------------------------------------------- 假控制面

#[derive(Default)]
struct Api {
    /// 每次要授权带来的东西，用来验 --name / --gate 这些参数有没有传下去。
    granted: Mutex<Vec<TunnelRequest>>,
}

async fn anon_sessions() -> Json<Value> {
    Json(json!({ "token": "tok-1", "expires_at": "2099-01-01T00:00:00Z" }))
}

async fn create_site() -> Json<Value> {
    Json(json!({
        "slug": SLUG,
        "url": PLAYER_URL,
        "title": "隧道测试",
        "created_at": "2026-09-07T00:00:00Z",
        "expires_at": "2026-09-08T03:30:00Z",
    }))
}

async fn grant_tunnel(
    State((api, edge_port)): State<(Arc<Api>, u16)>,
    UrlPath(slug): UrlPath<String>,
    Json(request): Json<TunnelRequest>,
) -> Json<TunnelGrant> {
    api.granted.lock().unwrap().push(request);
    Json(TunnelGrant {
        slug,
        url: PLAYER_URL.to_string(),
        connect_url: format!("ws://127.0.0.1:{edge_port}{WS_PATH}"),
        token: "pt1.test.token".into(),
        expires_at: "2099-01-01T00:00:00Z".into(),
        site_expires_at: Some("2026-09-08T03:30:00Z".into()),
    })
}

fn start_api(edge_port: u16) -> (String, Arc<Api>) {
    let api = Arc::new(Api::default());
    let state = (Arc::clone(&api), edge_port);
    let port = spawn_server(move |listener| async move {
        let app = Router::new()
            .route(routes::ANON_SESSIONS, post(anon_sessions))
            .route(routes::SITES, post(create_site))
            .route(routes::SITE_TUNNEL, post(grant_tunnel))
            .with_state(state);
        let _ = axum::serve(listener, app).await;
    });
    (format!("http://127.0.0.1:{port}"), api)
}

// ---------------------------------------------------------------- 跑起来的家伙什

/// 在自己的线程里起一个监听 `127.0.0.1:0` 的服务，返回它的端口。线程随进程一起结束。
fn spawn_server<F, Fut>(serve: F) -> u16
where
    F: FnOnce(TcpListener) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = ()>,
{
    let (tx, rx) = std::sync::mpsc::channel::<SocketAddr>();
    std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async move {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            tx.send(listener.local_addr().unwrap()).unwrap();
            serve(listener).await;
        });
    });
    rx.recv().expect("服务没起来").port()
}

fn cli(home: &Path, work: &Path, api: &str, args: &[&str]) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_playtest"));
    command
        .args(args)
        .current_dir(work)
        .env("HOME", home)
        .env("PLAYTEST_API", api)
        // 全走 127.0.0.1，代理设置只会碍事。
        .env("NO_PROXY", "*")
        .env_remove("APPDATA")
        .env_remove("HTTP_PROXY")
        .env_remove("HTTPS_PROXY")
        .env_remove("http_proxy")
        .env_remove("https_proxy")
        .env_remove("ALL_PROXY");
    command
}

fn stdout_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// `--json` 模式下 stdout 是一行一个 JSON。
fn events(output: &Output) -> Vec<Value> {
    stdout_of(output)
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            serde_json::from_str(line)
                .unwrap_or_else(|e| panic!("stdout 上这行不是 JSON（{e}）：{line}"))
        })
        .collect()
}

fn first_named<'a>(events: &'a [Value], name: &str) -> Option<&'a Value> {
    events.iter().find(|e| e["event"] == name)
}

/// 等一件事发生，等不到就让测试失败而不是挂住。
fn wait_until(what: &str, mut done: impl FnMut() -> bool) {
    let deadline = Instant::now() + LIMIT;
    while Instant::now() < deadline {
        if done() {
            return;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    panic!("等了 {} 秒还没等到：{what}", LIMIT.as_secs());
}

// ---------------------------------------------------------------- 测试

/// 一次完整的来回：连上 → 玩家的请求真的落到 dev server 上 → 边缘断开 → 自己重连 →
/// 第二次握手被 409 挤掉 → 说清楚然后退出。
#[test]
fn a_players_request_reaches_the_dev_server_and_a_lost_link_comes_back() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    let dev_port = start_dev_server();
    let (edge_port, edge) = start_edge(EdgeScript {
        replace_on_second: true,
        close_after_probe: true,
    });
    let (api, control) = start_api(edge_port);

    let output = cli(
        home.path(),
        work.path(),
        &api,
        &[&dev_port.to_string(), "--json", "--no-qr", "--name", "小球"],
    )
    .output()
    .expect("跑不起来 playtest");

    // 被挤掉是「别再连了」，退出码 1。
    assert_eq!(output.status.code(), Some(1), "{}", stderr_of(&output));

    let events = events(&output);
    let online = first_named(&events, "online").unwrap_or_else(|| panic!("没有 online：{events:?}"));
    assert_eq!(online["url"], PLAYER_URL);
    assert_eq!(online["slug"], SLUG);
    assert_eq!(online["expires_at"], "2026-09-08T03:30:00Z");
    assert_eq!(online["attempt"], 0);
    assert!(online["elapsed_ms"].is_u64(), "「几秒」要是个数字：{online}");
    assert!(
        online["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["message"].as_str().unwrap_or_default().contains("Vite")),
        "首页里有 /@vite/client，该认出来：{online}"
    );

    // 玩家的请求走完了整条路：边缘 → CLI → dev server → 原路回去。
    let echoed = edge.echo().expect("假边缘没拿到回显");
    assert!(echoed.starts_with("HTTP/1.1 200 OK"), "{echoed}");
    assert!(echoed.contains("dev server 收到：GET /echo"), "{echoed}");

    // 玩家来了又走，两次都说了一声。
    let counts: Vec<u64> = events
        .iter()
        .filter(|e| e["event"] == "players")
        .map(|e| e["count"].as_u64().unwrap())
        .collect();
    assert_eq!(counts, [1, 0], "玩家连接数变化该逐次报出来：{events:?}");

    // 断了之后自己回来了。
    let again =
        first_named(&events, "reconnecting").unwrap_or_else(|| panic!("没有 reconnecting：{events:?}"));
    assert_eq!(again["attempt"], 1);
    let waited = again["wait_ms"].as_u64().expect("要有等待时长");
    assert!((500..=1000).contains(&waited), "第一次退避该在 0.5–1 秒：{waited}");
    assert!(
        again["reason"].as_str().unwrap().contains("断开"),
        "{again}"
    );

    // 最后一行是那个失败对象，说清楚为什么不再重连。
    let last = events.last().expect("总得有输出");
    assert_eq!(last["ok"], false, "{last}");
    assert!(
        last["message"].as_str().unwrap().contains("接管"),
        "被挤掉要说人话：{last}"
    );

    // 两次握手带的东西都对。
    assert_eq!(edge.handshakes(), 2);
    for shook in edge.log.lock().unwrap().handshakes.iter() {
        assert_eq!(shook["path"], WS_PATH);
        assert_eq!(shook[AUTHORIZATION.as_str()], "Bearer pt1.test.token");
        assert_eq!(shook[HEADER_LOCAL_PORT], dev_port.to_string());
        assert_eq!(shook[SEC_WEBSOCKET_PROTOCOL.as_str()], WS_PROTOCOL);
    }

    // 作品名传到控制面了，门禁页上写的就是它。
    let asked = control.granted.lock().unwrap();
    assert_eq!(asked[0].title.as_deref(), Some("小球"));
    assert_eq!(
        asked.len(),
        1,
        "普通断线不换令牌：令牌还有一小时，重连用的还是同一张，链接才不会变"
    );
}

/// 人类模式：stdout 上只有链接，别的全在 stderr。
#[test]
fn the_human_output_puts_the_link_on_stdout_and_everything_else_on_stderr() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    let dev_port = start_dev_server();
    let (edge_port, _edge) = start_edge(EdgeScript {
        replace_on_second: true,
        close_after_probe: true,
    });
    let (api, _control) = start_api(edge_port);

    let output = cli(
        home.path(),
        work.path(),
        &api,
        &[&dev_port.to_string(), "--no-qr"],
    )
    .output()
    .expect("跑不起来 playtest");

    assert_eq!(output.status.code(), Some(1), "{}", stderr_of(&output));
    assert_eq!(
        stdout_of(&output),
        format!("{PLAYER_URL}\n"),
        "`playtest 5173 | pbcopy` 拿到的必须就是链接"
    );

    let stderr = stderr_of(&output);
    for expected in [
        "已连上",
        "这是匿名链接",
        "Vite",
        "server.hmr.clientPort = 443",
        "按 Ctrl-C 结束",
        "本次 ",
        "玩家连接：1",
        "和服务器断开了",
        "接管",
    ] {
        assert!(stderr.contains(expected), "stderr 里少了「{expected}」：{stderr}");
    }
}

/// 本地端口上没人在听：一句话说清楚，不去麻烦控制面。
#[test]
fn a_port_with_nothing_on_it_stops_before_asking_for_a_link() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    let (edge_port, _edge) = start_edge(EdgeScript {
        replace_on_second: false,
        close_after_probe: true,
    });
    let (api, control) = start_api(edge_port);
    // 借一个端口再立刻还回去，这个号上就没人在听了。
    let free_port = spawn_server(|listener| async move { drop(listener) });

    let output = cli(
        home.path(),
        work.path(),
        &api,
        &[&free_port.to_string(), "--json"],
    )
    .output()
    .expect("跑不起来 playtest");

    assert_eq!(output.status.code(), Some(1), "{}", stderr_of(&output));
    let events = events(&output);
    let only = events.first().expect("该有一个失败对象");
    assert_eq!(only["ok"], false);
    let message = only["message"].as_str().unwrap();
    assert!(message.contains("没有东西在监听"), "{message}");
    assert!(message.contains(&free_port.to_string()), "{message}");
    assert!(control.granted.lock().unwrap().is_empty(), "都没端口可接，别去要链接");
}

/// Ctrl-C：关掉隧道，报一句「已停止」，退出码 0。
#[cfg(unix)]
#[test]
fn ctrl_c_stops_the_tunnel_cleanly() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    let dev_port = start_dev_server();
    let (edge_port, edge) = start_edge(EdgeScript {
        replace_on_second: false,
        close_after_probe: false,
    });
    let (api, _control) = start_api(edge_port);

    let child = cli(
        home.path(),
        work.path(),
        &api,
        &[&dev_port.to_string(), "--json", "--no-qr"],
    )
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()
    .expect("跑不起来 playtest");

    // 等到玩家那次请求真的走完，说明隧道已经在干活了。
    wait_until("假边缘拿到回显", || edge.echo().is_some());

    let killed = Command::new("kill")
        .args(["-INT", &child.id().to_string()])
        .status()
        .expect("发不出 SIGINT");
    assert!(killed.success());

    let output = child.wait_with_output().expect("等不到 playtest 退出");
    assert_eq!(output.status.code(), Some(0), "{}", stderr_of(&output));

    let events = events(&output);
    let stopped = first_named(&events, "stopped").unwrap_or_else(|| panic!("没有 stopped：{events:?}"));
    assert_eq!(stopped["connections"], 1, "这次接过一个玩家连接");
    assert!(stopped["bytes"].as_u64().unwrap() > 0, "转发的字节要算进去");
}
