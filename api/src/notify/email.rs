use crate::db::NotificationRow;

use super::{
    Runtime, CONFIRM_TOKEN_HOURS, KIND_CONFIRM, KIND_DIGEST, KIND_SEND_LINK, KIND_SITE_VERSION,
};

pub struct Content {
    pub text: String,
    pub html: String,
}

pub fn render(runtime: &Runtime, row: &NotificationRow) -> Content {
    let (category, preview, action_label, reason) = match row.kind.as_str() {
        KIND_CONFIRM => (
            "确认关注",
            "确认邮箱后，你选择的通知才会生效。开发者看不到你的邮箱。",
            "确认关注",
            "你在 playtest.run 发起了关注请求，因此收到这封确认信。",
        ),
        KIND_SEND_LINK => (
            "换一台设备",
            "找回已关注的作品，让这台设备也能管理关注。无需密码。",
            "找回关注",
            "你请求找回在 playtest.run 上的关注，因此收到这封信。",
        ),
        KIND_SITE_VERSION => (
            "作品更新",
            "你关注的作品有新版本了。打开邀请函，就能再玩一次。",
            "试玩新版本",
            "你关注了这个作品，因此收到更新通知。同一作品 24 小时内最多一封。",
        ),
        KIND_DIGEST => (
            "新作品周报",
            "本周的新作品与开发者介绍。你可以随时管理关注或退订。",
            "查看作品",
            "你确认接收了 playtest.run 新作品周报，因此收到这封信。",
        ),
        _ => (
            "通知",
            "你在 playtest.run 上的通知。",
            "查看详情",
            "来自 playtest.run 的通知。",
        ),
    };
    let preview = if row.kind == KIND_SITE_VERSION {
        row.body
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .chars()
            .take(90)
            .collect::<String>()
    } else {
        preview.to_string()
    };
    let verification = matches!(row.kind.as_str(), KIND_CONFIRM | KIND_SEND_LINK);
    let message = row
        .body
        .lines()
        .filter(|line| !verification || Some(line.trim()) != row.url.as_deref())
        .collect::<Vec<_>>()
        .join("\n");
    let message = message.trim();
    let body = if row.kind == KIND_SITE_VERSION {
        format!(
            r#"<p class="muted" style="margin:0 0 22px;color:#62626c;font-size:15px;line-height:1.8">你关注的作品有新版本了。</p>
<div class="note" style="border-left:2px solid #c6c6ce;padding:2px 0 2px 20px;margin:0 0 4px">
<p class="muted" style="margin:0 0 10px;color:#62626c;font-size:12px;letter-spacing:1px">这一版的变化</p>{}</div>"#,
            paragraphs(runtime, message, false)
        )
    } else {
        paragraphs(runtime, message, row.kind == KIND_DIGEST)
    };
    let action_url = row.url.as_deref().filter(|url| player_url(runtime, url));
    let action = action_url
        .map(|url| {
            let url = escape(url);
            format!(
                r##"<table role="presentation" class="action-table" cellspacing="0" cellpadding="0" border="0" style="margin:28px 0 0"><tr>
<td class="button" bgcolor="#202024" style="border-radius:8px;text-align:center;mso-padding-alt:16px 28px">
<a class="button" href="{url}" style="display:block;padding:16px 28px;border:1px solid #202024;border-radius:8px;background:#202024;color:#ffffff;font-size:15px;font-weight:600;line-height:20px;text-decoration:none">{action_label}&nbsp; →</a>
</td></tr></table>"##
            )
        })
        .unwrap_or_else(|| {
            if row.url.is_some() {
                "<p style=\"margin:24px 0 0;font-size:14px;line-height:1.8\">这封信的链接暂不可用，请回到作品页面重新操作。</p>".to_string()
            } else {
                String::new()
            }
        });
    let security = verification.then(|| {
        format!(
            "链接 {CONFIRM_TOKEN_HOURS} 小时内有效，仅可使用一次。请在你自己的浏览器中打开，不要转发。不是你本人操作的，可以忽略这封信。"
        )
    });
    let safety = security
        .as_deref()
        .map(|text| format!(r#"<p class="muted" style="margin:22px 0 0;color:#62626c;font-size:12px;line-height:1.9">{text}</p>"#))
        .unwrap_or_default();
    let fallback = action_url
        .map(|url| {
            let url = escape(url);
            format!(
                r#"<p class="muted" style="margin:0 0 6px;color:#62626c;font-size:11px;line-height:1.8">按钮无法打开？复制这个链接到浏览器：</p>
<p style="margin:0 0 22px;font-size:11px;line-height:1.8;word-break:break-all;overflow-wrap:anywhere"><a class="muted" href="{url}" style="color:#62626c;text-decoration:underline">{url}</a></p>"#
            )
        })
        .unwrap_or_default();
    let mut text = format!("playtest.run · {category}\n\n{}\n\n{message}", row.subject);
    if let Some(url) = action_url {
        text.push_str(&format!("\n\n{action_label}：\n{url}"));
    } else if row.url.is_some() {
        text.push_str("\n\n这封信的链接暂不可用，请回到作品页面重新操作。");
    }
    if let Some(security) = security {
        text.push_str(&format!("\n\n{security}"));
    }
    text.push_str(&format!("\n\n—\n{reason}"));
    let mut footer_links = String::new();
    let me_url = runtime.me_url();
    if player_url(runtime, &me_url) {
        text.push_str(&format!("\n管理关注：{me_url}"));
        footer_links.push_str(&format!(
            r#"<a class="muted" href="{}" style="color:#62626c;text-decoration:underline">管理关注</a>"#,
            escape(&me_url)
        ));
    }
    if let Some(token) = row.unsubscribe_token.as_deref() {
        let url = runtime.unsubscribe_url(token);
        if player_url(runtime, &url) {
            text.push_str(&format!("\n一键退订全部关注：{url}"));
            if !footer_links.is_empty() {
                footer_links.push_str("&nbsp;&nbsp;·&nbsp;&nbsp;");
            }
            footer_links.push_str(&format!(
                r#"<a class="muted" href="{}" style="color:#62626c;text-decoration:underline">一键退订全部关注</a>"#,
                escape(&url)
            ));
        }
    }
    let html = format!(
        include_str!("email.html"),
        subject = escape(&row.subject),
        category = category,
        preview = escape(&preview),
        body = body,
        action = action,
        safety = safety,
        fallback = fallback,
        reason = reason,
        footer_links = footer_links,
    );
    Content { text, html }
}

fn paragraphs(runtime: &Runtime, text: &str, linkify: bool) -> String {
    text.split("\n\n")
        .filter(|paragraph| !paragraph.trim().is_empty())
        .map(|paragraph| {
            if linkify && paragraph.trim() == "—\n推广" {
                return "<p class=\"muted\" style=\"margin:24px 0 12px;color:#62626c;font-size:11px;letter-spacing:1px\">推广</p>".to_string();
            }
            if linkify {
                if let Some((heading, details)) = paragraph.split_once('\n') {
                    if let Some((title, developer)) = heading
                        .strip_prefix('《')
                        .and_then(|heading| heading.rsplit_once("》 "))
                        .filter(|_| details.lines().last().is_some_and(|url| player_url(runtime, url.trim())))
                    {
                        return format!(
                            r#"<div class="digest-item" style="border-top:1px solid #e8e8ec;padding:20px 0 0;margin:20px 0 0"><h2 style="margin:0 0 8px;font-size:18px;font-weight:600;line-height:1.6;overflow-wrap:anywhere">{}</h2><p class="muted" style="margin:0 0 12px;color:#62626c;font-size:12px;line-height:1.8">{}</p>{}</div>"#,
                            escape(title),
                            escape(developer),
                            paragraphs(runtime, details, true),
                        );
                    }
                }
            }
            let lines = paragraph
                .lines()
                .map(|line| {
                    let trimmed = line.trim();
                    if linkify && player_url(runtime, trimmed) {
                        format!(
                            r#"<a class="text-link" href="{}" style="color:#202024;font-weight:600;text-decoration:underline">打开邀请函 →</a>"#,
                            escape(trimmed)
                        )
                    } else {
                        escape(line)
                    }
                })
                .collect::<Vec<_>>()
                .join("<br>");
            format!(
                r#"<p style="margin:0 0 16px;font-size:15px;line-height:1.9;overflow-wrap:anywhere;word-break:break-word">{lines}</p>"#
            )
        })
        .collect()
}

fn player_url(runtime: &Runtime, value: &str) -> bool {
    let (Ok(root), Ok(url)) = (
        reqwest::Url::parse(runtime.root_url()),
        reqwest::Url::parse(value),
    ) else {
        return false;
    };
    let (Some(root_host), Some(host)) = (root.host_str(), url.host_str()) else {
        return false;
    };
    matches!(url.scheme(), "http" | "https")
        && url.scheme() == root.scheme()
        && url.port_or_known_default() == root.port_or_known_default()
        && url.username().is_empty()
        && url.password().is_none()
        && (host == root_host || host.ends_with(&format!(".{root_host}")))
}

fn escape(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            _ => escaped.push(character),
        }
    }
    escaped
}
