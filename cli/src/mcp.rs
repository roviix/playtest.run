//! `playtest mcp`：把这条命令变成 Cursor / Claude Code 里的一个工具。
//!
//! 用 AI 写小东西的人未必开终端，但一定在跟模型对话（DESIGN §3.2）。所以这里只是一个壳：
//! 工具做的事和命令行完全一样，回给对话的 JSON 和 `--json` 是同一个形状。
//!
//! **stdout 是协议的**。MCP 的 stdio 传输把 stdout 当 JSON-RPC 通道，往里写一个字节的日志
//! 都会让对面解析失败——所以这个模块里所有说给人听的话都走 stderr，工具的结果走返回值。
//!
//! 工具的说明写英文加中文各一句：模型读英文，人在配置界面里读中文。

use std::sync::Arc;
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
    /// 正在跑着的隧道。放在这里不是为了以后查它，是为了**不 drop 它**——
    /// 落地之后没人持有这个 handle，tokio 会把任务收走，链接当场就断。
    tunnels: Arc<tokio::sync::Mutex<Vec<tokio::task::JoinHandle<anyhow::Result<()>>>>>,
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
    /// How many testers are wanted; marks it "looking for testers" on the plaza, shown on the gate page and the invite card. Implies public.
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
    /// Title players see before they start. 玩家开始前看到的作品名。
    #[serde(default)]
    pub name: Option<String>,
}

#[tool_router]
impl Playtest {
    pub fn new(api: Option<String>) -> Self {
        Self {
            api,
            tunnels: Arc::default(),
        }
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
            seats: params.seats,
            community: params.community,
            isolated: match params.isolated {
                Some(true) => crate::args::Isolation::On,
                Some(false) => crate::args::Isolation::Off,
                None => crate::args::Isolation::Auto,
            },
            api: self.api.clone(),
            ..UploadArgs::default()
        };
        match upload::run(&args, &dir).await {
            Ok(mut report) => {
                // 进程从早上就开着，这次调用花了多久才是要报的数。
                let card = card::fetch(&report.card_url).await.ok();
                report.elapsed_ms = output::ms_since(started);
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
        let site = match crate::commands::look_up(&params.site, self.api.as_deref()).await {
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
            "url": playtest_common::door_url(&site.url, &site.slug),
            "card_url": card_url,
            "elapsed_ms": output::ms_since(started),
        });
        Ok(answer_with_card(&answered, Some(png), &card_url))
    }

    /// 把本地端口接出去。隧道留在这个进程里活着，所以工具本身立刻回答。
    #[tool(
        description = "Share a locally running dev server (e.g. vite on 5173) through a tunnel and \
                       get a public link. The tunnel stays up for as long as this MCP server runs; \
                       the link stops working when it exits. Use this when the user has something \
                       running but no built directory; use playtest_upload when they have a build. \
                       把本地开着的开发服务器接出去，拿到一条公网链接和二维码。\
                       隧道在这个 MCP server 活着的时候一直开着，退出就断。"
    )]
    async fn playtest_share(
        &self,
        Parameters(params): Parameters<PortParams>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        let port = params.port;
        let args = UploadArgs {
            target: Some(port.to_string()),
            name: params.name.clone(),
            api: self.api.clone(),
            ..UploadArgs::default()
        };

        let (tx, rx) = tokio::sync::oneshot::channel();
        // 隧道要一直跑着，工具调用不能一直不返回：把它放进后台，等「连上了」那一声。
        // handle 存在 server 状态里，不 detach——落地之后没人持有它，隧道会被 drop 掉。
        let task = tokio::spawn(async move { crate::tunnel::serve(&args, port, tx).await });

        let online = match tokio::time::timeout(SHARE_READY_TIMEOUT, rx).await {
            Ok(Ok(online)) => online,
            // 隧道自己先结束了（端口没人听、被挤掉、控制面拒绝）——它的错才是要说的那个。
            Ok(Err(_)) => {
                let said = match task.await {
                    Ok(Err(e)) => e,
                    Ok(Ok(())) => anyhow::anyhow!("The tunnel ended before it connected."),
                    Err(e) => anyhow::anyhow!("A background task failed to start: {e}"),
                };
                return Ok(refuse(&said, started));
            }
            Err(_) => {
                task.abort();
                return Ok(refuse(
                    &output::bad_input(format!(
                        "No connection after {} seconds, so giving up. Check that something is listening on port {port} and that the network is reachable.",
                        SHARE_READY_TIMEOUT.as_secs()
                    )),
                    started,
                ));
            }
        };

        let card_url = playtest_common::card_url(&online.url);
        // 拿不到卡不算失败：链接已经能用了，卡是锦上添花（`card::take` 同一条规矩）。
        let png = card::fetch(&card_url).await.ok();
        let answered = serde_json::json!({
            "ok": true,
            "action": "share",
            "slug": online.slug,
            "url": online.url,
            "port": port,
            "qr_text": crate::ui::qr_text(&online.url),
            "card_url": card_url,
            "expires_at": online.expires_at,
            // 说清楚它什么时候会断——不然助手会把它当成一条永久链接转述出去。
            "lasts": "while this MCP server is running",
            "elapsed_ms": output::ms_since(started),
        });
        self.tunnels.lock().await.push(task);
        Ok(answer_with_card(&answered, png, &card_url))
    }
}

/// 等隧道连上最多等这么久。超过多半是端口上没东西，或者网络出不去——
/// 那两件事都该当场说，不该让对话卡在这里。
const SHARE_READY_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(20);

// `version` 只收字面量，所以这里抄了一份 `Cargo.toml` 的版本号。抄岔了，tests/mcp.rs 里那条
// 握手测试会挂——它比对的是握手时真发出去的那个值。
#[tool_handler(
    name = "playtest",
    version = "0.4.1",
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
    eprintln!("playtest is running as an MCP server, waiting for an editor to connect. stdout carries the protocol only; everything readable goes here.");
    let service = Playtest::new(api)
        .serve(stdio())
        .await
        .context("the MCP server could not start")?;
    let reason = service
        .waiting()
        .await
        .context("the MCP connection dropped")?;
    eprintln!("MCP connection closed: {reason:?}");
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
            "The invite card is not rendered yet. It will be at {card_url} shortly, or call playtest_card again."
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

    eprintln!("Put this into your editor's MCP config:");
    eprintln!("  Cursor: .cursor/mcp.json for this project, ~/.cursor/mcp.json for every project");
    eprintln!("  Claude Code: .mcp.json in the project root, or claude mcp add playtest -- <the command and args above>");
    eprintln!(
        "If you already run other servers, add just the \"playtest\" entry under mcpServers."
    );
    eprintln!();
    println!("{}", output::to_json_pretty(&config));
    eprintln!();
    eprintln!("Once it is wired up, say \"publish ./dist\" in chat: the assistant calls playtest_upload and hands back the link, QR code and invite card.");
    eprintln!("Five tools: publish a directory, share a local port, list projects, check how one project is doing, fetch an invite card.");
}
