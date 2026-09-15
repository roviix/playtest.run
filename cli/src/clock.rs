//! 时间：解析控制面给的 RFC 3339，显示成本地时间。

use std::sync::OnceLock;

use time::format_description::well_known::Rfc3339;
use time::{OffsetDateTime, UtcOffset};

static LOCAL_OFFSET: OnceLock<Option<UtcOffset>> = OnceLock::new();

/// 在起线程之前调一次。取不到时区就退回 UTC，并在显示的时候标出来，不假装是本地时间。
pub fn init_local_offset() {
    let _ = LOCAL_OFFSET.set(UtcOffset::current_local_offset().ok());
}

pub fn now() -> OffsetDateTime {
    OffsetDateTime::now_utc()
}

pub fn parse_rfc3339(s: &str) -> Option<OffsetDateTime> {
    OffsetDateTime::parse(s, &Rfc3339).ok()
}

/// 给人看的时刻：`2026-09-08 11:30`。时区取不到时写成 `2026-09-08 03:30 UTC`。
pub fn human(s: &str) -> String {
    let Some(t) = parse_rfc3339(s) else {
        // 解析不了就把服务器给的原样打出来，总比编一个时间强。
        return s.to_string();
    };
    match LOCAL_OFFSET.get().copied().flatten() {
        Some(offset) => format_hm(t.to_offset(offset), ""),
        None => format_hm(t.to_offset(UtcOffset::UTC), " UTC"),
    }
}

/// 到那一刻还有多久。已经过了、或者解析不了，都是 `None`——不确定的事不说。
pub fn remaining(s: &str, now: OffsetDateTime) -> Option<time::Duration> {
    let t = parse_rfc3339(s)?;
    let left = t - now;
    (left > time::Duration::ZERO).then_some(left)
}

/// 「1h 20m」「35 minutes」「under a minute」。分钟以下不说秒，那不是人做决定的粒度。
pub fn human_duration(d: time::Duration) -> String {
    let minutes = d.whole_minutes();
    match (minutes / 60, minutes % 60) {
        (0, 0) => "under a minute".to_string(),
        (0, m) => crate::ui::count(m as u64, "minute"),
        (h, 0) => crate::ui::count(h as u64, "hour"),
        (h, m) => format!("{h}h {m}m"),
    }
}

fn format_hm(t: OffsetDateTime, suffix: &str) -> String {
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}{suffix}",
        t.year(),
        u8::from(t.month()),
        t.day(),
        t.hour(),
        t.minute()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unparsable_input_is_passed_through() {
        assert_eq!(human("tomorrow"), "tomorrow");
    }

    #[test]
    fn parses_rfc3339() {
        let t = parse_rfc3339("2026-09-08T03:30:00Z").expect("应该能解析");
        assert_eq!(t.year(), 2026);
        assert_eq!(t.hour(), 3);
    }

    #[test]
    fn formats_to_minutes() {
        let t = parse_rfc3339("2026-09-08T03:30:45Z").unwrap();
        assert_eq!(format_hm(t, " UTC"), "2026-09-08 03:30 UTC");
    }

    #[test]
    fn remaining_is_none_once_past_or_unreadable() {
        let now = parse_rfc3339("2026-09-08T03:30:00Z").unwrap();
        assert_eq!(
            remaining("2026-09-08T04:50:00Z", now),
            Some(time::Duration::minutes(80))
        );
        assert_eq!(remaining("2026-09-08T03:30:00Z", now), None);
        assert_eq!(remaining("2026-09-08T03:00:00Z", now), None);
        assert_eq!(remaining("tomorrow", now), None);
    }

    #[test]
    fn durations_read_like_a_person_would_say_them() {
        assert_eq!(
            human_duration(time::Duration::seconds(30)),
            "under a minute"
        );
        assert_eq!(human_duration(time::Duration::minutes(35)), "35 minutes");
        assert_eq!(human_duration(time::Duration::minutes(120)), "2 hours");
        assert_eq!(human_duration(time::Duration::minutes(80)), "1h 20m");
    }
}
