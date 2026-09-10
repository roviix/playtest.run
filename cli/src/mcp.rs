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
use base64::Engine as _;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ContentBlock};
use rmcp::transport::stdio;
use rmcp::{
    schemars, tool, tool_handler, tool_router, ErrorData as McpError, ServerHandler, ServiceExt,
};
use serde::Deserialize;

use crate::args::UploadArgs;
use crate::card;
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
    /// How many testers are wanted; shown on the gate page and the invite card. Implies looking-for-testers.
    /// 想找几位试玩者；门禁页和邀请卡上会写出来。蕴含「正在找人测」。
    #[serde(default)]
    pub seats: Option<u32>,
    /// Link to the developer's group chat (http/https), shown to players after they play.
    /// 开发者的群链接（http/https），玩家在门禁页和反馈之后看到。
    #[serde(default)]
    pub community: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SiteParams {
    /// The slug of the work, e.g. brisk-otter-41. 作品的 slug，例如 brisk-otter-41。
    pub slug: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CardParams {
    /// The slug of the work, or a directory that was published before. 作品的 slug，或者一个发过的目录。
    pub site: String,
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

    /// 上传目录（DESIGN §3.2 五个工具之一）。
    #[tool(
        description = "Publish a built web game directory: returns a shareable link, a QR code and \
                       the invite card as an image block — hand the image to the user, it is what \
                       they forward to a group chat. \
                       把构建好的目录发出去，拿到链接、二维码，以及一张可以直接转发的邀请卡图片。"
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
            seats: params.seats,
            community: params.community,
            isolated: params.isolated.unwrap_or(false),
            api: self.api.clone(),
            ..UploadArgs::default()
        };
        match upload::run(&args, &dir).await {
            Ok(mut report) => {
                // 进程从早上就开着，这次调用花了多久才是要报的数。
                report.elapsed_ms = output::ms_since(started);
                let card = report.take_card_png();
                Ok(answer_with_card(&report, card, &report.card_url))
            }
            Err(e) => Ok(refuse(&e, started)),
        }
    }

    /// 列出作品。
    #[tool(
        description = "List the works published from this machine, with their links, versions and \
                       follower counts. 列出这台机器上发过的作品：链接、版本、有多少人关注着。"
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

    /// 看这一版的结果。
    #[tool(
        description = "See how this version is doing: link, current version, how many people follow \
                       it, and the console page where the roster and feedback live. \
                       看这一版怎么样了：链接、当前版本、有多少人关注，以及去哪儿看谁玩过、说了什么。"
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
                // 「谁玩过、玩到哪、说了什么」在控制台里，不在这个返回里（DESIGN §3.5）：
                // 那是一页要看的东西，塞进对话不如把地址给出去。
                "console_url": output::console_url(&site.slug),
                "site": site,
                "elapsed_ms": output::ms_since(started),
            }))),
            Err(e) => Ok(refuse(&e, started)),
        }
    }

    /// 拿邀请卡。
    #[tool(
        description = "Fetch the invite card for a work as an image: a 1080x1350 PNG with the cover, \
                       the title, the seats left and a QR code. Hand the image to the user so they \
                       can forward it. 拿这个作品的邀请卡（一张竖版 PNG，含封面、作品名、名额和二维码），\
                       把图交给用户，他直接转发到群里。"
    )]
    async fn playtest_card(
        &self,
        Parameters(params): Parameters<CardParams>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        let site = match crate::sites::look_up(&params.site, self.api.as_deref()).await {
            Ok(site) => site,
            Err(e) => return Ok(refuse(&e, started)),
        };
        let png = match card::fetch_or_explain(&site).await {
            Ok(png) => png,
            Err(e) => return Ok(refuse(&e, started)),
        };
        let card_url = playtest_common::card_url(&site.url);
        let answered = serde_json::json!({
            "ok": true,
            "action": "card",
            "slug": site.slug,
            "title": site.title,
            "url": site.url,
            "card_url": card_url,
            "elapsed_ms": output::ms_since(started),
        });
        Ok(answer_with_card(&answered, Some(png), &card_url))
    }

    /// 分享端口。这条路还没上线，如实说（AGENTS 第 4 条）。
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
    version = "0.2.0",
    instructions = "playtest puts a playable build in front of specific people. Call \
                    playtest_upload with the directory your build step produced; give the user the \
                    returned url, paste qr_text verbatim in a code block when they will open it on \
                    a phone, and pass on the invite card image — that image is what they forward to \
                    a group chat. Every tool answers with one JSON object in its first content \
                    block; ok:false means it did not happen — read code and hint before retrying."
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

/// 同上，再贴一张邀请卡。
///
/// 图为什么值得单独贴一块：卡是用户真正要发出去的那个东西（DESIGN §3.4）。只给一个地址的话，
/// 对面得自己下载、再拖进微信；贴成图片块，编辑器里直接就能存、能转。
///
/// 第一块永远是那个 JSON，图排在后面——读结果的是程序，它按顺序取第一块。
/// 卡没拿到不算这次工具失败：链接已经能玩了，多说一句去哪儿拿就行。
fn answer_with_card<T: serde::Serialize>(
    value: &T,
    png: Option<Vec<u8>>,
    card_url: &str,
) -> CallToolResult {
    let mut blocks = vec![ContentBlock::text(output::to_json_pretty(value))];
    match png {
        Some(png) => blocks.push(ContentBlock::image(
            base64::engine::general_purpose::STANDARD.encode(png),
            "image/png".to_string(),
        )),
        None => blocks.push(ContentBlock::text(format!(
            "邀请卡这会儿还没渲染好，稍后在 {card_url} 能拿到，或者再调一次 playtest_card。"
        ))),
    }
    CallToolResult::success(blocks)
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
    eprintln!("装好之后在对话里说「把 ./dist 发出去」，助手会调 playtest_upload，把链接、二维码和邀请卡贴回来。");
    eprintln!("一共五件事：分享端口（还没上线）、上传目录、列出作品、看这一版的结果、拿邀请卡。");
}
