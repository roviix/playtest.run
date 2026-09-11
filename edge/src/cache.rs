//! 边缘从对象存储读东西时的两种缓存。
//!
//! 边缘对控制面只有写方向的调用，**从不为读回源**（DESIGN §4.1）：作品、广场、名额、
//! 关注数全部从对象存储里读控制面写好的文件。所以「读一份文件、短时间内别再读」这件事
//! 在边缘出现了五次——`current` 指针、清单、`live.json`、`plaza.json`、`capabilities.json`
//! ——五份几乎一样的代码，各有各的 TTL 判断和锁。
//!
//! 这里只有两个东西：
//!
//! - [`Cached`]：一份全局的文件（广场、capabilities）。
//! - [`CachedMap`]：一个 slug 一份（`current`、清单、`live.json`）。
//!
//! **上限不是可选的。** 随机打子域名的扫描器每个 slug 都会让我们读一次盘，无界的 map
//! 就是它们的免费内存。`live.json` 那一份原来记着这件事并加了上限，清单那一份没有
//! ——同一个论证对它一样成立（扫描器只要打到存在的 slug 的多个版本），所以上限做进类型里，
//! 新加一个缓存不会再漏掉。

use std::collections::HashMap;
use std::hash::Hash;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// 一个 map 缓存里最多留多少条。满了先清过期的，还满就整个清掉——
/// 代价只是下一波多碰几次磁盘，比被人用随机子域名撑爆内存好。
pub const CAP: usize = 1024;

/// 不会过期的东西（按内容寻址的清单）用它。仍然受 [`CAP`] 管。
pub const FOREVER: Duration = Duration::from_secs(60 * 60 * 24 * 365);

/// 一份全局的文件。读不到就用调用方给的默认值，不报错。
pub struct Cached<T> {
    slot: Mutex<Option<(Instant, Arc<T>)>>,
    ttl: Duration,
}

impl<T> Cached<T> {
    pub fn new(ttl: Duration) -> Self {
        Self {
            slot: Mutex::new(None),
            ttl,
        }
    }

    /// 拿缓存里那一份；过期或没有就调 `load`。
    ///
    /// `load` 回 `None` 表示读不到（文件还没写过、坏了、控制面挂着）——那也要缓存下来，
    /// 否则每个请求都去碰一次盘，而「控制面挂了」恰恰是请求最多的时候。
    pub async fn get<F, Fut>(&self, load: F) -> Arc<T>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Option<T>>,
        T: Default,
    {
        if let Some(hit) = self.slot.lock().ok().and_then(|s| {
            s.as_ref()
                .filter(|(at, _)| at.elapsed() < self.ttl)
                .map(|(_, v)| v.clone())
        }) {
            return hit;
        }
        let value = Arc::new(load().await.unwrap_or_default());
        if let Ok(mut slot) = self.slot.lock() {
            *slot = Some((Instant::now(), value.clone()));
        }
        value
    }
}

/// 一个键一份。带 TTL 和条数上限。
pub struct CachedMap<K, V> {
    entries: Mutex<HashMap<K, (Instant, Arc<V>)>>,
    ttl: Duration,
    cap: usize,
}

impl<K: Eq + Hash + Clone, V> CachedMap<K, V> {
    pub fn new(ttl: Duration) -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
            ttl,
            cap: CAP,
        }
    }

    /// 命中就返回，否则 `load`。`load` 回 `None` 时**不缓存**——
    /// 「这个 slug 不存在」和「这份文件读不到」由调用方区分：
    /// 前者用 [`CachedMap::get_or`] 把默认值也缓存下来，后者留给下一次重试。
    pub async fn get<F, Fut>(&self, key: K, load: F) -> Option<Arc<V>>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Option<V>>,
    {
        if let Some(hit) = self.hit(&key) {
            return Some(hit);
        }
        let value = Arc::new(load().await?);
        self.put(key, value.clone());
        Some(value)
    }

    /// 同上，但读不到时把默认值也缓存起来。
    ///
    /// 给 `live.json` 与 `current` 用：多数 slug 根本不存在（扫描器打进来的），
    /// 「不存在」这个答案本身值得缓存，否则每一次扫描都是一次磁盘往返。
    pub async fn get_or<F, Fut>(&self, key: K, load: F) -> Arc<V>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Option<V>>,
        V: Default,
    {
        if let Some(hit) = self.hit(&key) {
            return hit;
        }
        let value = Arc::new(load().await.unwrap_or_default());
        self.put(key, value.clone());
        value
    }

    fn hit(&self, key: &K) -> Option<Arc<V>> {
        self.entries.lock().ok().and_then(|e| {
            e.get(key)
                .filter(|(at, _)| at.elapsed() < self.ttl)
                .map(|(_, v)| v.clone())
        })
    }

    fn put(&self, key: K, value: Arc<V>) {
        let Ok(mut entries) = self.entries.lock() else {
            return;
        };
        if entries.len() >= self.cap {
            entries.retain(|_, (at, _)| at.elapsed() < self.ttl);
            if entries.len() >= self.cap {
                entries.clear();
            }
        }
        entries.insert(key, (Instant::now(), value));
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.entries.lock().unwrap().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[tokio::test]
    async fn a_global_file_is_read_once_inside_its_ttl() {
        let reads = AtomicUsize::new(0);
        let cache: Cached<String> = Cached::new(Duration::from_secs(30));
        for _ in 0..3 {
            let got = cache
                .get(|| async {
                    reads.fetch_add(1, Ordering::SeqCst);
                    Some("广场".to_string())
                })
                .await;
            assert_eq!(&*got, "广场");
        }
        assert_eq!(reads.load(Ordering::SeqCst), 1, "缓存期内只该读一次");
    }

    #[tokio::test]
    async fn a_file_that_is_not_there_yet_is_the_default_and_still_cached() {
        let reads = AtomicUsize::new(0);
        let cache: Cached<String> = Cached::new(Duration::from_secs(30));
        for _ in 0..3 {
            let got = cache
                .get(|| async {
                    reads.fetch_add(1, Ordering::SeqCst);
                    None
                })
                .await;
            assert!(got.is_empty());
        }
        // 控制面挂着的时候请求最多，不能每个请求都去碰一次盘。
        assert_eq!(reads.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn zero_ttl_reads_every_time() {
        let reads = AtomicUsize::new(0);
        let cache: Cached<String> = Cached::new(Duration::ZERO);
        for _ in 0..3 {
            cache
                .get(|| async {
                    reads.fetch_add(1, Ordering::SeqCst);
                    Some(String::new())
                })
                .await;
        }
        assert_eq!(reads.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn a_missing_key_is_not_cached_but_a_default_one_is() {
        let cache: CachedMap<String, String> = CachedMap::new(Duration::from_secs(30));
        assert!(cache
            .get("a".to_string(), || async { None })
            .await
            .is_none());
        assert_eq!(cache.len(), 0, "读不到就留给下一次重试");

        cache.get_or("b".to_string(), || async { None }).await;
        assert_eq!(cache.len(), 1, "「不存在」这个答案本身值得缓存");
    }

    /// 随机打子域名的扫描器不能把它撑大——这一条以前只有 live.json 那一份守着，
    /// 清单那一份漏了。现在上限在类型里，漏不掉。
    #[tokio::test]
    async fn scanning_random_keys_cannot_grow_it_forever() {
        let cache: CachedMap<String, String> = CachedMap::new(FOREVER);
        for i in 0..CAP + 50 {
            cache
                .get(format!("scan-{i}"), || async { Some(String::new()) })
                .await;
        }
        assert!(cache.len() <= CAP, "涨到了 {}", cache.len());
    }

    #[tokio::test]
    async fn entries_go_stale_on_their_own() {
        let cache: CachedMap<String, u32> = CachedMap::new(Duration::ZERO);
        cache.get("a".to_string(), || async { Some(1) }).await;
        let second = cache.get("a".to_string(), || async { Some(2) }).await;
        assert_eq!(*second.unwrap(), 2, "过期了就重读");
    }
}
