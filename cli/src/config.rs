//! `~/.config/playtest/config.json`：匿名令牌，以及「这个目录上次发到哪个作品」。
//!
//! 一个 JSON 文件加一个环境变量就够，所以不引 dirs、也不引 toml。文件里有令牌，权限 0600。

use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::clock;

/// 令牌剩不到这么多就当它过期，免得传到一半失效。
const EXPIRY_MARGIN_SECONDS: i64 = 60;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    /// 这个令牌是从哪个控制面拿的。换了地址就不能再用它。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
    /// RFC 3339。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token_expires_at: Option<String>,
    /// 规范化后的目录绝对路径 → slug。用 BTreeMap 是为了每次写出来的顺序一样。
    #[serde(default)]
    pub sites: BTreeMap<String, String>,
}

impl Config {
    /// 还能直接拿去用的令牌。控制面地址对不上、没有、快过期，都算没有。
    pub fn usable_token(&self, api: &str, now: OffsetDateTime) -> Option<&str> {
        if self.api.as_deref() != Some(api) {
            return None;
        }
        let token = self.token.as_deref()?;
        let expires = clock::parse_rfc3339(self.token_expires_at.as_deref()?)?;
        if expires - now <= time::Duration::seconds(EXPIRY_MARGIN_SECONDS) {
            return None;
        }
        Some(token)
    }

    pub fn remembered_slug(&self, dir_key: &str) -> Option<&str> {
        self.sites.get(dir_key).map(String::as_str)
    }

    pub fn remember(&mut self, dir_key: String, slug: String) {
        self.sites.insert(dir_key, slug);
    }

    /// 忘掉所有指向这个 slug 的目录。
    pub fn forget_slug(&mut self, slug: &str) {
        self.sites.retain(|_, v| v != slug);
    }
}

/// 配置文件在哪。
pub fn default_path() -> Result<PathBuf> {
    #[cfg(windows)]
    {
        let base = std::env::var_os("APPDATA")
            .filter(|v| !v.is_empty())
            .context("找不到 %APPDATA%，没法保存配置")?;
        Ok(PathBuf::from(base).join("playtest").join("config.json"))
    }
    #[cfg(not(windows))]
    {
        let base = std::env::var_os("HOME")
            .filter(|v| !v.is_empty())
            .context("找不到主目录（环境变量 HOME 是空的），没法保存配置")?;
        Ok(PathBuf::from(base)
            .join(".config")
            .join("playtest")
            .join("config.json"))
    }
}

/// 读配置。文件不在就当是全新的一台机器。
pub fn load(path: &Path) -> Result<Config> {
    match std::fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes).with_context(|| {
            format!("配置文件读不懂：{}。删掉它再运行一次就好。", path.display())
        }),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Config::default()),
        Err(e) => Err(e).with_context(|| format!("读不了配置文件 {}", path.display())),
    }
}

/// 写配置。先写同目录下的临时文件再改名，中途被打断也不会剩下半个文件。
pub fn save(path: &Path, config: &Config) -> Result<()> {
    let dir = path
        .parent()
        .with_context(|| format!("配置文件路径不对：{}", path.display()))?;
    std::fs::create_dir_all(dir)
        .with_context(|| format!("建不了配置目录 {}", dir.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700));
    }

    let body = serde_json::to_vec_pretty(config).context("配置写不成 JSON")?;
    let tmp = path.with_extension("json.tmp");
    {
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create(true).truncate(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options
            .open(&tmp)
            .with_context(|| format!("写不了 {}", tmp.display()))?;
        file.write_all(&body)
            .and_then(|()| file.write_all(b"\n"))
            .with_context(|| format!("写不了 {}", tmp.display()))?;
    }
    std::fs::rename(&tmp, path)
        .with_context(|| format!("保存不了配置文件 {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(s: &str) -> OffsetDateTime {
        clock::parse_rfc3339(s).unwrap()
    }

    #[test]
    fn round_trips_through_the_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("config.json");

        let mut written = Config {
            api: Some("http://127.0.0.1:8787".into()),
            token: Some("tok-1".into()),
            token_expires_at: Some("2026-09-08T03:30:00Z".into()),
            sites: BTreeMap::new(),
        };
        written.remember("/tmp/游戏/dist".into(), "brisk-otter-41".into());
        written.remember("/tmp/other".into(), "keen-yak-7".into());

        save(&path, &written).unwrap();
        assert_eq!(load(&path).unwrap(), written);
    }

    #[test]
    fn missing_file_reads_as_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(load(&dir.path().join("config.json")).unwrap(), Config::default());
    }

    #[cfg(unix)]
    #[test]
    fn file_is_not_readable_by_others() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        save(&path, &Config::default()).unwrap();
        let mode = std::fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600, "配置里有令牌，别人不该读得到");
    }

    #[test]
    fn token_is_only_usable_for_the_api_it_came_from() {
        let config = Config {
            api: Some("http://a".into()),
            token: Some("tok".into()),
            token_expires_at: Some("2026-09-08T03:30:00Z".into()),
            sites: BTreeMap::new(),
        };
        let now = at("2026-09-07T03:30:00Z");
        assert_eq!(config.usable_token("http://a", now), Some("tok"));
        assert_eq!(config.usable_token("http://b", now), None);
    }

    #[test]
    fn expired_or_nearly_expired_token_is_not_usable() {
        let config = Config {
            api: Some("http://a".into()),
            token: Some("tok".into()),
            token_expires_at: Some("2026-09-08T03:30:00Z".into()),
            sites: BTreeMap::new(),
        };
        assert_eq!(config.usable_token("http://a", at("2026-09-08T04:00:00Z")), None);
        assert_eq!(config.usable_token("http://a", at("2026-09-08T03:29:30Z")), None);
    }

    #[test]
    fn forgetting_a_slug_drops_every_directory_pointing_at_it() {
        let mut config = Config::default();
        config.remember("/a".into(), "same".into());
        config.remember("/b".into(), "same".into());
        config.remember("/c".into(), "other".into());
        config.forget_slug("same");
        assert_eq!(config.sites.len(), 1);
        assert_eq!(config.remembered_slug("/c"), Some("other"));
    }
}
