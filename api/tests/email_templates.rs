use std::sync::Arc;

use playtest_api::{clock, config, db, notify};

fn runtime() -> Arc<notify::Runtime> {
    let _ = rustls::crypto::ring::default_provider().install_default();
    notify::Runtime::new(
        config::Notify {
            email: config::EmailProvider::Off,
            root_url: "https://playtest.run".to_string(),
            ..Default::default()
        },
        notify::push::Vapid::generate(),
        reqwest::Client::new(),
    )
    .unwrap()
}

fn row(kind: &str, subject: &str, body: &str, url: Option<&str>) -> db::NotificationRow {
    db::NotificationRow {
        id: 1,
        player_id: "preview-player".to_string(),
        kind: kind.to_string(),
        subject: subject.to_string(),
        body: body.to_string(),
        url: url.map(str::to_string),
        channel: notify::CHANNEL_EMAIL.to_string(),
        attempts: 0,
        email: Some("preview@example.com".to_string()),
        push_subscription: None,
        unsubscribe_token: Some("not-a-real-unsubscribe-token".to_string()),
    }
}

#[tokio::test]
async fn queued_verification_letters_explain_the_action_and_expiry_in_both_formats() {
    let directory = tempfile::tempdir().unwrap();
    let database = db::Db::open(&directory.path().join("api.sqlite")).unwrap();
    let connection = database.lock().await;
    let now = clock::now();
    db::insert_player(
        &connection,
        &db::NewPlayer {
            id: "preview-player",
            email: Some("preview@example.com"),
            push_subscription: None,
            push_endpoint: None,
            unsubscribe_token: "not-a-real-unsubscribe-token",
            created_at: &clock::format(now),
        },
    )
    .unwrap();
    let runtime = runtime();
    let url = runtime.confirm_url("not-a-real-confirm-token");
    notify::enqueue_confirm(&connection, "preview-player", Some("小小星球"), &url, now).unwrap();
    notify::enqueue_confirm(&connection, "preview-player", None, &url, now).unwrap();
    notify::enqueue_send_link(&connection, "preview-player", &url, now).unwrap();
    let rows = db::due_notifications(&connection, &clock::format(now), 10).unwrap();
    assert_eq!(rows.len(), 3);
    for row in &rows {
        let content = notify::email::render(&runtime, row);
        for body in [&content.html, &content.text] {
            assert!(body.contains("48 小时内有效"));
            assert!(body.contains("仅可使用一次"));
            assert!(body.contains("不要转发"));
            assert!(body.contains("不是你本人操作"));
            assert!(body.contains("可以忽略这封信"));
            assert!(body.contains("管理关注"));
            assert!(body.contains("一键退订全部关注"));
            assert!(!body.contains("就算数"));
        }
        assert_eq!(content.text.matches(&url).count(), 1);
        assert_eq!(content.html.matches(&format!("href=\"{url}\"")).count(), 2);
        let preheader = content
            .html
            .split("<div style=")
            .nth(1)
            .unwrap()
            .split("</div>")
            .next()
            .unwrap();
        assert!(!preheader.contains("not-a-real-confirm-token"));
    }
    assert!(rows[0].body.contains("小小星球"));
    assert!(!rows[0].body.contains("周报"));
    assert!(rows[1].body.contains("每周最多收到一封"));
    assert!(notify::email::render(&runtime, &rows[2])
        .html
        .contains("找回关注&nbsp; →"));
}

#[test]
fn developer_text_is_not_html_and_tracking_parameters_survive() {
    let row = row(
        notify::KIND_SITE_VERSION,
        "《<img src=x onerror=alert(1)> & 星球》更新至 v8",
        "<script>alert('hello')</script>\n\n改善了移动操作 & 新手引导。{action}",
        Some("https://playtest.run/p/preview?from=notice&version=8"),
    );
    let content = notify::email::render(&runtime(), &row);
    assert!(content.html.contains("&lt;script&gt;"));
    assert!(content.html.contains("&lt;img src=x onerror=alert(1)&gt;"));
    assert!(!content.html.contains("<script>"));
    assert!(!content.html.contains("<img "));
    assert!(content
        .html
        .contains("改善了移动操作 &amp; 新手引导。{action}"));
    assert!(content.text.contains("<script>alert('hello')</script>"));
    assert!(content.html.contains("?from=notice&amp;version=8"));
    assert!(content.text.contains("?from=notice&version=8"));
    assert!(content.html.contains("查看新版本&nbsp; →"));
    assert!(content.html.contains("同一作品 24 小时内最多一封"));
}

#[test]
fn unexpected_action_urls_are_not_clickable_and_the_failure_is_visible() {
    for url in [
        "javascript:alert(1)",
        "data:text/html,hello",
        "https://playtest.roviix.com/login",
        "https://playtest.run.attacker.example/",
        "https://notplaytest.run/",
        "https://playtest.run@attacker.example/",
        "https://attacker@playtest.run/",
        "http://playtest.run/p/preview",
        "https://playtest.run:444/p/preview",
    ] {
        let row = row(notify::KIND_SITE_VERSION, "作品更新", "新版本", Some(url));
        let content = notify::email::render(&runtime(), &row);
        assert!(!content.html.contains(&format!("href=\"{url}\"")), "{url}");
        assert!(content.html.contains("链接暂不可用"), "{url}");
        assert!(content.text.contains("链接暂不可用"), "{url}");
    }
}

#[test]
fn digest_preserves_developer_names_and_promotion_labels_without_linkifying_the_web() {
    let items = [
        notify::digest::Item {
            title: "小小星球".to_string(),
            developer: "小雨".to_string(),
            summary: Some("<b>触摸星星</b>\nhttps://attacker.example".to_string()),
            url: "https://playtest.run/p/preview?from=notice".to_string(),
            boosted: false,
        },
        notify::digest::Item {
            title: "纸上远行".to_string(),
            developer: "阿树".to_string(),
            summary: Some("十分钟的水彩旅行。".to_string()),
            url: "https://paper.playtest.run/?from=notice".to_string(),
            boosted: true,
        },
    ];
    let row = row(
        notify::KIND_DIGEST,
        "本周新作品",
        &notify::digest::body(&items),
        None,
    );
    let content = notify::email::render(&runtime(), &row);
    assert_eq!(content.html.matches("打开邀请函 →").count(), 2);
    assert_eq!(
        content.html.matches("<div class=\"digest-item\"").count(),
        2
    );
    assert!(content.html.contains("推广"));
    assert!(content.html.contains("阿树"));
    assert!(content.html.contains("&lt;b&gt;触摸星星&lt;/b&gt;"));
    assert!(!content.html.contains("href=\"https://attacker.example\""));
    assert!(content.text.contains(&items[0].url));
    assert!(content.text.contains(&items[1].url));
}

#[test]
fn long_text_remains_complete_without_remote_assets() {
    let title = "给还没睡的人做的一段星际旅程".repeat(12);
    let note = "修复了长标题下的显示问题。\n".repeat(30);
    let row = row(
        notify::KIND_SITE_VERSION,
        &title,
        &note,
        Some("https://playtest.run/p/preview?from=notice"),
    );
    let content = notify::email::render(&runtime(), &row);
    assert!(content.html.contains(&title));
    assert!(content.text.contains(note.trim()));
    assert!(content.html.contains("overflow-wrap:anywhere"));
    assert!(content.html.contains("max-width: 620px"));
    for tag in ["<img", "<script", "<link ", "<iframe", "<svg", "url("] {
        assert!(!content.html.contains(tag), "{tag}");
    }
    assert!(content.html.len() < 30_000);
}

#[test]
fn local_self_hosted_preview_keeps_its_own_player_origin() {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let runtime = notify::Runtime::new(
        config::Notify::default(),
        notify::push::Vapid::generate(),
        reqwest::Client::new(),
    )
    .unwrap();
    let row = row(
        notify::KIND_SITE_VERSION,
        "作品更新",
        "新版本",
        Some("http://preview.localhost:8443/?from=notice"),
    );
    let content = notify::email::render(&runtime, &row);
    assert!(content
        .html
        .contains("href=\"http://preview.localhost:8443/?from=notice\""));
    assert!(content
        .text
        .contains("http://localhost:8443/me/unsubscribe/"));
}

#[test]
fn real_sending_cannot_start_with_localhost_or_malformed_player_links() {
    let mut config = config::Notify {
        email: config::EmailProvider::Resend {
            api_key: "not-a-real-key".to_string(),
        },
        ..Default::default()
    };
    for root in [
        "http://localhost:8443",
        "https://localhost",
        "https://preview.localhost",
        "https://127.0.0.1",
        "https://[::1]",
        "http://playtest.run",
        "https://playtest.roviix.com",
        "https://playtest.run/me",
        "https://playtest.run/?token=secret",
        "https://user:secret@playtest.run",
    ] {
        config.root_url = root.to_string();
        let error = config.validate().unwrap_err().to_string();
        assert!(error.contains("PLAYTEST_PUBLIC_ROOT_URL"), "{root}");
        assert!(!error.contains("secret"));
        assert!(!error.contains("not-a-real-key"));
    }
    for root in ["https://playtest.run", "https://games.example.com"] {
        config.root_url = root.to_string();
        assert!(config.validate().is_ok());
    }
    config.from = "not a mailbox".to_string();
    assert!(config
        .validate()
        .unwrap_err()
        .to_string()
        .contains("PLAYTEST_EMAIL_FROM"));
    config.email = config::EmailProvider::Log;
    config.root_url = "http://localhost:8443".to_string();
    assert!(config.validate().is_ok());
}
