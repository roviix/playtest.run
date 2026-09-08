//! `playtest mcp`：拿手写的 JSON-RPC 从 stdio 走一遍，看它是不是一个能用的 MCP server。
//!
//! 不引 MCP 客户端库——要验的正是「别人家的客户端接上来能不能用」，用同一个 SDK 两头对拍
//! 就证明不了这件事。这里只按协议写字节：一行一个 JSON。

use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

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
        let mut child = Command::new(env!("CARGO_BIN_EXE_playtest"))
            .args(["mcp"])
            .env("HOME", home)
            // 用不上：没有令牌就不会有请求。指到 1 号端口，万一发出去了立刻就能看出来。
            .env("PLAYTEST_API", "http://127.0.0.1:1")
            .env_remove("APPDATA")
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
    assert_eq!(
        names,
        [
            "playtest_list",
            "playtest_share_port",
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
            description.chars().any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c)),
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
    assert!(upload["inputSchema"]["properties"]["name"].is_object());
    assert!(upload["inputSchema"]["properties"]["note"].is_object());
    assert!(upload["inputSchema"]["properties"]["isolated"].is_object());

    // 这台机器上还没发过东西：一个空列表，不是一句话。
    let listed = server.call(3, "playtest_list", json!({}));
    assert_ne!(listed["result"]["isError"], json!(true), "{listed}");
    let body = payload(&listed);
    assert_eq!(body["ok"], true, "{body}");
    assert_eq!(body["action"], "list");
    assert_eq!(body["sites"], json!([]));
    assert!(body["elapsed_ms"].is_u64(), "{body}");
}

#[test]
fn the_tunnel_tool_says_it_is_not_built_yet_instead_of_pretending() {
    let home = tempfile::tempdir().unwrap();
    let mut server = Server::start(home.path());
    handshake(&mut server);

    let reply = server.call(2, "playtest_share_port", json!({ "port": 5173 }));
    assert_eq!(
        reply["result"]["isError"],
        json!(true),
        "失败得让对话看见：{reply}"
    );
    let body = payload(&reply);
    assert_eq!(body["ok"], false, "{body}");
    assert_eq!(body["code"], "not_implemented");
    let message = body["message"].as_str().unwrap();
    assert!(message.contains("还没上线"), "{message}");
    assert!(message.contains("5173"), "要说清是哪个端口：{message}");
    assert!(
        message.contains("playtest_upload"),
        "得告诉它现在能做什么：{message}"
    );
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
