use std::path::Path;

use playtest_api::{clock, config, db, notify};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let output = Path::new(env!("CARGO_MANIFEST_DIR")).join("../.data/email-preview");
    std::fs::create_dir_all(&output)?;
    let directory = tempfile::tempdir()?;
    let database = db::Db::open(&directory.path().join("api.sqlite"))?;
    let connection = database.lock().await;
    let now = clock::now();
    let timestamp = clock::format(now);
    let runtime = notify::Runtime::new(
        config::Notify {
            email: config::EmailProvider::Off,
            root_url: "https://playtest.run".to_string(),
            ..Default::default()
        },
        notify::push::Vapid::generate(),
        reqwest::Client::new(),
    )?;
    db::insert_player(
        &connection,
        &db::NewPlayer {
            id: "preview-player",
            email: Some("preview@example.com"),
            push_subscription: None,
            push_endpoint: None,
            unsubscribe_token: "preview-only-not-a-real-unsubscribe-token",
            created_at: &timestamp,
        },
    )?;
    let confirm_url = runtime.confirm_url("preview-only-not-a-real-confirm-token");
    notify::enqueue_confirm(
        &connection,
        "preview-player",
        Some("小小星球"),
        &confirm_url,
        now,
    )?;
    notify::enqueue_confirm(&connection, "preview-player", None, &confirm_url, now)?;
    notify::enqueue_send_link(&connection, "preview-player", &confirm_url, now)?;
    db::confirm_player_email(&connection, "preview-player", &timestamp)?;
    for slug in ["preview-planet", "preview-paper"] {
        db::insert_follow(
            &connection,
            "preview-player",
            "site",
            Some(slug),
            None,
            &timestamp,
        )?;
    }
    notify::enqueue_site_version(
        &connection,
        "preview-planet",
        "小小星球",
        8,
        Some("重新做了新手引导，现在可以直接拖动星星。\n\n也修好了手机上偶尔没有声音的问题。想请你再试试，看看这次会不会更顺手。"),
        "https://playtest.run/p/preview-planet",
        now,
    )?;
    notify::enqueue_site_version(
        &connection,
        "preview-paper",
        "纸上远行",
        3,
        None,
        "https://playtest.run/p/preview-paper",
        now,
    )?;
    let items = [
        notify::digest::Item {
            title: "小小星球".to_string(),
            developer: "小雨".to_string(),
            summary: Some("用手指拨动星星，给一颗小行星找到回家的路。".to_string()),
            url: "https://playtest.run/p/preview-planet?from=notice".to_string(),
            boosted: false,
        },
        notify::digest::Item {
            title: "纸上远行".to_string(),
            developer: "阿树".to_string(),
            summary: Some("一段十分钟的水彩旅行，不用急着到终点。".to_string()),
            url: "https://playtest.run/p/preview-paper?from=notice".to_string(),
            boosted: true,
        },
    ];
    db::enqueue_notification(
        &connection,
        &db::NewNotification {
            player_id: "preview-player",
            kind: notify::KIND_DIGEST,
            target_slug: None,
            subject: &notify::digest::subject(now),
            body: &notify::digest::body(&items),
            url: None,
            channel: notify::CHANNEL_EMAIL,
            not_before: &timestamp,
            created_at: &timestamp,
        },
    )?;
    notify::enqueue_confirm(
        &connection,
        "preview-player",
        Some(&"给还没睡的人做的一段星际旅程".repeat(5)),
        &runtime.confirm_url(&"preview-only-".repeat(12)),
        now,
    )?;
    let samples = [
        ("confirm", "确认关注"),
        ("weekly-confirm", "周报确认"),
        ("restore", "找回关注"),
        ("update", "作品更新"),
        ("update-no-note", "未写说明"),
        ("digest", "已有周报"),
        ("long-title", "长内容检查"),
    ];
    let rows = db::due_notifications(&connection, &timestamp, 20)?;
    anyhow::ensure!(rows.len() == samples.len(), "预览样例数量不一致");
    for (row, (name, _)) in rows.iter().zip(samples) {
        let content = notify::email::render(&runtime, row);
        std::fs::write(output.join(format!("{name}.html")), &content.html)?;
        std::fs::write(output.join(format!("{name}.txt")), &content.text)?;
        let message = notify::mailer::Letter {
            to: "preview@example.com".to_string(),
            subject: row.subject.clone(),
            body: content.text,
            html: content.html,
        }
        .smtp_message(config::DEFAULT_EMAIL_FROM)
        .map_err(|error| anyhow::anyhow!("生成 MIME 预览失败：{error}"))?;
        std::fs::write(output.join(format!("{name}.eml")), message.formatted())?;
    }
    let navigation = samples
        .iter()
        .map(|(name, title)| {
            format!("<a href=\"{name}.html\" target=\"email-preview\">{title}</a>")
        })
        .collect::<String>();
    std::fs::write(
        output.join("index.html"),
        format!(
            r#"<!doctype html><html lang="zh-CN"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>playtest.run 邮件预览</title>
<style>body{{margin:0;background:#f2f2f0;color:#202024;font:14px/1.6 -apple-system,BlinkMacSystemFont,"PingFang SC",sans-serif}}header{{padding:20px 24px;background:#fff;border-bottom:1px solid #ddd}}h1{{margin:0;font-size:18px}}p{{margin:4px 0 12px;color:#62626c;font-size:12px}}nav{{display:flex;gap:8px;flex-wrap:wrap}}a{{padding:5px 12px;border:1px solid #ddd;border-radius:6px;color:#202024;text-decoration:none;font-size:12px}}a:hover,a:focus-visible{{background:#202024;color:#fff}}iframe{{display:block;width:100%;height:calc(100vh - 158px);min-height:720px;border:0}}</style>
<header><h1>playtest.run / 邮件模板</h1><p>本机预览 · 虚构作品与无效令牌 · 没有发送邮件 · 不代表真实邮箱兼容性验收</p><nav>{navigation}</nav></header><iframe name="email-preview" title="邮件正文预览" sandbox src="update.html"></iframe></html>"#
        ),
    )?;
    println!(
        "邮件预览已生成：{}",
        output.join("index.html").canonicalize()?.display()
    );
    println!("7 组 HTML / 纯文本 / MIME 样例；仅使用虚构数据，没有发送邮件。");
    Ok(())
}
