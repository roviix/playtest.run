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
//!  "elapsed_ms":5100,"timings":{"hash_ms":300,"prepare_ms":900,"upload_ms":3100,"commit_ms":800},
//!  "expires_at":"2026-09-08T04:09:03Z","qr_text":"█▀▀▀▀▀█ …",
//!  "card_url":"https://brisk-otter-41.playtest.run/_playtest/card.png",
//!  "card_path":"./小球-邀请卡.png","seats":10,
//!  "plaza_url":"https://playtest.run/",
//!  "console_url":"https://playtest.roviix.com/console/#/s/brisk-otter-41",
//!  "findings":[{"level":"warn","message":"这个导出用到了 SharedArrayBuffer（线程）","hint":"加 --isolated"}]}
//! {"ok":true,"action":"list","elapsed_ms":120,
//!  "sites":[{"slug":"brisk-otter-41","url":"…","title":"小球","version":7,"followers":12,"expires_at":"…"}]}
//! {"ok":true,"action":"remove","slug":"brisk-otter-41","elapsed_ms":90}
//! {"ok":true,"action":"open","slug":"brisk-otter-41","url":"…","opened":false,"elapsed_ms":80}
//! {"ok":true,"action":"card","slug":"brisk-otter-41","title":"小球","card_url":"…","card_path":"./小球-邀请卡.png","elapsed_ms":700}
//! {"ok":true,"action":"followers","slug":"brisk-otter-41","title":"小球","followers":12,"elapsed_ms":80}
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
//! - `card_url` 总是有——那张卡由边缘按当前版本渲染，地址不随时间变；`card_path` 只有真的
//!   存下来了才有（加了 `--no-card`、或者这一刻边缘还没渲染好，就没有）。
//! - `console_url` 是开发者自己看结果的地方，玩家路径上不出现它（AGENTS 第 7 条）。
//! - `expires_at`、`hint`、`qr_text`、`card_path`、`seats`、`plaza_url` 可能不出现
//!   （没有就是没有），其余字段一定在。
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

use crate::client;
use crate::session::Session;
use crate::{card, clock, ui};

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

/// 一次上传里各段花了多久。四段加起来接近 `elapsed_ms`，差的是检查目录、画二维码这些本地的零碎。
#[derive(Debug, Default, Clone, Copy, Serialize)]
pub struct Timings {
    /// 走一遍目录、读文件、算哈希。
    pub hash_ms: u64,
    /// 和控制面的几个往返：拿令牌、认领作品、问「缺哪些文件」。慢多半慢在这里——每一步都是一次跨海的往返。
    pub prepare_ms: u64,
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
/// | 2 | 命令写错了 |
/// | 3 | 需要登录，或者身份失效了 |
/// | 4 | 网络不通 |
/// | 5 | 服务端出错 |
/// | 6 | 给的东西有问题（目录不在、超限、不像导出物） |
/// | 7 | 配额用完了 |
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Code {
    /// 命令写错了。
    Usage,
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
            Code::Usage => 2,
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

/// 给的东西有问题：目录不在、超限、不像导出物。
pub fn bad_input(message: impl Into<String>) -> anyhow::Error {
    fail(Code::BadInput, message, None)
}

/// 同上，但带一句「该怎么办」。
pub fn bad_input_with_hint(message: impl Into<String>, hint: impl Into<String>) -> anyhow::Error {
    fail(Code::BadInput, message, Some(hint.into()))
}

/// 连不上对面。控制面那边由 [`client::Error`] 归类，这条给别的地址用（比如拿邀请卡）。
pub fn network(message: impl Into<String>, hint: impl Into<String>) -> anyhow::Error {
    fail(Code::Network, message, Some(hint.into()))
}

/// 连上了，但对面这一刻给不了要的东西。
pub fn server_error(message: impl Into<String>, hint: impl Into<String>) -> anyhow::Error {
    fail(Code::ServerError, message, Some(hint.into()))
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
            ErrorCode::QuotaExceeded => (Code::QuotaExceeded, None),
            ErrorCode::Unauthorized | ErrorCode::TokenExpired => (
                Code::NeedsLogin,
                Some("匿名身份只保留 24 小时；再跑一次会自动换一个新的，链接也会是新的。想让作品留下来，playtest login。".into()),
            ),
            ErrorCode::LoginUnavailable | ErrorCode::LoginFailed => (Code::NeedsLogin, None),
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
    /// 那张邀请卡在哪儿（DESIGN §3.4）。边缘按当前版本渲染，所以这个地址一直有效。
    pub card_url: String,
    /// 存到本地哪儿了。没存成（`--no-card`，或者这一刻还拿不到）就没有这一项。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub card_path: Option<String>,
    /// 想找几位试玩者（`--seats`）。没说就没有这一项。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seats: Option<u32>,
    /// 广场的地址。没放到广场上就没有这一项。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plaza_url: Option<String>,
    /// 开发者自己看结果的地方。
    pub console_url: String,
    pub findings: Vec<Finding>,
    /// 这次上传之后作品在广场上的状态（DESIGN §3.8）。没动过广场就没有这一段。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plaza: Option<PlazaOut>,
    /// 卡的下场。人类模式靠它决定说哪一句；JSON 里看 `card_path` 有没有就够了。
    #[serde(skip)]
    pub card: card::Outcome,
    /// 卡的字节。只有 MCP 用得上（贴一张图回对话），不进 JSON——几百 KB 的 base64
    /// 塞进一行 stdout，对着管道读的脚本会很难受。
    #[serde(skip)]
    pub card_png: Option<Vec<u8>>,
    /// 这一版没有封面。JSON 里它是 `findings` 里的一条，人类模式排在耗时前面那一行。
    #[serde(skip)]
    pub without_cover: bool,
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
            card_url: playtest_common::card_url(&url),
            console_url: console_url(&slug),
            slug,
            url,
            title,
            version,
            elapsed_ms: elapsed_ms(),
            timings,
            expires_at,
            qr_text,
            card_path: None,
            seats: None,
            plaza_url: None,
            findings,
            plaza: None,
            card: card::Outcome::default(),
            card_png: None,
            without_cover: false,
        }
    }

    /// 广场那一段。`plaza_url` 是 DESIGN §3.2 点名的字段，`plaza` 是同一件事更细的一份：
    /// 只要地址的脚本读前者，要知道「标了求测没有」的 agent 读后者。
    pub fn on_plaza(&mut self, plaza: PlazaOut) {
        self.plaza_url = Some(plaza.url.clone());
        self.plaza = Some(plaza);
    }

    /// 那张邀请卡的下场。
    pub fn with_card(&mut self, taken: card::Taken) {
        self.card = taken.outcome;
        self.card_path = taken.path;
        self.card_png = taken.png;
    }

    /// 把卡的字节拿走（MCP 要贴进对话）。拿走而不是借用：几百 KB 的图只该有一份。
    pub fn take_card_png(&mut self) -> Option<Vec<u8>> {
        self.card_png.take()
    }
}

/// 开发者看结果的那一页（DESIGN §3.2）。它在开发者那一侧的域名上——玩家路径上永远不出现
/// 这个地址（AGENTS 第 7 条），但打给开发者自己看是对的。
pub fn console_url(slug: &str) -> String {
    format!("{}/console/#/s/{slug}", playtest_common::DEVELOPER_API_URL)
}

/// 上传做成了：JSON 模式给一个对象，人类模式给 DESIGN §3.2 列的那几行。
///
/// 行的顺序就是一个第一次用的人该按什么次序做下一件事：先拿到链接，再拿到能发出去的东西
/// （二维码、邀请卡），再知道这东西被放到了哪儿、能活多久，最后才是「回头去哪看结果」。
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
    for (tone, line) in after_the_link(report) {
        say_toned(tone, &line);
    }
}

/// 一行是平铺直叙，还是要人留意一下。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tone {
    Plain,
    Warn,
}

fn say_toned(tone: Tone, line: &str) {
    match tone {
        Tone::Plain => ui::say(line),
        Tone::Warn => ui::warn(line),
    }
}

/// 链接和二维码之后的那几行，按 DESIGN §3.2 的顺序。
///
/// 拎成一个纯函数是因为**顺序本身是产品的一部分**：先给能发出去的东西（卡），再说它被放到了
/// 哪儿、能活多久，最后才是回头看结果的地方和耗时。散在一串 ui 调用里的顺序没人测得了。
fn after_the_link(report: &UploadReport) -> Vec<(Tone, String)> {
    let mut lines = Vec::new();
    if let Some(line) = card_line(
        report.card,
        report.card_path.as_deref(),
        &report.card_url,
        &report.slug,
    ) {
        lines.push((Tone::Plain, line));
    }
    if let Some(plaza) = &report.plaza {
        lines.push((Tone::Plain, plaza_line(plaza)));
    }
    if let Some(seats) = report.seats {
        lines.push((Tone::Plain, seats_line(seats)));
    }
    if let Some(expires_at) = &report.expires_at {
        lines.push(expiry_line(expires_at, "想让它留下来：playtest login。"));
    }
    lines.push((Tone::Plain, console_line(&report.slug)));
    if report.without_cover {
        lines.push((Tone::Plain, NO_COVER.to_string()));
    }
    lines.push((Tone::Plain, timing_line(report.elapsed_ms, report.timings)));
    lines
}

/// 邀请卡那一行。`--no-card` 时一个字都不说——他说了不要。
fn card_line(outcome: card::Outcome, path: Option<&str>, url: &str, slug: &str) -> Option<String> {
    match (outcome, path) {
        (card::Outcome::Saved, Some(path)) => {
            Some(format!("邀请卡已存到 {path}——发到群里，别人长按识别就能玩"))
        }
        // 存下来了却没有路径，是不可能的；真出现了也当没拿到说，别打一句半截的话。
        (card::Outcome::Saved, None) | (card::Outcome::Missing, _) => Some(format!(
            "邀请卡稍后可以在 {url} 拿到，或 playtest card {slug}"
        )),
        (card::Outcome::Skipped, _) => None,
    }
}

/// 没有封面时那一句（DESIGN §4.2 的上传时检查，只提醒、不拦）。广场好不好看八成取决于封面，
/// 但我们不猜、不截图（DESIGN §3.12「从不运行用户代码」），只说一句。
pub const NO_COVER: &str = "没有封面：加 --cover 一张图，广场和邀请卡都会好看很多";

/// 「来的人玩成什么样」那一行。发完不说这句，「知道结果」这半个产品就没人知道在哪
/// （`docs/spikes/2026-09-08-dogfood-mofish-airdrop.md` 第四节第 4 条）。
fn console_line(slug: &str) -> String {
    format!("来的人玩成什么样，控制台里看得见：{}", console_url(slug))
}

/// 名额那一行（DESIGN §3.3 第 4 条）。「加入」的定义要说出来，不然发的人会以为是打开的人数。
fn seats_line(seats: u32) -> String {
    format!("想找 {seats} 位试玩者，门禁页和邀请卡上都写着；留了名字的人算加入")
}

/// 匿名链接还剩不到这么久就要提醒：一场测试从发链接到大家点开常常要一两个小时，
/// 中途失效比一开始就是新链接糟得多。
const EXPIRY_WARNING: time::Duration = time::Duration::hours(2);

/// 匿名链接的有效期那一句。快到期了就换成提醒，并说清到期之后会发生什么——
/// 再跑一次拿到的是新链接，玩家手里的旧链接打不开。
fn expiry_line(expires_at: &str, about_login: &str) -> (Tone, String) {
    match clock::remaining(expires_at, clock::now()) {
        Some(left) if left < EXPIRY_WARNING => (
            Tone::Warn,
            format!(
                "这条匿名链接只剩 {} 就失效（{}）。到期后再跑一次会拿到一条新链接，发出去的旧链接会打不开。",
                clock::human_duration(left),
                clock::human(expires_at)
            ),
        ),
        _ => (
            Tone::Plain,
            format!(
                "这是匿名链接，{} 后失效。{about_login}",
                clock::human(expires_at)
            ),
        ),
    }
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
/// DESIGN §8 要的那个数就是句首那个：从敲下命令到链接出现。分段是为了知道慢在哪一段；
/// 整体不到 2 秒时没有「慢在哪」可问，四个 0.0 只是噪声，不打。
fn timing_line(elapsed_ms: u64, timings: Timings) -> String {
    if elapsed_ms < 2000 {
        return format!("本次 {} 秒", seconds(elapsed_ms));
    }
    format!(
        "本次 {} 秒（哈希 {} · 准备 {} · 上传 {} · 提交 {}）",
        seconds(elapsed_ms),
        seconds(timings.hash_ms),
        seconds(timings.prepare_ms),
        seconds(timings.upload_ms),
        seconds(timings.commit_ms)
    )
}

fn seconds(ms: u64) -> String {
    format!("{:.1}", ms as f64 / 1000.0)
}

// ---------------------------------------------------------------- ls / rm / open

/// 一个作品在 JSON 里的样子。字段名和 `common` 里的 [`Site`] 对齐，只把
/// `current_version` 缩成 `version`——脚本里不需要「当前」这个限定。
#[derive(Debug, Serialize)]
pub struct SiteOut {
    pub slug: String,
    pub url: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<u32>,
    /// 有多少人关注着它（DESIGN §3.6）。开发者只看到这个数字，看不到是谁——
    /// 邮箱由我们保管、我们代发，这不是限制，是开发者不用自己扛的那份责任。
    pub followers: u32,
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
            followers: site.listing.followers,
            expires_at: site.expires_at,
        }
    }
}

#[derive(Debug, Serialize)]
struct HelpReport {
    ok: bool,
    action: &'static str,
    text: String,
    elapsed_ms: u64,
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
/// MCP 那一侧用它（`playtest_list`）。
pub async fn fetch_sites(api_flag: Option<&str>) -> Result<Vec<SiteOut>> {
    let session = Session::open(api_flag)?;
    let Some(client) = session.client()? else {
        return Ok(Vec::new());
    };
    Ok(client
        .list_sites()
        .await?
        .into_iter()
        .map(SiteOut::from)
        .collect())
}

/// 一个作品现在什么样。MCP 那一侧用它（`playtest_get`）。
pub async fn fetch_site(slug: &str, api_flag: Option<&str>) -> Result<SiteOut> {
    let session = Session::open(api_flag)?;
    let client = session.client_or_say("先发一个")?;
    Ok(client.get_site(slug).await?.into())
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
///
/// 混合模式（`playtest ./dist --backend 3000`）先按上传的形状写一个 `{"ok":true,"action":"upload",…}`
/// 对象，然后才是这一行一个的事件；`online` 里多一段 `backend`，说清后端接在哪个端口。
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
    /// 隧道作品一样有门禁页、一样有邀请卡（DESIGN §3.4）。
    pub card_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub card_path: Option<String>,
    pub console_url: String,
    /// 这次启动看出来的事：Vite 的热更新提示、页面太大该改用上传，等等。
    pub findings: Vec<Finding>,
    /// 混合模式里接在隧道上的后端。整作品隧道没有这一段。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backend: Option<BackendOut>,
    #[serde(skip)]
    pub card: card::Outcome,
}

/// 混合模式的后端在输出里的样子：上传目录里没有的路径都走到这个本地端口。
#[derive(Debug, Clone, Serialize)]
pub struct BackendOut {
    pub port: u16,
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
            card_url: playtest_common::card_url(&url),
            console_url: console_url(&slug),
            slug,
            url,
            expires_at,
            attempt: 0,
            elapsed_ms: elapsed_ms(),
            qr_text,
            card_path: None,
            findings,
            backend: None,
            card: card::Outcome::default(),
        }
    }

    pub fn with_card(&mut self, taken: card::Taken) {
        self.card = taken.outcome;
        self.card_path = taken.path;
    }
}

/// 隧道连上了。第一次连上给全套（链接、二维码、有效期、提示、耗时），
/// 重连回来只说一声——链接没变，把整块再刷一遍只会让人以为换了链接。
pub fn report_online(report: &OnlineReport) {
    if is_json() {
        emit(report);
        return;
    }
    if let Some(backend) = &report.backend {
        // 链接、二维码、有效期上传那一步刚说过，这里只说后端接上了没有。
        if report.attempt > 0 {
            ui::say("后端已重连。");
            return;
        }
        ui::say(&format!(
            "后端已接上：目录里有的文件玩家直接从边缘拿，目录里没有的路径（比如 /api/…）都走到你电脑的 {} 端口。",
            backend.port
        ));
        ui::say("按 Ctrl-C 结束；结束后页面照常能开，只是那些路径会回「后端不在线」（503）。");
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
    if let Some(line) = card_line(
        report.card,
        report.card_path.as_deref(),
        &report.card_url,
        &report.slug,
    ) {
        ui::say(&line);
    }
    if let Some(expires_at) = &report.expires_at {
        let (tone, line) = expiry_line(expires_at, "想让它留下来：playtest login。");
        say_toned(tone, &line);
    }
    ui::say(&console_line(&report.slug));
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
/// 混合模式（`hybrid`）下链接并没有离线——页面还是上传的那一版，只有后端接不上了。
pub fn report_stopped(connections: u64, bytes: u64, hybrid: bool) {
    if is_json() {
        emit(&StoppedEvent {
            event: "stopped",
            connections,
            bytes,
        });
        return;
    }
    if hybrid {
        ui::say("已停止。页面照常能开，目录里没有的路径现在回「后端不在线」。");
    } else {
        ui::say("已停止，链接现在显示离线。");
    }
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
/// `playtest login --json` 的那一个对象。
pub fn emit_login<T: Serialize>(value: &T) {
    emit(value);
}

/// 一条命令做完了，说出来。**唯一**的渲染入口（REWRITE §3.1）。
///
/// 人话走 [`Report::human`]（叙述到 stderr、要拿走的东西到 stdout），`--json` 走
/// [`Report::json`] 再盖上 `ok` / `action` / `elapsed_ms`。两条路只可能同时存在。
pub fn say(report: &crate::report::Report) {
    if is_json() {
        emit(&crate::report::Envelope {
            ok: true,
            action: report.action(),
            body: report.json(),
            elapsed_ms: elapsed_ms(),
        });
    } else {
        report.human();
    }
}

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
    fn every_code_has_its_own_exit_number() {
        for (code, exit) in [
            (Code::Unexpected, 1),
            (Code::Usage, 2),
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
                prepare_ms: 900,
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

    /// 一个刚发完的作品，用来试各种输出。
    fn a_report() -> UploadReport {
        UploadReport::new(
            "brisk-otter-41".into(),
            "https://brisk-otter-41.playtest.run".into(),
            "小球大冒险".into(),
            7,
            Timings::default(),
            None,
            None,
            Vec::new(),
        )
    }

    #[test]
    fn the_card_and_the_console_are_fields_of_their_own() {
        let mut report = a_report();
        let json = serde_json::to_value(&report).unwrap();
        // 卡的地址一直有效（边缘按当前版本渲染），所以它总在。
        assert_eq!(
            json["card_url"],
            "https://brisk-otter-41.playtest.run/_playtest/card.png"
        );
        assert_eq!(
            json["console_url"],
            format!(
                "{}/console/#/s/brisk-otter-41",
                playtest_common::DEVELOPER_API_URL
            )
        );
        assert!(json.get("card_path").is_none(), "还没存下来就不该有路径");
        assert!(json.get("seats").is_none(), "没说要找几位就不该有这一项");
        assert!(json.get("plaza_url").is_none(), "没上广场就不该有这一项");

        report.with_card(card::Taken {
            outcome: card::Outcome::Saved,
            path: Some("./小球大冒险-邀请卡.png".into()),
            png: Some(vec![1, 2, 3]),
        });
        report.seats = Some(10);
        report.on_plaza(PlazaOut {
            url: "https://playtest.run/".into(),
            public: true,
            seeking: true,
            seek_note: None,
        });
        let json = serde_json::to_value(&report).unwrap();
        assert_eq!(json["card_path"], "./小球大冒险-邀请卡.png");
        assert_eq!(json["seats"], 10);
        assert_eq!(json["plaza_url"], "https://playtest.run/");
        assert_eq!(json["plaza"]["seeking"], true);
        // 卡的字节只给 MCP 贴图用，几百 KB 的 base64 不该出现在 stdout 的那一行里。
        assert!(json.get("card_png").is_none());
        assert_eq!(report.take_card_png(), Some(vec![1, 2, 3]));
    }

    #[test]
    fn a_listed_work_says_how_many_people_follow_it() {
        let site = SiteOut::from(Site {
            slug: "brisk-otter-41".into(),
            url: "https://brisk-otter-41.playtest.run".into(),
            title: "小球大冒险".into(),
            current_version: Some(7),
            created_at: "2026-09-09T00:00:00Z".into(),
            expires_at: None,
            listing: playtest_common::api::Listing {
                followers: 12,
                ..Default::default()
            },
        });
        let json = serde_json::to_value(&site).unwrap();
        assert_eq!(json["followers"], 12);
        assert_eq!(json["version"], 7);
        // 一个人都没有也要有这个字段：脚本不该为「没有关注者」写一个分支。
        let none = serde_json::to_value(SiteOut::from(Site {
            listing: Default::default(),
            ..a_site()
        }))
        .unwrap();
        assert_eq!(none["followers"], 0);
    }

    fn a_site() -> Site {
        Site {
            slug: "brisk-otter-41".into(),
            url: "https://brisk-otter-41.playtest.run".into(),
            title: "小球大冒险".into(),
            current_version: Some(7),
            created_at: "2026-09-09T00:00:00Z".into(),
            expires_at: None,
            listing: Default::default(),
        }
    }

    /// 行序就是「接下来做什么」的次序，改动它要有理由（DESIGN §3.2）。
    #[test]
    fn the_lines_after_the_link_come_in_the_order_a_first_timer_needs_them() {
        let mut report = a_report();
        report.expires_at = Some("2126-09-10T20:59:00Z".into());
        report.with_card(card::Taken {
            outcome: card::Outcome::Saved,
            path: Some("./小球大冒险-邀请卡.png".into()),
            png: None,
        });
        report.on_plaza(PlazaOut {
            url: "https://playtest.run/".into(),
            public: true,
            seeking: true,
            seek_note: Some("新手引导看得懂吗".into()),
        });
        report.seats = Some(10);
        report.without_cover = true;

        let lines: Vec<String> = after_the_link(&report)
            .into_iter()
            .map(|(_, line)| line)
            .collect();
        let heads = [
            "邀请卡已存到",
            "已放到广场上",
            "想找 10 位",
            "这是匿名链接",
            "来的人玩成什么样",
            "没有封面",
            "本次",
        ];
        assert_eq!(lines.len(), heads.len(), "{lines:#?}");
        for (line, head) in lines.iter().zip(heads) {
            assert!(line.starts_with(head), "「{line}」该以「{head}」开头");
        }
        assert!(
            lines[4].contains("/console/#/s/brisk-otter-41"),
            "{}",
            lines[4]
        );
    }

    #[test]
    fn nothing_is_said_about_a_card_the_user_asked_us_not_to_save() {
        let report = a_report();
        assert_eq!(report.card, card::Outcome::Skipped);
        let lines = after_the_link(&report);
        assert!(
            !lines.iter().any(|(_, line)| line.contains("邀请卡")),
            "{lines:#?}"
        );
        // 拿不到的时候要说去哪儿拿，不能装作没有这回事。
        let mut missing = a_report();
        missing.with_card(card::Taken {
            outcome: card::Outcome::Missing,
            ..Default::default()
        });
        let said = &after_the_link(&missing)[0].1;
        assert!(said.contains("/_playtest/card.png"), "{said}");
        assert!(said.contains("playtest card brisk-otter-41"), "{said}");
    }

    #[test]
    fn the_timing_line_reads_like_a_sentence() {
        assert_eq!(
            timing_line(
                5100,
                Timings {
                    hash_ms: 300,
                    prepare_ms: 900,
                    upload_ms: 3100,
                    commit_ms: 800
                }
            ),
            "本次 5.1 秒（哈希 0.3 · 准备 0.9 · 上传 3.1 · 提交 0.8）"
        );
        assert_eq!(
            timing_line(
                240,
                Timings {
                    hash_ms: 20,
                    prepare_ms: 60,
                    upload_ms: 90,
                    commit_ms: 40
                }
            ),
            "本次 0.2 秒",
            "一下就完的上传不用拆段"
        );
    }
}
