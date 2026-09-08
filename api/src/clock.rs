//! 时间。库里和响应里出现的时间统一是 UTC、秒精度的 RFC 3339，例如 `2026-09-07T11:22:33Z`。
//!
//! 秒精度是为了人能直接读懂 `.data/api.sqlite` 里的一行；固定这个形态之后，
//! 字符串排序和时间排序一致，SQL 里可以直接比大小。

use time::format_description::well_known::Rfc3339;
use time::{Duration, OffsetDateTime};

pub fn now() -> OffsetDateTime {
    OffsetDateTime::now_utc()
        .replace_nanosecond(0)
        .expect("0 纳秒是合法的")
}

pub fn format(t: OffsetDateTime) -> String {
    t.format(&Rfc3339)
        .expect("UTC 时间总能格式化成 RFC 3339")
}

pub fn now_string() -> String {
    format(now())
}

pub fn plus_hours(t: OffsetDateTime, hours: u64) -> OffsetDateTime {
    t + Duration::hours(hours as i64)
}

pub fn parse(s: &str) -> Option<OffsetDateTime> {
    OffsetDateTime::parse(s, &Rfc3339).ok()
}

/// 解析不出来的时间一律当作「已经过期」：宁可让人重新拿一个链接，
/// 也不能因为库里有一行坏数据就放行一个说不清有效期的令牌。
pub fn is_expired(stored: &str) -> bool {
    match parse(stored) {
        Some(t) => t <= now(),
        None => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_without_fraction() {
        let s = now_string();
        assert!(s.ends_with('Z'), "{s}");
        assert!(!s.contains('.'), "{s}");
        assert_eq!(s.len(), "2026-09-07T11:22:33Z".len(), "{s}");
    }

    #[test]
    fn string_order_matches_time_order() {
        let a = now();
        let b = plus_hours(a, 24);
        assert!(format(a) < format(b));
    }

    #[test]
    fn expiry_treats_garbage_as_expired() {
        assert!(is_expired("昨天"));
        assert!(is_expired(&format(plus_hours(now(), 0))));
        assert!(!is_expired(&format(plus_hours(now(), 1))));
    }
}
