//! `playtest mcp`：拿手写的 JSON-RPC 从 stdio 走一遍，看它是不是一个能用的 MCP server。
//!
//! 不引 MCP 客户端库——要验的正是「别人家的客户端接上来能不能用」，用同一个 SDK 两头对拍
//! 就证明不了这件事。这里只按协议写字节：一行一个 JSON。

use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

use base64::Engine as _;
use serde_json::{json, Value};

/// 一句话都不该等这么久。等到了就是卡住了，不是慢。
const PATIENCE: Duration = Duration::from_secs(20);

struct Server {
    child: Child,
    stdin: ChildStdin,
    lines: Receiver<String>,
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Server {
    fn start(home: &Path) -> Self {
        // 用不上：没有令牌就不会有请求。指到 1 号端口，万一发出去了立刻就能看出来。
        Self::start_against(home, "http://127.0.0.1:1")
    }

    fn start_against(home: &Path, api: &str) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_playtest"))
            .args(["mcp"])
            .env("HOME", home)
            .env("PLAYTEST_API", api)
            .env("NO_PROXY", "*")
            .env_remove("APPDATA")
            .env_remove("HTTP_PROXY")
            .env_remove("HTTPS_PROXY")
            .env_remove("http_proxy")
            .env_remove("https_proxy")
            .env_remove("ALL_PROXY")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("跑不起来 playtest mcp");

        let stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();
        let (tx, lines) = mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let Ok(line) = line else { return };
                if tx.send(line).is_err() {
                    return;
                }
            }
        });
        Self {
            child,
            stdin,
            lines,
        }
    }

    fn send(&mut self, message: Value) {
        writeln!(self.stdin, "{message}").expect("写不进 MCP server 的 stdin");
        self.stdin.flush().unwrap();
    }

    /// 收到 `id` 对应的那条回应。路上的通知（没有 `id`）跳过。
    fn reply_to(&self, id: i64) -> Value {
        loop {
            let line = self
                .lines
                .recv_timeout(PATIENCE)
                .unwrap_or_else(|e| panic!("等 id={id} 的回应等不到（{e}）"));
            // stdout 是协议的通道，出现任何一行不是 JSON 的东西都会让真客户端解析失败。
            let message: Value = serde_json::from_str(&line)
                .unwrap_or_else(|e| panic!("stdout 上出现了不是 JSON 的一行（{e}）：{line:?}"));
            if message["id"] == json!(id) {
                return message;
            }
        }
    }

    fn call(&mut self, id: i64, tool: &str, arguments: Value) -> Value {
        self.send(json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "tools/call",
            "params": { "name": tool, "arguments": arguments },
        }));
        self.reply_to(id)
    }
}

/// 握手：`initialize` → `notifications/initialized`。之后才能调工具。
fn handshake(server: &mut Server) -> Value {
    server.send(json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": { "name": "手写的测试客户端", "version": "0" },
        },
    }));
    let reply = server.reply_to(1);
    server.send(json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }));
    reply
}

/// 工具回的那段文字本身是一个 JSON 对象——和 `--json` 一模一样的形状。
fn payload(reply: &Value) -> Value {
    let text = reply["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or_else(|| panic!("工具没回文字内容：{reply}"));
    serde_json::from_str(text).unwrap_or_else(|e| panic!("工具回的不是 JSON（{e}）：{text}"))
}

#[test]
fn a_handwritten_client_can_shake_hands_list_tools_and_call_one() {
    let home = tempfile::tempdir().unwrap();
    let mut server = Server::start(home.path());

    let hello = handshake(&mut server);
    assert_eq!(hello["jsonrpc"], "2.0");
    assert_eq!(hello["result"]["serverInfo"]["name"], "playtest");
    assert_eq!(
        hello["result"]["serverInfo"]["version"],
        env!("CARGO_PKG_VERSION")
    );
    assert!(
        hello["result"]["capabilities"]["tools"].is_object(),
        "得声明自己有工具：{hello}"
    );

    server.send(json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/list" }));
    let listed = server.reply_to(2);
    let tools = listed["result"]["tools"].as_array().expect("没有工具列表");
    let mut names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();
    names.sort_unstable();
    // DESIGN §3.2 说的那五件事：分享端口、上传目录、列出作品、看这一版的结果、拿邀请卡。
    assert_eq!(
        names,
        [
            "playtest_card",
            "playtest_list",
            "playtest_share",
            "playtest_site",
            "playtest_upload"
        ]
    );

    for tool in tools {
        let description = tool["description"].as_str().unwrap_or_default();
        assert!(
            description.chars().any(|c| c.is_ascii_alphabetic()),
            "{} 的说明该有英文给模型读：{description}",
            tool["name"]
        );
        assert!(
            description
                .chars()
                .any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c)),
            "{} 的说明该有中文给人读：{description}",
            tool["name"]
        );
    }

    // 上传那个工具得说清楚要给它什么。
    let upload = tools
        .iter()
        .find(|t| t["name"] == "playtest_upload")
        .unwrap();
    assert_eq!(upload["inputSchema"]["required"], json!(["dir"]));
    for optional in ["name", "note", "isolated", "seats", "community"] {
        assert!(
            upload["inputSchema"]["properties"][optional].is_object(),
            "上传该收 {optional}：{upload}"
        );
    }

    // 拿卡只要说哪个作品。
    let card = tools.iter().find(|t| t["name"] == "playtest_card").unwrap();
    assert_eq!(card["inputSchema"]["required"], json!(["site"]));

    // 这台机器上还没发过东西：一个空列表，不是一句话。
    let listed = server.call(3, "playtest_list", json!({}));
    assert_ne!(listed["result"]["isError"], json!(true), "{listed}");
    let body = payload(&listed);
    assert_eq!(body["ok"], true, "{body}");
    assert_eq!(body["action"], "list");
    assert_eq!(body["sites"], json!([]));
    assert!(body["elapsed_ms"].is_u64(), "{body}");
}

/// 端口上没东西在监听时，说的是这件事本身——而不是「这条路还没上线」。
///
/// 这条测试以前把「隧道还没做好」钉死了，而隧道其实早就能用：MCP 那一侧对着助手
/// 低报了自己的能力，方向和「不虚报」是反的，一样要修（AGENTS 第 4 条）。
#[test]
fn sharing_a_port_with_nothing_on_it_says_exactly_that() {
    let home = tempfile::tempdir().unwrap();
    let mut server = Server::start(home.path());
    handshake(&mut server);

    // 一个几乎不可能有人在听的端口。
    let reply = server.call(2, "playtest_share", json!({ "port": 1 }));
    assert_eq!(
        reply["result"]["isError"],
        json!(true),
        "失败得让对话看见：{reply}"
    );
    let body = payload(&reply);
    assert_eq!(body["ok"], false, "{body}");
    let message = body["message"].as_str().unwrap();
    assert!(
        message.contains("监听") || message.contains("端口"),
        "要说清是端口的事：{message}"
    );
    assert!(
        !message.contains("还没上线"),
        "隧道早就能用了，不许低报：{message}"
    );
}

/// 工具表就是命令表：助手看到的能力和 `playtest --help` 说的是同一套（REWRITE §3.2）。
#[test]
fn the_tool_list_matches_what_the_cli_can_do() {
    let home = tempfile::tempdir().unwrap();
    let mut server = Server::start(home.path());
    handshake(&mut server);

    server.send(json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {} }));
    let reply = server.reply_to(2);
    let tools = reply["result"]["tools"]
        .as_array()
        .unwrap_or_else(|| panic!("要有工具表：{reply}"));
    let names: Vec<&str> = tools
        .iter()
        .map(|t| t["name"].as_str().unwrap_or_default())
        .collect();
    for want in [
        "playtest_upload",
        "playtest_share",
        "playtest_list",
        "playtest_site",
        "playtest_card",
    ] {
        assert!(names.contains(&want), "少了 {want}：{names:?}");
    }
    assert!(
        !names.contains(&"playtest_share_port"),
        "改名之后旧名字不该还在：{names:?}"
    );

    // 描述里不许再有「还没上线」这种话——它是给助手读的，说错了它就不会用。
    let text = serde_json::to_string(tools).unwrap();
    assert!(!text.contains("NOT AVAILABLE"), "{text}");
    assert!(!text.contains("还没上线"), "{text}");
}

#[test]
fn a_directory_that_is_not_there_comes_back_as_a_readable_failure() {
    let home = tempfile::tempdir().unwrap();
    let mut server = Server::start(home.path());
    handshake(&mut server);

    let reply = server.call(2, "playtest_upload", json!({ "dir": "./这个目录不存在" }));
    assert_eq!(reply["result"]["isError"], json!(true), "{reply}");
    let body = payload(&reply);
    assert_eq!(body["code"], "bad_input", "{body}");
    assert!(
        body["message"].as_str().unwrap().contains("找不到"),
        "{body}"
    );
}

// ---------------------------------------------------------------- 邀请卡贴回对话

/// 一张「PNG」。只有头八个字节是真的——CLI 认的就是这八个字节加响应头里的类型。
const CARD_BYTES: &[u8] = b"\x89PNG\r\n\x1a\nnot-really-a-png";

/// 一个只回两件事的假服务器：这个作品长什么样、它的邀请卡在哪。它同时扮控制面和边缘。
fn start_fake_site() -> String {
    use axum::response::IntoResponse;
    use axum::routing::get;

    let (tx, rx) = std::sync::mpsc::channel::<std::net::SocketAddr>();
    std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async move {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let addr = listener.local_addr().unwrap();
            tx.send(addr).unwrap();
            let app = axum::Router::new()
                .route(
                    playtest_common::api::routes::SITE,
                    get(
                        move |axum::extract::Path(slug): axum::extract::Path<String>| async move {
                            axum::Json(json!({
                                "slug": slug,
                                "url": format!("http://{addr}"),
                                "title": "小球试玩",
                                "current_version": 7,
                                "created_at": "2026-09-09T00:00:00Z",
                                "listing": { "followers": 12, "has_cover": true },
                            }))
                        },
                    ),
                )
                .route(
                    playtest_common::CARD_PATH,
                    get(|| async {
                        (
                            [(axum::http::header::CONTENT_TYPE, "image/png")],
                            CARD_BYTES,
                        )
                            .into_response()
                    }),
                );
            axum::serve(listener, app).await.unwrap();
        });
    });
    format!("http://{}", rx.recv().expect("假服务器没起来"))
}

/// 把一个已经存好的令牌写进配置里，省掉在这个文件里再演一遍上传。
fn remember_token(home: &Path, api: &str) {
    let dir = home.join(".config").join("playtest");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("config.json"),
        json!({ "api": api, "token": "tok-1", "login": "octo" }).to_string(),
    )
    .unwrap();
}

#[test]
fn the_card_comes_back_as_an_image_the_assistant_can_hand_over() {
    let home = tempfile::tempdir().unwrap();
    let api = start_fake_site();
    remember_token(home.path(), &api);
    let mut server = Server::start_against(home.path(), &api);
    handshake(&mut server);

    let reply = server.call(2, "playtest_card", json!({ "site": "brisk-otter-41" }));
    assert_ne!(reply["result"]["isError"], json!(true), "{reply}");

    // 第一块永远是那个 JSON——读结果的程序按顺序取第一块。
    let body = payload(&reply);
    assert_eq!(body["ok"], true, "{body}");
    assert_eq!(body["action"], "card");
    assert_eq!(body["slug"], "brisk-otter-41");
    assert_eq!(body["card_url"], format!("{api}/_playtest/card.png"));

    // 第二块是那张图，按 MCP 的形状：type / data（base64）/ mimeType。
    let image = &reply["result"]["content"][1];
    assert_eq!(image["type"], "image", "{reply}");
    assert_eq!(image["mimeType"], "image/png", "{reply}");
    let data = image["data"].as_str().expect("图没有 data");
    assert!(!data.contains('\n'), "base64 不该带换行：{data}");
    assert_eq!(
        base64::engine::general_purpose::STANDARD
            .decode(data)
            .unwrap(),
        CARD_BYTES,
        "贴回去的得是服务器上那张卡"
    );
}

#[test]
fn looking_at_this_version_says_how_many_people_follow_it_and_where_to_look() {
    let home = tempfile::tempdir().unwrap();
    let api = start_fake_site();
    remember_token(home.path(), &api);
    let mut server = Server::start_against(home.path(), &api);
    handshake(&mut server);

    let reply = server.call(2, "playtest_site", json!({ "slug": "brisk-otter-41" }));
    assert_ne!(reply["result"]["isError"], json!(true), "{reply}");
    let body = payload(&reply);
    assert_eq!(body["site"]["followers"], 12, "{body}");
    assert!(
        body["console_url"]
            .as_str()
            .unwrap()
            .ends_with("/console/#/s/brisk-otter-41"),
        "{body}"
    );
}

#[test]
fn setup_prints_a_config_you_can_paste() {
    let home = tempfile::tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_playtest"))
        .args(["mcp", "--setup"])
        .env("HOME", home.path())
        .env_remove("APPDATA")
        .output()
        .expect("跑不起来 playtest mcp --setup");
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let config: Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("--setup 该只打印一段 JSON（{e}）：{stdout}"));
    let entry = &config["mcpServers"]["playtest"];
    assert_eq!(entry["args"], json!(["mcp"]));
    let command = entry["command"].as_str().unwrap();
    assert!(
        Path::new(command).is_absolute(),
        "编辑器起 server 时的 PATH 和终端里不一样，得写绝对路径：{command}"
    );
    assert!(Path::new(command).exists(), "{command} 不存在");

    // 怎么用写在 stderr 上，粘的时候不会跟着进去。
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains(".cursor/mcp.json"), "{stderr}");
}
