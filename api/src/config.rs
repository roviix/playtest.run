//! 环境变量。默认值对着 `docs/KICKOFF.md` §3 的本机约定，什么都不设也能起来。

use std::net::SocketAddr;
use std::path::PathBuf;

use anyhow::Context;

pub const LISTEN_ENV: &str = "PLAYTEST_API_LISTEN";
pub const DATA_DIR_ENV: &str = "PLAYTEST_DATA_DIR";
pub const SITE_URL_TEMPLATE_ENV: &str = "PLAYTEST_SITE_URL_TEMPLATE";

pub const DEFAULT_LISTEN: &str = "127.0.0.1:8787";
pub const DEFAULT_DATA_DIR: &str = ".data";
pub const DEFAULT_SITE_URL_TEMPLATE: &str = "http://{slug}.localhost:8443";

/// 链接模板里被 slug 替换掉的那一段。
pub const SLUG_PLACEHOLDER: &str = "{slug}";

#[derive(Debug, Clone)]
pub struct Config {
    pub listen: SocketAddr,
    pub data_dir: PathBuf,
    /// 例如 `http://{slug}.localhost:8443`。玩家链接只用这个域，登录和令牌在另一个域（DESIGN §4.1）。
    pub site_url_template: String,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let listen_raw = env_or(LISTEN_ENV, DEFAULT_LISTEN);
        let listen = listen_raw.parse().with_context(|| {
            format!("{LISTEN_ENV} 要写成「地址:端口」的样子，例如 {DEFAULT_LISTEN}，现在是「{listen_raw}」")
        })?;

        let site_url_template = env_or(SITE_URL_TEMPLATE_ENV, DEFAULT_SITE_URL_TEMPLATE);
        if !site_url_template.contains(SLUG_PLACEHOLDER) {
            anyhow::bail!(
                "{SITE_URL_TEMPLATE_ENV} 里必须有 {SLUG_PLACEHOLDER}，否则每个作品的链接都一样，现在是「{site_url_template}」"
            );
        }

        Ok(Self {
            listen,
            data_dir: PathBuf::from(env_or(DATA_DIR_ENV, DEFAULT_DATA_DIR)),
            site_url_template,
        })
    }

    /// 对象存储的根。edge 读同一个目录（KICKOFF §3）。
    pub fn store_root(&self) -> PathBuf {
        self.data_dir.join("store")
    }

    pub fn sqlite_path(&self) -> PathBuf {
        self.data_dir.join("api.sqlite")
    }
}

fn env_or(key: &str, default: &str) -> String {
    match std::env::var(key) {
        Ok(v) if !v.trim().is_empty() => v,
        _ => default.to_string(),
    }
}
