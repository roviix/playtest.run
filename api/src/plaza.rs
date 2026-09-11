//! 广场（DESIGN §3.8）：把该出现在根域那一页上的作品整理成 `plaza.json`，写进对象存储。
//!
//! 边缘只读那份文件（DESIGN §4.5），所以这里是广场内容唯一的出口。什么时候重写：
//! 作品公开 / 撤下 / 求测状态变了、提交了新版本、作品删除或到期，以及每 5 分钟一次让人数跟上。
//!
//! 自动撤下也在这里判：同一作品 24 小时内被 3 个不同会话举报，就从广场上拿下来，
//! 作品链接照常能开，等人复核。**判断和写文件放在同一个地方**，是为了不存在
//! 「库里已经撤下、广场上还挂着」的窗口——每次重写之前先判一遍。

use playtest_common::boost::MAX_SLOTS;
use playtest_common::plaza::{
    Plaza, PlazaItem, PLAYERS_WINDOW_DAYS, REPORTS_TO_HIDE, REPORTS_WINDOW_HOURS, SCHEMA,
};
use time::Duration;

use crate::clock;
use crate::db;
use crate::project;
use crate::state::AppState;

/// 多久主动重写一次（人数会变，别的都由事件触发）。
pub const REFRESH_INTERVAL: std::time::Duration = std::time::Duration::from_secs(5 * 60);

/// 自动撤下时写进 `hidden_reason` 的原话，控制台照着它告诉开发者。
pub const HIDDEN_BY_REPORTS: &str = "24 小时内被多人举报，已从广场撤下，等人复核";

/// 重新整理并写出 `plaza.json`。返回写出去了几个作品。
///
/// 失败只记日志、不向调用方冒泡：广场是提交、公开这些动作的副产品，
/// 它写不出去不该让「上传成功」变成「上传失败」。要拿到错误用 [`rebuild`]。
pub async fn publish(state: &AppState) {
    match rebuild(state).await {
        Ok(count) => tracing::debug!(count, "重写了广场"),
        Err(err) => tracing::error!(error = format!("{err:#}"), "广场没写出去，下次再试"),
    }
}

pub async fn rebuild(state: &AppState) -> anyhow::Result<usize> {
    let now = clock::now();
    let now_string = clock::format(now);
    let players_since = clock::format(now - Duration::days(PLAYERS_WINDOW_DAYS));
    let reports_since = clock::format(now - Duration::hours(REPORTS_WINDOW_HOURS));

    let (items, club_followers) = {
        let conn = state.db().lock().await;
        let players = db::players_since(&conn, &players_since)?;
        let joined = db::joined_counts(&conn)?;
        let followers = db::followers_counts(&conn)?;
        // 在位的推广，按上位时间排；超过 MAX_SLOTS 的先不上——位子就这么多（DESIGN §3.11）。
        let boosted: Vec<String> = db::live_plaza_boosts(&conn)?
            .into_iter()
            .map(|b| b.slug)
            .take(MAX_SLOTS)
            .collect();
        let candidates = db::plaza_candidates(&conn, &now_string)?;
        let mut items = Vec::with_capacity(candidates.len());
        for candidate in candidates {
            // 只数窗口内、且在人工复核之后的举报：复核恢复过的作品不能被同一批举报再撤一次。
            // 时间是秒精度，「之后」取复核那一秒的下一秒起，复核当秒的旧举报不算。
            let since = match candidate
                .site
                .listing
                .reviewed_at
                .as_deref()
                .and_then(clock::parse)
                .map(|t| clock::format(t + Duration::seconds(1)))
            {
                Some(after_review) if after_review > reports_since => after_review,
                _ => reports_since.clone(),
            };
            let reports = db::report_count(&conn, &candidate.site.slug, &since)?;
            if reports >= REPORTS_TO_HIDE {
                if db::hide_from_plaza(&conn, &candidate.site.slug, &now_string, HIDDEN_BY_REPORTS)?
                {
                    tracing::warn!(slug = %candidate.site.slug, reports, "举报到阈值，从广场撤下");
                }
                continue;
            }
            let counts = Counts {
                players: players
                    .get(&candidate.site.slug)
                    .copied()
                    .unwrap_or_default(),
                joined: joined
                    .get(&candidate.site.slug)
                    .copied()
                    .unwrap_or_default(),
                followers: followers
                    .get(&candidate.site.slug)
                    .copied()
                    .unwrap_or_default(),
                boosted: boosted.contains(&candidate.site.slug),
            };
            if let Some(item) = to_item(state, candidate, counts) {
                items.push(item);
            }
        }
        (items, db::plaza_followers_count(&conn)?)
    };

    let mut items = items;
    sort_default(&mut items);
    let count = items.len();
    state
        .store()
        .put_plaza(&Plaza {
            schema: SCHEMA,
            generated_at: now_string,
            items,
            club_followers,
        })
        .await?;
    Ok(count)
}

/// 广场的默认顺序（DESIGN §3.9、§3.11）：推广位在最前（按上位时间，已经在 `boosted`
/// 里排好，这里只把它们提到前面），然后正在找人测的，其余按最近更新；同一时间按 slug 稳定。
pub fn sort_default(items: &mut [PlazaItem]) {
    items.sort_by(|a, b| {
        b.boosted
            .cmp(&a.boosted)
            .then_with(|| b.seeking.cmp(&a.seeking))
            .then_with(|| b.updated_at.cmp(&a.updated_at))
            .then_with(|| a.slug.cmp(&b.slug))
    });
}

/// 一个作品身上那几个要现算的数字。
#[derive(Default, Clone, Copy)]
struct Counts {
    players: u32,
    joined: u32,
    followers: u32,
    boosted: bool,
}

fn to_item(state: &AppState, candidate: db::PlazaCandidate, counts: Counts) -> Option<PlazaItem> {
    // 没发过版本的作品不上墙：卡上没有封面、没有版本、点开是一页「还没有内容」。
    candidate.site.current_version?;
    let url = state.site_url(&candidate.site.slug);
    let owner = db::SiteOwner {
        user_id: String::new(),
        kind: String::new(),
        display_name: candidate.developer,
        avatar_url: candidate.avatar_url,
    };
    let mut card = project::compose(
        candidate.site,
        Some(owner),
        url,
        project::Extras {
            players: counts.players,
            joined: counts.joined,
            followers: counts.followers,
            ..Default::default()
        },
    )
    .card();
    // 在位与否由这一轮现算的推广名单说了算，不从作品那一行读。
    card.boosted = counts.boosted;
    Some(card)
}

/// 每 [`REFRESH_INTERVAL`] 重写一次 `plaza.json`。进程一起来也写一次：
/// 边缘读的是对象存储里的文件，控制面重启前的那份可能已经旧了。
pub struct Refresh;

impl crate::scheduler::Job for Refresh {
    fn name(&self) -> &'static str {
        "plaza"
    }
    fn every(&self) -> std::time::Duration {
        REFRESH_INTERVAL
    }
    fn run<'a>(&'a self, state: &'a AppState) -> crate::scheduler::JobFuture<'a> {
        Box::pin(async move {
            publish(state).await;
            Ok(1)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(slug: &str, seeking: bool, updated_at: &str) -> PlazaItem {
        PlazaItem {
            slug: slug.into(),
            url: format!("https://{slug}.playtest.run"),
            title: slug.into(),
            developer: "某某".into(),
            summary: None,
            note: None,
            engine: None,
            is_game: false,
            version: 1,
            updated_at: updated_at.into(),
            expires_at: None,
            cover_hash: None,
            players: 0,
            seeking,
            seats: None,
            joined: 0,
            followers: 0,
            avatar_url: None,
            boosted: false,
        }
    }

    #[test]
    fn seeking_first_then_newest() {
        let mut items = vec![
            item("old", false, "2026-09-01T00:00:00Z"),
            item("new", false, "2026-09-08T00:00:00Z"),
            item("seek-old", true, "2026-09-02T00:00:00Z"),
            item("seek-new", true, "2026-09-07T00:00:00Z"),
        ];
        sort_default(&mut items);
        let order: Vec<&str> = items.iter().map(|i| i.slug.as_str()).collect();
        assert_eq!(order, ["seek-new", "seek-old", "new", "old"]);
    }

    #[test]
    fn boosted_goes_above_everything_else() {
        let mut items = vec![
            item("new", false, "2026-09-08T00:00:00Z"),
            item("seek-new", true, "2026-09-07T00:00:00Z"),
            item("paid", false, "2026-09-01T00:00:00Z"),
        ];
        items[2].boosted = true;
        sort_default(&mut items);
        let order: Vec<&str> = items.iter().map(|i| i.slug.as_str()).collect();
        assert_eq!(order, ["paid", "seek-new", "new"]);
    }
}
