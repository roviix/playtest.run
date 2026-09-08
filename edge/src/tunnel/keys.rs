//! 隧道令牌的验签公钥从哪来（DESIGN §4.5：边缘只验签，不回源）。
//!
//! 两个来源，环境变量优先：
//!
//! 1. `PLAYTEST_TUNNEL_VERIFYING_KEY`（base64url）。多边缘、或者不和控制面共用磁盘时用它。
//! 2. 对象存储里的 [`key_files::VERIFYING_KEY_OBJECT`]，控制面第一次启动时写进去。
//!
//! **不能只在启动时读一次。** 本机开发常常是边缘先起、api 后起，那一刻文件还不存在；
//! 控制面换密钥时文件内容也会变。所以缓存里连 mtime 和长度一起记，每次握手前 stat 一下，
//! 变了就重读。始终没有 → 握手回 503，日志里说清是哪一个都没有，而不是把令牌判成「签名不对」。

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::SystemTime;

use playtest_common::tunnel::VerifyingKey;

/// 有它就不读文件。
pub const ENV: &str = "PLAYTEST_TUNNEL_VERIFYING_KEY";

/// 文件「变没变」的判据。内容哈希更准，但公钥文件一次握手读一遍，
/// stat 比读+算便宜得多，而 mtime 加长度已经足够挡住「控制面换了一把新钥匙」。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Stamp {
    modified: Option<SystemTime>,
    len: u64,
}

#[derive(Debug)]
struct Cached {
    key: VerifyingKey,
    stamp: Stamp,
}

#[derive(Debug)]
pub struct Keys {
    /// 环境变量给的那把。进程一生不变，所以只读一次。
    fixed: Option<VerifyingKey>,
    path: PathBuf,
    cached: Mutex<Option<Cached>>,
}

impl Keys {
    pub fn new(path: PathBuf) -> Self {
        let fixed = std::env::var(ENV)
            .ok()
            .filter(|raw| !raw.trim().is_empty())
            .and_then(|raw| match VerifyingKey::from_base64(&raw) {
                Some(key) => Some(key),
                None => {
                    tracing::warn!("{ENV} 不是 base64url 的 Ed25519 公钥，忽略它，改从对象存储读");
                    None
                }
            });
        Self {
            fixed,
            path,
            cached: Mutex::new(None),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// 此刻该用哪把公钥验签。没有就是没有——调用方回 503，不要退化成「验不过」。
    pub async fn current(&self) -> Option<VerifyingKey> {
        if let Some(key) = &self.fixed {
            return Some(key.clone());
        }

        let stamp = self.stamp().await;
        let cached = self.cached.lock().ok().and_then(|c| {
            c.as_ref()
                .map(|c| (c.key.clone(), Some(c.stamp) == stamp))
        });
        match cached {
            Some((key, true)) => return Some(key),
            // 文件这一刻读不到（被换名字、磁盘抖了一下），但我们手上有一把读到过的。
            // 为此把所有在线的隧道都拒掉不值得。
            Some((key, false)) if stamp.is_none() => return Some(key),
            _ => {}
        }
        let stamp = stamp?;

        let raw = match tokio::fs::read_to_string(&self.path).await {
            Ok(raw) => raw,
            Err(err) => {
                tracing::debug!(path = %self.path.display(), %err, "验签公钥读不出来");
                return None;
            }
        };
        let Some(key) = VerifyingKey::from_base64(&raw) else {
            tracing::warn!(path = %self.path.display(), "验签公钥文件里不是 base64url 的 Ed25519 公钥");
            return None;
        };
        if let Ok(mut slot) = self.cached.lock() {
            *slot = Some(Cached {
                key: key.clone(),
                stamp,
            });
        }
        Some(key)
    }

    async fn stamp(&self) -> Option<Stamp> {
        let meta = tokio::fs::metadata(&self.path).await.ok()?;
        Some(Stamp {
            modified: meta.modified().ok(),
            len: meta.len(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use playtest_common::tunnel::SigningKey;

    #[tokio::test]
    async fn reads_the_file_and_notices_when_it_changes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("keys/tunnel.pub");
        let keys = Keys::new(path.clone());

        // api 还没起，文件不存在：说没有，而不是抛错。
        assert!(keys.current().await.is_none());

        let first = SigningKey::generate().verifying_key();
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, first.to_base64()).unwrap();
        assert_eq!(keys.current().await, Some(first.clone()));
        // 第二次走缓存，仍是同一把。
        assert_eq!(keys.current().await, Some(first));

        // 控制面换了钥匙。长度一样，靠 mtime 认出来，所以要真的让 mtime 走一格。
        let second = SigningKey::generate().verifying_key();
        std::thread::sleep(std::time::Duration::from_millis(20));
        std::fs::write(&path, second.to_base64()).unwrap();
        assert_eq!(keys.current().await, Some(second.clone()));

        // 文件被删了：手上那把还留着，不把在线的隧道全拒掉。
        std::fs::remove_file(&path).unwrap();
        assert_eq!(keys.current().await, Some(second));
    }

    #[tokio::test]
    async fn a_file_that_is_not_a_key_is_not_a_key() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tunnel.pub");
        std::fs::write(&path, "这不是公钥").unwrap();
        assert!(Keys::new(path).current().await.is_none());
    }
}
