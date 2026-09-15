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
        Some("Tiny Planet"),
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
        "Tiny Planet",
        8,
        Some("The tutorial is rebuilt: you can drag the stars directly now.\n\nThe missing sound on phones is fixed too. Worth another try to see whether it reads better this time."),
        "https://playtest.run/p/preview-planet",
        now,
    )?;
    notify::enqueue_site_version(
        &connection,
        "preview-paper",
        "Paper Passage",
        3,
        None,
        "https://playtest.run/p/preview-paper",
        now,
    )?;
    let items = [
        notify::digest::Item {
            title: "Tiny Planet".to_string(),
            developer: "Rain".to_string(),
            summary: Some(
                "Nudge the stars with a finger and send a small planet home.".to_string(),
            ),
            url: "https://playtest.run/p/preview-planet?from=notice".to_string(),
            boosted: false,
        },
        notify::digest::Item {
            title: "Paper Passage".to_string(),
            developer: "Tree".to_string(),
            summary: Some("A ten-minute watercolour trip. No hurry to arrive.".to_string()),
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
        Some(
            "An interstellar trip for anyone still awake "
                .repeat(5)
                .trim(),
        ),
        &runtime.confirm_url(&"preview-only-".repeat(12)),
        now,
    )?;
    let samples = [
        ("confirm", "Confirm follow"),
        ("weekly-confirm", "Confirm digest"),
        ("restore", "Sign-in link"),
        ("update", "New version"),
        ("update-no-note", "No note"),
        ("digest", "Weekly digest"),
        ("long-title", "Long content"),
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
            r#"<!doctype html><html lang="en"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>playtest.run email preview</title>
<style>body{{margin:0;background:#f2f2f0;color:#202024;font:14px/1.6 -apple-system,BlinkMacSystemFont,"Helvetica Neue",sans-serif}}header{{padding:20px 24px;background:#fff;border-bottom:1px solid #ddd}}h1{{margin:0;font-size:18px}}p{{margin:4px 0 12px;color:#62626c;font-size:12px}}nav{{display:flex;gap:8px;flex-wrap:wrap}}a{{padding:5px 12px;border:1px solid #ddd;border-radius:6px;color:#202024;text-decoration:none;font-size:12px}}a:hover,a:focus-visible{{background:#202024;color:#fff}}iframe{{display:block;width:100%;height:calc(100vh - 158px);min-height:720px;border:0}}</style>
<header><h1>playtest.run / email templates</h1><p>Local preview · made-up projects and dead tokens · nothing was sent · not a check of real inbox rendering</p><nav>{navigation}</nav></header><iframe name="email-preview" title="Email body preview" sandbox src="update.html"></iframe></html>"#
        ),
    )?;
    println!(
        "Email preview written to {}",
        output.join("index.html").canonicalize()?.display()
    );
    println!(
        "{} as HTML, plain text and MIME. Made-up data only; nothing was sent.",
        playtest_api::words::count(samples.len() as u64, "sample")
    );
    Ok(())
}
