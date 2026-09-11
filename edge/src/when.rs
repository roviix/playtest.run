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

/// RFC 3339 → 「9 月 9 日」。版本那一行的日期，门禁页和邀请卡票根是同一种写法。
pub fn day(raw: &str) -> Option<String> {
    let at = local(raw)?;
    Some(format!("{} 月 {} 日", at.month() as u8, at.day()))
}

/// RFC 3339 → 「9 月 10 日 20:59」。到期时间用：日期不够，玩家要知道还来得及不。
pub fn day_time(raw: &str) -> Option<String> {
    let at = local(raw)?;
    Some(format!(
        "{} 月 {} 日 {:02}:{:02}",
        at.month() as u8,
        at.day(),
        at.hour(),
        at.minute()
    ))
}

/// 门禁页上那句到期时间：第一个值给 `<time datetime>`（机器读），
/// 第二个是没有 JS 时直接显示的那串，带着时区免得被当成本地时间。
pub fn deadline(raw: &str) -> Option<(String, String)> {
    let at = OffsetDateTime::parse(raw, &Rfc3339).ok()?;
    let human = format!("{}（UTC+{OFFSET_HOURS}）", day_time(raw)?);
    Some((at.format(&Rfc3339).ok()?, human))
}

/// 「刚刚发布」「40 分钟前」「2 天前」。广场卡右下那件事实。
pub fn ago(then: OffsetDateTime, now: OffsetDateTime) -> String {
    let span = now - then;
    let minutes = span.whole_minutes();
    if minutes < 1 {
        "刚刚发布".to_string()
    } else if minutes < 60 {
        format!("{minutes} 分钟前")
    } else if span.whole_hours() < 24 {
        format!("{} 小时前", span.whole_hours())
    } else {
        format!("{} 天前", span.whole_days().max(1))
    }
}

/// 「还剩 5 小时」。匿名作品才有到期，广场页缓存 30 秒，误差一分钟以内。
pub fn remaining(until: OffsetDateTime, now: OffsetDateTime) -> String {
    let left = until - now;
    let minutes = left.whole_minutes();
    if minutes <= 0 {
        "即将下线".to_string()
    } else if minutes < 60 {
        format!("还剩 {minutes} 分钟")
    } else {
        format!("还剩 {} 小时", left.whole_hours())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    #[test]
    fn dates_are_written_in_utc_plus_eight() {
        assert_eq!(day("2026-09-08T20:30:00Z").as_deref(), Some("9 月 9 日"));
        assert_eq!(
            day_time("2026-09-08T12:59:00Z").as_deref(),
            Some("9 月 8 日 20:59")
        );
        let (machine, human) = deadline("2026-09-08T04:30:00Z").unwrap();
        assert_eq!(machine, "2026-09-08T04:30:00Z");
        assert_eq!(human, "9 月 8 日 12:30（UTC+8）");
        assert!(day("not a date").is_none());
    }

    #[test]
    fn relative_wording() {
        let now = datetime!(2026-09-08 04:00:00 UTC);
        assert_eq!(ago(datetime!(2026-09-08 03:59:30 UTC), now), "刚刚发布");
        assert_eq!(ago(datetime!(2026-09-08 03:20:00 UTC), now), "40 分钟前");
        assert_eq!(ago(datetime!(2026-09-08 03:00:00 UTC), now), "1 小时前");
        assert_eq!(ago(datetime!(2026-09-06 04:00:00 UTC), now), "2 天前");
        assert_eq!(
            remaining(datetime!(2026-09-08 09:30:00 UTC), now),
            "还剩 5 小时"
        );
        assert_eq!(
            remaining(datetime!(2026-09-08 04:20:00 UTC), now),
            "还剩 20 分钟"
        );
        assert_eq!(
            remaining(datetime!(2026-09-08 03:00:00 UTC), now),
            "即将下线"
        );
    }
}
