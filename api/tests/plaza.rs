//! 广场（DESIGN §3.8）在控制面这一侧的全部规矩：
//! 谁能上、什么时候重写 `plaza.json`、举报到阈值自动撤下、人数怎么数、封面怎么跟着版本走。
//!
//! 和 `upload_flow.rs` 一样直接喂 `Router`，不起端口。

use axum::body::{Body, Bytes};
use axum::http::{header, Request, StatusCode};
use axum::Router;
use playtest_api::{app, AppState, Config};
use playtest_common::api::{
    routes as paths, AnonSessionResponse, CommitUploadResponse, CreateSiteRequest, ErrorBody,
    ErrorCode, PrepareUploadRequest, PrepareUploadResponse, Site, UpdateSiteRequest,
};
use playtest_common::hash;
use playtest_common::ingest::{routes as ingest_paths, EdgeBatch, EdgeEvent};
use playtest_common::manifest::{Cover, FileEntry, GateMode};
use playtest_common::plaza::{Plaza, REPORTS_TO_HIDE};
use playtest_common::store::FsStore;
use serde::de::DeserializeOwned;
use serde::Serialize;
use tower::ServiceExt;

const INDEX_HTML: &[u8] =
    b"<!doctype html><meta charset=utf-8><title>\xe5\xb0\x8f\xe7\x90\x83</title>";
const COVER_PNG: &[u8] = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR pretend cover";

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

    fn error(&self, status: StatusCode, code: ErrorCode) -> ErrorBody {
        assert_eq!(self.status, status, "状态码不对，响应是 {}", self.text());
        let body: ErrorBody = serde_json::from_slice(&self.body).unwrap();
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
            github: None,
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

    async fn call<T: Serialize>(
        &self,
        method: &str,
        path: &str,
        token: Option<&str>,
        value: Option<&T>,
    ) -> Reply {
        let mut builder = Request::builder().method(method).uri(path);
        if let Some(token) = token {
            builder = builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
        }
        let body = match value {
            Some(v) => {
                builder = builder.header(header::CONTENT_TYPE, "application/json");
                Body::from(serde_json::to_vec(v).unwrap())
            }
            None => Body::empty(),
        };
        self.send(builder.body(body).unwrap()).await
    }

    async fn anon_token(&self) -> String {
        self.call::<()>("POST", paths::ANON_SESSIONS, None, None)
            .await
            .json::<AnonSessionResponse>()
            .token
    }

    async fn new_site(&self, token: &str) -> Site {
        self.call(
            "POST",
            paths::SITES,
            Some(token),
            Some(&CreateSiteRequest::default()),
        )
        .await
        .json()
    }

    async fn get_site(&self, token: &str, slug: &str) -> Site {
        self.call::<()>("GET", &paths::site(slug), Some(token), None)
            .await
            .json()
    }

    async fn patch(&self, token: &str, slug: &str, request: &UpdateSiteRequest) -> Reply {
        self.call("PATCH", &paths::site(slug), Some(token), Some(request))
            .await
    }

    /// 发一版：一个 index.html，可选带一句话介绍和封面。返回提交结果。
    async fn publish(
        &self,
        token: &str,
        slug: &str,
        summary: Option<&str>,
        cover: Option<&Cover>,
    ) -> CommitUploadResponse {
        let request = PrepareUploadRequest {
            files: vec![FileEntry {
                path: "index.html".into(),
                hash: hash::hash_bytes(INDEX_HTML),
                size: INDEX_HTML.len() as u64,
            }],
            title: Some("小球".into()),
            note: None,
            summary: summary.map(str::to_string),
            cover: cover.cloned(),
            gate: GateMode::Once,
            isolated: false,
            spa: false,
            engine: Some("phaser".into()),
        };
        let prepared: PrepareUploadResponse = self
            .call(
                "POST",
                &paths::site_uploads(slug),
                Some(token),
                Some(&request),
            )
            .await
            .json();
        for missing in &prepared.missing {
            let bytes: &[u8] = if *missing == hash::hash_bytes(INDEX_HTML) {
                INDEX_HTML
            } else {
                COVER_PNG
            };
            let reply = self
                .send(
                    Request::builder()
                        .method("PUT")
                        .uri(paths::blob(missing))
                        .header(header::AUTHORIZATION, format!("Bearer {token}"))
                        .body(Body::from(bytes.to_vec()))
                        .unwrap(),
                )
                .await;
            assert!(reply.status.is_success(), "{}", reply.text());
        }
        self.call::<()>(
            "POST",
            &paths::site_upload_commit(slug, &prepared.upload_id),
            Some(token),
            None,
        )
        .await
        .json()
    }

    async fn plaza(&self) -> Plaza {
        self.store
            .get_plaza()
            .await
            .unwrap()
            .expect("控制面该写过 plaza.json 了")
    }

    /// 边缘补送的一批事件，带内容域的 Origin。
    async fn edge_events(&self, events: Vec<EdgeEvent>) -> Reply {
        let body = serde_json::to_vec(&EdgeBatch { events }).unwrap();
        self.send(
            Request::builder()
                .method("POST")
                .uri(ingest_paths::EDGE)
                .header(header::ORIGIN, "http://brisk-otter-41.localhost:8443")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
    }
}

fn cover() -> Cover {
    Cover {
        hash: hash::hash_bytes(COVER_PNG),
        size: COVER_PNG.len() as u64,
        mime: "image/png".into(),
    }
}

fn edge_event(kind: &str, slug: &str, sid: &str, referer: &str) -> EdgeEvent {
    EdgeEvent {
        ts: playtest_api::clock::now_string(),
        kind: kind.into(),
        slug: slug.into(),
        version: 1,
        sid: sid.into(),
        ua: "Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) AppleWebKit/605.1.15 Safari/604.1".into(),
        referer: referer.into(),
        wechat: false,
        reason: None,
        detail: None,
    }
}

#[tokio::test]
async fn nothing_is_public_until_the_developer_says_so() {
    let h = Harness::start().await;
    let token = h.anon_token().await;
    let site = h.new_site(&token).await;
    h.publish(&token, &site.slug, Some("三关五分钟"), None)
        .await;

    // 发了版本、控制面起来时写过一次广场：里面没有它。
    playtest_api::plaza::rebuild(&h.state).await.unwrap();
    assert!(h.plaza().await.items.is_empty());
    let shown = h.get_site(&token, &site.slug).await;
    assert!(!shown.listing.public);
    assert_eq!(shown.listing.summary.as_deref(), Some("三关五分钟"));
}

#[tokio::test]
async fn going_public_rewrites_the_plaza_at_once() {
    let h = Harness::start().await;
    let token = h.anon_token().await;
    let site = h.new_site(&token).await;
    h.publish(&token, &site.slug, Some("三关五分钟"), Some(&cover()))
        .await;

    let updated: Site = h
        .patch(
            &token,
            &site.slug,
            &UpdateSiteRequest {
                public: None,
                seeking: Some(true),
                seek_note: Some("新手引导看得懂吗".into()),
            },
        )
        .await
        .json();
    // 「正在找人测」蕴含「公开」。
    assert!(updated.listing.public);
    assert!(updated.listing.seeking);
    assert!(updated.listing.has_cover);
    assert!(!updated.listing.hidden);

    let plaza = h.plaza().await;
    assert_eq!(plaza.items.len(), 1);
    let item = &plaza.items[0];
    assert_eq!(item.slug, site.slug);
    assert_eq!(item.url, format!("http://{}.localhost:8443", site.slug));
    assert_eq!(item.title, "小球");
    assert_eq!(item.developer, "匿名开发者");
    assert_eq!(item.summary.as_deref(), Some("三关五分钟"));
    assert_eq!(item.engine.as_deref(), Some("phaser"));
    assert!(item.is_game);
    assert_eq!(item.version, 1);
    assert!(item.seeking);
    assert_eq!(item.seek_note.as_deref(), Some("新手引导看得懂吗"));
    assert!(
        item.expires_at.is_some(),
        "匿名作品带到期时间，广场上要显示还剩多久"
    );
    let expected_cover = format!(
        "http://{}.localhost:8443/_playtest/cover?v={}",
        site.slug,
        &hash::hash_bytes(COVER_PNG)[..8]
    );
    assert_eq!(item.cover_url.as_deref(), Some(expected_cover.as_str()));
    assert_eq!(item.players, 0);

    // 再发一版：公开着的作品，广场上的版本号立刻跟上，封面和介绍这版没给就沿用。
    let v2 = h.publish(&token, &site.slug, None, None).await;
    assert_eq!(v2.version, 2);
    let plaza = h.plaza().await;
    assert_eq!(plaza.items[0].version, 2);
    assert_eq!(plaza.items[0].summary.as_deref(), Some("三关五分钟"));
    assert!(plaza.items[0].cover_url.is_some());
    let manifest = h.store.get_manifest(&site.slug, 2).await.unwrap().unwrap();
    assert_eq!(manifest.cover, Some(cover()));
    assert_eq!(manifest.summary.as_deref(), Some("三关五分钟"));

    // 拿下来：广场立刻空了，作品本身还在。
    let unlisted: Site = h
        .patch(
            &token,
            &site.slug,
            &UpdateSiteRequest {
                public: Some(false),
                seeking: None,
                seek_note: None,
            },
        )
        .await
        .json();
    assert!(!unlisted.listing.public);
    assert!(!unlisted.listing.seeking, "收回公开就同时不再求测");
    assert!(h.plaza().await.items.is_empty());
    assert!(h.store.get_current(&site.slug).await.unwrap().is_some());
}

#[tokio::test]
async fn a_work_without_a_version_cannot_go_public() {
    let h = Harness::start().await;
    let token = h.anon_token().await;
    let site = h.new_site(&token).await;
    let body = h
        .patch(
            &token,
            &site.slug,
            &UpdateSiteRequest {
                public: Some(true),
                seeking: None,
                seek_note: None,
            },
        )
        .await
        .error(StatusCode::BAD_REQUEST, ErrorCode::Invalid);
    assert!(body.message.contains("先发一版"), "{}", body.message);

    // 别人的作品动不了。
    let other = h.anon_token().await;
    h.patch(
        &other,
        &site.slug,
        &UpdateSiteRequest {
            public: Some(false),
            seeking: None,
            seek_note: None,
        },
    )
    .await
    .error(StatusCode::NOT_FOUND, ErrorCode::NotFound);

    // 「想让你看什么」有长度上限。
    h.publish(&token, &site.slug, None, None).await;
    let too_long = "字".repeat(playtest_common::limits::MAX_SEEK_NOTE_CHARS + 1);
    h.patch(
        &token,
        &site.slug,
        &UpdateSiteRequest {
            public: Some(true),
            seeking: Some(true),
            seek_note: Some(too_long),
        },
    )
    .await
    .error(StatusCode::BAD_REQUEST, ErrorCode::Invalid);
}

#[tokio::test]
async fn enough_reports_take_it_off_the_plaza_but_not_off_the_air() {
    let h = Harness::start().await;
    let token = h.anon_token().await;
    let site = h.new_site(&token).await;
    h.publish(&token, &site.slug, None, None).await;
    h.patch(
        &token,
        &site.slug,
        &UpdateSiteRequest {
            public: Some(true),
            seeking: None,
            seek_note: None,
        },
    )
    .await
    .json::<Site>();
    assert_eq!(h.plaza().await.items.len(), 1);

    // 同一个人举报三次不算三个人。
    for _ in 0..REPORTS_TO_HIDE {
        h.edge_events(vec![edge_event("report", &site.slug, &"a".repeat(32), "")])
            .await;
    }
    assert_eq!(h.plaza().await.items.len(), 1, "一个会话刷举报不该撤下");

    // 三个不同会话：撤下，立刻生效。
    for i in 0..REPORTS_TO_HIDE {
        let sid = format!("{i:0>32}");
        h.edge_events(vec![edge_event("report", &site.slug, &sid, "")])
            .await;
    }
    assert!(h.plaza().await.items.is_empty());
    let shown = h.get_site(&token, &site.slug).await;
    assert!(shown.listing.public, "开发者的意愿没变");
    assert!(
        shown.listing.hidden,
        "但广场上没有了，控制台要把这件事告诉他"
    );
    // 作品链接照常能开：清单和指针都在。
    assert!(h.store.get_current(&site.slug).await.unwrap().is_some());

    // 人工复核后恢复：之前那批举报不再算数，重算不会把它再撤一次……
    {
        let conn = h.state.db().lock().await;
        playtest_api::db::unhide_from_plaza(&conn, &site.slug, &playtest_api::clock::now_string())
            .unwrap();
    }
    playtest_api::plaza::rebuild(&h.state).await.unwrap();
    assert_eq!(h.plaza().await.items.len(), 1);
    assert!(!h.get_site(&token, &site.slug).await.listing.hidden);

    // ……复核之后又来三个新的举报，才会再撤。
    tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
    for i in 10..(10 + REPORTS_TO_HIDE) {
        let sid = format!("{i:0>32}");
        h.edge_events(vec![edge_event("report", &site.slug, &sid, "")])
            .await;
    }
    assert!(h.plaza().await.items.is_empty());
}

#[tokio::test]
async fn players_are_counted_and_plaza_visitors_are_marked() {
    let h = Harness::start().await;
    let token = h.anon_token().await;
    let site = h.new_site(&token).await;
    h.publish(&token, &site.slug, None, None).await;
    h.patch(
        &token,
        &site.slug,
        &UpdateSiteRequest {
            public: Some(true),
            seeking: None,
            seek_note: None,
        },
    )
    .await
    .json::<Site>();

    // 三个人点了「开始」，其中两个是从广场（根域）点进来的；一个只看了门禁页没点开始。
    let from_plaza = "http://localhost:8443/";
    h.edge_events(vec![
        edge_event("start", &site.slug, &"1".repeat(32), from_plaza),
        edge_event("start", &site.slug, &"2".repeat(32), from_plaza),
        edge_event(
            "start",
            &site.slug,
            &"3".repeat(32),
            "https://discord.com/channels/1/2",
        ),
        edge_event("gate_view", &site.slug, &"4".repeat(32), from_plaza),
    ])
    .await;

    playtest_api::plaza::rebuild(&h.state).await.unwrap();
    assert_eq!(h.plaza().await.items[0].players, 3, "只数点了开始的，去重");

    let kinds: Vec<(String, Option<String>)> = {
        let conn = h.state.db().lock().await;
        let mut stmt = conn
            .prepare("SELECT id, referrer_kind FROM sessions WHERE slug = ?1 ORDER BY id")
            .unwrap();
        stmt.query_map([&site.slug], |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap()
    };
    assert_eq!(kinds.len(), 4);
    assert_eq!(kinds[0].1.as_deref(), Some("plaza"));
    assert_eq!(kinds[1].1.as_deref(), Some("plaza"));
    assert_eq!(kinds[2].1.as_deref(), Some("discord"));
    assert_eq!(kinds[3].1.as_deref(), Some("plaza"));
}

#[tokio::test]
async fn the_cover_is_a_blob_like_any_other_and_is_checked() {
    let h = Harness::start().await;
    let token = h.anon_token().await;
    let site = h.new_site(&token).await;

    // 准备上传：缺的哈希里有封面的。
    let request = PrepareUploadRequest {
        files: vec![FileEntry {
            path: "index.html".into(),
            hash: hash::hash_bytes(INDEX_HTML),
            size: INDEX_HTML.len() as u64,
        }],
        title: Some("小球".into()),
        note: None,
        summary: None,
        cover: Some(cover()),
        gate: GateMode::Once,
        isolated: false,
        spa: false,
        engine: None,
    };
    let prepared: PrepareUploadResponse = h
        .call(
            "POST",
            &paths::site_uploads(&site.slug),
            Some(&token),
            Some(&request),
        )
        .await
        .json();
    assert!(prepared.missing.contains(&hash::hash_bytes(COVER_PNG)));
    assert_eq!(
        prepared.missing_bytes,
        (INDEX_HTML.len() + COVER_PNG.len()) as u64
    );

    // 只传了 index.html 就提交：提交说封面没传上来。
    h.send(
        Request::builder()
            .method("PUT")
            .uri(paths::blob(&hash::hash_bytes(INDEX_HTML)))
            .header(header::AUTHORIZATION, format!("Bearer {token}"))
            .body(Body::from(INDEX_HTML.to_vec()))
            .unwrap(),
    )
    .await;
    let body = h
        .call::<()>(
            "POST",
            &paths::site_upload_commit(&site.slug, &prepared.upload_id),
            Some(&token),
            None,
        )
        .await
        .error(StatusCode::CONFLICT, ErrorCode::BlobsMissing);
    assert!(body.message.contains("（封面）"), "{}", body.message);

    // 不是图片的封面在准备那一步就被拦下。
    let mut bad = request.clone();
    bad.cover = Some(Cover {
        hash: hash::hash_bytes(b"<svg/>"),
        size: 6,
        mime: "image/svg+xml".into(),
    });
    let body = h
        .call(
            "POST",
            &paths::site_uploads(&site.slug),
            Some(&token),
            Some(&bad),
        )
        .await
        .error(StatusCode::BAD_REQUEST, ErrorCode::Invalid);
    assert!(body.message.contains("PNG"), "{}", body.message);

    let mut huge = request.clone();
    huge.cover = Some(Cover {
        size: playtest_common::limits::MAX_COVER_BYTES + 1,
        ..cover()
    });
    h.call(
        "POST",
        &paths::site_uploads(&site.slug),
        Some(&token),
        Some(&huge),
    )
    .await
    .error(StatusCode::BAD_REQUEST, ErrorCode::Invalid);
}
