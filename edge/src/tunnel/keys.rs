//! 隧道令牌的验签公钥从哪来（DESIGN §4.5：边缘只验签，不回源）。
//!
//! 两个来源，环境变量优先：
//!
//! 1. `PLAYTEST_TUNNEL_VERIFYING_KEY`（base64url）。多边缘、或者不和控制面共用磁盘时用它。
//! 2. 对象存储里的 [`key_files::VERIFYING_KEY_OBJECT`]，控制面第一次启动时写进去。
//!
//! **不能只在启动时读一次。** 本机开发常常是边缘先起、api 后起，那一刻对象还不存在；
//! 控制面换密钥时内容也会变。最多缓存一秒，刷新失败则沿用上一次确实读到过的公钥；始终
//! 没有就让握手回 503，不能把存储故障说成「签名不对」。

use std::sync::Mutex;
use std::time::{Duration, Instant};

use playtest_common::store::Store;
use playtest_common::tunnel::key_files;
use playtest_common::tunnel::VerifyingKey;

/// 有它就不读文件。
pub const ENV: &str = "PLAYTEST_TUNNEL_VERIFYING_KEY";

const TTL: Duration = Duration::from_secs(1);

#[derive(Debug)]
struct Cached {
    key: VerifyingKey,
    checked_at: Instant,
}

#[derive(Debug)]
pub struct Keys {
    /// 环境变量给的那把。进程一生不变，所以只读一次。
    fixed: Option<VerifyingKey>,
    store: Store,
    ttl: Duration,
    cached: Mutex<Option<Cached>>,
}

impl Keys {
    pub fn new(store: Store) -> Self {
        Self::with_ttl(store, TTL)
    }

    fn with_ttl(store: Store, ttl: Duration) -> Self {
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
            store,
            ttl,
            cached: Mutex::new(None),
        }
    }

    /// 此刻该用哪把公钥验签。没有就是没有——调用方回 503，不要退化成「验不过」。
    pub async fn current(&self) -> Option<VerifyingKey> {
        if let Some(key) = &self.fixed {
            return Some(key.clone());
        }

        if let Some(key) = self.cached.lock().ok().and_then(|cached| {
            cached
                .as_ref()
                .filter(|cached| cached.checked_at.elapsed() < self.ttl)
                .map(|cached| cached.key.clone())
        }) {
            return Some(key);
        }

        let raw = match self
            .store
            .get_small(key_files::VERIFYING_KEY_OBJECT, 1024)
            .await
        {
            Ok(Some(raw)) => raw,
            Ok(None) => return self.stale(),
            Err(error) => {
                tracing::debug!(%error, "验签公钥从对象存储读不出来");
                return self.stale();
            }
        };
        let raw = String::from_utf8_lossy(&raw);
        let Some(key) = VerifyingKey::from_base64(&raw) else {
            tracing::warn!(
                object = key_files::VERIFYING_KEY_OBJECT,
                "验签公钥对象里不是 base64url 的 Ed25519 公钥"
            );
            return self.stale();
        };
        if let Ok(mut slot) = self.cached.lock() {
            *slot = Some(Cached {
                key: key.clone(),
                checked_at: Instant::now(),
            });
        }
        Some(key)
    }

    fn stale(&self) -> Option<VerifyingKey> {
        // 对象短暂不可用时，手上读到过的公钥仍然可信；控制面换钥只会让新令牌暂时验不过，
        // 不该顺带踢掉所有还拿着旧令牌的在线隧道。
        self.cached
            .lock()
            .ok()
            .and_then(|cached| cached.as_ref().map(|cached| cached.key.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use playtest_common::tunnel::SigningKey;

    #[tokio::test]
    async fn reads_the_object_and_notices_when_it_changes() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::new(dir.path());
        let keys = Keys::with_ttl(store.clone(), Duration::ZERO);

        // api 还没起，对象不存在：说没有，而不是抛错。
        assert!(keys.current().await.is_none());

        let first = SigningKey::generate().verifying_key();
        store
            .put_bytes(
                key_files::VERIFYING_KEY_OBJECT,
                first.to_base64().as_bytes(),
            )
            .await
            .unwrap();
        assert_eq!(keys.current().await, Some(first.clone()));
        assert_eq!(keys.current().await, Some(first));

        let second = SigningKey::generate().verifying_key();
        store
            .put_bytes(
                key_files::VERIFYING_KEY_OBJECT,
                second.to_base64().as_bytes(),
            )
            .await
            .unwrap();
        assert_eq!(keys.current().await, Some(second.clone()));

        // 源站这一刻读不到：手上那把还留着，不把在线的隧道全拒掉。
        std::fs::remove_dir_all(dir.path().join("keys")).unwrap();
        assert_eq!(keys.current().await, Some(second));
    }

    #[tokio::test]
    async fn a_file_that_is_not_a_key_is_not_a_key() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::new(dir.path());
        store
            .put_bytes(key_files::VERIFYING_KEY_OBJECT, b"this-is-not-a-key")
            .await
            .unwrap();
        assert!(Keys::new(store).current().await.is_none());
    }
}
