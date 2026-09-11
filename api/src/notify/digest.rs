//! 每周一封的新作品周报（DESIGN §3.6、§3.9）。
//!
//! 关注广场的人收它。没有新作品的那一周不发——「这周没什么新东西」也是一封信，
//! 收多了人就退订了，而退订是不可逆的。

use std::path::Path;

use playtest_common::boost::MAX_DIGEST_SLOTS;
use playtest_common::follow::{
    notice_url, DIGEST_HOUR, DIGEST_ISO_WEEKDAY, DIGEST_MAX_ITEMS, DIGEST_UTC_OFFSET_HOURS,
};
use rusqlite::Connection;
use time::{Date, OffsetDateTime, Time};

use crate::clock;
use crate::db;
use crate::scheduler::{Job, JobFuture};
use crate::state::AppState;

/// 周报只看最近这么久的新版本。
const WINDOW_DAYS: i64 = 7;

/// 下一次该发周报的时刻（UTC）。
///
/// 契约里的时间是「周一 09:00，UTC+8」；这里换算回 UTC 再找日历上的下一个。
/// 一天一天往前试而不是算术推：闰秒、月末、跨年都不用单独想。
pub fn next_run_at(after: OffsetDateTime) -> OffsetDateTime {
    let mut date = after.date();
    for _ in 0..8 {
        if let Some(candidate) = run_at_on(date) {
            if candidate > after {
                return candidate;
            }
        }
        date = date.next_day().expect("日历不会走到头");
    }
    // 走不到：八天里一定有一个周一。
    after + time::Duration::days(7)
}

/// 这一天如果是「周报日」，那一刻是几点（UTC）。
fn run_at_on(date: Date) -> Option<OffsetDateTime> {
    // 本地 09:00 换成 UTC 可能落到前一天：把小时数和日期一起挪。
    let raw = DIGEST_HOUR as i64 - DIGEST_UTC_OFFSET_HOURS as i64;
    let (day_shift, hour) = if raw < 0 {
        (-1, raw + 24)
    } else if raw >= 24 {
        (1, raw - 24)
    } else {
        (0, raw)
    };
    // `date` 是 UTC 上的那一天；对应的本地日期要反着挪回去才能判周几。
    let local_date = match day_shift {
        -1 => date.next_day()?,
        1 => date.previous_day()?,
        _ => date,
    };
    if local_date.weekday().number_from_monday() != DIGEST_ISO_WEEKDAY {
        return None;
    }
    let time = Time::from_hms(hour as u8, 0, 0).ok()?;
    Some(date.with_time(time).assume_utc())
}

/// 周报里的一项。
pub struct Item {
    pub title: String,
    pub developer: String,
    pub summary: Option<String>,
    pub url: String,
    /// 推广位来的那几条要明说（DESIGN §3.11「标『推广』」）。
    pub boosted: bool,
}

/// 攒出这一期的内容。没有新作品就是 `None`，这周不发。
pub fn compose(
    conn: &Connection,
    now: OffsetDateTime,
    site_url: impl Fn(&str) -> String,
) -> rusqlite::Result<Option<Vec<Item>>> {
    let now_text = clock::format(now);
    let since = clock::format(now - time::Duration::days(WINDOW_DAYS));

    let mut fresh: Vec<db::PlazaCandidate> = db::plaza_candidates(conn, &now_text)?
        .into_iter()
        .filter(|c| {
            c.site
                .listing
                .updated_at
                .as_deref()
                .is_some_and(|at| at >= since.as_str())
        })
        .collect();
    // 新的在前。
    fresh.sort_by(|a, b| b.site.listing.updated_at.cmp(&a.site.listing.updated_at));
    fresh.truncate(DIGEST_MAX_ITEMS);

    if fresh.is_empty() {
        return Ok(None);
    }

    let mut items: Vec<Item> = fresh
        .into_iter()
        .map(|c| Item {
            title: c.site.title.clone(),
            developer: c.developer.clone(),
            summary: c.site.listing.summary.clone(),
            url: notice_url(&site_url(&c.site.slug)),
            boosted: false,
        })
        .collect();

    // 推广那一段跟在后面，最多两条，每条都标出来。
    let mut promoted = 0;
    for boost in db::live_digest_boosts(conn)? {
        if promoted >= MAX_DIGEST_SLOTS {
            break;
        }
        let Some(site) = db::find_site(conn, &boost.slug)? else {
            continue;
        };
        let Some(owner) = db::site_owner(conn, &boost.slug)? else {
            continue;
        };
        items.push(Item {
            title: site.title,
            developer: owner.display_name,
            summary: site.listing.summary,
            url: notice_url(&site_url(&boost.slug)),
            boosted: true,
        });
        promoted += 1;
    }

    Ok(Some(items))
}

pub fn subject(now: OffsetDateTime) -> String {
    format!("playtest.run 本周新作品（{}）", &clock::format(now)[..10])
}

pub fn body(items: &[Item]) -> String {
    let mut text = String::from("这周有这些新版本可以试：\n");
    for item in items.iter().filter(|i| !i.boosted) {
        text.push_str(&format!("\n《{}》 {}\n", item.title, item.developer));
        if let Some(summary) = &item.summary {
            text.push_str(summary);
            text.push('\n');
        }
        text.push_str(&item.url);
        text.push('\n');
    }
    let promoted: Vec<&Item> = items.iter().filter(|i| i.boosted).collect();
    if !promoted.is_empty() {
        text.push_str("\n—\n推广\n");
        for item in promoted {
            text.push_str(&format!("\n《{}》 {}\n", item.title, item.developer));
            if let Some(summary) = &item.summary {
                text.push_str(summary);
                text.push('\n');
            }
            text.push_str(&item.url);
            text.push('\n');
        }
    }
    text
}

/// 攒好就给每个关注广场的人入队一封。返回入队的封数。
pub fn enqueue(
    conn: &Connection,
    now: OffsetDateTime,
    site_url: impl Fn(&str) -> String,
) -> rusqlite::Result<usize> {
    let Some(items) = compose(conn, now, site_url)? else {
        return Ok(0);
    };
    let subject = subject(now);
    let body = body(&items);
    let now_text = clock::format(now);
    let mut queued = 0;
    for recipient in db::confirmed_followers(conn, "plaza", None)? {
        // 周报只走邮件。浏览器通知适合「你关注的作品出新版本了」这种当下的事，
        // 一周一次的摘要塞进弹窗里没人看。
        let Some(_email) = recipient.email.as_deref() else {
            continue;
        };
        db::enqueue_notification(
            conn,
            &db::NewNotification {
                player_id: &recipient.player_id,
                kind: super::KIND_DIGEST,
                target_slug: None,
                subject: &subject,
                body: &body,
                url: None,
                channel: super::CHANNEL_EMAIL,
                not_before: &now_text,
                created_at: &now_text,
            },
        )?;
        queued += 1;
    }
    Ok(queued)
}

/// 上一期是什么时候发的。放在数据目录的一个小文件里，重启不会重发。
/// 不放库里是因为它跟迁移无关，而且出问题时删掉这个文件就能强制补发一期。
pub struct Marker {
    path: std::path::PathBuf,
}

#[derive(serde::Serialize, serde::Deserialize, Default)]
struct Stored {
    last_digest_at: Option<String>,
}

impl Marker {
    pub fn new(data_dir: &Path) -> Self {
        Self {
            path: data_dir.join("digest.json"),
        }
    }

    pub fn last(&self) -> Option<OffsetDateTime> {
        let text = std::fs::read_to_string(&self.path).ok()?;
        let stored: Stored = serde_json::from_str(&text).ok()?;
        stored.last_digest_at.as_deref().and_then(clock::parse)
    }

    pub fn record(&self, at: OffsetDateTime) {
        let stored = Stored {
            last_digest_at: Some(clock::format(at)),
        };
        if let Ok(text) = serde_json::to_string_pretty(&stored) {
            if let Err(e) = std::fs::write(&self.path, text) {
                tracing::warn!(error = %e, path = %self.path.display(), "记周报时间失败");
            }
        }
    }

    /// 该发了没有：到了这一期的点，而且这一期还没发过。
    pub fn due(&self, now: OffsetDateTime) -> bool {
        match self.last() {
            Some(last) => next_run_at(last) <= now,
            // 第一次跑：等到下一个周一，不要一起来就发一封。
            None => false,
        }
    }

    /// 第一次跑时把「上一期」记成现在，这样第一封会在下一个周一发出。
    pub fn seed(&self, now: OffsetDateTime) {
        if self.last().is_none() {
            self.record(now);
        }
    }
}

/// 周报那一档：到点了就攒一期发出去（DESIGN §3.6）。
///
/// 每分钟看一眼而不是「睡到下周一」：进程会重启，睡着的那个 sleep 不会跨过重启活下来。
/// 上一期的时间在 [`Marker`] 里，所以重启也不会重发。
pub const DIGEST_TICK: std::time::Duration = std::time::Duration::from_secs(60);

pub struct WeeklyDigest {
    marker: Marker,
}

impl WeeklyDigest {
    pub fn new(data_dir: &Path) -> Self {
        let marker = Marker::new(data_dir);
        // 第一次跑不补发上一期：新装的机器不该一起来就给所有人寄一封。
        marker.seed(clock::now());
        Self { marker }
    }
}

impl Job for WeeklyDigest {
    fn name(&self) -> &'static str {
        "weekly_digest"
    }
    fn every(&self) -> std::time::Duration {
        DIGEST_TICK
    }
    fn on_start(&self) -> bool {
        false
    }
    fn run<'a>(&'a self, state: &'a AppState) -> JobFuture<'a> {
        Box::pin(async move {
            let now = clock::now();
            if !self.marker.due(now) {
                return Ok(0);
            }
            let queued = {
                let conn = state.db().lock().await;
                enqueue(&conn, now, |slug| state.site_url(slug))?
            };
            // 这一周没有新作品就不发，但「这一期过去了」照样记下——
            // 否则下一分钟会再试一次，一直试到下周一。
            self.marker.record(now);
            if queued > 0 {
                tracing::info!(queued, "周报入队");
            } else {
                tracing::info!("这一周没有新作品，周报不发");
            }
            Ok(queued)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_run_is_monday_morning_in_utc_plus_eight() {
        // 2026-09-09 是周三。
        let wednesday = clock::parse("2026-09-09T00:00:00Z").unwrap();
        let next = next_run_at(wednesday);
        assert_eq!(clock::format(next), "2026-09-14T01:00:00Z");
        // UTC 01:00 就是 UTC+8 的周一 09:00。
        assert_eq!(
            next.date().weekday().number_from_monday(),
            DIGEST_ISO_WEEKDAY
        );
    }

    #[test]
    fn next_run_skips_the_one_that_just_passed() {
        let just_after = clock::parse("2026-09-14T01:00:01Z").unwrap();
        assert_eq!(
            clock::format(next_run_at(just_after)),
            "2026-09-21T01:00:00Z"
        );
    }

    #[test]
    fn marker_is_not_due_until_a_week_later() {
        let dir = tempfile::tempdir().unwrap();
        let marker = Marker::new(dir.path());
        let now = clock::parse("2026-09-14T01:00:00Z").unwrap();
        assert!(!marker.due(now), "没记过就先不发");
        marker.record(now);
        assert!(!marker.due(clock::parse("2026-09-18T01:00:00Z").unwrap()));
        assert!(marker.due(clock::parse("2026-09-21T01:00:00Z").unwrap()));
    }
}
