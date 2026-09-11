//! `sites/<slug>/live.json` 的读缓存（DESIGN §4.5）。
//!
//! 清单不可变，但门禁页上有几样东西随时在变：名额与已加入人数、关注数、开发者的群、
//! 公开反馈、头像、在不在广场上。控制面在这些变化时重写这一份，边缘只读、短缓存
//! ——控制面挂了门禁页照常出，只是数字旧几十秒（DESIGN §4.1「从不为读回源」）。
//!
//! 读不到（没写过、坏了、slug 不合法）一律当作「什么都没有」：对应的那几行不出现，
//! 不报错、不解释（AGENTS 第 4 条）。缓存本身在 [`crate::cache`]。

use std::sync::Arc;
use std::time::Duration;

use playtest_common::live::SiteLive;
use playtest_common::store::FsStore;

use crate::cache::CachedMap;

/// 缓存寿命。30 秒内的旧数字没人分得出来，和广场那一份同一个量级。
pub const TTL: Duration = Duration::from_secs(30);

pub struct LiveCache {
    store: FsStore,
    cached: CachedMap<String, SiteLive>,
}

impl LiveCache {
    pub fn new(store: FsStore) -> Self {
        Self::with_ttl(store, TTL)
    }

    pub fn with_ttl(store: FsStore, ttl: Duration) -> Self {
        Self {
            store,
            cached: CachedMap::new(ttl),
        }
    }

    pub async fn get(&self, slug: &str) -> Arc<SiteLive> {
        let mut live = self
            .cached
            .get_or(slug.to_string(), || async {
                match self.store.get_live(slug).await {
                    Ok(live) => live,
                    Err(err) => {
                        tracing::warn!(slug, %err, "读 live.json 失败，按什么都没有出");
                        None
                    }
                }
            })
            .await;
        // 没写过这份文件的作品，`slug` 会是空的——门禁页拿它去拼链接，所以补上。
        if live.slug.is_empty() {
            live = Arc::new(SiteLive::empty(slug));
        }
        live
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn nothing_written_reads_as_nothing_to_show() {
        let dir = tempfile::tempdir().unwrap();
        let cache = LiveCache::new(FsStore::new(dir.path()));
        let live = cache.get("brisk-otter-41").await;
        assert_eq!(live.slug, "brisk-otter-41");
        assert_eq!(live.seats, None);
        assert!(!live.listed);
    }

    #[tokio::test]
    async fn a_written_file_is_served_and_then_cached() {
        let dir = tempfile::tempdir().unwrap();
        let store = FsStore::new(dir.path());
        let mut live = SiteLive::empty("brisk-otter-41");
        live.seats = Some(10);
        live.joined = 6;
        store.put_live(&live).await.unwrap();

        let cache = LiveCache::new(store.clone());
        assert_eq!(cache.get("brisk-otter-41").await.joined, 6);

        // 缓存期内改文件，读到的还是旧的——这正是「数字旧几十秒」。
        live.joined = 7;
        store.put_live(&live).await.unwrap();
        assert_eq!(cache.get("brisk-otter-41").await.joined, 6);

        // TTL 为 0 的缓存每次都读盘。
        let fresh = LiveCache::with_ttl(store, Duration::ZERO);
        assert_eq!(fresh.get("brisk-otter-41").await.joined, 7);
    }

    #[tokio::test]
    async fn a_bad_slug_is_not_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let cache = LiveCache::new(FsStore::new(dir.path()));
        assert_eq!(cache.get("../../etc").await.seats, None);
    }
}
