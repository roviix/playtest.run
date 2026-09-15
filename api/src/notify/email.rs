use crate::db::NotificationRow;

use super::digest::HEADING_SEPARATOR;
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
            "Confirm follow",
            "Confirm your email and the updates you asked for start. The author never sees your address.",
            "Confirm follow",
            "You asked to follow something on playtest.run, so we sent this confirmation.",
        ),
        KIND_SEND_LINK => (
            "Sign in",
            "Sign in and manage what you follow from this device. No password.",
            "Sign in",
            "You asked playtest.run for a sign-in link, so we sent this email.",
        ),
        KIND_SITE_VERSION => (
            "New version",
            "A project you follow has a new version. Open the invite to try it.",
            "Open the new version",
            "You follow this project, so you get its updates. At most one email per project per 24 hours.",
        ),
        KIND_DIGEST => (
            "Weekly digest",
            "New projects this week, and who made them. You can manage your follows or unsubscribe any time.",
            "See the projects",
            "You confirmed the playtest.run weekly digest, so we sent this email.",
        ),
        _ => (
            "Notice",
            "A notice about your playtest.run follows.",
            "Open",
            "A notice from playtest.run.",
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
            r#"<p class="muted" style="margin:0 0 22px;color:#62626c;font-size:15px;line-height:1.8">A project you follow has a new version.</p>
<div class="note" style="border-left:2px solid #c6c6ce;padding:2px 0 2px 20px;margin:0 0 4px">
<p class="muted" style="margin:0 0 10px;color:#62626c;font-size:12px;letter-spacing:1px">What changed</p>{}</div>"#,
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
                "<p style=\"margin:24px 0 0;font-size:14px;line-height:1.8\">The link in this email is not usable. Go back to the project page and start again.</p>".to_string()
            } else {
                String::new()
            }
        });
    let security = verification.then(|| {
        format!(
            "This link works for {CONFIRM_TOKEN_HOURS} hours and can be used once. Open it in your own browser and do not forward it. If you did not ask for this, ignore this email."
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
                r#"<p class="muted" style="margin:0 0 6px;color:#62626c;font-size:11px;line-height:1.8">Button not working? Copy this link into your browser:</p>
<p style="margin:0 0 22px;font-size:11px;line-height:1.8;word-break:break-all;overflow-wrap:anywhere"><a class="muted" href="{url}" style="color:#62626c;text-decoration:underline">{url}</a></p>"#
            )
        })
        .unwrap_or_default();
    let mut text = format!("playtest.run · {category}\n\n{}\n\n{message}", row.subject);
    if let Some(url) = action_url {
        text.push_str(&format!("\n\n{action_label}:\n{url}"));
    } else if row.url.is_some() {
        text.push_str("\n\nThe link in this email is not usable. Go back to the project page and start again.");
    }
    if let Some(security) = security {
        text.push_str(&format!("\n\n{security}"));
    }
    text.push_str(&format!("\n\n—\n{reason}"));
    let mut footer_links = String::new();
    let me_url = runtime.me_url();
    if player_url(runtime, &me_url) {
        text.push_str(&format!("\nManage your follows: {me_url}"));
        footer_links.push_str(&format!(
            r#"<a class="muted" href="{}" style="color:#62626c;text-decoration:underline">Manage your follows</a>"#,
            escape(&me_url)
        ));
    }
    if let Some(token) = row.unsubscribe_token.as_deref() {
        let url = runtime.unsubscribe_url(token);
        if player_url(runtime, &url) {
            text.push_str(&format!(
                "\nUnsubscribe from everything in one click: {url}"
            ));
            if !footer_links.is_empty() {
                footer_links.push_str("&nbsp;&nbsp;·&nbsp;&nbsp;");
            }
            footer_links.push_str(&format!(
                r#"<a class="muted" href="{}" style="color:#62626c;text-decoration:underline">Unsubscribe from everything</a>"#,
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
            if linkify && paragraph.trim() == "—\nSponsored" {
                return "<p class=\"muted\" style=\"margin:24px 0 12px;color:#62626c;font-size:11px;letter-spacing:1px\">Sponsored</p>".to_string();
            }
            if linkify {
                if let Some((heading, details)) = paragraph.split_once('\n') {
                    if let Some((title, developer)) = heading
                        .rsplit_once(HEADING_SEPARATOR)
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
                            r#"<a class="text-link" href="{}" style="color:#202024;font-weight:600;text-decoration:underline">Open the invite →</a>"#,
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
