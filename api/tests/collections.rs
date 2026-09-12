use axum::{
    body::Body,
    http::{Request, StatusCode},
    Router,
};
use playtest_api::{app, clock, collections, db, AppState, Config};
use playtest_common::{
    api::{routes as projects, PrepareUploadResponse},
    collection::CollectionDraft,
    hash,
};
use serde_json::{json, Value};
use tower::ServiceExt;

struct Harness {
    state: AppState,
    router: Router,
    _directory: tempfile::TempDir,
}

impl Harness {
    async fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let config = Config {
            data_dir: directory.path().to_owned(),
            admin_token: Some("test-admin".into()),
            ..Config::default()
        };
        let state = AppState::from_config(&config).await.unwrap();
        Self {
            router: app(state.clone()),
            state,
            _directory: directory,
        }
    }

    async fn user(&self, name: &str, kind: &str) -> String {
        let token = format!("collection-test-token-{name}");
        let conn = self.state.db().lock().await;
        let now = clock::now_string();
        db::insert_user(
            &conn,
            &db::NewUser {
                id: name,
                kind,
                display_name: name,
                created_at: &now,
                expires_at: None,
            },
        )
        .unwrap();
        db::insert_token(&conn, &hash::hash_bytes(token.as_bytes()), name, &now, None).unwrap();
        token
    }

    async fn send(
        &self,
        method: &str,
        path: &str,
        token: &str,
        body: Body,
        expected: StatusCode,
    ) -> Value {
        let request = Request::builder()
            .method(method)
            .uri(path)
            .header("content-type", "application/json")
            .header("authorization", format!("Bearer {token}"))
            .body(body)
            .unwrap();
        let response = self.router.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let bytes = axum::body::to_bytes(response.into_body(), 1024 * 1024)
            .await
            .unwrap();
        assert_eq!(
            status,
            expected,
            "{method} {path}: {}",
            String::from_utf8_lossy(&bytes)
        );
        serde_json::from_slice(&bytes).unwrap_or(Value::Null)
    }

    async fn call(
        &self,
        method: &str,
        path: &str,
        token: &str,
        value: Value,
        expected: StatusCode,
    ) -> Value {
        self.send(method, path, token, Body::from(value.to_string()), expected)
            .await
    }

    async fn project(&self, token: &str, name: &str, public: bool) -> String {
        let site = self
            .call(
                "POST",
                projects::SITES,
                token,
                json!({"title":name,"slug":name}),
                StatusCode::OK,
            )
            .await;
        let slug = site["slug"].as_str().unwrap().to_string();
        let content = b"<!doctype html><title>Original test work</title><button>Play</button>";
        let file_hash = hash::hash_bytes(content);
        let prepared: PrepareUploadResponse = serde_json::from_value(self.call("POST", &projects::site_uploads(&slug), token, json!({"files":[{"path":"index.html","hash":file_hash,"size":content.len()}],"title":name,"gate":"once","isolated":false,"spa":false}), StatusCode::OK).await).unwrap();
        for missing in prepared.missing {
            self.send(
                "PUT",
                &projects::blob(&missing),
                token,
                Body::from(content.to_vec()),
                StatusCode::CREATED,
            )
            .await;
        }
        self.call(
            "POST",
            &projects::site_upload_commit(&slug, &prepared.upload_id),
            token,
            Value::Null,
            StatusCode::OK,
        )
        .await;
        self.call(
            "PATCH",
            &projects::site(&slug),
            token,
            json!({"public":public}),
            StatusCode::OK,
        )
        .await;
        slug
    }

    async fn challenge(&self, token: &str) {
        self.call("POST", "/v1/collections", token, json!({"slug":"pelican","title":"鹈鹕挑战","kind":"challenge","prompt":"让鹈鹕骑单车","public":true}), StatusCode::OK).await;
    }
}

#[tokio::test]
async fn publication_is_explicit_and_ownership_is_enforced() {
    let harness = Harness::new().await;
    let owner = harness.user("owner", "github").await;
    let other = harness.user("other", "github").await;
    let anon = harness.user("anon", "anon").await;
    harness
        .call(
            "POST",
            "/v1/collections",
            &anon,
            json!({"title":"temporary"}),
            StatusCode::FORBIDDEN,
        )
        .await;
    harness
        .call(
            "POST",
            "/v1/collections",
            &owner,
            json!({"slug":"portfolio","title":"自己的作品"}),
            StatusCode::OK,
        )
        .await;
    assert!(harness
        .state
        .store()
        .get_plaza()
        .await
        .unwrap()
        .unwrap()
        .collections
        .is_empty());
    harness
        .call(
            "GET",
            "/v1/collections/portfolio",
            &other,
            Value::Null,
            StatusCode::NOT_FOUND,
        )
        .await;
    harness
        .call(
            "PUT",
            "/v1/collections/portfolio",
            &other,
            json!({"title":"stolen"}),
            StatusCode::NOT_FOUND,
        )
        .await;
    harness
        .call(
            "PUT",
            "/v1/collections/portfolio",
            &owner,
            json!({"title":"自己的作品","public":true}),
            StatusCode::OK,
        )
        .await;
    let private = harness.project(&owner, "private-work", false).await;
    harness
        .call(
            "POST",
            "/v1/collections/portfolio/entries",
            &owner,
            json!({"slug":private}),
            StatusCode::BAD_REQUEST,
        )
        .await;
    let public = harness.project(&owner, "public-work", true).await;
    harness
        .call(
            "POST",
            "/v1/collections/portfolio/entries",
            &other,
            json!({"slug":public}),
            StatusCode::NOT_FOUND,
        )
        .await;
    harness
        .call(
            "POST",
            "/v1/collections/portfolio/entries",
            &owner,
            json!({"slug":public}),
            StatusCode::OK,
        )
        .await;
    let snapshot = harness.state.store().get_plaza().await.unwrap().unwrap();
    assert_eq!(snapshot.collections[0].entries.len(), 1);
    assert_eq!(snapshot.collections[0].entries[0].title, "public-work");
}

#[tokio::test]
async fn challenge_accepts_only_owned_public_long_lived_uploads() {
    let harness = Harness::new().await;
    let owner = harness.user("owner", "github").await;
    let author = harness.user("author", "github").await;
    harness.challenge(&owner).await;
    let slug = harness.project(&author, "test-pelican", true).await;
    harness
        .call(
            "POST",
            "/v1/collections/pelican/entries",
            &owner,
            json!({"slug":slug}),
            StatusCode::NOT_FOUND,
        )
        .await;
    harness.call("POST", "/v1/collections/pelican/entries", &author, json!({"slug":slug,"model":"Example 1","method":"one_shot","prompt":"<script>alert(1)</script>"}), StatusCode::OK).await;
    let result = harness
        .call(
            "POST",
            "/v1/collections/pelican/entries",
            &author,
            json!({"slug":slug,"model":"Example 2","method":"iterated"}),
            StatusCode::OK,
        )
        .await;
    assert_eq!(result["entries"].as_array().unwrap().len(), 1);
    assert_eq!(result["entries"][0]["model"], "Example 2");
    harness
        .call(
            "PATCH",
            &projects::site(&slug),
            &author,
            json!({"public":false}),
            StatusCode::OK,
        )
        .await;
    assert!(harness
        .state
        .store()
        .get_plaza()
        .await
        .unwrap()
        .unwrap()
        .collections[0]
        .entries
        .is_empty());
    harness.call("PUT", "/v1/collections/pelican", &owner, json!({"title":"鹈鹕挑战","kind":"challenge","prompt":"私密投稿也不允许换题","public":true}), StatusCode::BAD_REQUEST).await;
    harness
        .call(
            "PATCH",
            &projects::site(&slug),
            &author,
            json!({"public":true}),
            StatusCode::OK,
        )
        .await;
    {
        let conn = harness.state.db().lock().await;
        conn.execute(
            "UPDATE sites SET expires_at='2030-01-01T00:00:00Z' WHERE slug=?1",
            [&slug],
        )
        .unwrap();
    }
    harness
        .call(
            "POST",
            "/v1/collections/pelican/entries",
            &author,
            json!({"slug":slug}),
            StatusCode::BAD_REQUEST,
        )
        .await;
    {
        let conn = harness.state.db().lock().await;
        conn.execute("UPDATE sites SET expires_at=NULL WHERE slug=?1", [&slug])
            .unwrap();
    }
    let mut manifest = harness
        .state
        .store()
        .get_manifest(&slug, 1)
        .await
        .unwrap()
        .unwrap();
    manifest.files.clear();
    harness.state.store().put_manifest(&manifest).await.unwrap();
    harness
        .call(
            "POST",
            "/v1/collections/pelican/entries",
            &author,
            json!({"slug":slug}),
            StatusCode::BAD_REQUEST,
        )
        .await;
}

#[tokio::test]
async fn deadlines_moderation_and_withdrawal_are_real_states() {
    let harness = Harness::new().await;
    let owner = harness.user("owner", "github").await;
    let author = harness.user("author", "github").await;
    harness.challenge(&owner).await;
    let slug = harness.project(&author, "test-pelican", true).await;
    let path = format!("/v1/collections/pelican/entries/{slug}");
    harness
        .call(
            "POST",
            "/v1/collections/pelican/entries",
            &author,
            json!({"slug":slug}),
            StatusCode::OK,
        )
        .await;
    harness
        .call("DELETE", &path, &owner, Value::Null, StatusCode::NO_CONTENT)
        .await;
    harness
        .call(
            "POST",
            "/v1/collections/pelican/entries",
            &author,
            json!({"slug":slug}),
            StatusCode::BAD_REQUEST,
        )
        .await;
    harness
        .call(
            "DELETE",
            &format!("/v1/collections/pelican/blocks/{slug}"),
            &owner,
            Value::Null,
            StatusCode::NO_CONTENT,
        )
        .await;
    harness
        .call(
            "POST",
            "/v1/collections/pelican/entries",
            &author,
            json!({"slug":slug}),
            StatusCode::OK,
        )
        .await;
    {
        let conn = harness.state.db().lock().await;
        conn.execute(
            "UPDATE collections SET closes_at='2020-01-01T00:00:00Z' WHERE slug='pelican'",
            [],
        )
        .unwrap();
    }
    harness
        .call(
            "POST",
            "/v1/collections/pelican/entries",
            &author,
            json!({"slug":slug}),
            StatusCode::BAD_REQUEST,
        )
        .await;
    harness
        .call(
            "DELETE",
            &path,
            &author,
            Value::Null,
            StatusCode::NO_CONTENT,
        )
        .await;
    harness
        .call(
            "PUT",
            "/v1/admin/collections/pelican",
            "test-admin",
            json!({"hidden":true}),
            StatusCode::NO_CONTENT,
        )
        .await;
    harness.call("PUT", "/v1/collections/pelican", &owner, json!({"title":"still hidden","kind":"challenge","prompt":"让鹈鹕骑单车","public":true}), StatusCode::OK).await;
    assert!(harness
        .state
        .store()
        .get_plaza()
        .await
        .unwrap()
        .unwrap()
        .collections
        .is_empty());
    harness
        .call(
            "GET",
            "/v1/collections/pelican",
            &author,
            Value::Null,
            StatusCode::NOT_FOUND,
        )
        .await;
    harness
        .call(
            "DELETE",
            "/v1/collections/pelican",
            &owner,
            Value::Null,
            StatusCode::NO_CONTENT,
        )
        .await;
    harness
        .call(
            "GET",
            &projects::site(&slug),
            &author,
            Value::Null,
            StatusCode::OK,
        )
        .await;
}

#[tokio::test]
async fn follow_requires_confirmation_and_digest_rechecks_privacy() {
    let harness = Harness::new().await;
    let owner = harness.user("owner", "github").await;
    harness.challenge(&owner).await;
    let slug = harness.project(&owner, "test-pelican", true).await;
    harness.call("POST", "/v1/follow", "", json!({"target":{"kind":"collection","slug":"pelican"},"channel":{"kind":"email","email":"collection@example.com"}}), StatusCode::OK).await;
    let confirmation = {
        let conn = harness.state.db().read().await;
        assert!(
            db::confirmed_followers(&conn, "collection", Some("pelican"))
                .unwrap()
                .is_empty()
        );
        let body: String = conn
            .query_row(
                "SELECT body FROM notifications WHERE kind='confirm'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(body.contains("每周有新投稿"));
        body.split("/me/confirm/")
            .nth(1)
            .unwrap()
            .split_whitespace()
            .next()
            .unwrap()
            .to_string()
    };
    let confirmed = harness
        .call(
            "POST",
            "/v1/follow/confirm",
            "",
            json!({"token":confirmation}),
            StatusCode::OK,
        )
        .await;
    let token = confirmed["me_token"].as_str().unwrap();
    assert_eq!(confirmed["me"]["follows"][0]["title"], "鹈鹕挑战");
    harness
        .call(
            "POST",
            "/v1/collections/pelican/entries",
            &owner,
            json!({"slug":slug}),
            StatusCode::OK,
        )
        .await;
    let now = clock::now();
    {
        let conn = harness.state.db().lock().await;
        conn.execute(
            "UPDATE follows SET created_at=?1 WHERE target_kind='collection'",
            [clock::format(now - time::Duration::days(1))],
        )
        .unwrap();
        assert_eq!(
            collections::enqueue_digests(&conn, now, harness.state.notify().root_url()).unwrap(),
            1
        );
        assert_eq!(
            collections::enqueue_digests(&conn, now, harness.state.notify().root_url()).unwrap(),
            0
        );
        let mut row = db::due_notifications(&conn, &clock::format(now), 30)
            .unwrap()
            .into_iter()
            .find(|row| row.kind == "collection_digest")
            .unwrap();
        assert!(collections::refresh_digest(
            &conn,
            &mut row,
            harness.state.notify().root_url(),
            now
        )
        .unwrap());
        conn.execute("UPDATE sites SET public=0 WHERE slug=?1", [&slug])
            .unwrap();
        assert!(!collections::refresh_digest(
            &conn,
            &mut row,
            harness.state.notify().root_url(),
            now
        )
        .unwrap());
    }
    harness
        .call(
            "POST",
            "/v1/me/unfollow",
            "",
            json!({"me_token":token,"target":{"kind":"collection","slug":"pelican"}}),
            StatusCode::OK,
        )
        .await;
    let conn = harness.state.db().read().await;
    assert!(
        db::confirmed_followers(&conn, "collection", Some("pelican"))
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn dates_are_normalized_and_text_is_bounded() {
    let harness = Harness::new().await;
    let owner = harness.user("owner", "github").await;
    let value = harness.call("POST", "/v1/collections", &owner, json!({"title":"deadline","kind":"challenge","prompt":"same prompt","closes_at":"2030-01-01T12:00:00+08:00","public":true}), StatusCode::OK).await;
    assert_eq!(value["closes_at"], "2030-01-01T04:00:00Z");
    harness
        .call(
            "POST",
            "/v1/collections",
            &owner,
            json!({"title":"x".repeat(81)}),
            StatusCode::BAD_REQUEST,
        )
        .await;
    harness
        .call(
            "POST",
            "/v1/collections",
            &owner,
            json!({"title":"no prompt","kind":"challenge","public":true}),
            StatusCode::BAD_REQUEST,
        )
        .await;
    let draft: CollectionDraft = serde_json::from_value(json!({"title":"draft"})).unwrap();
    assert!(!draft.public);
}
