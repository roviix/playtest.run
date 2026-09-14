//! 玩家页面上的时间怎么写。门禁页、邀请卡、广场卡三处原来各写了一份，
//! 「9 月 9 日」在一处多了个空格、在另一处少了个空格——现在只有这一份。
//!
//! 服务端没有访客的时区，一律按 UTC+8 写（玩家绝大多数在这个时区；DESIGN §5）。
//! 门禁页上那一行到期时间另外带 `<time datetime>`，有 JS 的浏览器会换成访客自己的时区。

use time::format_description::well_known::Rfc3339;
use time::{OffsetDateTime, UtcOffset};

/// 页面上写日期用的时区。
pub const OFFSET_HOURS: i8 = 8;

fn local(raw: &str) -> Option<OffsetDateTime> {
    let at = OffsetDateTime::parse(raw, &Rfc3339).ok()?;
    Some(at.to_offset(UtcOffset::from_hms(OFFSET_HOURS, 0, 0).ok()?))
}

pub fn month_short(month: time::Month) -> &'static str {
    match month {
        time::Month::January => "Jan",
        time::Month::February => "Feb",
        time::Month::March => "Mar",
        time::Month::April => "Apr",
        time::Month::May => "May",
        time::Month::June => "Jun",
        time::Month::July => "Jul",
        time::Month::August => "Aug",
        time::Month::September => "Sep",
        time::Month::October => "Oct",
        time::Month::November => "Nov",
        time::Month::December => "Dec",
    }
}

/// RFC 3339 → 「Sep 9」.
pub fn day(raw: &str) -> Option<String> {
    let at = local(raw)?;
    Some(format!("{} {}", month_short(at.month()), at.day()))
}

/// RFC 3339 → 「Sep 8, 20:59」.
pub fn day_time(raw: &str) -> Option<String> {
    let at = local(raw)?;
    Some(format!(
        "{} {}, {:02}:{:02}",
        month_short(at.month()),
        at.day(),
        at.hour(),
        at.minute()
    ))
}

/// 门禁页上那句到期时间：第一个值给 `<time datetime>`（机器读），
/// 第二个是没有 JS 时直接显示的那串，带着时区免得被当成本地时间。
pub fn deadline(raw: &str) -> Option<(String, String)> {
    let at = OffsetDateTime::parse(raw, &Rfc3339).ok()?;
    let human = format!("{} (UTC+{OFFSET_HOURS})", day_time(raw)?);
    Some((at.format(&Rfc3339).ok()?, human))
}

/// 「Just now」「40m ago」「2d ago」.
pub fn ago(then: OffsetDateTime, now: OffsetDateTime) -> String {
    let span = now - then;
    let minutes = span.whole_minutes();
    if minutes < 1 {
        "Just now".to_string()
    } else if minutes < 60 {
        format!("{minutes}m ago")
    } else if span.whole_hours() < 24 {
        format!("{}h ago", span.whole_hours())
    } else {
        format!("{}d ago", span.whole_days().max(1))
    }
}

/// 「5h left」.
pub fn remaining(until: OffsetDateTime, now: OffsetDateTime) -> String {
    let left = until - now;
    let minutes = left.whole_minutes();
    if minutes <= 0 {
        "Expiring soon".to_string()
    } else if minutes < 60 {
        format!("{minutes}m left")
    } else {
        format!("{}h left", left.whole_hours())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    #[test]
    fn dates_are_written_in_utc_plus_eight() {
        assert_eq!(day("2026-09-08T20:30:00Z").as_deref(), Some("Sep 9"));
        assert_eq!(
            day_time("2026-09-08T12:59:00Z").as_deref(),
            Some("Sep 8, 20:59")
        );
        let (machine, human) = deadline("2026-09-08T04:30:00Z").unwrap();
        assert_eq!(machine, "2026-09-08T04:30:00Z");
        assert_eq!(human, "Sep 8, 12:30 (UTC+8)");
        assert!(day("not a date").is_none());
    }

    #[test]
    fn relative_wording() {
        let now = datetime!(2026-09-08 04:00:00 UTC);
        assert_eq!(ago(datetime!(2026-09-08 03:59:30 UTC), now), "Just now");
        assert_eq!(ago(datetime!(2026-09-08 03:20:00 UTC), now), "40m ago");
        assert_eq!(ago(datetime!(2026-09-08 03:00:00 UTC), now), "1h ago");
        assert_eq!(ago(datetime!(2026-09-06 04:00:00 UTC), now), "2d ago");
        assert_eq!(
            remaining(datetime!(2026-09-08 09:30:00 UTC), now),
            "5h left"
        );
        assert_eq!(
            remaining(datetime!(2026-09-08 04:20:00 UTC), now),
            "20m left"
        );
        assert_eq!(
            remaining(datetime!(2026-09-08 03:00:00 UTC), now),
            "Expiring soon"
        );
    }
}
