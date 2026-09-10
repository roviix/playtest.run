//! 通知：入队、投递、重试（DESIGN §4.10）。
//!
//! 所有出站的信都先落进 `notifications` 表再由一个后台任务发。为什么不直接发：
//! 发信是网络操作，会慢会失败；玩家点「关注」时不该等它，开发者发版本时更不该
//! 因为邮件服务商抽风而失败。队列还顺带给了 24 小时合并和退订后不再发的落脚点。

pub mod digest;
pub mod mailer;
pub mod push;
pub mod worker;

use std::path::Path;
use std::sync::Arc;

use playtest_common::follow::notice_url;
use rusqlite::Connection;
use time::OffsetDateTime;

use crate::clock;
use crate::config::Notify as NotifyConfig;
use crate::db;

/// 通知的四种由头，和 `notifications.kind` 一一对应。
pub const KIND_CONFIRM: &str = "confirm";
pub const KIND_SITE_VERSION: &str = "site_version";
pub const KIND_DIGEST: &str = "digest";
pub const KIND_SEND_LINK: &str = "send_link";

pub const CHANNEL_EMAIL: &str = "email";
pub const CHANNEL_PUSH: &str = "push";

/// 确认信里的链接有效期（DESIGN §4.8：确认链接是一次性的，不长期有效）。
pub const CONFIRM_TOKEN_HOURS: u64 = 48;

/// 发信要用的东西，进程里一份。
pub struct Runtime {
    mailer: Box<dyn mailer::Mailer>,
    vapid: push::Vapid,
    config: NotifyConfig,
}

impl Runtime {
    pub fn new(
        config: NotifyConfig,
        vapid: push::Vapid,
        http: reqwest::Client,
    ) -> anyhow::Result<Arc<Self>> {
        let mailer = mailer::from_config(&config.email, http)?;
        Ok(Arc::new(Self {
            mailer,
            vapid: vapid.with_subject(config.vapid_subject.clone()),
            config,
        }))
    }

    pub fn email_on(&self) -> bool {
        self.config.email.is_on()
    }

    pub fn push_public_key(&self) -> &str {
        self.vapid.public_b64()
    }

    pub fn root_url(&self) -> &str {
        &self.config.root_url
    }

    pub fn confirm_url(&self, token: &str) -> String {
        format!("{}/me/confirm/{token}", self.config.root_url)
    }

    pub fn unsubscribe_url(&self, token: &str) -> String {
        format!("{}/me/unsubscribe/{token}", self.config.root_url)
    }

    pub fn me_url(&self) -> String {
        format!("{}/me", self.config.root_url)
    }
}

/// VAPID 密钥对：第一次启动生成，之后一直用同一把——换了的话所有已有的订阅都失效。
/// 私钥只落在数据目录里，不打日志（AGENTS 第 9 条）。
pub fn load_or_create_vapid(data_dir: &Path) -> anyhow::Result<push::Vapid> {
    #[derive(serde::Serialize, serde::Deserialize)]
    struct Stored {
        public_key: String,
        private_key: String,
    }

    let path = data_dir.join("vapid.json");
    if let Ok(text) = std::fs::read_to_string(&path) {
        let stored: Stored = serde_json::from_str(&text)
            .map_err(|e| anyhow::anyhow!("{} 读不懂：{e}", path.display()))?;
        return push::Vapid::from_secret_b64(&stored.private_key, None);
    }
    let vapid = push::Vapid::generate();
    std::fs::create_dir_all(data_dir)?;
    let stored = Stored {
        public_key: vapid.public_b64().to_string(),
        private_key: vapid.secret_b64(),
    };
    std::fs::write(&path, serde_json::to_string_pretty(&stored)?)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
    }
    tracing::info!(path = %path.display(), "生成了一对新的 VAPID 密钥");
    Ok(vapid)
}

/// 这个人该走哪条渠道。两样都有就发邮件：邮件不依赖浏览器还开着。
fn channel_for(recipient: &db::Recipient) -> Option<&'static str> {
    if recipient.email.is_some() {
        Some(CHANNEL_EMAIL)
    } else if recipient.push_subscription.is_some() {
        Some(CHANNEL_PUSH)
    } else {
        None
    }
}

/// 「确认关注」那封信。
pub fn enqueue_confirm(
    conn: &Connection,
    player_id: &str,
    title: Option<&str>,
    confirm_url: &str,
    now: OffsetDateTime,
) -> rusqlite::Result<()> {
    let subject = match title {
        Some(title) => format!("确认关注《{title}》"),
        None => "确认关注 playtest.run 的新作品通知".to_string(),
    };
    let body = match title {
        Some(title) => format!(
            "你在 playtest.run 上留了邮箱，想在《{title}》出新版本时收到一封信。\n\
             点下面这个链接就算数：\n\n{confirm_url}\n\n\
             不是你操作的可以忽略这封信。"
        ),
        None => format!(
            "你在 playtest.run 上留了邮箱，想每周收到一封新作品的信。\n\
             点下面这个链接就算数：\n\n{confirm_url}\n\n\
             不是你操作的可以忽略这封信。"
        ),
    };
    let now = clock::format(now);
    db::enqueue_notification(
        conn,
        &db::NewNotification {
            player_id,
            kind: KIND_CONFIRM,
            target_slug: None,
            subject: &subject,
            body: &body,
            url: Some(confirm_url),
            channel: CHANNEL_EMAIL,
            not_before: &now,
            created_at: &now,
        },
    )?;
    Ok(())
}

/// 「把『我的』的链接寄给我」那封信（DESIGN §3.10 换设备的路）。
pub fn enqueue_send_link(
    conn: &Connection,
    player_id: &str,
    link: &str,
    now: OffsetDateTime,
) -> rusqlite::Result<()> {
    let body = format!(
        "这是能打开你在 playtest.run 上「我的」那一页的链接：\n\n{link}\n\n\
         在这台设备上点开一次，以后就不用再找它了。\n\
         不是你操作的可以忽略这封信。"
    );
    let now = clock::format(now);
    db::enqueue_notification(
        conn,
        &db::NewNotification {
            player_id,
            kind: KIND_SEND_LINK,
            target_slug: None,
            subject: "打开「我的」的链接",
            body: &body,
            url: Some(link),
            channel: CHANNEL_EMAIL,
            not_before: &now,
            created_at: &now,
        },
    )?;
    Ok(())
}

/// 发布了新版本：通知这个作品每一个确认过、没退订的关注者。
///
/// 24 小时合并（DESIGN §3.6「同一个作品每 24 小时最多一封」）：同一个人同一个作品
/// 24 小时内已经有过一封——还没发出去就把它改成最新这一版，已经发出去了就这次不发。
/// 开发者一天连发五版是常事，玩家的收件箱不该跟着响五次。
pub fn enqueue_site_version(
    conn: &Connection,
    slug: &str,
    title: &str,
    version: u32,
    note: Option<&str>,
    site_url: &str,
    now: OffsetDateTime,
) -> rusqlite::Result<usize> {
    let subject = format!("《{title}》出了 v{version}");
    let body = match note.map(str::trim).filter(|n| !n.is_empty()) {
        Some(note) => note.to_string(),
        None => "开发者发了新版本。".to_string(),
    };
    let url = notice_url(site_url);
    let now_text = clock::format(now);
    let since = clock::format(
        now - time::Duration::hours(playtest_common::follow::PER_SITE_NOTICE_INTERVAL_HOURS),
    );

    let mut queued = 0;
    for recipient in db::confirmed_followers(conn, "site", Some(slug))? {
        let Some(channel) = channel_for(&recipient) else {
            continue;
        };
        match db::recent_notice(conn, &recipient.player_id, slug, KIND_SITE_VERSION, &since)? {
            Some((id, status)) if status == "pending" => {
                db::rewrite_pending_notification(conn, id, &subject, &body, Some(&url))?;
            }
            Some(_) => continue,
            None => {
                db::enqueue_notification(
                    conn,
                    &db::NewNotification {
                        player_id: &recipient.player_id,
                        kind: KIND_SITE_VERSION,
                        target_slug: Some(slug),
                        subject: &subject,
                        body: &body,
                        url: Some(&url),
                        channel,
                        not_before: &now_text,
                        created_at: &now_text,
                    },
                )?;
                queued += 1;
            }
        }
    }
    Ok(queued)
}

/// 每封信底下那一行。退订链接必须在每封信里（DESIGN §4.8），不是「登录后到设置里关」。
fn footer(runtime: &Runtime, unsubscribe_token: &str, with_me: bool) -> String {
    let mut lines = String::from("\n\n—\n不想再收到？一键退订：");
    lines.push_str(&runtime.unsubscribe_url(unsubscribe_token));
    if with_me {
        lines.push_str("\n管理关注：");
        lines.push_str(&runtime.me_url());
    }
    lines
}

/// 队列里的一行拼成一封信。
pub fn render(runtime: &Runtime, row: &db::NotificationRow) -> String {
    let mut body = row.body.clone();
    if let Some(url) = &row.url {
        // 确认信和「我的」链接那两封的正文里已经有链接了，别印两遍。
        if !body.contains(url.as_str()) {
            body.push_str("\n\n");
            body.push_str(url);
        }
    }
    if let Some(token) = &row.unsubscribe_token {
        body.push_str(&footer(runtime, token, row.kind == KIND_DIGEST));
    }
    body
}
