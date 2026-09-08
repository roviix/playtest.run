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
        assert_eq!(human("明天"), "明天");
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
}
