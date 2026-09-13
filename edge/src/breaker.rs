//! 每 slug 每小时的流量熔断（DESIGN §4.8、§6）。
//!
//! 免费档的月配额挡不住分钟级的 DDoS——SIMMER.io 2025-04 就是这样死的，有 billing alert
//! 也没来得及，创始人最后删掉自己的存储桶止血。所以额度是双重上限：月配额在控制面那一层，
//! 这里是第二重，**在边缘本地判定、不回源**。控制面被打满的时候，正是最需要熔断的时候。
//!
//! 实现刻意做得很笨：进程内一个 `HashMap<slug, 12 个五分钟格子>`，滚动一小时求和。
//!
//! - **重启后计数归零**，这是接受的代价。熔断是止血不是记账，真正的账在控制面按 60 秒
//!   上报的用量里（DESIGN §4.5）。边缘重启一次就白送一小时额度，好过为了准确在热路径上写盘。
//! - **多个边缘节点各算各的**：v0.1 只有一个节点（DESIGN §4.4）；将来多节点时上限等于
//!   逐节点上限之和，那时要么按节点数分摊、要么真的共享一份计数，届时再改。
//! - 计的是**准备发出去的字节数**（`Content-Length`），不是实际写进 socket 的字节数。
//!   玩家中途取消下载会被算全额；换取的是不必给每个流挂一个计数器。

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use playtest_common::limits;
use playtest_common::manifest::Manifest;

/// 滚动窗口长度。DESIGN §4.8 说的就是「每 slug 每小时」。
pub const WINDOW: Duration = Duration::from_secs(60 * 60);
/// 窗口切成几格。12 格 = 每格 5 分钟：格子越细内存越多，越粗窗口边缘抖得越厉害。
const BUCKETS: usize = 12;
const BUCKET_SECS: u64 = WINDOW.as_secs() / BUCKETS as u64;

/// 最多同时盯多少个 slug。超过就把窗口里已经空了的那些丢掉——被刷的时候
/// 攻击者可以枚举 slug，这张表不能跟着无限长。
const MAX_TRACKED: usize = 4096;

/// 这个 slug 这一小时的上限。匿名链接更紧：它 24 小时总共才 1 GiB（DESIGN §6）。
pub fn limit_for(manifest: &Manifest) -> u64 {
    if manifest.expires_at.is_some() {
        limits::ANON_SLUG_HOURLY_BYTES
    } else {
        limits::SLUG_HOURLY_BYTES
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Verdict {
    /// 还能出。
    pub allowed: bool,
    /// 滚动一小时里已经出了多少字节。
    pub total: u64,
    /// 最早可能好转的秒数，给 `Retry-After`。
    pub retry_after: u64,
    /// 这一次是本轮第一次超。只有它写事件——被刷的时候日志自己会变成放大器。
    pub first_trip: bool,
}

#[derive(Debug, Default)]
struct Counter {
    /// 每格的字节数，环形复用。
    buckets: [u64; BUCKETS],
    /// `buckets` 里最新那一格对应的格号（从 [`Breaker`] 创建那一刻算起）。
    newest: u64,
    /// 已经为这一轮超限写过事件了。掉回上限之下时清掉。
    reported: bool,
}

impl Counter {
    /// 把时间推到 `slot`，中间跳过的格子清零。
    fn roll(&mut self, slot: u64) {
        if slot <= self.newest {
            return;
        }
        let steps = (slot - self.newest).min(BUCKETS as u64);
        for step in 1..=steps {
            let idx = ((self.newest + step) % BUCKETS as u64) as usize;
            self.buckets[idx] = 0;
        }
        self.newest = slot;
    }

    fn total(&self) -> u64 {
        self.buckets.iter().fold(0u64, |a, b| a.saturating_add(*b))
    }

    fn add(&mut self, bytes: u64) {
        let idx = (self.newest % BUCKETS as u64) as usize;
        self.buckets[idx] = self.buckets[idx].saturating_add(bytes);
    }
}

#[derive(Debug)]
pub struct Breaker {
    origin: Instant,
    slugs: Mutex<HashMap<String, Counter>>,
}

impl Default for Breaker {
    fn default() -> Self {
        Self::new()
    }
}

impl Breaker {
    pub fn new() -> Self {
        Self {
            origin: Instant::now(),
            slugs: Mutex::new(HashMap::new()),
        }
    }

    /// 这个 slug 现在还能不能出东西。**不改计数**，判定在出字节之前做。
    pub fn check(&self, slug: &str, limit: u64) -> Verdict {
        self.check_at(slug, limit, Instant::now())
    }

    /// 记下准备发出去的字节。判定和计数分开，是因为 304、HEAD、416 这些不出体的响应
    /// 也要过判定，但不该记账。
    pub fn record(&self, slug: &str, bytes: u64) {
        self.record_at(slug, bytes, Instant::now())
    }

    pub fn check_at(&self, slug: &str, limit: u64, now: Instant) -> Verdict {
        let slot = self.slot(now);
        let Ok(mut slugs) = self.slugs.lock() else {
            // 锁中毒说明别的线程在持锁时 panic 了。熔断是止血手段，坏了就放行——
            // 让一个 slug 多出一点流量，好过把所有作品一起关掉。
            tracing::warn!(slug, "熔断计数的锁坏了，这一次放行");
            return Verdict {
                allowed: true,
                total: 0,
                retry_after: 0,
                first_trip: false,
            };
        };
        let Some(counter) = slugs.get_mut(slug) else {
            return Verdict {
                allowed: true,
                total: 0,
                retry_after: 0,
                first_trip: false,
            };
        };
        counter.roll(slot);
        let total = counter.total();
        if total < limit {
            counter.reported = false;
            return Verdict {
                allowed: true,
                total,
                retry_after: 0,
                first_trip: false,
            };
        }
        let first_trip = !counter.reported;
        counter.reported = true;
        Verdict {
            allowed: false,
            total,
            retry_after: self.retry_after(now),
            first_trip,
        }
    }

    pub fn record_at(&self, slug: &str, bytes: u64, now: Instant) {
        if bytes == 0 {
            return;
        }
        let slot = self.slot(now);
        let Ok(mut slugs) = self.slugs.lock() else {
            tracing::warn!(slug, "熔断计数的锁坏了，这一次不计数");
            return;
        };
        if !slugs.contains_key(slug) && slugs.len() >= MAX_TRACKED {
            slugs.retain(|_, c| {
                c.roll(slot);
                c.total() > 0
            });
        }
        // 命中时不分配，只有第一次见到这个 slug 才复制一次字符串。
        if !slugs.contains_key(slug) {
            slugs.insert(slug.to_string(), Counter::default());
        }
        if let Some(counter) = slugs.get_mut(slug) {
            counter.roll(slot);
            counter.add(bytes);
        }
    }

    /// 滚动一小时里已经出了多少。测试和将来的用量上报用。
    pub fn total(&self, slug: &str) -> u64 {
        self.total_at(slug, Instant::now())
    }

    pub fn total_at(&self, slug: &str, now: Instant) -> u64 {
        let slot = self.slot(now);
        let Ok(mut slugs) = self.slugs.lock() else {
            return 0;
        };
        match slugs.get_mut(slug) {
            Some(counter) => {
                counter.roll(slot);
                counter.total()
            }
            None => 0,
        }
    }

    fn slot(&self, now: Instant) -> u64 {
        now.saturating_duration_since(self.origin).as_secs() / BUCKET_SECS
    }

    /// 最老的那一格什么时候掉出窗口——在那之前总量不可能变小。
    fn retry_after(&self, now: Instant) -> u64 {
        let elapsed = now.saturating_duration_since(self.origin).as_secs();
        let next_boundary = (elapsed / BUCKET_SECS + 1) * BUCKET_SECS;
        (next_boundary - elapsed).max(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use playtest_common::manifest::{GateMode, Manifest, SCHEMA};

    const SLUG: &str = "brisk-otter-41";

    fn manifest(expires_at: Option<&str>) -> Manifest {
        Manifest {
            schema: SCHEMA,
            slug: SLUG.into(),
            version: 7,
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
            kind: playtest_common::manifest::WorkKind::Web,
            entry: None,
            article: None,
            files: vec![],
        }
    }

    #[test]
    fn anonymous_links_get_the_tighter_limit() {
        assert_eq!(limit_for(&manifest(None)), limits::SLUG_HOURLY_BYTES);
        assert_eq!(
            limit_for(&manifest(Some("2026-09-08T00:00:00Z"))),
            limits::ANON_SLUG_HOURLY_BYTES
        );
        const { assert!(limits::ANON_SLUG_HOURLY_BYTES < limits::SLUG_HOURLY_BYTES) };
    }

    #[test]
    fn a_quiet_slug_is_never_tripped() {
        let b = Breaker::new();
        let v = b.check(SLUG, 100);
        assert!(v.allowed);
        assert_eq!(v.total, 0);
        assert_eq!(b.total(SLUG), 0);
    }

    #[test]
    fn trips_at_the_limit_and_reports_only_once() {
        let b = Breaker::new();
        let t0 = b.origin;
        b.record_at(SLUG, 60, t0);
        assert!(b.check_at(SLUG, 100, t0).allowed);

        b.record_at(SLUG, 40, t0);
        let v = b.check_at(SLUG, 100, t0);
        assert!(!v.allowed, "到了上限就是超了，不是超过才算");
        assert_eq!(v.total, 100);
        assert!(v.first_trip);
        assert!(v.retry_after >= 1);

        // 后面每一次还是拦，但不再重复写事件。
        for _ in 0..5 {
            let again = b.check_at(SLUG, 100, t0);
            assert!(!again.allowed);
            assert!(!again.first_trip);
        }
    }

    #[test]
    fn each_slug_counts_on_its_own() {
        let b = Breaker::new();
        let t0 = b.origin;
        b.record_at(SLUG, 500, t0);
        assert!(!b.check_at(SLUG, 100, t0).allowed);
        assert!(b.check_at("keen-gecko-9", 100, t0).allowed);
    }

    #[test]
    fn the_window_rolls_off_after_an_hour() {
        let b = Breaker::new();
        let t0 = b.origin;
        b.record_at(SLUG, 100, t0);
        assert!(!b.check_at(SLUG, 100, t0).allowed);

        // 55 分钟后那一格还在窗口里。
        let almost = t0 + Duration::from_secs(55 * 60);
        assert_eq!(b.total_at(SLUG, almost), 100);
        assert!(!b.check_at(SLUG, 100, almost).allowed);

        // 一小时零一格之后掉出去，重新放行。
        let after = t0 + WINDOW + Duration::from_secs(BUCKET_SECS);
        assert_eq!(b.total_at(SLUG, after), 0);
        let v = b.check_at(SLUG, 100, after);
        assert!(v.allowed);

        // 掉回上限之下之后再超，会重新报一次。
        b.record_at(SLUG, 100, after);
        assert!(b.check_at(SLUG, 100, after).first_trip);
    }

    #[test]
    fn a_long_silence_does_not_leave_stale_bytes_behind() {
        let b = Breaker::new();
        let t0 = b.origin;
        b.record_at(SLUG, 100, t0);
        // 隔了一整天再来一次：中间的格子必须是干净的，不能把昨天的字节转回来。
        let tomorrow = t0 + Duration::from_secs(24 * 60 * 60);
        b.record_at(SLUG, 7, tomorrow);
        assert_eq!(b.total_at(SLUG, tomorrow), 7);
    }

    #[test]
    fn bytes_accumulate_across_buckets() {
        let b = Breaker::new();
        let t0 = b.origin;
        for i in 0..BUCKETS as u64 {
            b.record_at(SLUG, 10, t0 + Duration::from_secs(i * BUCKET_SECS));
        }
        let last = t0 + Duration::from_secs((BUCKETS as u64 - 1) * BUCKET_SECS);
        assert_eq!(b.total_at(SLUG, last), 10 * BUCKETS as u64);
    }

    #[test]
    fn retry_after_never_promises_zero() {
        let b = Breaker::new();
        let t0 = b.origin;
        b.record_at(SLUG, 100, t0);
        for offset in [0, 1, 60, BUCKET_SECS - 1, BUCKET_SECS, BUCKET_SECS + 7] {
            let v = b.check_at(SLUG, 100, t0 + Duration::from_secs(offset));
            assert!(
                v.retry_after >= 1 && v.retry_after <= BUCKET_SECS,
                "{offset}"
            );
        }
    }
}
