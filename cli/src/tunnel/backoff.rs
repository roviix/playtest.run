//! 断了之后等多久再连：1 秒起步，每次翻倍，30 秒封顶，带抖动（DESIGN §4.3）。

use std::time::Duration;

use playtest_common::tunnel::{BACKOFF_MAX_SECS, BACKOFF_MIN_SECS};

#[derive(Debug)]
pub struct Backoff {
    /// 下一次的基准秒数，真正等的时间在它的一半到它之间。
    base_secs: u64,
}

impl Default for Backoff {
    fn default() -> Self {
        Self::new()
    }
}

impl Backoff {
    pub fn new() -> Self {
        Self {
            base_secs: BACKOFF_MIN_SECS,
        }
    }

    /// 连上了就从头再来，下一次断线不该继承上一次的等待。
    pub fn reset(&mut self) {
        self.base_secs = BACKOFF_MIN_SECS;
    }

    /// 这一次要等多久，并把下一次的基准翻倍。
    pub fn next_wait(&mut self) -> Duration {
        self.next_with(jitter())
    }

    /// 抖动比例由外面给，好测。
    fn next_with(&mut self, fraction: f64) -> Duration {
        let base = self.base_secs as f64;
        self.base_secs = (self.base_secs * 2).min(BACKOFF_MAX_SECS);
        // 一半固定、一半抖：等待落在 [base/2, base]。
        // 不用全抖（[0, base]）是因为那样第一次重连几乎立刻发生，一批 CLI 同时醒来反而更挤。
        let half = base / 2.0;
        Duration::from_secs_f64(half + half * fraction.clamp(0.0, 1.0))
    }
}

/// 0 到 1 之间的一个随机数。
///
/// 为了抖动引一个随机数库不值得：标准库的 `RandomState` 每建一个都换种子，
/// 拿它哈希一次空输入就够散了。这个数只用来错开重连时刻，不用于任何安全用途。
fn jitter() -> f64 {
    use std::hash::{BuildHasher, Hasher};
    let n = std::collections::hash_map::RandomState::new()
        .build_hasher()
        .finish();
    // 取高 53 位：f64 能精确表示的整数就到这儿。
    (n >> 11) as f64 / (1u64 << 53) as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_base_doubles_up_to_the_cap_and_stays_there() {
        let mut backoff = Backoff::new();
        let bases: Vec<u64> = (0..8)
            .map(|_| {
                let base = backoff.base_secs;
                backoff.next_with(1.0);
                base
            })
            .collect();
        assert_eq!(bases, [1, 2, 4, 8, 16, 30, 30, 30]);
    }

    #[test]
    fn every_wait_lands_between_half_the_base_and_the_base() {
        let mut backoff = Backoff::new();
        for expected in [1u64, 2, 4, 8, 16, 30, 30] {
            let base = backoff.base_secs;
            assert_eq!(base, expected);
            let low = backoff.next_with(0.0);
            backoff.base_secs = base;
            let high = backoff.next_with(1.0);
            assert_eq!(low, Duration::from_secs_f64(base as f64 / 2.0));
            assert_eq!(high, Duration::from_secs(base));
        }
    }

    #[test]
    fn connecting_again_starts_the_wait_over() {
        let mut backoff = Backoff::new();
        for _ in 0..5 {
            backoff.next_wait();
        }
        backoff.reset();
        assert!(backoff.next_wait() <= Duration::from_secs(BACKOFF_MIN_SECS));
    }

    #[test]
    fn the_jitter_stays_inside_zero_to_one() {
        for _ in 0..200 {
            let f = jitter();
            assert!((0.0..1.0).contains(&f), "抖动跑出范围了：{f}");
        }
    }

    #[test]
    fn two_processes_do_not_wake_up_at_the_same_moment() {
        // 抖动的意义就在这里：一样的退避序列，等出来的时间不该一样。
        let mut a = Backoff::new();
        let mut b = Backoff::new();
        let differ = (0..10).any(|_| a.next_wait() != b.next_wait());
        assert!(differ, "十次都一模一样，抖动没起作用");
    }
}
