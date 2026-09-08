//! 到期的匿名作品自己消失。
//!
//! 匿名链接只活 24 小时（DESIGN §4.8：这是滥用面的兜底）。到期不能只靠令牌失效——
//! 令牌管的是开发者那一侧，玩家那一侧的链接是边缘按对象存储服务的，得真的把清单删掉。

use std::time::Duration;

use crate::clock;
use crate::db;
use crate::state::AppState;

/// 多久扫一次。链接是 24 小时到期，晚十分钟消失不影响谁；扫得太勤只是白转。
pub const INTERVAL: Duration = Duration::from_secs(10 * 60);

pub fn spawn(state: AppState) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(INTERVAL);
        loop {
            ticker.tick().await;
            if let Err(err) = sweep_once(&state).await {
                tracing::error!(
                    error = format!("{err:#}"),
                    "清理过期匿名作品没做完，下一轮再试"
                );
            }
        }
    })
}

/// 返回这一轮清掉了几个作品。
pub async fn sweep_once(state: &AppState) -> anyhow::Result<usize> {
    let now = clock::now_string();
    let slugs = {
        let conn = state.db().lock().await;
        db::expired_site_slugs(&conn, &now)?
    };

    for slug in &slugs {
        // 和手工删除同一个顺序：先让链接失效，再改库。
        state.store().remove_site(slug).await?;
        let conn = state.db().lock().await;
        db::mark_site_deleted(&conn, slug, &now)?;
    }

    let tokens = {
        let conn = state.db().lock().await;
        db::delete_expired_tokens(&conn, &now)?
    };

    if !slugs.is_empty() || tokens > 0 {
        tracing::info!(sites = slugs.len(), tokens, "清掉了过期的匿名作品和令牌");
    }
    Ok(slugs.len())
}
