//! slug → 当前该出什么。
//!
//! 只读对象存储，不问控制面（DESIGN §4.5：控制面挂了已有链接照常能开）。
//! 缓存分两种寿命：清单按 `(slug, version)` 不可变，可以一直留着；
//! `current` 指针只留 1 秒，回滚要立刻在玩家那边生效。

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use playtest_common::manifest::Manifest;
use playtest_common::store::FsStore;
use std::sync::Arc;
use time::OffsetDateTime;

/// `current` 指针的缓存寿命。再短就是每个请求都读一次盘，再长回滚就不「立刻」了。
pub const CURRENT_TTL: Duration = Duration::from_secs(1);

/// 只说对象存储里有什么。隧道在不在线是另一回事，问 `tunnel::Tunnels`——
/// 一个 slug 可以既上传过又开着隧道，两边都不知道对方的存在，由 `app.rs` 决定先看谁。
#[derive(Debug, Clone)]
pub enum SiteState {
    Live(Arc<Manifest>),
    /// 没有这个 slug，或者对象存储里被删了。
    Missing,
    Expired(Arc<Manifest>),
}

pub struct SiteStore {
    store: FsStore,
    manifests: Mutex<HashMap<(String, u32), Arc<Manifest>>>,
    current: Mutex<HashMap<String, CachedCurrent>>,
    ttl: Duration,
}

struct CachedCurrent {
    at: Instant,
    version: Option<u32>,
}

impl SiteStore {
    pub fn new(store: FsStore) -> Self {
        Self::with_ttl(store, CURRENT_TTL)
    }

    pub fn with_ttl(store: FsStore, ttl: Duration) -> Self {
        Self {
            store,
            manifests: Mutex::new(HashMap::new()),
            current: Mutex::new(HashMap::new()),
            ttl,
        }
    }

    pub fn store(&self) -> &FsStore {
        &self.store
    }

    pub async fn resolve(&self, slug: &str) -> SiteState {
        let Some(version) = self.current_version(slug).await else {
            return SiteState::Missing;
        };
        let Some(manifest) = self.manifest(slug, version).await else {
            // 指针指向一个不存在的版本：api 写坏了或者对象被删了一半。
            // 对玩家只能是「这个链接不存在」，但日志里要留得下痕迹。
            tracing::warn!(slug, version, "current 指向的清单读不到");
            return SiteState::Missing;
        };
        if is_expired(&manifest, OffsetDateTime::now_utc()) {
            return SiteState::Expired(manifest);
        }
        SiteState::Live(manifest)
    }

    async fn current_version(&self, slug: &str) -> Option<u32> {
        if let Some(hit) = self
            .current
            .lock()
            .ok()
            .and_then(|c| c.get(slug).filter(|e| e.at.elapsed() < self.ttl).map(|e| e.version))
        {
            return hit;
        }
        let version = match self.store.get_current(slug).await {
            Ok(cur) => cur.map(|c| c.version),
            Err(err) => {
                tracing::warn!(slug, %err, "读 current 指针失败");
                None
            }
        };
        if let Ok(mut cache) = self.current.lock() {
            cache.insert(
                slug.to_string(),
                CachedCurrent {
                    at: Instant::now(),
                    version,
                },
            );
        }
        version
    }

    async fn manifest(&self, slug: &str, version: u32) -> Option<Arc<Manifest>> {
        let key = (slug.to_string(), version);
        if let Some(hit) = self.manifests.lock().ok().and_then(|m| m.get(&key).cloned()) {
            return Some(hit);
        }
        let manifest = match self.store.get_manifest(slug, version).await {
            Ok(Some(m)) => Arc::new(m),
            Ok(None) => return None,
            Err(err) => {
                tracing::warn!(slug, version, %err, "读清单失败");
                return None;
            }
        };
        if let Ok(mut cache) = self.manifests.lock() {
            cache.insert(key, manifest.clone());
        }
        Some(manifest)
    }
}

/// `expires_at` 解析不了时按「没过期」处理：这个字段是 api 自己写的 RFC 3339，
/// 解析失败是我们的 bug，让所有链接一起 410 比多活一会儿糟得多。
pub fn is_expired(manifest: &Manifest, now: OffsetDateTime) -> bool {
    let Some(raw) = manifest.expires_at.as_deref() else {
        return false;
    };
    match OffsetDateTime::parse(raw, &time::format_description::well_known::Rfc3339) {
        Ok(at) => now >= at,
        Err(err) => {
            tracing::warn!(slug = %manifest.slug, raw, %err, "expires_at 不是 RFC 3339，按未过期处理");
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use playtest_common::manifest::{GateMode, SCHEMA};
    use time::macros::datetime;

    fn manifest(expires_at: Option<&str>) -> Manifest {
        Manifest {
            schema: SCHEMA,
            slug: "brisk-otter-41".into(),
            version: 1,
            title: "测试".into(),
            developer: "某某".into(),
            note: None,
            created_at: "2026-09-07T00:00:00Z".into(),
            expires_at: expires_at.map(str::to_string),
            badge: true,
            gate: GateMode::Once,
            isolated: false,
            spa: false,
            engine: None,
            files: vec![],
        }
    }

    #[test]
    fn expiry_compares_against_now() {
        let now = datetime!(2026-09-07 12:00:00 UTC);
        assert!(!is_expired(&manifest(None), now));
        assert!(!is_expired(&manifest(Some("2026-09-08T00:00:00Z")), now));
        assert!(is_expired(&manifest(Some("2026-09-07T11:59:59Z")), now));
        // 带时区偏移的写法也要认。
        assert!(is_expired(&manifest(Some("2026-09-07T19:00:00+08:00")), now));
        // 解析不了就当没过期。
        assert!(!is_expired(&manifest(Some("明天")), now));
    }
}
