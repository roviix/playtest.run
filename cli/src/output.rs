//! 机器模式：`--json` 的输出形状、分层退出码、一次命令花了多少时间。
//!
//! 人看的那一套在 [`crate::ui`]，这里管的是脚本和 agent 看的那一套。两者的分界只有一条：
//! **加了 `--json`，stdout 上就只有一个 JSON 对象**，别的什么都没有；说明、进度、提醒、
//! 报错的原文全部走 stderr。没加 `--json` 时这个模块只负责结尾那行耗时。
//!
//! # stdout 上那个对象长什么样
//!
//! 字段只加不删、不改名——写脚本的人不该因为我们发了个新版本就得改一遍。
//!
//! 成功，`action` 说明这是哪条命令的结果：
//!
//! ```json
//! {"ok":true,"action":"upload","slug":"brisk-otter-41",
//!  "url":"https://brisk-otter-41.playtest.run","version":7,
//!  "elapsed_ms":4200,"timings":{"hash_ms":300,"upload_ms":3100,"commit_ms":800},
//!  "expires_at":"2026-09-08T04:09:03Z","qr_text":"█▀▀▀▀▀█ …",
//!  "findings":[{"level":"warn","message":"这个导出用到了 SharedArrayBuffer（线程）","hint":"加 --isolated"}]}
//! {"ok":true,"action":"list","elapsed_ms":120,
//!  "sites":[{"slug":"brisk-otter-41","url":"…","title":"小球","version":7,"expires_at":"…"}]}
//! {"ok":true,"action":"remove","slug":"brisk-otter-41","elapsed_ms":90}
//! {"ok":true,"action":"open","slug":"brisk-otter-41","url":"…","opened":false,"elapsed_ms":80}
//! {"ok":true,"action":"help","text":"…帮助全文…","elapsed_ms":1}
//! ```
//!
//! 失败：
//!
//! ```json
//! {"ok":false,"code":"quota_exceeded","message":"服务器说：这个版本太大了",
//!  "hint":"匿名上传每个版本最多 200 MB。","elapsed_ms":800}
//! ```
//!
//! 几条约定：
//!
//! - `qr_text` 是终端二维码的原文，**JSON 模式下不往终端画**，交给 agent 自己贴回对话；
//!   加了 `--no-qr` 就没有这个字段。
//! - `expires_at`、`hint`、`qr_text` 可能不出现（没有就是没有），其余字段一定在。
//! - `findings` 是上传时对导出物的检查结果，见 [`Finding`]；没有发现就是空数组。
//! - `elapsed_ms` 从进程启动算到打印这一刻，`timings` 是其中几段。
//!
//! # 退出码
//!
//! 人类模式和 JSON 模式一致，见 [`Code`]。

use std::io::Write;
use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;
use std::time::Instant;

use anyhow::Result;
use playtest_common::api::{ErrorCode, Site};
use serde::{Serialize, Serializer};

use crate::client::{self, Client};
use crate::config::{self, Config};
use crate::{args, clock, ui};

// ---------------------------------------------------------------- 模式与秒表

static JSON: AtomicBool = AtomicBool::new(false);
static STARTED: OnceLock<Instant> = OnceLock::new();

/// 在 `main` 的第一行调用：记下开始时间，并先按命令行里有没有 `--json` 定一次模式。
///
/// 为什么不等 clap 解析完再定：命令写错的时候也得按机器模式回答，而那时候还没有解析结果。
/// 解析成功之后 [`set_json`] 会用真正的解析结果覆盖一次，`-m "--json"` 这种误判就纠正回来了。
pub fn begin() {
    let _ = STARTED.set(Instant::now());
    JSON.store(
        std::env::args_os().skip(1).any(|arg| arg == "--json"),
        Ordering::Relaxed,
    );
}

pub fn set_json(on: bool) {
    JSON.store(on, Ordering::Relaxed);
}

pub fn is_json() -> bool {
    JSON.load(Ordering::Relaxed)
}

/// 从进程启动到现在。[`begin`] 没被调用过就是 0。
pub fn elapsed_ms() -> u64 {
    STARTED.get().map_or(0, |start| ms_since(*start))
}

pub fn ms_since(start: Instant) -> u64 {
    start.elapsed().as_millis() as u64
}

/// 一次上传里各段花了多久。加起来不等于 `elapsed_ms`——中间还有建会话、建作品这些往返。
#[derive(Debug, Default, Clone, Copy, Serialize)]
pub struct Timings {
    /// 走一遍目录、读文件、算哈希。
    pub hash_ms: u64,
    /// 把服务器缺的那些字节传上去。服务器都已经有了就是 0。
    pub upload_ms: u64,
    /// 提交清单，换回版本号和链接。
    pub commit_ms: u64,
}

// ---------------------------------------------------------------- 退出码

/// 脚本靠退出码分流，不用去猜错误文案。
///
/// | 码 | 什么意思 |
/// |---|---|
/// | 0 | 做成了 |
/// | 1 | 没预料到的错误 |
/// | 2 | 命令写错了，或者要的功能还没做好 |
/// | 3 | 需要登录，或者身份失效了 |
/// | 4 | 网络不通 |
/// | 5 | 服务端出错 |
/// | 6 | 给的东西有问题（目录不在、超限、不像导出物） |
/// | 7 | 配额用完了 |
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Code {
    /// 命令写错了。
    Usage,
    /// 这个功能还没做好。和 [`Code::Usage`] 共用退出码 2——两者都是「换个命令」，
    /// 要分辨就看 JSON 里的 `code`。
    NotImplemented,
    NeedsLogin,
    Network,
    ServerError,
    BadInput,
    QuotaExceeded,
    /// 没归到类的错。
    Unexpected,
}

impl Code {
    pub fn exit(self) -> u8 {
        match self {
            Code::Unexpected => 1,
            Code::Usage | Code::NotImplemented => 2,
            Code::NeedsLogin => 3,
            Code::Network => 4,
            Code::ServerError => 5,
            Code::BadInput => 6,
            Code::QuotaExceeded => 7,
        }
    }

    /// JSON 里的 `code`。这些字符串是接口的一部分，不改。
    pub fn as_str(self) -> &'static str {
        match self {
            Code::Usage => "usage",
            Code::NotImplemented => "not_implemented",
            Code::NeedsLogin => "needs_login",
            Code::Network => "network",
            Code::ServerError => "server_error",
            Code::BadInput => "bad_input",
            Code::QuotaExceeded => "quota_exceeded",
            Code::Unexpected => "error",
        }
    }
}

impl Serialize for Code {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

// ---------------------------------------------------------------- 错误

/// 一个已经归好类的错误。挂在 `anyhow` 链上，[`classify`] 认得它。
#[derive(Debug)]
pub struct Fail {
    code: Code,
    message: String,
    hint: Option<String>,
}

impl std::fmt::Display for Fail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for Fail {}

fn fail(code: Code, message: impl Into<String>, hint: Option<String>) -> anyhow::Error {
    Fail {
        code,
        message: message.into(),
        hint,
    }
    .into()
}

/// 命令写错了。
pub fn usage(message: impl Into<String>) -> anyhow::Error {
    fail(Code::Usage, message, None)
}

/// 这个功能还没做好。说清楚现在能做什么，不要只说「不支持」。
pub fn not_implemented(message: impl Into<String>) -> anyhow::Error {
    fail(Code::NotImplemented, message, None)
}

/// 给的东西有问题：目录不在、超限、不像导出物。
pub fn bad_input(message: impl Into<String>) -> anyhow::Error {
    fail(Code::BadInput, message, None)
}

/// 同上，但带一句「该怎么办」。
pub fn bad_input_with_hint(message: impl Into<String>, hint: impl Into<String>) -> anyhow::Error {
    fail(Code::BadInput, message, Some(hint.into()))
}

/// 把一段本地检查里出的错都算成「给的东西有问题」。已经归过类的原样放行。
pub fn as_bad_input(e: anyhow::Error) -> anyhow::Error {
    if e.chain()
        .any(|cause| cause.is::<Fail>() || cause.is::<client::Error>())
    {
        return e;
    }
    bad_input(format!("{e:#}"))
}

/// 失败时 stdout 上那个对象。
#[derive(Debug, Serialize)]
pub struct Failure {
    ok: bool,
    code: Code,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    hint: Option<String>,
    elapsed_ms: u64,
}

impl Failure {
    fn new(code: Code, message: String, hint: Option<String>) -> Self {
        Self {
            ok: false,
            code,
            message,
            hint,
            elapsed_ms: elapsed_ms(),
        }
    }

    /// MCP 里没有「进程启动」这回事，一次工具调用花了多久由调用方填。
    pub fn with_elapsed(mut self, elapsed_ms: u64) -> Self {
        self.elapsed_ms = elapsed_ms;
        self
    }
}

/// 一个错误该算哪一类。
///
/// 沿着整条因果链找第一个认得的错——中间套过 `.context("传不上去 x")` 也还能认出来底下
/// 那个是「连不上」还是「配额用完了」。消息用整条链的原文，哪一步出的事不会丢。
pub fn classify(e: &anyhow::Error) -> Failure {
    let message = format!("{e:#}");
    for cause in e.chain() {
        if let Some(known) = cause.downcast_ref::<Fail>() {
            return Failure::new(known.code, message, known.hint.clone());
        }
        if let Some(from_server) = cause.downcast_ref::<client::Error>() {
            let (code, hint) = classify_client(from_server);
            return Failure::new(code, message, hint);
        }
    }
    Failure::new(Code::Unexpected, message, None)
}

fn classify_client(e: &client::Error) -> (Code, Option<String>) {
    match e {
        client::Error::Transport { .. } => (
            Code::Network,
            Some("控制面地址可以用 --api 或环境变量 PLAYTEST_API 指定。".into()),
        ),
        client::Error::Local { .. } => (Code::BadInput, None),
        client::Error::Unexpected { .. } => (
            Code::ServerError,
            Some("这个地址回的不是 playtest 控制面的格式，确认一下 --api。".into()),
        ),
        client::Error::Server { status, body } => match body.code {
            ErrorCode::QuotaExceeded => (
                Code::QuotaExceeded,
                Some("匿名上传有额度上限，登录之后额度更大（登录还没做好）。".into()),
            ),
            ErrorCode::Unauthorized | ErrorCode::TokenExpired => (
                Code::NeedsLogin,
                Some("匿名身份只保留 24 小时。再跑一次会自动换一个新的，链接也会是新的。".into()),
            ),
            ErrorCode::Internal => (Code::ServerError, Some("过一会儿再试一次。".into())),
            _ if *status >= 500 => (Code::ServerError, Some("过一会儿再试一次。".into())),
            _ => (Code::BadInput, None),
        },
    }
}

/// 把失败说出去，并给出这一次的退出码。
pub fn report_failure(failure: &Failure) -> ExitCode {
    if is_json() {
        emit(failure);
    } else {
        ui::fail(&failure.message);
        if let Some(hint) = &failure.hint {
            ui::say(hint);
        }
    }
    ExitCode::from(failure.code.exit())
}

// ---------------------------------------------------------------- 上传的结果

/// 上传时对导出物的检查发现的一件事。
///
/// 这是留给「上传时检查」那部分的接口：检查只管产出 `Finding`，往哪儿说、说成什么样由这里决定
/// ——人类模式一条一行走 stderr，JSON 模式进 `findings` 数组。
#[derive(Debug, Clone, Serialize)]
pub struct Finding {
    pub level: Level,
    /// 一句话，第一次用的人看得懂，不带内部名词。
    pub message: String,
    /// 该怎么办。没有可操作的建议就别硬写。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    /// 说一声，不影响。
    Note,
    /// 可能跑不起来。照传，但要让人看见。
    Warn,
    /// 传上去也一定打不开。
    Blocker,
}

impl Finding {
    pub fn note(message: impl Into<String>) -> Self {
        Self {
            level: Level::Note,
            message: message.into(),
            hint: None,
        }
    }

    pub fn warn(message: impl Into<String>) -> Self {
        Self {
            level: Level::Warn,
            message: message.into(),
            hint: None,
        }
    }

    pub fn blocker(message: impl Into<String>) -> Self {
        Self {
            level: Level::Blocker,
            message: message.into(),
            hint: None,
        }
    }

    pub fn hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }
}

/// 人类模式下把检查结果说出来。JSON 模式下它们在 `findings` 里，这里就不重复了。
pub fn say_findings(findings: &[Finding]) {
    if is_json() {
        return;
    }
    for finding in findings {
        let line = match &finding.hint {
            Some(hint) => format!("{}。{hint}", finding.message),
            None => finding.message.clone(),
        };
        match finding.level {
            Level::Note => ui::say(&line),
            Level::Warn | Level::Blocker => ui::warn(&line),
        }
    }
}

/// 一次上传做成了之后要说的全部东西。
///
/// `upload` 只负责把它填出来，不负责打印——MCP 那条路要的是同一份数据，但一个字都不能往
/// stdout 写（那是 JSON-RPC 的通道）。
#[derive(Debug, Serialize)]
pub struct UploadReport {
    ok: bool,
    action: &'static str,
    pub slug: String,
    pub url: String,
    /// 玩家在门禁页上看到的作品名。发完就打出来，别让人到玩家那边才发现叫《v2》。
    pub title: String,
    pub version: u32,
    /// 从进程启动到这一刻。MCP 里没有「进程启动」这回事，由调用方改写成这次工具调用的耗时。
    pub elapsed_ms: u64,
    pub timings: Timings,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub qr_text: Option<String>,
    pub findings: Vec<Finding>,
    /// 这次上传之后作品在广场上的状态（DESIGN §3.8）。没动过广场就没有这一段。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plaza: Option<PlazaOut>,
}

/// 广场状态在输出里的样子。
#[derive(Debug, Clone, Serialize)]
pub struct PlazaOut {
    /// 广场的地址（根域）。
    pub url: String,
    pub public: bool,
    pub seeking: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seek_note: Option<String>,
}

impl UploadReport {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        slug: String,
        url: String,
        title: String,
        version: u32,
        timings: Timings,
        expires_at: Option<String>,
        qr_text: Option<String>,
        findings: Vec<Finding>,
    ) -> Self {
        Self {
            ok: true,
            action: "upload",
            slug,
            url,
            title,
            version,
            elapsed_ms: elapsed_ms(),
            timings,
            expires_at,
            qr_text,
            findings,
            plaza: None,
        }
    }
}

/// 上传做成了：JSON 模式给一个对象，人类模式给链接、二维码和耗时。
pub fn report_upload(report: &UploadReport) {
    if is_json() {
        emit(report);
        return;
    }
    ui::say(&format!("已发布《{}》v{}", report.title, report.version));
    ui::blank();
    ui::link(&report.url);
    ui::blank();
    if let Some(qr) = &report.qr_text {
        if ui::can_draw_qr() {
            ui::print_qr(qr);
            ui::say("手机扫码就能玩。");
        }
    }
    if let Some(plaza) = &report.plaza {
        ui::say(&plaza_line(plaza));
    }
    if let Some(expires_at) = &report.expires_at {
        ui::say(&format!(
            "这是匿名链接，{} 后失效。保留、改名需要登录（登录还没做好）。",
            clock::human(expires_at)
        ));
    }
    ui::say(&format!(
        "谁打开了、玩到哪、报了什么错：{}/console/（令牌在 playtest 的配置文件里）",
        playtest_common::DEVELOPER_API_URL
    ));
    ui::say(&timing_line(report.elapsed_ms, report.timings));
}

pub fn plaza_line(plaza: &PlazaOut) -> String {
    match (plaza.public, plaza.seeking) {
        (true, true) => format!("已放到广场上，标了「正在找人测」：{}", plaza.url),
        (true, false) => format!("已放到广场上：{}", plaza.url),
        (false, _) => "已从广场上拿下来，链接照常能开。".to_string(),
    }
}

/// 「本次 4.2 秒（哈希 0.3 · 上传 3.1 · 提交 0.8）」。
///
/// DESIGN §8 要的那个数就是句首那个：从敲下命令到链接出现。分段是为了知道慢在哪一段。
fn timing_line(elapsed_ms: u64, timings: Timings) -> String {
    format!(
        "本次 {} 秒（哈希 {} · 上传 {} · 提交 {}）",
        seconds(elapsed_ms),
        seconds(timings.hash_ms),
        seconds(timings.upload_ms),
        seconds(timings.commit_ms)
    )
}

fn seconds(ms: u64) -> String {
    format!("{:.1}", ms as f64 / 1000.0)
}

// ---------------------------------------------------------------- ls / rm / open

#[derive(Debug, Serialize)]
struct ListReport {
    ok: bool,
    action: &'static str,
    elapsed_ms: u64,
    sites: Vec<SiteOut>,
}

/// 一个作品在 JSON 里的样子。字段名和 `common` 里的 [`Site`] 对齐，只把
/// `current_version` 缩成 `version`——脚本里不需要「当前」这个限定。
#[derive(Debug, Serialize)]
pub struct SiteOut {
    pub slug: String,
    pub url: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
}

impl From<Site> for SiteOut {
    fn from(site: Site) -> Self {
        Self {
            slug: site.slug,
            url: site.url,
            title: site.title,
            version: site.current_version,
            expires_at: site.expires_at,
        }
    }
}

#[derive(Debug, Serialize)]
struct RemoveReport {
    ok: bool,
    action: &'static str,
    slug: String,
    elapsed_ms: u64,
}

#[derive(Debug, Serialize)]
struct OpenReport {
    ok: bool,
    action: &'static str,
    slug: String,
    url: String,
    /// JSON 模式下不弹浏览器：跑在 agent 或 CI 里多半没有浏览器，也不该抢焦点。
    opened: bool,
    elapsed_ms: u64,
}

#[derive(Debug, Serialize)]
struct HelpReport {
    ok: bool,
    action: &'static str,
    text: String,
    elapsed_ms: u64,
}

/// `playtest ls --json`。
pub async fn ls(api_flag: Option<&str>) -> Result<()> {
    let sites = fetch_sites(api_flag).await?;
    emit(&ListReport {
        ok: true,
        action: "list",
        elapsed_ms: elapsed_ms(),
        sites,
    });
    Ok(())
}

/// `playtest rm <slug> --json`。
pub async fn rm(slug: &str, yes: bool, api_flag: Option<&str>) -> Result<()> {
    if !yes {
        return Err(usage(format!(
            "--json 模式不会停下来问你。确定要删就加 -y：playtest rm {slug} -y --json"
        )));
    }
    let api = args::api_base(api_flag);
    let config_path = config::default_path()?;
    let mut config = config::load(&config_path)?;
    let Some(client) = saved_client(&api, &config)? else {
        return Err(bad_input(format!(
            "这台机器上还没发过东西，没有 {slug} 可以删。"
        )));
    };
    client.delete_site(slug).await?;
    config.forget_slug(slug);
    config::save(&config_path, &config)?;
    emit(&RemoveReport {
        ok: true,
        action: "remove",
        slug: slug.to_string(),
        elapsed_ms: elapsed_ms(),
    });
    Ok(())
}

/// `playtest open <slug 或目录> --json`：只回链接，不弹浏览器。
pub async fn open(target: &str, api_flag: Option<&str>) -> Result<()> {
    let api = args::api_base(api_flag);
    let config = config::load(&config::default_path()?)?;
    let slug = slug_of(target, &config)?;
    let Some(client) = saved_client(&api, &config)? else {
        return Err(bad_input("这台机器上还没发过东西，没有链接可以打开。"));
    };
    let site = client.get_site(&slug).await?;
    emit(&OpenReport {
        ok: true,
        action: "open",
        slug: site.slug,
        url: site.url,
        opened: false,
        elapsed_ms: elapsed_ms(),
    });
    Ok(())
}

/// `playtest --help --json`：帮助本身就是这次命令的回答，装进对象里给出去。
pub fn help(text: String) {
    emit(&HelpReport {
        ok: true,
        action: "help",
        text,
        elapsed_ms: elapsed_ms(),
    });
}

/// 这台机器发过的作品。没有令牌就是还没发过，回空表而不是报错——问「我有什么」的答案是「没有」。
pub async fn fetch_sites(api_flag: Option<&str>) -> Result<Vec<SiteOut>> {
    let api = args::api_base(api_flag);
    let config = config::load(&config::default_path()?)?;
    let Some(client) = saved_client(&api, &config)? else {
        return Ok(Vec::new());
    };
    Ok(client
        .list_sites()
        .await?
        .into_iter()
        .map(SiteOut::from)
        .collect())
}

/// 一个作品现在什么样。
pub async fn fetch_site(slug: &str, api_flag: Option<&str>) -> Result<SiteOut> {
    let api = args::api_base(api_flag);
    let config = config::load(&config::default_path()?)?;
    let Some(client) = saved_client(&api, &config)? else {
        return Err(bad_input(
            "这台机器上还没发过东西，先发一个：playtest ./dist",
        ));
    };
    Ok(client.get_site(slug).await?.into())
}

/// 参数可以是 slug，也可以是一个发过的目录。
fn slug_of(target: &str, config: &Config) -> Result<String> {
    let path = std::path::Path::new(target);
    if !path.is_dir() {
        return Ok(target.to_string());
    }
    let key = std::fs::canonicalize(path)
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|_| target.to_string());
    match config.remembered_slug(&key) {
        Some(slug) => Ok(slug.to_string()),
        None => Err(bad_input(format!(
            "{target} 这个目录还没发过。先运行 playtest {target} 把它发出去。"
        ))),
    }
}

/// 用已经存下来的令牌建客户端。没有就是 `None`——看和删都是看已有的东西，
/// 为此凭空要一个新的匿名令牌只会看到空列表。
fn saved_client(api: &str, config: &Config) -> Result<Option<Client>> {
    let Some(token) = config.usable_token(api, clock::now()) else {
        return Ok(None);
    };
    let mut client = Client::new(api)?;
    client.set_token(Some(token.to_string()));
    Ok(Some(client))
}

// ---------------------------------------------------------------- 隧道的结果

/// 隧道模式的 stdout 是**一行一个事件**，不是一个对象。
///
/// `playtest ./dist` 做完就结束，一个对象说得完；`playtest 5173` 要一直跑着，中途会上线、
/// 掉线、重连、来人、走人。所以这里破例：每发生一件事写一行 JSON，用 `event` 区分是哪一件。
/// 失败仍然是那个 `{"ok":false,…}` 对象加分层退出码，和别的命令一样。
///
/// ```json
/// {"event":"online","slug":"brisk-otter-41","url":"https://brisk-otter-41.playtest.run",
///  "expires_at":"2026-09-08T04:09:03Z","attempt":0,"elapsed_ms":1200,"qr_text":"█▀▀▀▀▀█ …",
///  "findings":[{"level":"note","message":"检测到 Vite：…"}]}
/// {"event":"players","count":3}
/// {"event":"reconnecting","attempt":2,"wait_ms":4000,"reason":"和服务器断开了"}
/// {"event":"stopped","connections":12,"bytes":3407872}
/// ```
///
/// `attempt` 是第几次连上：0 是这次运行的第一次，之后每重连成功一次加一——链接不变，
/// 所以 `online` 会再出现一次而不是另起一个事件名。
#[derive(Debug, Serialize)]
pub struct OnlineReport {
    event: &'static str,
    pub slug: String,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
    pub attempt: u32,
    /// 从进程启动到链接出现在屏幕上。DESIGN §3.2 要的那个数。
    pub elapsed_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub qr_text: Option<String>,
    /// 这次启动看出来的事：Vite 的热更新提示、页面太大该改用上传，等等。
    pub findings: Vec<Finding>,
}

impl OnlineReport {
    pub fn new(
        slug: String,
        url: String,
        expires_at: Option<String>,
        qr_text: Option<String>,
        findings: Vec<Finding>,
    ) -> Self {
        Self {
            event: "online",
            slug,
            url,
            expires_at,
            attempt: 0,
            elapsed_ms: elapsed_ms(),
            qr_text,
            findings,
        }
    }
}

/// 隧道连上了。第一次连上给全套（链接、二维码、有效期、提示、耗时），
/// 重连回来只说一声——链接没变，把整块再刷一遍只会让人以为换了链接。
pub fn report_online(report: &OnlineReport) {
    if is_json() {
        emit(report);
        return;
    }
    if report.attempt > 0 {
        ui::say("已重连，链接没变。");
        return;
    }
    ui::say("已连上，这条链接现在能玩了。");
    ui::blank();
    ui::link(&report.url);
    ui::blank();
    if let Some(qr) = &report.qr_text {
        if ui::can_draw_qr() {
            ui::print_qr(qr);
            ui::say("手机扫码就能玩。");
        }
    }
    if let Some(expires_at) = &report.expires_at {
        ui::say(&format!(
            "这是匿名链接，{} 后失效。保留、改名、查看结果需要登录（登录还没做好）。",
            clock::human(expires_at)
        ));
    }
    say_findings(&report.findings);
    ui::say("按 Ctrl-C 结束，结束后玩家会看到「开发者的电脑暂时不在线」。");
    ui::say(&format!(
        "本次 {} 秒（从敲命令到链接出来）",
        seconds(report.elapsed_ms)
    ));
}

#[derive(Debug, Serialize)]
struct PlayersEvent {
    event: &'static str,
    count: usize,
}

/// 同时连着的玩家连接数变了。只在变化时说一句，不每秒刷屏。
pub fn report_players(count: usize) {
    if is_json() {
        emit(&PlayersEvent {
            event: "players",
            count,
        });
        return;
    }
    ui::say(&format!("玩家连接：{count}"));
}

#[derive(Debug, Serialize)]
struct ReconnectingEvent<'a> {
    event: &'static str,
    attempt: u32,
    wait_ms: u64,
    reason: &'a str,
}

/// 断了，准备重连。`reason` 是一句中文，说清楚为什么断。
pub fn report_reconnecting(attempt: u32, wait: std::time::Duration, reason: &str) {
    if is_json() {
        emit(&ReconnectingEvent {
            event: "reconnecting",
            attempt,
            wait_ms: wait.as_millis() as u64,
            reason,
        });
        return;
    }
    if wait.is_zero() {
        ui::say(&format!("{reason}，正在重连…"));
    } else {
        ui::say(&format!(
            "{reason}，{} 秒后重连（第 {attempt} 次）",
            seconds(wait.as_millis() as u64)
        ));
    }
}

#[derive(Debug, Serialize)]
struct StoppedEvent {
    event: &'static str,
    connections: u64,
    bytes: u64,
}

/// Ctrl-C 之后的最后一句。`connections` 是这次一共接过多少个玩家连接，`bytes` 是转发的总字节。
pub fn report_stopped(connections: u64, bytes: u64) {
    if is_json() {
        emit(&StoppedEvent {
            event: "stopped",
            connections,
            bytes,
        });
        return;
    }
    ui::say("已停止，链接现在显示离线。");
    if connections > 0 {
        ui::say(&format!(
            "这次一共接了 {connections} 个玩家连接，转发 {}。",
            ui::bytes(bytes)
        ));
    }
}

// ---------------------------------------------------------------- 写出去

/// stdout 上那一个对象，一行，写完就刷。
///
/// stderr 不带缓冲、stdout 在管道里带，不刷一次两边会错行。
fn emit<T: Serialize>(value: &T) {
    let line = serde_json::to_string(value).unwrap_or_else(|e| unprintable(&e));
    let _ = std::io::stderr().flush();
    println!("{line}");
    let _ = std::io::stdout().flush();
}

/// 同一个对象，但排版给人看——MCP 把它贴进对话，模型和人都要读。
pub fn to_json_pretty<T: Serialize>(value: &T) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|e| unprintable(&e))
}

fn unprintable(e: &serde_json::Error) -> String {
    format!(
        r#"{{"ok":false,"code":"error","message":"结果写不成 JSON：{}","elapsed_ms":0}}"#,
        e.to_string().replace('"', "'")
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use playtest_common::api::ErrorBody;

    fn server(status: u16, code: ErrorCode) -> anyhow::Error {
        client::Error::Server {
            status,
            body: ErrorBody {
                code,
                message: "服务器说的话".into(),
            },
        }
        .into()
    }

    #[test]
    fn every_code_has_its_own_exit_number_except_the_two_that_share_one() {
        for (code, exit) in [
            (Code::Unexpected, 1),
            (Code::Usage, 2),
            (Code::NotImplemented, 2),
            (Code::NeedsLogin, 3),
            (Code::Network, 4),
            (Code::ServerError, 5),
            (Code::BadInput, 6),
            (Code::QuotaExceeded, 7),
        ] {
            assert_eq!(code.exit(), exit, "{}", code.as_str());
        }
    }

    #[test]
    fn server_faults_land_on_the_right_exit_code() {
        assert_eq!(
            classify(&server(413, ErrorCode::QuotaExceeded)).code,
            Code::QuotaExceeded
        );
        assert_eq!(
            classify(&server(401, ErrorCode::TokenExpired)).code,
            Code::NeedsLogin
        );
        assert_eq!(
            classify(&server(401, ErrorCode::Unauthorized)).code,
            Code::NeedsLogin
        );
        assert_eq!(
            classify(&server(500, ErrorCode::Internal)).code,
            Code::ServerError
        );
        // 服务器拒了我们给的东西，不是服务器坏了。
        assert_eq!(
            classify(&server(400, ErrorCode::Invalid)).code,
            Code::BadInput
        );
        assert_eq!(
            classify(&server(404, ErrorCode::NotFound)).code,
            Code::BadInput
        );
    }

    #[test]
    fn an_unclassified_error_keeps_its_whole_message() {
        let e = anyhow::anyhow!("底下那层").context("上面这层");
        let failure = classify(&e);
        assert_eq!(failure.code, Code::Unexpected);
        assert_eq!(failure.message, "上面这层: 底下那层");
    }

    #[test]
    fn tagging_never_overwrites_an_error_that_was_already_sorted() {
        let already = as_bad_input(server(401, ErrorCode::TokenExpired));
        assert_eq!(classify(&already).code, Code::NeedsLogin);

        let plain = as_bad_input(anyhow::anyhow!("找不到 ./dist"));
        assert_eq!(classify(&plain).code, Code::BadInput);
        assert_eq!(classify(&plain).message, "找不到 ./dist");
    }

    #[test]
    fn failures_serialise_with_the_documented_field_names() {
        let json = serde_json::to_value(classify(&usage("命令写错了"))).unwrap();
        assert_eq!(json["ok"], false);
        assert_eq!(json["code"], "usage");
        assert_eq!(json["message"], "命令写错了");
        assert!(json.get("hint").is_none(), "没有建议时不该有这个字段");
        assert!(json["elapsed_ms"].is_u64());
    }

    #[test]
    fn an_upload_serialises_with_the_documented_field_names() {
        let report = UploadReport::new(
            "brisk-otter-41".into(),
            "https://brisk-otter-41.playtest.run".into(),
            "小球大冒险".into(),
            7,
            Timings {
                hash_ms: 300,
                upload_ms: 3100,
                commit_ms: 800,
            },
            Some("2026-09-08T04:09:03Z".into()),
            None,
            vec![Finding::warn("这个导出用到了 SharedArrayBuffer（线程）").hint("加 --isolated")],
        );
        let json = serde_json::to_value(&report).unwrap();
        assert_eq!(json["ok"], true);
        assert_eq!(json["action"], "upload");
        assert_eq!(json["slug"], "brisk-otter-41");
        assert_eq!(json["version"], 7);
        assert_eq!(json["timings"]["upload_ms"], 3100);
        assert_eq!(json["findings"][0]["level"], "warn");
        assert_eq!(json["findings"][0]["hint"], "加 --isolated");
        assert!(json.get("qr_text").is_none(), "--no-qr 时不该有这个字段");
    }

    #[test]
    fn the_timing_line_reads_like_a_sentence() {
        assert_eq!(
            timing_line(
                4200,
                Timings {
                    hash_ms: 300,
                    upload_ms: 3100,
                    commit_ms: 800
                }
            ),
            "本次 4.2 秒（哈希 0.3 · 上传 3.1 · 提交 0.8）"
        );
    }
}
