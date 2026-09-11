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

/// blob 多久没人引用才删。上传是「先传 blob、再提交清单」，提交之前的 blob 在任何清单里都找不到，
/// 给足一天，再慢的上传也提交完了；匿名作品到期删清单之后，它的 blob 一天内消失。
pub const BLOB_GRACE: Duration = Duration::from_secs(24 * 60 * 60);

/// 多久回收一次 blob。比清作品慢得多：要读全部清单，一天一次够了。
pub const BLOB_GC_INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);

/// 玩家数据保留多久（DESIGN §3.4「数据默认保留 90 天」）。按会话的最后一次活动算。
pub const RETENTION_DAYS: i64 = 90;

/// 每天一次的慢活：回收孤儿 blob，删过期的会话数据。
pub fn spawn_blob_gc(state: AppState) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(BLOB_GC_INTERVAL);
        loop {
            ticker.tick().await;
            match collect_blobs(&state).await {
                Ok((removed, bytes)) if removed > 0 => {
                    tracing::info!(removed, bytes, "回收了没有清单引用的 blob")
                }
                Ok(_) => {}
                Err(err) => {
                    tracing::error!(error = format!("{err:#}"), "blob 回收没做完，明天再试")
                }
            }
            match expire_sessions(&state).await {
                Ok(n) if n > 0 => {
                    tracing::info!(sessions = n, "删掉了超过 {RETENTION_DAYS} 天的会话数据")
                }
                Ok(_) => {}
                Err(err) => {
                    tracing::error!(error = format!("{err:#}"), "会话数据清理没做完，明天再试")
                }
            }
        }
    })
}

/// 删掉最后一次活动早于 [`RETENTION_DAYS`] 天前的会话及其事件、反馈。
pub async fn expire_sessions(state: &AppState) -> anyhow::Result<usize> {
    let before = clock::format(clock::now() - time::Duration::days(RETENTION_DAYS));
    let mut conn = state.db().lock().await;
    Ok(db::delete_sessions_before(&mut conn, &before)?)
}

/// 删掉所有「没有任何活着的清单引用、且落盘超过 [`BLOB_GRACE`]」的 blob。返回（个数，字节数）。
///
/// 任何一份清单读不出来就整轮放弃：宁可多留一天垃圾，也不能误删一个还在被玩的文件。
pub async fn collect_blobs(state: &AppState) -> anyhow::Result<(usize, u64)> {
    let store = state.store();
    let referenced = store.referenced_hashes().await?;
    let cutoff = std::time::SystemTime::now() - BLOB_GRACE;
    let mut removed = 0usize;
    let mut bytes = 0u64;
    for (hash, modified) in store.list_blobs().await? {
        if referenced.contains(&hash) || modified > cutoff {
            continue;
        }
        let size = tokio::fs::metadata(store.blob_path(&hash)?)
            .await
            .map(|m| m.len())
            .unwrap_or(0);
        store.remove_blob(&hash).await?;
        let conn = state.db().lock().await;
        db::delete_blob(&conn, &hash)?;
        removed += 1;
        bytes += size;
    }
    Ok((removed, bytes))
}

/// 返回这一轮清掉了几个作品。
pub async fn sweep_once(state: &AppState) -> anyhow::Result<usize> {
    let now = clock::now_string();
    let slugs = {
        let conn = state.db().read().await;
        db::expired_site_slugs(&conn, &now)?
    };

    for slug in &slugs {
        // 和手工删除同一个顺序：先让链接失效，再改库。
        state.store().remove_site(slug).await?;
        let conn = state.db().lock().await;
        db::mark_site_deleted(&conn, slug, &now)?;
        db::end_boosts_of(&conn, slug, &now)?;
    }

    let tokens = {
        let conn = state.db().lock().await;
        db::delete_expired_tokens(&conn, &now)?
    };

    // 推广到点上位、到点下位（DESIGN §3.11）。
    let boosts_moved = {
        let conn = state.db().lock().await;
        crate::boosts::advance(&conn, &now)?
    };

    if !slugs.is_empty() || tokens > 0 {
        tracing::info!(sites = slugs.len(), tokens, "清掉了过期的匿名作品和令牌");
    }
    if !slugs.is_empty() || boosts_moved {
        // 到期的作品不能还挂在广场上（DESIGN §3.8「到期自动下来」）；推广换人了也要重排。
        crate::plaza::publish(state).await;
    }
    Ok(slugs.len())
}

/// 周报那一档：到点了就攒一期发出去（DESIGN §3.6）。
///
/// 每分钟看一眼而不是「睡到下周一」：进程会重启，睡着的那个 sleep 不会跨过重启活下来。
pub const DIGEST_TICK: Duration = Duration::from_secs(60);

pub fn spawn_digest(state: AppState) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let marker = crate::notify::digest::Marker::new(state.data_dir());
        // 第一次跑不补发上一期：新装的机器不该一起来就给所有人寄一封。
        marker.seed(clock::now());
        let mut ticker = tokio::time::interval(DIGEST_TICK);
        loop {
            ticker.tick().await;
            let now = clock::now();
            if !marker.due(now) {
                continue;
            }
            let queued = {
                let conn = state.db().lock().await;
                crate::notify::digest::enqueue(&conn, now, |slug| state.site_url(slug))
            };
            match queued {
                // 这一周没有新作品就不发，但「这一期过去了」照样记下——
                // 否则下一分钟会再试一次，一直试到下周一。
                Ok(count) => {
                    marker.record(now);
                    if count > 0 {
                        tracing::info!(count, "周报入队");
                    } else {
                        tracing::info!("这一周没有新作品，周报不发");
                    }
                }
                Err(err) => tracing::error!(error = %err, "攒周报失败，下一分钟再试"),
            }
        }
    })
}
