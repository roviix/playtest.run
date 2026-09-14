//! 边缘的运行配置。
//!
//! 全部来自环境变量且都有默认值：从仓库根目录 `cargo run -p playtest-edge` 不写任何配置
//! 就该能起来，这是 KICKOFF §3 的本机约定。

use std::net::SocketAddr;
use std::path::PathBuf;

use anyhow::Context;

pub const DEFAULT_LISTEN: &str = "127.0.0.1:8443";
pub const DEFAULT_DATA_DIR: &str = ".data";
pub const DEFAULT_HOST_SUFFIX: &str = "localhost";
const MIN_EDGE_INGEST_TOKEN_BYTES: usize = 32;

#[derive(Debug, Clone)]
pub struct Config {
    pub listen: SocketAddr,
    pub data_dir: PathBuf,
    /// 泛域名后缀。本机是 `localhost`，上线是 `playtest.run`。
    pub host_suffix: String,
    /// 拼 OG 链接与角标链接用。边缘自己永远只听明文，这个值说的是玩家在地址栏里看到的东西。
    pub public_scheme: String,
    /// 控制面的内网地址（`PLAYTEST_API_INTERNAL_URL`，例如 compose 里的 `http://api:8787`）。
    /// 事件批量上报和关注登记都往这里送；没设就是「控制面不在」——作品照常能玩，
    /// 关注会如实说「现在登记不了」（DESIGN §4.1）。
    pub api_internal_url: Option<String>,
    /// 边缘批量上报事件时使用；只在边缘与控制面的内网请求中出现。
    pub edge_ingest_token: Option<String>,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let listen_raw = env_or("PLAYTEST_EDGE_LISTEN", DEFAULT_LISTEN);
        let listen = listen_raw.parse::<SocketAddr>().with_context(|| {
            format!("PLAYTEST_EDGE_LISTEN 不是「地址:端口」的形式：{listen_raw}")
        })?;
        let host_suffix = env_or("PLAYTEST_HOST_SUFFIX", DEFAULT_HOST_SUFFIX).to_ascii_lowercase();
        let public_scheme = std::env::var("PLAYTEST_PUBLIC_SCHEME")
            .ok()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| default_scheme(&host_suffix).to_string());
        let api_internal_url = Self::api_internal_url();
        let edge_ingest_token = env_opt("PLAYTEST_EDGE_INGEST_TOKEN");
        anyhow::ensure!(
            api_internal_url.is_none() || edge_ingest_token.is_some(),
            "设置 PLAYTEST_API_INTERNAL_URL 时也必须设置 PLAYTEST_EDGE_INGEST_TOKEN；否则事件不能安全上报"
        );
        anyhow::ensure!(
            edge_ingest_token
                .as_ref()
                .is_none_or(|token| token.len() >= MIN_EDGE_INGEST_TOKEN_BYTES),
            "PLAYTEST_EDGE_INGEST_TOKEN 至少要有 {MIN_EDGE_INGEST_TOKEN_BYTES} 字节；请生成随机值，不要使用示例值"
        );
        Ok(Self {
            listen,
            data_dir: PathBuf::from(env_or("PLAYTEST_DATA_DIR", DEFAULT_DATA_DIR)),
            host_suffix,
            public_scheme,
            api_internal_url,
            edge_ingest_token,
        })
    }

    /// 共享 cookie 的域名（带前导点，如 `.playtest.run` 或 `.localhost`）。
    pub fn cookie_domain(&self) -> String {
        let clean = self.host_suffix.trim_start_matches('.');
        format!(".{clean}")
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

    /// 第一层数据先落这里，第三周再送控制面（DESIGN §3.4）。
    pub fn events_path(&self) -> PathBuf {
        self.data_dir.join("edge-events.jsonl")
    }

    /// 隧道「上次在线」的记录。放边缘自己的目录、不放对象存储：
    /// 这是边缘的观察，api 既不写也不读它。
    pub fn tunnels_dir(&self) -> PathBuf {
        self.data_dir.join("tunnels")
    }

    /// 边缘事件往哪送（`ship.rs`）。`PLAYTEST_API_INTERNAL_URL`，例如 compose 里的 `http://api:8787`；
    /// 没设就不送，只落本地 JSONL——本机一个人调试时用不着。
    pub fn api_internal_url() -> Option<String> {
        std::env::var("PLAYTEST_API_INTERNAL_URL")
            .ok()
            .filter(|s| !s.is_empty())
    }

    /// 边缘本地不可变 Blob 缓存目录（针对 S3 对象按需拉取落盘）。
    pub fn blob_cache_dir(&self) -> PathBuf {
        self.data_dir.join("cache").join("blobs")
    }

    /// 本地 Blob 缓存上限字节数（默认 10 GiB，环境变量 PLAYTEST_EDGE_CACHE_MAX_BYTES）。
    pub fn blob_cache_max_bytes(&self) -> u64 {
        std::env::var("PLAYTEST_EDGE_CACHE_MAX_BYTES")
            .ok()
            .and_then(|v| v.trim().parse::<u64>().ok())
            .unwrap_or(10 * 1024 * 1024 * 1024)
    }
}

fn env_or(key: &str, fallback: &str) -> String {
    std::env::var(key)
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| fallback.to_string())
}

fn env_opt(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

/// `*.localhost` 在 Chrome / Firefox 里是安全上下文但走明文，本机链接得写 `http`。
pub fn default_scheme(host_suffix: &str) -> &'static str {
    if host_suffix == "localhost" || host_suffix.ends_with(".localhost") {
        "http"
    } else {
        "https"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scheme_follows_suffix() {
        assert_eq!(default_scheme("localhost"), "http");
        assert_eq!(default_scheme("edge.localhost"), "http");
        assert_eq!(default_scheme("playtest.run"), "https");
    }
}
