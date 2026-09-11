//! `capabilities.json` 的读缓存（DESIGN §4.5）。
//!
//! 「有新版本时告诉我」那一行显示邮箱输入、浏览器通知按钮、还是整行不出现，取决于控制面
//! 有没有配发信实现、有没有 Web Push 的公钥。控制面启动时把答案写进对象存储，边缘读它。
//!
//! 读不到就是 [`Capabilities::default`]——什么都不显示。**做不到的就不显示，不解释**
//! （AGENTS 第 4 条）：一个点了没反应的按钮比没有按钮糟。

use std::sync::Arc;
use std::time::Duration;

use playtest_common::capabilities::Capabilities;
use playtest_common::store::FsStore;

use crate::cache::Cached;

/// 缓存寿命。这份文件一天也不见得变一次（控制面启动时写），60 秒足够快地跟上一次重启。
pub const TTL: Duration = Duration::from_secs(60);

pub struct CapabilitiesCache {
    store: FsStore,
    cached: Cached<Capabilities>,
}

impl CapabilitiesCache {
    pub fn new(store: FsStore) -> Self {
        Self::with_ttl(store, TTL)
    }

    pub fn with_ttl(store: FsStore, ttl: Duration) -> Self {
        Self {
            store,
            cached: Cached::new(ttl),
        }
    }

    pub async fn get(&self) -> Arc<Capabilities> {
        self.cached
            .get(|| async {
                match self.store.get_capabilities().await {
                    Ok(caps) => caps,
                    Err(err) => {
                        tracing::warn!(%err, "读 capabilities.json 失败，按什么都做不了出");
                        None
                    }
                }
            })
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn nothing_written_means_no_follow_ui() {
        let dir = tempfile::tempdir().unwrap();
        let cache = CapabilitiesCache::new(FsStore::new(dir.path()));
        assert!(cache.get().await.nothing());
    }

    #[tokio::test]
    async fn what_the_control_plane_wrote_is_what_the_page_offers() {
        let dir = tempfile::tempdir().unwrap();
        let store = FsStore::new(dir.path());
        store
            .put_capabilities(&Capabilities {
                schema: playtest_common::capabilities::SCHEMA,
                generated_at: "2026-09-09T00:00:00Z".into(),
                email: true,
                push_public_key: Some("BPublicKey".into()),
            })
            .await
            .unwrap();
        let cache = CapabilitiesCache::new(store);
        let caps = cache.get().await;
        assert!(caps.email);
        assert_eq!(caps.push_public_key.as_deref(), Some("BPublicKey"));
    }
}
