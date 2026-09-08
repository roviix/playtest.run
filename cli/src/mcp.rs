//! `playtest mcp`：把这条命令变成 Cursor / Claude Code 里的一个工具。
//!
//! 用 AI 写小东西的人未必开终端，但一定在跟模型对话（DESIGN §3.2）。所以这里只是一个壳：
//! 工具做的事和命令行完全一样，回给对话的 JSON 和 `--json` 是同一个形状。
//!
//! **stdout 是协议的**。MCP 的 stdio 传输把 stdout 当 JSON-RPC 通道，往里写一个字节的日志
//! 都会让对面解析失败——所以这个模块里所有说给人听的话都走 stderr，工具的结果走返回值。
//!
//! 工具的说明写英文加中文各一句：模型读英文，人在配置界面里读中文。

use std::time::Instant;

use anyhow::{Context, Result};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ContentBlock};
use rmcp::transport::stdio;
use rmcp::{
    schemars, tool, tool_handler, tool_router, ErrorData as McpError, ServerHandler, ServiceExt,
};
use serde::Deserialize;

use crate::args::UploadArgs;
use crate::output;
use crate::upload;

/// 服务本身。`api` 是 `playtest mcp --api …` 传进来的控制面地址，转给每一次工具调用。
#[derive(Debug, Clone)]
pub struct Playtest {
    api: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct UploadParams {
    /// Directory of the built game, e.g. ./dist or ./build/web. 已经构建好的目录，例如 ./dist。
    pub dir: String,
    /// Title players see before they start. 玩家开始前看到的作品名，默认用目录名。
    #[serde(default)]
    pub name: Option<String>,
    /// One line about what changed in this version. 这一版改了什么，一句话。
    #[serde(default)]
    pub note: Option<String>,
    /// Serve cross-origin isolated; Godot 4 threaded exports need it. Godot 4 的线程导出需要这个。
    #[serde(default)]
    pub isolated: Option<bool>,
    /// One line describing what the work is; shown on the gate page, share card and plaza card (≤140 chars).
    /// 一句话介绍这个作品是什么，门禁页、分享卡片、广场卡片上都用（最多 140 字）。
    #[serde(default)]
    pub summary: Option<String>,
    /// Path to a cover image (PNG/JPEG/WebP, ≤2 MB). 封面图的路径（PNG / JPEG / WebP，2 MB 以内）。
    #[serde(default)]
    pub cover: Option<String>,
    /// Also list the work on the public plaza (playtest.run home) so passers-by can play it. Default false.
    /// 上传后放到广场（playtest.run 首页）上，路过的人点开就能玩。默认不放。
    #[serde(default)]
    pub public: Option<bool>,
    /// Mark it "looking for testers" on the plaza and tell them what to look at (≤140 chars); implies public.
    /// 在广场上标「正在找人测」并告诉来的人重点看什么（最多 140 字）；蕴含 public。
    #[serde(default)]
    pub seek: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SiteParams {
    /// The slug of the work, e.g. brisk-otter-41. 作品的 slug，例如 brisk-otter-41。
    pub slug: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct PortParams {
    /// Local port the dev server listens on, e.g. 5173. 本地开发服务器的端口，例如 5173。
    pub port: u16,
}

#[tool_router]
impl Playtest {
    pub fn new(api: Option<String>) -> Self {
        Self { api }
    }

    #[tool(
        description = "Publish a built web game directory and get a shareable link and QR code. \
                       把构建好的目录发出去，拿到一条可以直接分享的链接和一张二维码。"
    )]
    async fn playtest_upload(
        &self,
        Parameters(params): Parameters<UploadParams>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        let dir = params.dir.clone();
        let args = UploadArgs {
            target: Some(params.dir),
            name: params.name,
            note: params.note,
            summary: params.summary,
            cover: params.cover.map(std::path::PathBuf::from),
            public: params.public.unwrap_or(false),
            seek: params.seek,
            isolated: params.isolated.unwrap_or(false),
            api: self.api.clone(),
            ..UploadArgs::default()
        };
        match upload::run(&args, &dir).await {
            Ok(mut report) => {
                // 进程从早上就开着，这次调用花了多久才是要报的数。
                report.elapsed_ms = output::ms_since(started);
                Ok(answer(&report))
            }
            Err(e) => Ok(refuse(&e, started)),
        }
    }

    #[tool(
        description = "List the works published from this machine, with their links and versions. \
                       列出这台机器上发过的作品、它们的链接和版本。"
    )]
    async fn playtest_list(&self) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        match output::fetch_sites(self.api.as_deref()).await {
            Ok(sites) => Ok(answer(&serde_json::json!({
                "ok": true,
                "action": "list",
                "sites": sites,
                "elapsed_ms": output::ms_since(started),
            }))),
            Err(e) => Ok(refuse(&e, started)),
        }
    }

    #[tool(
        description = "Look up one work by its slug: link, title, current version, expiry. \
                       按 slug 查一个作品：链接、名字、当前版本、什么时候失效。"
    )]
    async fn playtest_site(
        &self,
        Parameters(params): Parameters<SiteParams>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        match output::fetch_site(&params.slug, self.api.as_deref()).await {
            Ok(site) => Ok(answer(&serde_json::json!({
                "ok": true,
                "action": "site",
                "site": site,
                "elapsed_ms": output::ms_since(started),
            }))),
            Err(e) => Ok(refuse(&e, started)),
        }
    }

    #[tool(
        description = "Share a locally running dev server through a tunnel. NOT AVAILABLE YET — \
                       this call always fails; upload a built directory instead. \
                       把本地开着的开发服务器接出去。这条路还没上线，调了一定失败，先用上传。"
    )]
    async fn playtest_share_port(
        &self,
        Parameters(params): Parameters<PortParams>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        let port = params.port;
        Ok(refuse(
            &output::not_implemented(format!(
                "隧道路径还没上线，接不了本地端口 {port}。\
                 现在能做的是把构建好的目录发出去：先跑一次构建，再调 playtest_upload。"
            )),
            started,
        ))
    }
}

// `version` 只收字面量，所以这里抄了一份 `Cargo.toml` 的版本号。抄岔了，tests/mcp.rs 里那条
// 握手测试会挂——它比对的是握手时真发出去的那个值。
#[tool_handler(
    name = "playtest",
    version = "0.1.0",
    instructions = "playtest puts a playable build in front of specific people. Call \
                    playtest_upload with the directory your build step produced; give the user the \
                    returned url, and paste qr_text verbatim in a code block when they will open it \
                    on a phone. Every tool answers with one JSON object; ok:false means it did not \
                    happen — read code and hint before retrying."
)]
impl ServerHandler for Playtest {}

pub async fn run(setup: bool, api: Option<String>) -> Result<()> {
    if setup {
        print_setup(api.as_deref());
        return Ok(());
    }
    eprintln!("playtest 正在以 MCP server 的方式运行，等编辑器接进来（stdout 只走协议，说明都在这条流上）。");
    let service = Playtest::new(api)
        .serve(stdio())
        .await
        .context("MCP 服务起不来")?;
    let reason = service.waiting().await.context("MCP 连接断了")?;
    eprintln!("MCP 连接结束：{reason:?}");
    Ok(())
}

/// 工具做成了：把和 `--json` 一模一样的对象贴回对话。
fn answer<T: serde::Serialize>(value: &T) -> CallToolResult {
    CallToolResult::success(vec![ContentBlock::text(output::to_json_pretty(value))])
}

/// 工具没做成。用 `CallToolResult::error` 而不是协议错——协议错在多数客户端里只显示
/// 「内部错误」，我们写的那句话对面根本看不到。
fn refuse(e: &anyhow::Error, started: Instant) -> CallToolResult {
    let failure = output::classify(e).with_elapsed(output::ms_since(started));
    CallToolResult::error(vec![ContentBlock::text(output::to_json_pretty(&failure))])
}

/// `playtest mcp --setup`：一段能直接粘进编辑器配置的 JSON。
///
/// `command` 填的是这个二进制此刻的绝对路径，不是 `playtest`——编辑器起 MCP server 时的
/// PATH 常常和终端里的不一样，写死路径能省掉一整类「为什么它说找不到命令」。
fn print_setup(api: Option<&str>) {
    let exe = std::env::current_exe()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|_| "playtest".to_string());
    let mut args = vec!["mcp".to_string()];
    if let Some(api) = api {
        args.push("--api".to_string());
        args.push(api.to_string());
    }
    let config = serde_json::json!({
        "mcpServers": {
            "playtest": { "command": exe, "args": args }
        }
    });

    eprintln!("把下面这段放进编辑器的 MCP 配置里：");
    eprintln!("  Cursor：这个项目用 .cursor/mcp.json，所有项目都用 ~/.cursor/mcp.json");
    eprintln!("  Claude Code：项目根目录的 .mcp.json，或者 claude mcp add playtest -- <上面的 command 和 args>");
    eprintln!("已经有别的 server 的话，只把 \"playtest\" 那一项加进 mcpServers 里。");
    eprintln!();
    println!("{}", output::to_json_pretty(&config));
    eprintln!();
    eprintln!("装好之后在对话里说「把 ./dist 发出去」，助手会调 playtest_upload 并把链接贴回来。");
}
