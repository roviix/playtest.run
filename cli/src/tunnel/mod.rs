//! `playtest 5173`：把本地端口经一条出站 WebSocket 接到边缘，拿到链接和二维码（DESIGN §4.3）。
//!
//! 一次运行的形状：
//!
//! ```text
//! 看一眼本地端口 → 拿匿名令牌、认领作品 → 要一张隧道授权 → 连边缘
//!   → 边缘每来一条流就连一次 127.0.0.1:<port>，双向拷贝
//!   → 断了按 1 s → 30 s 退避重连，链接不变
//!   → Ctrl-C：关掉连接，玩家那边变成「开发者的电脑暂时不在线」
//! ```
//!
//! 和上传路径共用同一套约定：同一个目录记住同一个作品，`--site` / `--new` 一样管用，
//! 令牌存在同一个配置文件里。不一样的是这条命令**要一直跑着**——所以 stdout 上是一行一个
//! 事件而不是一个对象（见 [`crate::output::OnlineReport`]）。
//!
//! 退出码：连不上本地端口、被别的进程挤掉、作品被下架，这些都是 1（没预料到的结束）；
//! 控制面那边的错沿用上传那套分层（令牌失效 3、网络 4、服务端 5……）。

mod backoff;
mod probe;
mod session;

use std::future::Future;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use playtest_common::tunnel::{TunnelGrant, TunnelRequest};
use tokio::sync::watch;

use crate::args::{self, UploadArgs};
use crate::client::Client;
use crate::config::{self, Config};
use crate::output::{self, BackendOut, Finding, OnlineReport};
use crate::upload::{self, SlugSource};
use crate::{clock, ui};

use backoff::Backoff;
use session::{Ended, Next, Tally};

/// 令牌剩不到这么久就在下一次重连之前换新。正常在线时不为换令牌断开连接
/// ——边缘对已经建立的连接不看令牌有效期（`common/src/tunnel.rs`）。
const TOKEN_RENEW_MARGIN: Duration = Duration::from_secs(5 * 60);

/// 连上之后最多再等这么久，等那个「这一页有多大」的估算。
///
/// 估算和握手是并行跑的，多数时候早就算完了；算不完就不说，链接不能为一句提示等着。
const WEIGH_GRACE: Duration = Duration::from_millis(1500);

/// 连着 401 这么多次还是不行，就当成网络问题按退避重连，别在「换令牌」上打转。
const MAX_TOKEN_SWAPS: u32 = 2;

/// 混合模式（`playtest ./dist --backend 3000`）里挂到已上传作品上的那个后端。
///
/// 作品由上传那一步定下来，隧道只是挂上去：slug 不再按当前目录去记忆里找，
/// 也不会在服务器不认时悄悄换一个——换了静态文件和后端就各在一个作品上了。
pub struct Backend {
    pub slug: String,
    pub title: String,
    pub port: u16,
}

/// `playtest 5173`：整个作品都走隧道。
pub async fn run(cli_args: &UploadArgs, port: u16) -> Result<()> {
    run_with(cli_args, port, None).await
}

/// `playtest ./dist --backend 3000`：目录照常上传，再把目录里没有的路径接到本地端口。
///
/// 上传的清单就是分界线（DESIGN §4.3）：目录里有的文件玩家从边缘拿，不经过开发者的家宽；
/// 目录里没有的路径（`/api/x`、WebSocket……）才穿隧道到后端。没有路径约定，也不用配置。
/// Ctrl-C 之后页面照常能开，只是那些路径回「后端不在线」。先看端口再上传——
/// 传完才发现后端没起，链接已经占了名额。
pub async fn run_hybrid(cli_args: &UploadArgs, shown: &str, port: u16) -> Result<()> {
    if !probe::is_listening(port).await {
        return Err(output::bad_input(format!(
            "端口 {port} 上没有东西在监听。先把后端跑起来，再运行 playtest {shown} --backend {port}。"
        )));
    }
    let report = upload::run(cli_args, shown).await?;
    output::report_upload(&report);
    run_with(
        cli_args,
        port,
        Some(Backend {
            slug: report.slug,
            title: report.title,
            port,
        }),
    )
    .await
}

async fn run_with(cli_args: &UploadArgs, port: u16, backend: Option<Backend>) -> Result<()> {
    let stop = Stop::install();
    let hybrid = backend.is_some();
    if !hybrid {
        say_ignored(cli_args);
    }

    // 1. 本地端口上得先有东西在听。没有的话拿到链接的人只会看到一片 502。
    if !probe::is_listening(port).await {
        return Err(anyhow!(
            "端口 {port} 上没有东西在监听。先把你的开发服务器跑起来，再运行 playtest {port}。"
        ));
    }
    // 只请求这一次首页，之后再不主动碰本地端口。
    // 混合模式下这个端口是后端不是一页 HTML：不看首页、不称重，一次也不碰。
    let (page, weighing) = if hybrid {
        (probe::Page::default(), None)
    } else {
        let page = probe::look(port).await;
        let weighing = tokio::spawn({
            let page = page.clone();
            async move { probe::weigh(port, &page).await }
        });
        (page, Some(weighing))
    };

    // 2. 令牌与作品，和上传共用。
    let mut control = Control::start(cli_args, backend).await?;
    let Some(granted) = stop.race(control.first_grant()).await else {
        output::report_stopped(0, 0, hybrid);
        return Ok(());
    };
    let mut grant = granted?;

    // 3. 连上、跑、断了再连。
    let tally = Arc::new(Tally::new());
    let mut backoff = Backoff::new();
    let mut connected = 0u32;
    let mut retries = 0u32;
    let mut token_swaps = 0u32;
    let mut weighing = weighing;

    let ending = loop {
        if expiring_soon(&grant) {
            // 在线时不为换令牌断开，只在下一次连接之前换。
            grant = control.regrant().await?;
        }

        let next = match stop.race(session::open(&grant, port)).await {
            None => break Ok(()),
            Some(Ok(wire)) => {
                backoff.reset();
                retries = 0;
                token_swaps = 0;
                let attempt = connected;
                connected += 1;
                announce(
                    &grant,
                    attempt,
                    cli_args,
                    &page,
                    weighing.take(),
                    control.backend_out(),
                )
                .await;

                match session::serve(wire, port, &stop, Arc::clone(&tally)).await {
                    Ended::ByUser => break Ok(()),
                    Ended::Closed(next) => next,
                }
            }
            Some(Err(next)) => next,
        };

        match next {
            Next::Stop(said) => break Err(anyhow!(said)),
            Next::NewGrant(said) => {
                token_swaps += 1;
                retries += 1;
                if token_swaps > MAX_TOKEN_SWAPS {
                    // 一直说令牌不对，多半不是令牌的事。当普通断线处理，别空转。
                    let wait = backoff.next_wait();
                    output::report_reconnecting(retries, wait, "连不上边缘");
                    if stop.race(tokio::time::sleep(wait)).await.is_none() {
                        break Ok(());
                    }
                    continue;
                }
                output::report_reconnecting(retries, Duration::ZERO, &said);
                grant = control.regrant().await?;
            }
            Next::Retry(said) => {
                retries += 1;
                let wait = backoff.next_wait();
                output::report_reconnecting(retries, wait, &said);
                if stop.race(tokio::time::sleep(wait)).await.is_none() {
                    break Ok(());
                }
            }
        }
    };

    match ending {
        Ok(()) => {
            output::report_stopped(tally.total(), tally.bytes(), hybrid);
            Ok(())
        }
        Err(e) => Err(e),
    }
}

/// 第一次连上时把该说的都说了；重连回来只说一声。
///
/// 混合模式下链接和二维码上传那一步已经打过了，这里不再画一遍——同一条链接出现两次，
/// 看的人会以为换了链接。
async fn announce(
    grant: &TunnelGrant,
    attempt: u32,
    cli_args: &UploadArgs,
    page: &probe::Page,
    weighing: Option<tokio::task::JoinHandle<Option<u64>>>,
    backend: Option<BackendOut>,
) {
    let mut report = OnlineReport::new(
        grant.slug.clone(),
        grant.url.clone(),
        grant.site_expires_at.clone(),
        None,
        Vec::new(),
    );
    report.attempt = attempt;
    let hybrid = backend.is_some();
    report.backend = backend;
    if attempt == 0 && !hybrid {
        if !cli_args.no_qr {
            report.qr_text = ui::qr_text(&grant.url);
        }
        report.findings = findings_for(page, weighing).await;
    }
    output::report_online(&report);
}

/// 这次启动看出来的事。算不出来的就不说，不编数字（AGENTS.md 第 4 条）。
async fn findings_for(
    page: &probe::Page,
    weighing: Option<tokio::task::JoinHandle<Option<u64>>>,
) -> Vec<Finding> {
    let mut findings = Vec::new();
    findings.extend(page.vite_hint());
    findings.extend(page.exposure_hint());
    let Some(weighing) = weighing else {
        return findings;
    };
    // 没量完、量不准、任务出错：都当作不知道。
    if let Ok(Ok(Some(total))) = tokio::time::timeout(WEIGH_GRACE, weighing).await {
        findings.extend(probe::heavy_hint(total));
    }
    findings
}

/// 用不上的参数说一句就过去，不报错——脚本里带着一串通用参数调过来是常事。
fn say_ignored(cli_args: &UploadArgs) {
    let mut ignored: Vec<&str> = Vec::new();
    if cli_args.note.is_some() {
        ignored.push("--note");
    }
    if cli_args.spa {
        ignored.push("--spa");
    }
    if cli_args.no_isolated {
        ignored.push("--no-isolated");
    }
    if ignored.is_empty() {
        return;
    }
    ui::say(&format!(
        "{} 只在上传目录时有用，这次是隧道模式，先忽略了。",
        ignored.join("、")
    ));
}

/// 令牌快到期了吗。看不懂服务器给的时间就当没到期——宁可让边缘拒一次，也不要没事就换。
fn expiring_soon(grant: &TunnelGrant) -> bool {
    let Some(expires) = clock::parse_rfc3339(&grant.expires_at) else {
        return false;
    };
    let left = expires - clock::now();
    left <= time::Duration::seconds(TOKEN_RENEW_MARGIN.as_secs() as i64)
}

/// 和控制面打交道的那一半：令牌、作品、授权。
struct Control<'a> {
    args: &'a UploadArgs,
    client: Client,
    config: Config,
    config_path: PathBuf,
    api: String,
    dir_key: String,
    title: String,
    slug: String,
    source: SlugSource,
    /// 混合模式的后端；整作品隧道没有。
    backend: Option<Backend>,
}

impl<'a> Control<'a> {
    async fn start(args: &'a UploadArgs, backend: Option<Backend>) -> Result<Control<'a>> {
        let here = std::env::current_dir().context("看不了当前目录")?;
        let api = args::api_base(args.api.as_deref());
        let config_path = config::default_path()?;
        let mut config = config::load(&config_path)?;
        let mut client = Client::new(&api)?;
        upload::ensure_token(&mut client, &mut config, &config_path, &api).await?;
        let dir_key = dir_key(&here);

        // 混合模式：作品就是刚传完的那个，不查记忆也不认领新的。上传刚用过这个令牌，
        // 所以 ensure_token 拿到的一定是同一个身份。
        let (title, slug, source) = match &backend {
            Some(b) => (b.title.clone(), b.slug.clone(), SlugSource::Explicit),
            None => {
                let title = upload::title_for(args, &here, None).map_err(output::as_bad_input)?;
                let (slug, source) = upload::choose_site(
                    &mut client,
                    args,
                    &mut config,
                    &config_path,
                    &api,
                    &dir_key,
                    &title,
                )
                .await?;
                (title, slug, source)
            }
        };

        Ok(Control {
            args,
            client,
            config,
            config_path,
            api,
            dir_key,
            title,
            slug,
            source,
            backend,
        })
    }

    fn request(&self) -> TunnelRequest {
        TunnelRequest {
            title: Some(self.title.clone()),
            gate: self.args.gate,
            isolated: self.args.isolated,
            hybrid: self.backend.is_some(),
        }
    }

    /// 输出里「后端接在哪」那一段。
    fn backend_out(&self) -> Option<BackendOut> {
        self.backend.as_ref().map(|b| BackendOut { port: b.port })
    }

    /// 还没把链接给出去之前要的第一张授权。
    ///
    /// 记住的那个作品服务器不认了就换一个，恢复方式和上传完全一致：404 只换作品，
    /// 401 才换身份——反过来的话，之前发过的东西会跟着旧身份一起丢。
    async fn first_grant(&mut self) -> Result<TunnelGrant> {
        let request = self.request();
        let recoverable = self.source == SlugSource::Remembered;
        // 先把结果落地再 match：不然那个借着 self.client 的 future 会活到 match 结束，
        // 恢复分支里就没法再拿 &mut self 了。
        let asked = self.client.tunnel_grant(&self.slug, &request).await;
        match asked {
            Ok(grant) => Ok(grant),
            Err(e) if recoverable && e.means_anonymous_link_gone() => {
                if e.means_token_gone() {
                    ui::say("上次的匿名身份已经失效（匿名身份只保留 24 小时），这是一个新链接。");
                    self.config.token = None;
                    upload::ensure_token(
                        &mut self.client,
                        &mut self.config,
                        &self.config_path,
                        &self.api,
                    )
                    .await?;
                } else {
                    ui::say("上次的作品已经不在了（匿名作品只保留 24 小时），这是一个新链接。");
                }
                self.claim_new_site().await?;
                Ok(self.client.tunnel_grant(&self.slug, &request).await?)
            }
            Err(e) => Err(e.into()),
        }
    }

    /// 跑起来之后再要一张（令牌快到期，或者边缘说令牌过期了）。
    ///
    /// 这时候**不换作品也不换身份**：链接已经发给别人了，悄悄换一条等于把那条弄坏。
    /// 真的续不下去就停下来说清楚。
    async fn regrant(&mut self) -> Result<TunnelGrant> {
        let request = self.request();
        self.client
            .tunnel_grant(&self.slug, &request)
            .await
            .map_err(|e| {
                let sentence = if e.means_token_gone() {
                    "这个匿名身份过期了（匿名身份只保留 24 小时），这条链接续不下去。\
                     重新跑一次 playtest 会给你一条新链接"
                        .to_string()
                } else {
                    "换新令牌失败，隧道停在这里".to_string()
                };
                anyhow::Error::from(e).context(sentence)
            })
    }

    /// 忘掉记住的那个作品，认领一个新的。
    async fn claim_new_site(&mut self) -> Result<()> {
        self.config.sites.remove(&self.dir_key);
        let (slug, source) = upload::choose_site(
            &mut self.client,
            self.args,
            &mut self.config,
            &self.config_path,
            &self.api,
            &self.dir_key,
            &self.title,
        )
        .await?;
        self.slug = slug;
        self.source = source;
        Ok(())
    }
}

/// 配置里按目录记 slug，键是规范化之后的绝对路径，和上传那边一样。
fn dir_key(here: &Path) -> String {
    std::fs::canonicalize(here)
        .unwrap_or_else(|_| here.to_path_buf())
        .to_string_lossy()
        .into_owned()
}

/// Ctrl-C。
///
/// 进程一开始就装上，不然启动那几步里按 Ctrl-C 会走系统默认动作（直接杀掉），
/// 边缘那边要等空闲超时才知道我们走了。
pub struct Stop {
    pressed: watch::Receiver<bool>,
}

impl Stop {
    pub fn install() -> Self {
        let (tx, pressed) = watch::channel(false);
        tokio::spawn(async move {
            if tokio::signal::ctrl_c().await.is_ok() {
                let _ = tx.send(true);
            }
        });
        Self { pressed }
    }

    /// 按下去之后完成；没按就一直等。
    pub async fn wait(&self) {
        let mut pressed = self.pressed.clone();
        loop {
            if *pressed.borrow_and_update() {
                return;
            }
            if pressed.changed().await.is_err() {
                // 装信号的那个任务没了（运行时在收摊），那就永远等下去，由别的分支收场。
                std::future::pending::<()>().await;
            }
        }
    }

    /// 让一件事和 Ctrl-C 赛跑。被按了就是 `None`。
    pub async fn race<F: Future>(&self, work: F) -> Option<F::Output> {
        tokio::select! {
            biased;
            _ = self.wait() => None,
            out = work => Some(out),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use playtest_common::tunnel::TOKEN_TTL_SECS;

    /// CLI 不带 time 的 formatting 特性（只解析、不格式化），所以这里自己拼一个 RFC 3339。
    fn rfc3339(t: time::OffsetDateTime) -> String {
        let t = t.to_offset(time::UtcOffset::UTC);
        format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
            t.year(),
            u8::from(t.month()),
            t.day(),
            t.hour(),
            t.minute(),
            t.second()
        )
    }

    fn grant_expiring_in(seconds: i64) -> TunnelGrant {
        TunnelGrant {
            slug: "brisk-otter-41".into(),
            url: "https://brisk-otter-41.playtest.run".into(),
            connect_url: "wss://brisk-otter-41.playtest.run/_playtest/tunnel".into(),
            token: "pt1.a.b".into(),
            expires_at: rfc3339(clock::now() + time::Duration::seconds(seconds)),
            site_expires_at: None,
        }
    }

    #[test]
    fn a_fresh_token_is_left_alone_and_a_dying_one_is_swapped() {
        assert!(!expiring_soon(&grant_expiring_in(TOKEN_TTL_SECS)));
        assert!(!expiring_soon(&grant_expiring_in(6 * 60)));
        assert!(expiring_soon(&grant_expiring_in(4 * 60)));
        assert!(expiring_soon(&grant_expiring_in(-1)));
    }

    #[test]
    fn a_time_we_cannot_read_is_not_treated_as_expiring() {
        let mut grant = grant_expiring_in(60);
        grant.expires_at = "下午三点".into();
        assert!(!expiring_soon(&grant), "看不懂就别没事换令牌");
    }
}
