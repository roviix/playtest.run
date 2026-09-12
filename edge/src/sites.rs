//! slug → 当前该出什么。
//!
//! 只读对象存储，不问控制面（DESIGN §4.5：控制面挂了已有链接照常能开）。
//! 缓存分两种寿命：清单按 `(slug, version)` 不可变，可以一直留着；
//! `current` 指针只留 1 秒，回滚要立刻在玩家那边生效。

use std::sync::Arc;
use std::time::Duration;

use playtest_common::manifest::Manifest;
use playtest_common::store::FsStore;
use time::OffsetDateTime;

use crate::cache::{CachedMap, FOREVER};

/// `current` 指针的缓存寿命。再短就是每个请求都读一次盘，再长回滚就不「立刻」了。
pub const CURRENT_TTL: Duration = Duration::from_secs(1);

/// 只说对象存储里有什么。隧道在不在线是另一回事，问 `tunnel::Tunnels`——
/// 一个 slug 可以既上传过又开着隧道，两边都不知道对方的存在，由 `app.rs` 决定先看谁。
#[derive(Debug, Clone)]
pub enum SiteState {
    Live(Arc<Manifest>),
    /// 没有这个 slug，或者对象存储里被删了。
    Missing,
    /// 对象存储这一刻不可用；不能把它缓存或伪装成作品不存在。
    Unavailable,
    Expired(Arc<Manifest>),
}

#[derive(Debug, Clone)]
enum ManifestLoad {
    Found(Arc<Manifest>),
    Missing,
}

pub struct SiteStore {
    store: FsStore,
    /// 清单按 `(slug, version)` 不可变，留多久都行——但**不能无限多个**：
    /// 随机打子域名的扫描器每个 slug 都会让我们读一次盘（`crate::cache` 的上限管着这件事）。
    manifests: CachedMap<(String, u32), ManifestLoad>,
    /// 「这个 slug 现在是第几版」。`None` 表示没有这个作品，那个答案也要缓存——
    /// 打进来的多数 slug 根本不存在。
    current: CachedMap<String, Option<u32>>,
}

impl SiteStore {
    pub fn new(store: FsStore) -> Self {
        Self::with_ttl(store, CURRENT_TTL)
    }

    pub fn with_ttl(store: FsStore, ttl: Duration) -> Self {
        Self {
            store,
            manifests: CachedMap::new(FOREVER),
            current: CachedMap::new(ttl),
        }
    }

    pub fn store(&self) -> &FsStore {
        &self.store
    }

    pub async fn resolve(&self, slug: &str) -> SiteState {
        let version = match self.current_version(slug).await {
            Ok(Some(version)) => version,
            Ok(None) => return SiteState::Missing,
            Err(()) => return SiteState::Unavailable,
        };
        let manifest = match self.manifest(slug, version).await {
            Ok(Some(manifest)) => manifest,
            Ok(None) => {
                tracing::warn!(slug, version, "current 指向的清单不存在");
                return SiteState::Unavailable;
            }
            Err(()) => return SiteState::Unavailable,
        };
        if is_expired(&manifest, OffsetDateTime::now_utc()) {
            return SiteState::Expired(manifest);
        }
        SiteState::Live(manifest)
    }

    async fn current_version(&self, slug: &str) -> Result<Option<u32>, ()> {
        let loaded = self
            .current
            .get(slug.to_string(), || async {
                match self.store.get_current(slug).await {
                    Ok(cur) => Some(cur.map(|c| c.version)),
                    Err(err) => {
                        tracing::warn!(slug, %err, "读 current 指针失败");
                        // 读盘出错和「没有这个作品」不一样：别把它缓存成「没有」，
                        // 下一次请求再试一遍。
                        None
                    }
                }
            })
            .await
            .ok_or(())?;
        Ok(*loaded)
    }

    async fn manifest(&self, slug: &str, version: u32) -> Result<Option<Arc<Manifest>>, ()> {
        let loaded = self
            .manifests
            .get((slug.to_string(), version), || async {
                match self.store.get_manifest(slug, version).await {
                    Ok(Some(manifest)) => Some(ManifestLoad::Found(Arc::new(manifest))),
                    Ok(None) => Some(ManifestLoad::Missing),
                    Err(err) => {
                        tracing::warn!(slug, version, %err, "读清单失败");
                        None
                    }
                }
            })
            .await
            .ok_or(())?;
        match loaded.as_ref() {
            ManifestLoad::Found(manifest) => Ok(Some(manifest.clone())),
            ManifestLoad::Missing => Ok(None),
        }
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
            summary: None,
            cover: None,
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
        assert!(is_expired(
            &manifest(Some("2026-09-07T19:00:00+08:00")),
            now
        ));
        // 解析不了就当没过期。
        assert!(!is_expired(&manifest(Some("明天")), now));
    }
}
