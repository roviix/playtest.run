//! 助手第一次遇到 playtest 时看到的那几个文件真的在线上（REWRITE §3.2）。
//!
//! 这几条走真的 `Router`：它们是我们对 AI 助手的**唯一**自我介绍，而助手不会先读
//! 我们的仓库。文件里说错一句，它就会拿错工具——比如把一个静态报告塞进试玩广场，
//! 或者以为隧道不能用而退回去让用户自己想办法。

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use axum::Router;
use playtest_api::{app, AppState, Config};
use playtest_common::api::routes as paths;
use serde_json::Value;
use tower::ServiceExt;

async fn router() -> (Router, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let config = Config {
        listen: "127.0.0.1:0".parse().unwrap(),
        data_dir: dir.path().to_path_buf(),
        site_url_template: "http://{slug}.localhost:8443".to_string(),
        github: None,
        ..Config::default()
    };
    let state = AppState::from_config(&config).await.unwrap();
    (app(state), dir)
}

struct Got {
    status: StatusCode,
    content_type: String,
    body: String,
}

async fn get(path: &str) -> Got {
    let (router, _dir) = router().await;
    let response = router
        .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let content_type = response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .to_string();
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    Got {
        status,
        content_type,
        body: String::from_utf8(body.to_vec()).unwrap(),
    }
}

/// 没有令牌也能读——助手是在还不认识我们的时候来读它的。
#[tokio::test]
async fn an_assistant_can_read_all_of_them_without_a_token() {
    for path in [
        paths::LLMS_TXT,
        paths::LLMS_FULL_TXT,
        paths::SKILL_MD,
        paths::OPENAPI_JSON,
        paths::AGENT_JSON,
    ] {
        let got = get(path).await;
        assert_eq!(got.status, StatusCode::OK, "{path} 没出来");
        assert!(!got.body.trim().is_empty(), "{path} 是空的");
    }
}

#[tokio::test]
async fn each_one_is_served_as_the_type_it_is() {
    assert!(get(paths::LLMS_TXT)
        .await
        .content_type
        .starts_with("text/plain"));
    assert!(get(paths::SKILL_MD)
        .await
        .content_type
        .starts_with("text/markdown"));
    assert!(get(paths::OPENAPI_JSON)
        .await
        .content_type
        .starts_with("application/json"));
}

/// 最要紧的一条：说清楚什么时候**别**用我们。
///
/// 这正是那位助手把 playtest.run 判成「不对口」时缺的东西——它只能从首页猜。
/// 猜对了是运气；写下来才是产品。
#[tokio::test]
async fn the_first_page_says_when_not_to_use_us() {
    let body = get(paths::LLMS_TXT).await.body;
    assert!(body.contains("When NOT to use it"));
    assert!(
        body.contains("dashboard") || body.contains("report"),
        "静态报告那一类要点名：{body}"
    );
    assert!(body.contains("never run user code"));
}

/// 隧道能用。以前 MCP 那一侧写着「调了一定失败」，助手照着就不会用它。
#[tokio::test]
async fn nothing_tells_the_assistant_a_working_feature_is_missing() {
    for path in [paths::LLMS_TXT, paths::LLMS_FULL_TXT, paths::SKILL_MD] {
        let body = get(path).await.body;
        assert!(!body.contains("NOT AVAILABLE"), "{path}");
        assert!(!body.contains("not available yet"), "{path}");
        assert!(!body.contains("coming soon"), "{path}");
    }
    let body = get(paths::SKILL_MD).await.body;
    assert!(body.contains("playtest 5173"), "隧道是一条真路，要写出来");
    assert!(body.contains("--backend"), "带后端的那条也是");
}

/// 助手要把二维码和邀请卡递给用户，而不只是提一句它们存在。
#[tokio::test]
async fn the_skill_says_to_hand_over_the_qr_and_the_card() {
    let body = get(paths::SKILL_MD).await.body;
    assert!(body.contains("qr_text"));
    assert!(body.contains("code block"), "二维码要在等宽块里才扫得出");
    assert!(body.contains("card_path"));
    assert!(body.contains("Attach the image"));
}

/// skill 文件要能被直接装：前言是 YAML，带 name 与 description。
#[tokio::test]
async fn the_skill_file_has_a_front_matter_an_installer_can_read() {
    let body = get(paths::SKILL_MD).await.body;
    assert!(body.starts_with("---\n"), "要有 YAML 前言：{}", &body[..40]);
    let front = body.split("---").nth(1).expect("前言");
    assert!(front.contains("name: playtest"));
    assert!(front.contains("description:"));
}

/// 不装二进制的助手照着 OpenAPI 也能发布。
#[tokio::test]
async fn the_spec_carries_the_whole_publish_flow() {
    let spec: Value = serde_json::from_str(&get(paths::OPENAPI_JSON).await.body).unwrap();
    assert_eq!(spec["openapi"], "3.1.0");
    let listed = &spec["paths"];
    assert!(listed[paths::SITES]["post"].is_object(), "建作品");
    assert!(listed[paths::SITE_UPLOADS]["post"].is_object(), "问缺哪些");
    assert!(listed[paths::BLOB]["put"].is_object(), "传文件");
    assert!(
        listed[paths::SITE_UPLOAD_COMMIT]["post"].is_object(),
        "提交上线"
    );
    assert!(spec["components"]["securitySchemes"]["bearer"].is_object());
}

/// 发现清单把别的几个文件指出来，助手拿到任何一个入口都能找到其余的。
#[tokio::test]
async fn the_discovery_manifest_points_at_the_rest() {
    let manifest: Value = serde_json::from_str(&get(paths::AGENT_JSON).await.body).unwrap();
    assert_eq!(manifest["name"], "playtest");
    for key in ["documentation", "openapi", "skill"] {
        let url = manifest[key].as_str().unwrap_or_default();
        assert!(url.starts_with("https://"), "{key} 要是完整地址：{url}");
    }
    // MCP 是首选路径，清单里要说怎么起它。
    assert_eq!(manifest["mcp"]["command"], "playtest");
}

/// 它们不该出现在玩家域上。玩家域只放玩家看的东西（AGENTS 第 7 条）；
/// 这一条守的是「控制面自己不越界」，边缘那一侧另有测试。
#[tokio::test]
async fn these_live_on_the_developer_domain() {
    let manifest: Value = serde_json::from_str(&get(paths::AGENT_JSON).await.body).unwrap();
    for key in ["documentation", "openapi", "skill"] {
        let url = manifest[key].as_str().unwrap();
        assert!(
            url.contains(playtest_common::DEVELOPER_HOST),
            "{key} 指到了别的域：{url}"
        );
        assert!(
            !url.contains("playtest.run/"),
            "{key} 不该在玩家域上：{url}"
        );
    }
}
