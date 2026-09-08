//! 走一遍 CLI 会走的那条路，以及它会撞上的墙。
//!
//! 请求直接喂给 `Router`（`tower::ServiceExt::oneshot`），不起监听端口：
//! 中间层、提取器、处理函数和真跑起来时是同一套，只是省掉了一次 TCP。
//! 真机上用 curl 走的那一遍记在 `docs/spikes/2026-09-07-api-upload-flow.md`。

use axum::body::{Body, Bytes};
use axum::http::{header, Request, StatusCode};
use axum::Router;
use playtest_api::{app, AppState, Config};
use playtest_common::api::{
    routes as paths, AnonSessionResponse, CommitUploadResponse, CreateSiteRequest, ErrorBody,
    ErrorCode, PrepareUploadRequest, PrepareUploadResponse, Site, VersionList,
};
use playtest_common::hash;
use playtest_common::manifest::{FileEntry, GateMode};
use playtest_common::store::FsStore;
use serde::de::DeserializeOwned;
use serde::Serialize;
use tower::ServiceExt;

const INDEX_HTML: &[u8] = b"<!doctype html><meta charset=utf-8><title>\xe5\xb0\x8f\xe7\x90\x83</title><canvas id=game></canvas>";
const SPRITE_PNG: &[u8] = b"\x89PNG\r\n\x1a\n not really a png";

struct Harness {
    router: Router,
    store: FsStore,
    state: AppState,
    _dir: tempfile::TempDir,
}

struct Reply {
    status: StatusCode,
    body: Bytes,
}

impl Reply {
    fn json<T: DeserializeOwned>(&self) -> T {
        assert!(
            self.status.is_success(),
            "本来该成功的：{} {}",
            self.status,
            self.text()
        );
        serde_json::from_slice(&self.body)
            .unwrap_or_else(|err| panic!("响应不是预期的 JSON：{err}；原文是 {}", self.text()))
    }

    /// 断言这是一次失败，并把错误体拿出来。
    fn error(&self, status: StatusCode, code: ErrorCode) -> ErrorBody {
        assert_eq!(self.status, status, "状态码不对，响应是 {}", self.text());
        let body: ErrorBody = serde_json::from_slice(&self.body)
            .unwrap_or_else(|err| panic!("错误响应不是 ErrorBody：{err}；原文是 {}", self.text()));
        assert_eq!(body.code, code, "错误码不对：{}", body.message);
        body
    }

    fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }
}

impl Harness {
    async fn start() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let config = Config {
            listen: "127.0.0.1:0".parse().unwrap(),
            data_dir: dir.path().to_path_buf(),
            site_url_template: "http://{slug}.localhost:8443".to_string(),
        };
        let state = AppState::from_config(&config).await.unwrap();
        Self {
            router: app(state.clone()),
            store: FsStore::new(config.store_root()),
            state,
            _dir: dir,
        }
    }

    async fn send(&self, request: Request<Body>) -> Reply {
        let response = self.router.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        Reply { status, body }
    }

    async fn request(
        &self,
        method: &str,
        path: &str,
        token: Option<&str>,
        body: Body,
        json: bool,
    ) -> Reply {
        let mut builder = Request::builder().method(method).uri(path);
        if let Some(token) = token {
            builder = builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
        }
        if json {
            builder = builder.header(header::CONTENT_TYPE, "application/json");
        }
        self.send(builder.body(body).unwrap()).await
    }

    async fn get(&self, path: &str, token: &str) -> Reply {
        self.request("GET", path, Some(token), Body::empty(), false)
            .await
    }

    async fn post<T: Serialize>(&self, path: &str, token: Option<&str>, value: &T) -> Reply {
        let body = Body::from(serde_json::to_vec(value).unwrap());
        self.request("POST", path, token, body, true).await
    }

    async fn put_bytes(&self, path: &str, token: &str, bytes: &[u8]) -> Reply {
        self.request("PUT", path, Some(token), Body::from(bytes.to_vec()), false)
            .await
    }

    async fn anon_token(&self) -> String {
        let reply = self
            .request("POST", paths::ANON_SESSIONS, None, Body::empty(), false)
            .await;
        reply.json::<AnonSessionResponse>().token
    }

    async fn new_site(&self, token: &str) -> Site {
        self.post(paths::SITES, Some(token), &CreateSiteRequest::default())
            .await
            .json()
    }

    /// 直接往库里塞一个昨天就该失效的令牌。
    async fn stale_token(&self) -> String {
        let token = "expired-token-for-test";
        let conn = self.state.db().lock().await;
        playtest_api::db::insert_user(
            &conn,
            &playtest_api::db::NewUser {
                id: "user-expired",
                kind: "anon",
                display_name: "匿名开发者",
                created_at: "2020-01-01T00:00:00Z",
                expires_at: Some("2020-01-02T00:00:00Z"),
            },
        )
        .unwrap();
        playtest_api::db::insert_token(
            &conn,
            &hash::hash_bytes(token.as_bytes()),
            "user-expired",
            "2020-01-01T00:00:00Z",
            Some("2020-01-02T00:00:00Z"),
        )
        .unwrap();
        token.to_string()
    }

    /// 临时目录里不该留下东西——每条失败路径都要能自己收拾干净。
    fn assert_no_leftovers(&self) {
        let tmp = self.store.tmp_dir();
        if !tmp.exists() {
            return;
        }
        let leftovers: Vec<_> = std::fs::read_dir(&tmp)
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        assert!(leftovers.is_empty(), "临时目录里剩了东西：{leftovers:?}");
    }
}

fn entry(path: &str, bytes: &[u8]) -> FileEntry {
    FileEntry {
        path: path.to_string(),
        hash: hash::hash_bytes(bytes),
        size: bytes.len() as u64,
    }
}

fn prepare_request(files: Vec<FileEntry>) -> PrepareUploadRequest {
    PrepareUploadRequest {
        files,
        title: Some("小球".to_string()),
        note: Some("第一版：能动了".to_string()),
        summary: None,
        cover: None,
        gate: GateMode::Once,
        isolated: true,
        spa: false,
        engine: None,
    }
}

#[tokio::test]
async fn health_is_plain_ok() {
    let h = Harness::start().await;
    let reply = h
        .request("GET", paths::HEALTH, None, Body::empty(), false)
        .await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.text(), "ok");
}

/// 匿名会话 → 建作品 → 准备 → 传缺的 → 提交 → 再来一次拿到 v2。
#[tokio::test]
async fn anonymous_upload_reaches_v2() {
    let h = Harness::start().await;
    let token = h.anon_token().await;

    let site = h.new_site(&token).await;
    assert_eq!(site.url, format!("http://{}.localhost:8443", site.slug));
    assert_eq!(site.current_version, None);
    assert!(site.expires_at.is_some(), "匿名作品要有到期时间");
    // 形容词-动物-两位数
    let parts: Vec<&str> = site.slug.split('-').collect();
    assert_eq!(parts.len(), 3, "随机名字长得不对：{}", site.slug);
    assert!(parts[2]
        .parse::<u32>()
        .is_ok_and(|n| (10..=99).contains(&n)));

    let files = vec![
        entry("index.html", INDEX_HTML),
        entry("art/ball.png", SPRITE_PNG),
    ];
    let prepared: PrepareUploadResponse = h
        .post(
            &paths::site_uploads(&site.slug),
            Some(&token),
            &prepare_request(files.clone()),
        )
        .await
        .json();
    assert_eq!(prepared.missing.len(), 2, "两个文件都没见过");
    assert_eq!(
        prepared.missing_bytes,
        (INDEX_HTML.len() + SPRITE_PNG.len()) as u64
    );
    // 按清单顺序，不是哈希顺序。
    assert_eq!(prepared.missing[0], files[0].hash);

    for (file, bytes) in files.iter().zip([INDEX_HTML, SPRITE_PNG]) {
        let reply = h.put_bytes(&paths::blob(&file.hash), &token, bytes).await;
        assert_eq!(reply.status, StatusCode::CREATED, "第一次传应该是 201");
    }
    // 再传一次同样的内容：已经有了，200，不重复占盘。
    let again = h
        .put_bytes(&paths::blob(&files[0].hash), &token, INDEX_HTML)
        .await;
    assert_eq!(again.status, StatusCode::OK);

    let committed: CommitUploadResponse = h
        .post(
            &paths::site_upload_commit(&site.slug, &prepared.upload_id),
            Some(&token),
            &serde_json::json!({}),
        )
        .await
        .json();
    assert_eq!(committed.version, 1);
    assert_eq!(committed.slug, site.slug);
    assert_eq!(committed.url, site.url);

    // 对象存储里要有清单和指针，边缘只认这两样。
    let manifest = h.store.get_manifest(&site.slug, 1).await.unwrap().unwrap();
    assert_eq!(manifest.schema, playtest_common::manifest::SCHEMA);
    assert_eq!(manifest.title, "小球");
    assert_eq!(manifest.developer, "匿名开发者");
    assert_eq!(manifest.note.as_deref(), Some("第一版：能动了"));
    assert!(manifest.badge, "匿名作品的门禁页要带角标");
    assert!(manifest.isolated);
    assert!(!manifest.spa);
    assert_eq!(manifest.expires_at, site.expires_at);
    assert_eq!(
        manifest
            .files
            .iter()
            .map(|f| f.path.as_str())
            .collect::<Vec<_>>(),
        ["art/ball.png", "index.html"],
        "清单要按路径排序"
    );
    assert!(manifest.has_index());

    let current = h.store.get_current(&site.slug).await.unwrap().unwrap();
    assert_eq!(current.version, 1);

    // 同样的文件再来一遍：一个都不用传。
    let second: PrepareUploadResponse = h
        .post(
            &paths::site_uploads(&site.slug),
            Some(&token),
            &prepare_request(files.clone()),
        )
        .await
        .json();
    assert!(second.missing.is_empty(), "内容没变就不该再传一遍");
    assert_eq!(second.missing_bytes, 0);

    let v2: CommitUploadResponse = h
        .post(
            &paths::site_upload_commit(&site.slug, &second.upload_id),
            Some(&token),
            &serde_json::json!({}),
        )
        .await
        .json();
    assert_eq!(v2.version, 2);
    assert_eq!(
        h.store
            .get_current(&site.slug)
            .await
            .unwrap()
            .unwrap()
            .version,
        2
    );
    // 旧版本不可变，回滚要靠它。
    assert!(h.store.get_manifest(&site.slug, 1).await.unwrap().is_some());

    let seen: Site = h.get(&paths::site(&site.slug), &token).await.json();
    assert_eq!(seen.current_version, Some(2));

    let mine: Vec<Site> = h.get(paths::SITES, &token).await.json();
    assert_eq!(mine.len(), 1);
    assert_eq!(mine[0].slug, site.slug);

    // 版本列表：新的在前，标出当前。
    let list: VersionList = h
        .get(&paths::site_versions(&site.slug), &token)
        .await
        .json();
    assert_eq!(list.current_version, Some(2));
    assert_eq!(
        list.versions.iter().map(|v| v.version).collect::<Vec<_>>(),
        [2, 1]
    );
    assert!(list.versions[0].current && !list.versions[1].current);
    assert_eq!(list.versions[1].note.as_deref(), Some("第一版：能动了"));

    // 回滚到 v1：指针动了，清单没动，玩家立刻看到 v1。
    let rolled: Site = h
        .post(
            &paths::site_version_activate(&site.slug, 1),
            Some(&token),
            &serde_json::json!({}),
        )
        .await
        .json();
    assert_eq!(rolled.current_version, Some(1));
    assert_eq!(
        h.store
            .get_current(&site.slug)
            .await
            .unwrap()
            .unwrap()
            .version,
        1
    );
    assert!(
        h.store.get_manifest(&site.slug, 2).await.unwrap().is_some(),
        "回滚不删任何版本"
    );

    // 回滚到不存在的版本：说清楚。
    let nope = h
        .post(
            &paths::site_version_activate(&site.slug, 9),
            Some(&token),
            &serde_json::json!({}),
        )
        .await;
    let body = nope.error(StatusCode::NOT_FOUND, ErrorCode::NotFound);
    assert!(body.message.contains("v9"), "{}", body.message);

    // 回滚之后再发一版：是 v3，不是 v2——版本号只增不减，否则会话与反馈按版本归档就乱了。
    let third: PrepareUploadResponse = h
        .post(
            &paths::site_uploads(&site.slug),
            Some(&token),
            &prepare_request(files.clone()),
        )
        .await
        .json();
    let v3: CommitUploadResponse = h
        .post(
            &paths::site_upload_commit(&site.slug, &third.upload_id),
            Some(&token),
            &serde_json::json!({}),
        )
        .await
        .json();
    assert_eq!(v3.version, 3);
    assert_eq!(
        h.store
            .get_current(&site.slug)
            .await
            .unwrap()
            .unwrap()
            .version,
        3
    );

    h.assert_no_leftovers();
}

#[tokio::test]
async fn deleting_a_site_takes_the_link_down() {
    let h = Harness::start().await;
    let token = h.anon_token().await;
    let site = h.new_site(&token).await;

    let file = entry("index.html", INDEX_HTML);
    let prepared: PrepareUploadResponse = h
        .post(
            &paths::site_uploads(&site.slug),
            Some(&token),
            &prepare_request(vec![file.clone()]),
        )
        .await
        .json();
    h.put_bytes(&paths::blob(&file.hash), &token, INDEX_HTML)
        .await;
    h.post(
        &paths::site_upload_commit(&site.slug, &prepared.upload_id),
        Some(&token),
        &serde_json::json!({}),
    )
    .await
    .json::<CommitUploadResponse>();

    let deleted = h
        .request(
            "DELETE",
            &paths::site(&site.slug),
            Some(&token),
            Body::empty(),
            false,
        )
        .await;
    assert_eq!(deleted.status, StatusCode::NO_CONTENT);

    assert!(h.store.get_current(&site.slug).await.unwrap().is_none());
    h.get(&paths::site(&site.slug), &token)
        .await
        .error(StatusCode::NOT_FOUND, ErrorCode::NotFound);
    assert!(h
        .get(paths::SITES, &token)
        .await
        .json::<Vec<Site>>()
        .is_empty());

    // blob 是跨作品去重的，删作品不删内容。
    assert!(h.store.has_blob(&file.hash).await.unwrap());
}

#[tokio::test]
async fn wrong_bytes_are_rejected_and_nothing_is_kept() {
    let h = Harness::start().await;
    let token = h.anon_token().await;
    let claimed = hash::hash_bytes(INDEX_HTML);

    let reply = h
        .put_bytes(&paths::blob(&claimed), &token, SPRITE_PNG)
        .await;
    let body = reply.error(StatusCode::BAD_REQUEST, ErrorCode::HashMismatch);
    assert!(
        body.message.contains(&claimed),
        "要说清是哪个哈希对不上：{}",
        body.message
    );

    assert!(
        !h.store.has_blob(&claimed).await.unwrap(),
        "对不上的内容不能落地"
    );
    h.assert_no_leftovers();
}

#[tokio::test]
async fn hash_must_look_like_a_hash() {
    let h = Harness::start().await;
    let token = h.anon_token().await;
    // 想拿哈希那一段去翻目录的，先在这里被挡住。
    let reply = h.put_bytes("/v1/blobs/..%2f..%2fetc", &token, b"x").await;
    reply.error(StatusCode::BAD_REQUEST, ErrorCode::Invalid);
    h.assert_no_leftovers();
}

#[tokio::test]
async fn commit_without_the_files_says_which_ones() {
    let h = Harness::start().await;
    let token = h.anon_token().await;
    let site = h.new_site(&token).await;

    let files = vec![
        entry("index.html", INDEX_HTML),
        entry("art/ball.png", SPRITE_PNG),
    ];
    let prepared: PrepareUploadResponse = h
        .post(
            &paths::site_uploads(&site.slug),
            Some(&token),
            &prepare_request(files.clone()),
        )
        .await
        .json();
    // 只传一个就急着提交。
    h.put_bytes(&paths::blob(&files[0].hash), &token, INDEX_HTML)
        .await;

    let reply = h
        .post(
            &paths::site_upload_commit(&site.slug, &prepared.upload_id),
            Some(&token),
            &serde_json::json!({}),
        )
        .await;
    let body = reply.error(StatusCode::CONFLICT, ErrorCode::BlobsMissing);
    assert!(
        body.message.contains("art/ball.png"),
        "要点名缺哪个：{}",
        body.message
    );

    // 没提交成，指针不能动。
    assert!(h.store.get_current(&site.slug).await.unwrap().is_none());
}

#[tokio::test]
async fn committing_twice_is_refused() {
    let h = Harness::start().await;
    let token = h.anon_token().await;
    let site = h.new_site(&token).await;
    let file = entry("index.html", INDEX_HTML);

    let prepared: PrepareUploadResponse = h
        .post(
            &paths::site_uploads(&site.slug),
            Some(&token),
            &prepare_request(vec![file.clone()]),
        )
        .await
        .json();
    h.put_bytes(&paths::blob(&file.hash), &token, INDEX_HTML)
        .await;

    let commit_path = paths::site_upload_commit(&site.slug, &prepared.upload_id);
    h.post(&commit_path, Some(&token), &serde_json::json!({}))
        .await
        .json::<CommitUploadResponse>();
    h.post(&commit_path, Some(&token), &serde_json::json!({}))
        .await
        .error(StatusCode::BAD_REQUEST, ErrorCode::Invalid);

    assert_eq!(
        h.store
            .get_current(&site.slug)
            .await
            .unwrap()
            .unwrap()
            .version,
        1
    );
}

#[tokio::test]
async fn no_token_and_stale_token_say_different_things() {
    let h = Harness::start().await;

    let anonymous = h
        .post(paths::SITES, None, &CreateSiteRequest::default())
        .await
        .error(StatusCode::UNAUTHORIZED, ErrorCode::Unauthorized);
    assert!(
        anonymous.message.contains("playtest"),
        "{}",
        anonymous.message
    );

    let unknown = h
        .post(
            paths::SITES,
            Some("nobody-issued-this"),
            &CreateSiteRequest::default(),
        )
        .await
        .error(StatusCode::UNAUTHORIZED, ErrorCode::Unauthorized);
    assert!(!unknown.message.is_empty());

    let stale = h.stale_token().await;
    let expired = h
        .post(paths::SITES, Some(&stale), &CreateSiteRequest::default())
        .await
        .error(StatusCode::UNAUTHORIZED, ErrorCode::TokenExpired);
    assert!(
        expired.message.contains("24 小时") && expired.message.contains("新链接"),
        "过期的说法要告诉人下一步做什么：{}",
        expired.message
    );
}

#[tokio::test]
async fn anonymous_users_stop_at_three_sites() {
    let h = Harness::start().await;
    let token = h.anon_token().await;

    let mut slugs = Vec::new();
    for _ in 0..3 {
        slugs.push(h.new_site(&token).await.slug);
    }
    slugs.sort();
    slugs.dedup();
    assert_eq!(slugs.len(), 3, "随机名字撞车了");

    let body = h
        .post(paths::SITES, Some(&token), &CreateSiteRequest::default())
        .await
        .error(StatusCode::FORBIDDEN, ErrorCode::QuotaExceeded);
    assert!(body.message.contains('3'), "{}", body.message);

    // 删掉一个就能再建一个。
    h.request(
        "DELETE",
        &paths::site(&slugs[0]),
        Some(&token),
        Body::empty(),
        false,
    )
    .await;
    h.new_site(&token).await;
}

#[tokio::test]
async fn dangerous_paths_never_reach_the_manifest() {
    let h = Harness::start().await;
    let token = h.anon_token().await;
    let site = h.new_site(&token).await;

    for bad in ["../secrets.env", "/etc/passwd", "a//b.js", "art\\ball.png"] {
        let reply = h
            .post(
                &paths::site_uploads(&site.slug),
                Some(&token),
                &prepare_request(vec![entry(bad, INDEX_HTML)]),
            )
            .await;
        let body = reply.error(StatusCode::BAD_REQUEST, ErrorCode::Invalid);
        assert!(
            body.message.contains(bad),
            "报错要指出是哪条路径：{}",
            body.message
        );
    }
}

#[tokio::test]
async fn oversized_version_is_a_quota_error() {
    let h = Harness::start().await;
    let token = h.anon_token().await;
    let site = h.new_site(&token).await;

    let huge = FileEntry {
        path: "Build/game.data".to_string(),
        hash: hash::hash_bytes(b"pretend this is huge"),
        size: playtest_common::limits::ANON_MAX_VERSION_BYTES + 1,
    };
    let body = h
        .post(
            &paths::site_uploads(&site.slug),
            Some(&token),
            &prepare_request(vec![huge]),
        )
        .await
        .error(StatusCode::PAYLOAD_TOO_LARGE, ErrorCode::QuotaExceeded);
    assert!(body.message.contains("上限"), "{}", body.message);
}

#[tokio::test]
async fn other_peoples_sites_are_invisible() {
    let h = Harness::start().await;
    let mine = h.anon_token().await;
    let theirs = h.anon_token().await;
    let site = h.new_site(&mine).await;

    h.get(&paths::site(&site.slug), &theirs)
        .await
        .error(StatusCode::NOT_FOUND, ErrorCode::NotFound);
    h.post(
        &paths::site_uploads(&site.slug),
        Some(&theirs),
        &prepare_request(vec![entry("index.html", INDEX_HTML)]),
    )
    .await
    .error(StatusCode::NOT_FOUND, ErrorCode::NotFound);
}

/// 中间层（方法不对、地址不存在）产生的失败也要是同一种错误体，客户端只写一套解析。
#[tokio::test]
async fn every_failure_is_an_error_body() {
    let h = Harness::start().await;

    h.request("GET", paths::ANON_SESSIONS, None, Body::empty(), false)
        .await
        .error(StatusCode::METHOD_NOT_ALLOWED, ErrorCode::Invalid);

    h.request("GET", "/v1/nope", None, Body::empty(), false)
        .await
        .error(StatusCode::NOT_FOUND, ErrorCode::NotFound);

    let token = h.anon_token().await;
    h.request(
        "POST",
        paths::SITES,
        Some(&token),
        Body::from("{ 这不是 JSON"),
        true,
    )
    .await
    .error(StatusCode::BAD_REQUEST, ErrorCode::Invalid);
}

#[tokio::test]
async fn expired_anonymous_sites_are_swept() {
    let h = Harness::start().await;
    let token = h.anon_token().await;
    let site = h.new_site(&token).await;
    let file = entry("index.html", INDEX_HTML);

    let prepared: PrepareUploadResponse = h
        .post(
            &paths::site_uploads(&site.slug),
            Some(&token),
            &prepare_request(vec![file.clone()]),
        )
        .await
        .json();
    h.put_bytes(&paths::blob(&file.hash), &token, INDEX_HTML)
        .await;
    h.post(
        &paths::site_upload_commit(&site.slug, &prepared.upload_id),
        Some(&token),
        &serde_json::json!({}),
    )
    .await
    .json::<CommitUploadResponse>();

    // 还没到期，扫一遍什么都不该动。
    assert_eq!(
        playtest_api::sweeper::sweep_once(&h.state).await.unwrap(),
        0
    );
    assert!(h.store.get_current(&site.slug).await.unwrap().is_some());

    // 把这个人的到期时间拨到过去，再扫。
    {
        let conn = h.state.db().lock().await;
        conn.execute("UPDATE users SET expires_at = '2020-01-01T00:00:00Z'", [])
            .unwrap();
    }
    assert_eq!(
        playtest_api::sweeper::sweep_once(&h.state).await.unwrap(),
        1
    );
    assert!(
        h.store.get_current(&site.slug).await.unwrap().is_none(),
        "到期之后链接就该打不开了"
    );
    h.get(&paths::site(&site.slug), &token)
        .await
        .error(StatusCode::UNAUTHORIZED, ErrorCode::TokenExpired);
}

/// blob 是跨作品去重的，删作品时不动它；一天一次的回收只删「没有任何清单引用、且落盘超过一天」的。
#[tokio::test]
async fn orphaned_blobs_are_collected_but_never_live_ones() {
    let h = Harness::start().await;
    let token = h.anon_token().await;

    // 作品 A 用 index.html；作品 B 用 index.html + 一个只有它有的文件。
    let shared = entry("index.html", INDEX_HTML);
    let only_b = entry("game.js", b"console.log('b')");
    let a = h.new_site(&token).await;
    let b = h.new_site(&token).await;
    for (site, files) in [
        (&a, vec![shared.clone()]),
        (&b, vec![shared.clone(), only_b.clone()]),
    ] {
        let prepared: PrepareUploadResponse = h
            .post(
                &paths::site_uploads(&site.slug),
                Some(&token),
                &prepare_request(files.clone()),
            )
            .await
            .json();
        for f in &files {
            if prepared.missing.contains(&f.hash) {
                let bytes: &[u8] = if f.path == "index.html" {
                    INDEX_HTML
                } else {
                    b"console.log('b')"
                };
                h.put_bytes(&paths::blob(&f.hash), &token, bytes).await;
            }
        }
        h.post(
            &paths::site_upload_commit(&site.slug, &prepared.upload_id),
            Some(&token),
            &serde_json::json!({}),
        )
        .await
        .json::<CommitUploadResponse>();
    }
    // 还有一个谁都没提交过的 blob（上传了一半的人）。
    let dangling = entry("stray.bin", b"nobody committed me");
    h.put_bytes(&paths::blob(&dangling.hash), &token, b"nobody committed me")
        .await;

    // 把三个 blob 的修改时间都拨到两天前，否则「落盘不到一天」的保护会让什么都不删。
    let two_days_ago = std::time::SystemTime::now() - std::time::Duration::from_secs(2 * 86400);
    for hash in [&shared.hash, &only_b.hash, &dangling.hash] {
        let path = h.store.blob_path(hash).unwrap();
        let f = std::fs::File::options().write(true).open(&path).unwrap();
        f.set_modified(two_days_ago).unwrap();
    }

    // 两个作品都活着：只有那个没人提交的 blob 该被回收。
    let (removed, _) = playtest_api::sweeper::collect_blobs(&h.state)
        .await
        .unwrap();
    assert_eq!(removed, 1);
    assert!(!h.store.has_blob(&dangling.hash).await.unwrap());
    assert!(h.store.has_blob(&shared.hash).await.unwrap());
    assert!(h.store.has_blob(&only_b.hash).await.unwrap());

    // 删掉作品 B：它独有的 game.js 变成孤儿，index.html 仍被 A 引用。
    h.request(
        "DELETE",
        &paths::site(&b.slug),
        Some(&token),
        Body::empty(),
        false,
    )
    .await;
    let (removed, _) = playtest_api::sweeper::collect_blobs(&h.state)
        .await
        .unwrap();
    assert_eq!(removed, 1);
    assert!(!h.store.has_blob(&only_b.hash).await.unwrap());
    assert!(h.store.has_blob(&shared.hash).await.unwrap(), "A 还在用它");

    // 回收过的 blob 再上传时，控制面要重新说「缺」——库里的记录也删了。
    let prepared: PrepareUploadResponse = h
        .post(
            &paths::site_uploads(&a.slug),
            Some(&token),
            &prepare_request(vec![shared.clone(), only_b.clone()]),
        )
        .await
        .json();
    assert_eq!(prepared.missing, vec![only_b.hash.clone()]);
}
