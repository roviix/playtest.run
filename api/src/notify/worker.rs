//! 队列的那一头：每 30 秒把到点的信发出去。
//!
//! 失败不是异常，是常态（对方限流、域名刚配好还没生效、玩家的邮箱早就注销了）。
//! 所以失败要退避重试，试够三次进死信，死信里留下原话给运维看（DESIGN §4.10）。

use time::Duration;

use crate::clock;
use crate::db;
use crate::scheduler::{Job, JobFuture};
use crate::state::AppState;

/// 多久扫一次队列。
const TICK: std::time::Duration = std::time::Duration::from_secs(30);
/// 一轮最多发多少封。上限是为了别在一次滴答里把进程占住。
const BATCH: usize = 50;
/// 第 1、2、3 次失败之后各等多久再试。三次都不行就不再试了。
const BACKOFF_MINUTES: [i64; 3] = [1, 5, 30];

/// 队列的这一头：每 [`TICK`] 把到点的信发出去。
pub struct Deliver;

impl Job for Deliver {
    fn name(&self) -> &'static str {
        "deliver_notifications"
    }
    fn every(&self) -> std::time::Duration {
        TICK
    }
    fn run<'a>(&'a self, state: &'a AppState) -> JobFuture<'a> {
        Box::pin(async move { Ok(run_once(state).await) })
    }
}

/// 发一轮。返回成功送出的封数。
pub async fn run_once(state: &AppState) -> usize {
    let now = clock::now();
    let due = {
        let conn = state.db().read().await;
        match db::due_notifications(&conn, &clock::format(now), BATCH) {
            Ok(rows) => rows,
            Err(e) => {
                tracing::warn!(error = %e, "读通知队列失败");
                return 0;
            }
        }
    };

    let mut sent = 0;
    for mut row in due {
        if row.kind == "collection_digest" {
            let conn = state.db().lock().await;
            match crate::collections::refresh_digest(
                &conn,
                &mut row,
                state.notify().root_url(),
                now,
            ) {
                Ok(true) => {}
                Ok(false) => {
                    let _ = db::mark_notification_failed(
                        &conn,
                        row.id,
                        "dead",
                        &clock::format(now),
                        "合集、投稿或订阅已失效，未发送",
                    );
                    continue;
                }
                Err(error) => {
                    tracing::warn!(%error, "合集摘要重新核对失败，稍后再试");
                    continue;
                }
            }
        }
        let outcome = match row.channel.as_str() {
            super::CHANNEL_PUSH => send_push(state, &row).await,
            // 退订那一行只加在邮件里：推送弹窗放不下，而浏览器自己就有「关闭此站点通知」。
            _ => send_email(state, &row).await,
        };

        let conn = state.db().lock().await;
        let at = clock::format(clock::now());
        match outcome {
            Ok(()) => {
                if let Err(e) = db::mark_notification_sent(&conn, row.id, &at) {
                    tracing::warn!(error = %e, "记通知已发失败");
                }
                sent += 1;
            }
            Err(error) => {
                let permanent = matches!(error, super::mailer::SendError::Permanent(_));
                let attempts = row.attempts as usize;
                let dead = permanent || attempts + 1 >= BACKOFF_MINUTES.len();
                let (status, next) = if dead {
                    ("dead", now)
                } else {
                    (
                        "pending",
                        now + Duration::minutes(BACKOFF_MINUTES[attempts]),
                    )
                };
                let message = error.to_string();
                if dead {
                    tracing::warn!(id = row.id, kind = %row.kind, error = %message, "这封通知不再重试");
                }
                if let Err(e) = db::mark_notification_failed(
                    &conn,
                    row.id,
                    status,
                    &clock::format(next),
                    &message,
                ) {
                    tracing::warn!(error = %e, "记通知失败状态失败");
                }
            }
        }
    }
    sent
}

async fn send_email(state: &AppState, row: &db::NotificationRow) -> super::mailer::SendResult {
    let Some(to) = row.email.clone() else {
        // 人把邮箱去掉了（或者从来就是推送那条路），这封信没有收件人。
        return Err(super::mailer::SendError::Permanent(
            "这个人没有邮箱".to_string(),
        ));
    };
    let runtime = state.notify();
    let content = super::email::render(runtime, row);
    runtime
        .mailer
        .send(
            &runtime.config.from,
            &super::mailer::Letter {
                to,
                subject: row.subject.clone(),
                body: content.text,
                html: content.html,
            },
        )
        .await
}

async fn send_push(state: &AppState, row: &db::NotificationRow) -> super::mailer::SendResult {
    let Some(raw) = row.push_subscription.as_deref() else {
        return Err(super::mailer::SendError::Permanent(
            "这个人没有浏览器订阅".to_string(),
        ));
    };
    let Some(subscription) = super::push::parse_subscription(raw) else {
        return Err(super::mailer::SendError::Permanent(
            "库里这条浏览器订阅读不懂".to_string(),
        ));
    };
    let runtime = state.notify();
    let url = row.url.clone().unwrap_or_else(|| runtime.me_url());
    let outcome = super::push::send(
        state.http(),
        &runtime.vapid,
        &subscription,
        &row.subject,
        &row.body,
        &url,
    )
    .await;

    match outcome {
        Ok(true) => Ok(()),
        Ok(false) => {
            // 410 / 404：这个订阅已经没了，删掉，别再往这里发。
            let conn = state.db().lock().await;
            if let Err(e) = db::clear_player_push(&conn, &row.player_id) {
                tracing::warn!(error = %e, "删失效的推送订阅失败");
            }
            Ok(())
        }
        Err(e) => Err(e),
    }
}
