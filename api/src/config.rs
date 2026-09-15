//! 环境变量。默认值对着 `docs/KICKOFF.md` §3 的本机约定，什么都不设也能起来。

use std::net::SocketAddr;
use std::path::PathBuf;

use anyhow::Context;

pub const LISTEN_ENV: &str = "PLAYTEST_API_LISTEN";
pub const DATA_DIR_ENV: &str = "PLAYTEST_DATA_DIR";
pub const SITE_URL_TEMPLATE_ENV: &str = "PLAYTEST_SITE_URL_TEMPLATE";

pub const GITHUB_CLIENT_ID_ENV: &str = "PLAYTEST_GITHUB_CLIENT_ID";
pub const GITHUB_CLIENT_SECRET_ENV: &str = "PLAYTEST_GITHUB_CLIENT_SECRET";
pub const CONSOLE_URL_ENV: &str = "PLAYTEST_CONSOLE_URL";
/// 测试用：把「GitHub」指到本机的假服务器。线上不设。
pub const GITHUB_BASE_ENV: &str = "PLAYTEST_GITHUB_BASE_URL";

pub const EMAIL_PROVIDER_ENV: &str = "PLAYTEST_EMAIL_PROVIDER";
pub const EMAIL_FROM_ENV: &str = "PLAYTEST_EMAIL_FROM";
pub const RESEND_API_KEY_ENV: &str = "PLAYTEST_RESEND_API_KEY";
pub const SMTP_URL_ENV: &str = "PLAYTEST_SMTP_URL";
pub const PUBLIC_ROOT_URL_ENV: &str = "PLAYTEST_PUBLIC_ROOT_URL";
pub const VAPID_SUBJECT_ENV: &str = "PLAYTEST_VAPID_SUBJECT";
pub const ADMIN_TOKEN_ENV: &str = "PLAYTEST_ADMIN_TOKEN";
pub const EDGE_INGEST_TOKEN_ENV: &str = "PLAYTEST_EDGE_INGEST_TOKEN";
pub const MIN_EDGE_INGEST_TOKEN_BYTES: usize = 32;

/// 集成测试构造 `Config::default()` 时使用；生产进程只认环境变量，不会落到这个值。
pub const TEST_EDGE_INGEST_TOKEN: &str = "test-edge-ingest-token-000000000";

pub const DEFAULT_LISTEN: &str = "127.0.0.1:8787";
pub const DEFAULT_DATA_DIR: &str = ".data";
pub const DEFAULT_SITE_URL_TEMPLATE: &str = "http://{slug}.localhost:8443";
/// 本机开发时控制台是 vite dev server（console/vite.config.ts）。
pub const DEFAULT_CONSOLE_URL: &str = "http://localhost:5273/";
/// 玩家侧的根域。`/me`、退订链接、确认链接都挂在它下面（DESIGN §3.10）。
pub const DEFAULT_PUBLIC_ROOT_URL: &str = "http://localhost:8443";
pub const DEFAULT_EMAIL_FROM: &str = "playtest.run <notice@playtest.run>";

/// 链接模板里被 slug 替换掉的那一段。
pub const SLUG_PLACEHOLDER: &str = "{slug}";

#[derive(Debug, Clone)]
pub struct Config {
    pub listen: SocketAddr,
    pub data_dir: PathBuf,
    /// 例如 `http://{slug}.localhost:8443`。玩家链接只用这个域，登录和令牌在另一个域（DESIGN §4.1）。
    pub site_url_template: String,
    /// 没配就是这个控制面不提供 GitHub 登录，匿名链接照常。
    pub github: Option<GitHubApp>,
    pub notify: Notify,
    /// 没配就是这台机器没有管理接口：`/admin/*` 整组不注册，外面看到的是 404。
    pub admin_token: Option<String>,
    /// 只给边缘事件批量上报使用；浏览器和普通 API 调用方不应拿到。
    pub edge_ingest_token: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            listen: DEFAULT_LISTEN.parse().expect("默认监听地址是常量"),
            data_dir: PathBuf::from(DEFAULT_DATA_DIR),
            site_url_template: DEFAULT_SITE_URL_TEMPLATE.to_string(),
            github: None,
            notify: Notify::default(),
            admin_token: None,
            edge_ingest_token: Some(TEST_EDGE_INGEST_TOKEN.to_string()),
        }
    }
}

/// 怎么把信送出去（DESIGN §4.10）。
#[derive(Debug, Clone)]
pub struct Notify {
    pub email: EmailProvider,
    /// 发件人，形如 `playtest.run <notice@playtest.run>`。
    pub from: String,
    /// 玩家侧根域，信里的链接都从它拼。
    pub root_url: String,
    /// VAPID 的 `sub`，推送服务出问题时他们照这个找我们；形如 `mailto:hi@example.com`。
    pub vapid_subject: Option<String>,
}

impl Default for Notify {
    fn default() -> Self {
        Self {
            // 默认只打日志：本机跑起来能看见整封信长什么样，又不会真发出去。
            email: EmailProvider::Log,
            from: DEFAULT_EMAIL_FROM.to_string(),
            root_url: DEFAULT_PUBLIC_ROOT_URL.to_string(),
            vapid_subject: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EmailProvider {
    /// 整封信打进日志。默认，也是本机验证用的那档。
    Log,
    Resend {
        api_key: String,
    },
    /// `smtps://user:pass@host:465`。
    Smtp {
        url: String,
    },
    /// 明确关掉：`capabilities.json` 里 `email = false`，边缘就不显示留邮箱那一栏。
    Off,
}

impl EmailProvider {
    pub fn is_on(&self) -> bool {
        !matches!(self, Self::Off)
    }
}

impl Notify {
    pub fn validate(&self) -> anyhow::Result<()> {
        if matches!(self.email, EmailProvider::Log | EmailProvider::Off) {
            return Ok(());
        }
        let root = reqwest::Url::parse(&self.root_url).ok();
        let valid = root.as_ref().is_some_and(|url| {
            url.scheme() == "https"
                && url.username().is_empty()
                && url.password().is_none()
                && url.path() == "/"
                && url.query().is_none()
                && url.fragment().is_none()
                && url.host_str().is_some_and(|host| {
                    host != "localhost"
                        && !host.ends_with(".localhost")
                        && host != "playtest.roviix.com"
                        && !host.ends_with(".playtest.roviix.com")
                        && !host
                            .trim_matches(['[', ']'])
                            .parse::<std::net::IpAddr>()
                            .is_ok_and(|address| address.is_loopback() || address.is_unspecified())
                })
        });
        anyhow::ensure!(
            valid,
            "Before sending real email, set {PUBLIC_ROOT_URL_ENV} to an HTTPS root address players can reach. Not localhost, not a loopback address, not the Developer Console domain, and without a path."
        );
        anyhow::ensure!(
            self.from.parse::<lettre::message::Mailbox>().is_ok(),
            "{EMAIL_FROM_ENV} has to be a valid sender mailbox, for example: playtest.run <notice@playtest.run>"
        );
        Ok(())
    }
}

/// GitHub OAuth App。`client_id` 是公开的；`client_secret` 只有网页授权码流程（控制台）要，
/// 终端的设备码流程不用它。两样都只从环境变量来，不进仓库（AGENTS 第 9 条）。
#[derive(Debug, Clone)]
pub struct GitHubApp {
    pub client_id: String,
    pub client_secret: Option<String>,
    /// 网页流程回来的地址，也是控制台自己的地址，例如 `https://playtest.roviix.com/console/`。
    /// GitHub 要求它是 OAuth App 里登记的回调地址本身或其子路径。
    pub console_url: String,
    /// `https://github.com` 与 `https://api.github.com`；测试时都指向同一个假服务器。
    pub web_base: String,
    pub api_base: String,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let listen_raw = env_or(LISTEN_ENV, DEFAULT_LISTEN);
        let listen = listen_raw.parse().with_context(|| {
            format!(
                "{LISTEN_ENV} takes an address:port pair, for example {DEFAULT_LISTEN}. It is currently {listen_raw}"
            )
        })?;

        let site_url_template = env_or(SITE_URL_TEMPLATE_ENV, DEFAULT_SITE_URL_TEMPLATE);
        if !site_url_template.contains(SLUG_PLACEHOLDER) {
            anyhow::bail!(
                "{SITE_URL_TEMPLATE_ENV} has to contain {SLUG_PLACEHOLDER}, or every project gets the same link. It is currently {site_url_template}"
            );
        }

        let edge_ingest_token = env_opt(EDGE_INGEST_TOKEN_ENV);
        anyhow::ensure!(
            edge_ingest_token
                .as_ref()
                .is_none_or(|token| token.len() >= MIN_EDGE_INGEST_TOKEN_BYTES),
            "{EDGE_INGEST_TOKEN_ENV} needs at least {MIN_EDGE_INGEST_TOKEN_BYTES} bytes. Generate a random value; do not use the one from the example."
        );

        Ok(Self {
            listen,
            data_dir: PathBuf::from(env_or(DATA_DIR_ENV, DEFAULT_DATA_DIR)),
            site_url_template,
            github: github_from_env(),
            notify: notify_from_env()?,
            admin_token: env_opt(ADMIN_TOKEN_ENV),
            edge_ingest_token,
        })
    }

    /// 本机文件系统后端的根；S3 后端不把作品持久化到这里。
    pub fn store_root(&self) -> PathBuf {
        self.data_dir.join("store")
    }

    pub fn store_config(&self) -> anyhow::Result<playtest_common::store::StoreConfig> {
        playtest_common::store::StoreConfig::from_env(
            self.store_root(),
            self.data_dir.join("upload-tmp"),
        )
    }

    pub fn sqlite_path(&self) -> PathBuf {
        self.data_dir.join("api.sqlite")
    }
}

fn github_from_env() -> Option<GitHubApp> {
    let client_id = std::env::var(GITHUB_CLIENT_ID_ENV)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())?;
    let client_secret = std::env::var(GITHUB_CLIENT_SECRET_ENV)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty());
    let (web_base, api_base) = match std::env::var(GITHUB_BASE_ENV) {
        Ok(base) if !base.trim().is_empty() => {
            let base = base.trim().trim_end_matches('/').to_string();
            (base.clone(), base)
        }
        _ => (
            "https://github.com".to_string(),
            "https://api.github.com".to_string(),
        ),
    };
    let mut console_url = env_or(CONSOLE_URL_ENV, DEFAULT_CONSOLE_URL);
    if !console_url.ends_with('/') {
        console_url.push('/');
    }
    Some(GitHubApp {
        client_id,
        client_secret,
        console_url,
        web_base,
        api_base,
    })
}

fn notify_from_env() -> anyhow::Result<Notify> {
    let provider = env_or(EMAIL_PROVIDER_ENV, "log");
    let email = match provider.trim().to_ascii_lowercase().as_str() {
        "log" => EmailProvider::Log,
        "off" | "none" => EmailProvider::Off,
        "resend" => {
            let api_key = env_opt(RESEND_API_KEY_ENV).ok_or_else(|| {
                anyhow::anyhow!(
                    "{EMAIL_PROVIDER_ENV}=resend also needs {RESEND_API_KEY_ENV}. \
                     To send nothing for now, set {EMAIL_PROVIDER_ENV} to off."
                )
            })?;
            EmailProvider::Resend { api_key }
        }
        "smtp" => {
            let url = env_opt(SMTP_URL_ENV).ok_or_else(|| {
                anyhow::anyhow!(
                    "{EMAIL_PROVIDER_ENV}=smtp also needs {SMTP_URL_ENV}, shaped like \
                     smtps://user:password@mail.example.com:465"
                )
            })?;
            EmailProvider::Smtp { url }
        }
        other => anyhow::bail!(
            "{EMAIL_PROVIDER_ENV} takes one of log, resend, smtp, off. It is currently {other}"
        ),
    };
    Ok(Notify {
        email,
        from: env_or(EMAIL_FROM_ENV, DEFAULT_EMAIL_FROM),
        root_url: env_or(PUBLIC_ROOT_URL_ENV, DEFAULT_PUBLIC_ROOT_URL)
            .trim_end_matches('/')
            .to_string(),
        vapid_subject: env_opt(VAPID_SUBJECT_ENV),
    })
}

fn env_or(key: &str, default: &str) -> String {
    match std::env::var(key) {
        Ok(v) if !v.trim().is_empty() => v,
        _ => default.to_string(),
    }
}

fn env_opt(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}
