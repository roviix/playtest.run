//! `sites/<slug>/live.json`（DESIGN §4.5）：一个作品**会变的那些**。
//!
//! 清单不可变、每版一份；但门禁页上还有几样随时在变又不值得发一个版本的东西——名额与已加入
//! 人数、关注数、群链接、公开反馈、开发者头像。控制面在这些变化时重写这一份，边缘只读。
//!
//! 什么时候重写：有人留名、关注变化、开发者改设置、反馈被公开或隐藏、登录时头像变了、
//! 提交新版本，外加每 [`REFRESH_INTERVAL`] 一次给最近有动静的作品兜底。
//! **作品删除时不用单独删这份文件**：它在 `sites/<slug>/` 目录里，`FsStore::remove_site`
//! 删的是整个目录（`store.rs`），清单、指针和它一起消失。

use std::time::Duration;

use playtest_common::live::{PublicFeedbackItem, SiteLive, PUBLIC_FEEDBACK_ON_GATE, SCHEMA};

use crate::clock;
use crate::db;
use crate::state::AppState;

/// 多久给最近有动静的作品刷一遍（DESIGN §4.5 表里那个「每 5 分钟」）。
pub const REFRESH_INTERVAL: Duration = Duration::from_secs(5 * 60);

/// 定时那一档只管这段时间里有动静的作品。
pub const ACTIVE_WINDOW_HOURS: i64 = 24;

/// 重写一个作品的 `live.json`。失败只记日志——它是留名、关注这些动作的副产品，
/// 写不出去不该让「留名成功」变成「留名失败」。要拿到错误用 [`rewrite`]。
pub async fn publish(state: &AppState, slug: &str) {
    if let Err(err) = rewrite(state, slug).await {
        tracing::error!(%slug, error = format!("{err:#}"), "live.json 没写出去，下次再试");
    }
}

/// 一次重写多个（关注变化会同时影响好几个作品时用）。
pub async fn publish_all<I, S>(state: &AppState, slugs: I)
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    for slug in slugs {
        publish(state, slug.as_ref()).await;
    }
}

/// 从库里算出这个作品此刻的样子，写进对象存储。作品不存在（已删）就什么都不做。
pub async fn rewrite(state: &AppState, slug: &str) -> anyhow::Result<()> {
    let Some(live) = compose(state, slug).await? else {
        return Ok(());
    };
    state.store().put_live(&live).await?;
    Ok(())
}

async fn compose(state: &AppState, slug: &str) -> anyhow::Result<Option<SiteLive>> {
    let conn = state.db().lock().await;
    let Some(site) = db::find_site(&conn, slug)? else {
        return Ok(None);
    };
    let listing = site.listing;
    let feedback_public = listing.feedback_public;
    let public_feedback = if feedback_public {
        db::recent_public_feedback(&conn, slug, PUBLIC_FEEDBACK_ON_GATE)?
            .into_iter()
            .map(|row| PublicFeedbackItem {
                name: row.name,
                text: row.text,
                version: row.version,
                at: row.at,
            })
            .collect()
    } else {
        Vec::new()
    };
    Ok(Some(SiteLive {
        schema: SCHEMA,
        slug: slug.to_string(),
        generated_at: clock::now_string(),
        seats: listing.seats.filter(|n| *n > 0),
        joined: db::joined_count(&conn, slug)?,
        followers: db::followers_count(&conn, slug)?,
        community_url: listing.community_url,
        feedback_public,
        public_feedback,
        avatar_url: db::site_owner(&conn, slug)?.and_then(|owner| owner.avatar_url),
        // 「在广场上」是开发者的意愿加上我们没有撤下它；门禁页的「分享」按这个判（DESIGN §3.4）。
        listed: listing.public && listing.hidden_at.is_none(),
        seeking: listing.seeking,
    }))
}

/// 每 [`REFRESH_INTERVAL`] 给最近有动静的作品刷一遍。返回刷了几个。
pub async fn refresh_active(state: &AppState) -> anyhow::Result<usize> {
    let since = clock::format(clock::now() - time::Duration::hours(ACTIVE_WINDOW_HOURS));
    let slugs = {
        let conn = state.db().lock().await;
        db::active_slugs_since(&conn, &since)?
    };
    for slug in &slugs {
        rewrite(state, slug).await?;
    }
    Ok(slugs.len())
}

pub fn spawn_refresh(state: AppState) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(REFRESH_INTERVAL);
        loop {
            ticker.tick().await;
            match refresh_active(&state).await {
                Ok(count) => tracing::debug!(count, "刷了一轮 live.json"),
                Err(err) => {
                    tracing::error!(
                        error = format!("{err:#}"),
                        "刷 live.json 没做完，下一轮再试"
                    )
                }
            }
        }
    })
}
