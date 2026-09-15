use axum::body::{to_bytes, Body};
use axum::http::{header, HeaderMap, Request, StatusCode};
use playtest_api::{app, AppState, Config};
use playtest_common::follow::routes as follow_paths;
use serde_json::{json, Value};
use tower::ServiceExt;

const ROOT: &str = "https://playtest.run";

struct Harness {
    state: AppState,
    _directory: tempfile::TempDir,
}

impl Harness {
    async fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let mut config = Config {
            data_dir: directory.path().to_path_buf(),
            ..Config::default()
        };
        config.notify.root_url = ROOT.to_string();
        let state = AppState::from_config(&config).await.unwrap();
        Self {
            state,
            _directory: directory,
        }
    }

    async fn request(
        &self,
        method: &str,
        path: &str,
        cookie: Option<&str>,
        origin: Option<&str>,
        body: Value,
    ) -> (StatusCode, HeaderMap, Value) {
        let mut request = Request::builder()
            .method(method)
            .uri(path)
            .header(header::CONTENT_TYPE, "application/json");
        if let Some(cookie) = cookie {
            request = request.header(header::COOKIE, cookie);
        }
        if let Some(origin) = origin {
            request = request.header(header::ORIGIN, origin);
        }
        let response = app(self.state.clone())
            .oneshot(request.body(Body::from(body.to_string())).unwrap())
            .await
            .unwrap();
        let status = response.status();
        let headers = response.headers().clone();
        let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
        let body = if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes).unwrap()
        };
        (status, headers, body)
    }

    async fn email_token(
        &self,
        email: &str,
        cookie: Option<&str>,
        link: bool,
        return_to: &str,
    ) -> String {
        let (status, _, body) = self
            .request(
                "POST",
                "/v1/account/email",
                cookie,
                Some(ROOT),
                json!({"email":email,"link":link,"return_to":return_to}),
            )
            .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let conn = self.state.db().read().await;
        let url: String = conn.query_row("SELECT n.url FROM notifications n JOIN players p ON p.id=n.player_id WHERE p.email=?1 ORDER BY n.id DESC LIMIT 1",[email],|row|row.get(0)).unwrap();
        reqwest::Url::parse(&url)
            .unwrap()
            .query_pairs()
            .find(|(key, _)| key == "email_token")
            .unwrap()
            .1
            .into_owned()
    }

    async fn login(&self, email: &str) -> String {
        let token = self.email_token(email, None, false, "/me").await;
        let (status, headers, body) = self
            .request(
                "POST",
                "/v1/account/email/confirm",
                None,
                Some(ROOT),
                json!({"token":token}),
            )
            .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let cookie = headers.get(header::SET_COOKIE).unwrap().to_str().unwrap();
        assert!(cookie.starts_with("__Host-pt_session="));
        assert!(
            cookie.contains("HttpOnly")
                && cookie.contains("Secure")
                && cookie.contains("SameSite=Lax")
                && cookie.contains("Path=/")
        );
        assert!(!cookie.contains("Domain="));
        cookie.split(';').next().unwrap().to_string()
    }
}

#[tokio::test]
async fn first_email_login_is_a_long_term_account_without_subscriptions() {
    let harness = Harness::new().await;
    let cookie = harness.login("creator@example.test").await;
    let (status, headers, body) = harness
        .request("GET", "/v1/account", Some(&cookie), None, Value::Null)
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(headers[header::CACHE_CONTROL], "no-store");
    assert_eq!(body["account"]["me"]["kind"], "email");
    assert_eq!(body["account"]["email"], "creator@example.test");
    let conn = harness.state.db().read().await;
    assert_eq!(
        conn.query_row("SELECT count(*) FROM follows", [], |row| row
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
    drop(conn);
    let (status, _, body) = harness
        .request("GET", "/v1/projects", Some(&cookie), None, Value::Null)
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body, json!([]));
}

#[tokio::test]
async fn email_preview_does_not_consume_the_single_use_link() {
    let harness = Harness::new().await;
    let token = harness
        .email_token(
            "reader@example.test",
            None,
            false,
            "/console/#/collections/topic",
        )
        .await;
    for _ in 0..2 {
        let (status, _, body) = harness
            .request(
                "POST",
                "/v1/account/email/preview",
                None,
                Some(ROOT),
                json!({"token":token}),
            )
            .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert!(body["email"].as_str().unwrap().contains("example.test"));
    }
    let (status, _, body) = harness
        .request(
            "POST",
            "/v1/account/email/confirm",
            None,
            Some(ROOT),
            json!({"token":token}),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["return_to"], "/console/#/collections/topic");
    assert_eq!(
        harness
            .request(
                "POST",
                "/v1/account/email/confirm",
                None,
                Some(ROOT),
                json!({"token":token})
            )
            .await
            .0,
        StatusCode::BAD_REQUEST
    );
}

#[tokio::test]
async fn same_account_has_independent_revocable_device_sessions() {
    let harness = Harness::new().await;
    let first = harness.login("repeat@example.test").await;
    let second = harness.login("repeat@example.test").await;
    assert_ne!(first, second);
    let conn = harness.state.db().read().await;
    assert_eq!(
        conn.query_row("SELECT count(*) FROM users", [], |row| row.get::<_, i64>(0))
            .unwrap(),
        1
    );
    drop(conn);
    assert_eq!(
        harness
            .request(
                "POST",
                "/v1/account/logout",
                Some(&first),
                Some(ROOT),
                json!({})
            )
            .await
            .0,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        harness
            .request("GET", "/v1/me", Some(&first), None, Value::Null)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        harness
            .request("GET", "/v1/me", Some(&second), None, Value::Null)
            .await
            .0,
        StatusCode::OK
    );
}

#[tokio::test]
async fn publishing_and_following_use_the_same_revocable_identity() {
    let harness = Harness::new().await;
    let cookie = harness.login("both@example.test").await;
    let token = cookie.split_once('=').unwrap().1;
    let (status, _, body) = harness
        .request(
            "POST",
            follow_paths::FOLLOW,
            None,
            None,
            json!({"target":{"kind":"plaza"},"channel":{"kind":"me","me_token":token}}),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let (status, _, body) = harness
        .request(
            "POST",
            follow_paths::ME_VIEW,
            None,
            None,
            json!({"me_token":token}),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["follows"].as_array().unwrap().len(), 1);
    harness
        .request(
            "POST",
            "/v1/account/logout",
            Some(&cookie),
            Some(ROOT),
            json!({}),
        )
        .await;
    assert_eq!(
        harness
            .request(
                "POST",
                follow_paths::ME_VIEW,
                None,
                None,
                json!({"me_token":token})
            )
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn sibling_origins_and_ambiguous_cookies_cannot_manage_the_account() {
    let harness = Harness::new().await;
    let cookie = harness.login("security@example.test").await;
    for origin in [
        None,
        Some("https://evil.playtest.run"),
        Some("https://attacker.test"),
    ] {
        assert_eq!(
            harness
                .request(
                    "POST",
                    "/v1/account/tokens",
                    Some(&cookie),
                    origin,
                    json!({})
                )
                .await
                .0,
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            harness
                .request(
                    "PATCH",
                    "/v1/account",
                    Some(&cookie),
                    origin,
                    json!({"display_name":"changed"})
                )
                .await
                .0,
            StatusCode::FORBIDDEN
        );
    }
    let duplicate = format!("{cookie}; {cookie}");
    assert_eq!(
        harness
            .request("GET", "/v1/me", Some(&duplicate), None, Value::Null)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn return_urls_cannot_escape_the_platform() {
    let harness = Harness::new().await;
    for (index, target) in [
        "https://evil.test",
        "//evil.test",
        "/\\evil.test",
        "/\t/evil.test",
    ]
    .into_iter()
    .enumerate()
    {
        let token = harness
            .email_token(&format!("return{index}@example.test"), None, false, target)
            .await;
        let (status, _, body) = harness
            .request(
                "POST",
                "/v1/account/email/confirm",
                None,
                Some(ROOT),
                json!({"token":token}),
            )
            .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["return_to"], "/");
    }
}

#[tokio::test]
async fn email_association_never_silently_merges_another_account() {
    let harness = Harness::new().await;
    let first = harness.login("one@example.test").await;
    let _second = harness.login("two@example.test").await;
    let token = harness
        .email_token("two@example.test", Some(&first), true, "/console/#/token")
        .await;
    assert_eq!(
        harness
            .request(
                "POST",
                "/v1/account/email/confirm",
                None,
                Some(ROOT),
                json!({"token":token})
            )
            .await
            .0,
        StatusCode::BAD_REQUEST
    );
    let (status, _, body) = harness
        .request(
            "POST",
            "/v1/account/email/confirm",
            Some(&first),
            Some(ROOT),
            json!({"token":token}),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert_eq!(
        harness
            .request("GET", "/v1/account", Some(&first), None, Value::Null)
            .await
            .2["account"]["email"],
        "one@example.test"
    );
}

#[tokio::test]
async fn developer_tokens_are_separate_from_browser_sessions() {
    let harness = Harness::new().await;
    let cookie = harness.login("tokens@example.test").await;
    let (status, _, body) = harness
        .request(
            "POST",
            "/v1/account/tokens",
            Some(&cookie),
            Some(ROOT),
            json!({}),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let token = body["token"].as_str().unwrap();
    let fake_cookie = format!("__Host-pt_session={token}");
    assert_eq!(
        harness
            .request("GET", "/v1/me", Some(&fake_cookie), None, Value::Null)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    let response = app(harness.state.clone())
        .oneshot(
            Request::builder()
                .uri("/v1/me")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let browser_token = cookie.split_once('=').unwrap().1;
    let response = app(harness.state.clone())
        .oneshot(
            Request::builder()
                .uri("/v1/me")
                .header(header::AUTHORIZATION, format!("Bearer {browser_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let (_, _, listed) = harness
        .request(
            "GET",
            "/v1/account/tokens",
            Some(&cookie),
            None,
            Value::Null,
        )
        .await;
    let id = listed[0]["id"].as_str().unwrap();
    assert!(!listed.to_string().contains(token));
    assert_eq!(
        harness
            .request(
                "DELETE",
                &format!("/v1/account/tokens/{id}"),
                Some(&cookie),
                Some(ROOT),
                Value::Null
            )
            .await
            .0,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        harness
            .request("GET", "/v1/me", Some(&cookie), None, Value::Null)
            .await
            .0,
        StatusCode::OK
    );
}
