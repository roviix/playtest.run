//! 隧道令牌这条路：CLI 要一个短期令牌，边缘只用公钥验它（DESIGN §4.3、§4.5）。
//!
//! 和 `upload_flow` 一样把请求直接喂给 `Router`，不起监听端口。验签用的公钥从对象存储里读，
//! 走的就是边缘会走的那一步——这里不 import 任何私钥，测的是「边缘拿得到的东西够不够」。
//! 真机上用 curl 走的那一遍记在 `docs/spikes/2026-09-07-api-tunnel-token.md`。

use axum::body::{Body, Bytes};
use axum::http::{header, Request, StatusCode};
use axum::Router;
use playtest_common::api::{
    routes as paths, AnonSessionResponse, CreateSiteRequest, ErrorBody, ErrorCode, Site,
};
use playtest_common::hash;
use playtest_common::limits;
use playtest_common::manifest::GateMode;
use playtest_common::tunnel::{
    key_files, Claims, SigningKey, TunnelGrant, TunnelRequest, VerifyingKey, TOKEN_TTL_SECS,
    WS_PATH,
};
use playtest_api::{app, tunnel_keys, AppState, Config};
use serde::de::DeserializeOwned;
use serde::Serialize;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use tower::ServiceExt;

const LOCAL_TEMPLATE: &str = "http://{slug}.localhost:8443";
const PROD_TEMPLATE: &str = "https://{slug}.playtest.run";

struct Harness {
    router: Router,
    state: AppState,
    config: Config,
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
        Self::with_template(LOCAL_TEMPLATE).await
    }

    async fn with_template(template: &str) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let config = Config {
            listen: "127.0.0.1:0".parse().unwrap(),
            data_dir: dir.path().to_path_buf(),
            site_url_template: template.to_string(),
        };
        let state = AppState::from_config(&config).await.unwrap();
        Self {
            router: app(state.clone()),
            state,
            config,
            _dir: dir,
        }
    }

    /// 边缘手上只有这个：对象存储里的一行公钥。
    fn edge_key(&self) -> VerifyingKey {
        let path = self.config.store_root().join(key_files::VERIFYING_KEY_OBJECT);
        let raw = std::fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("边缘要从 {} 读公钥，读不到：{err}", path.display()));
        VerifyingKey::from_base64(&raw)
            .unwrap_or_else(|| panic!("公钥文件里应该是一行 base64url，实际是「{raw}」"))
    }

    async fn send(&self, request: Request<Body>) -> Reply {
        let response = self.router.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        Reply { status, body }
    }

    async fn post<T: Serialize>(&self, path: &str, token: Option<&str>, value: &T) -> Reply {
        let mut builder = Request::builder()
            .method("POST")
            .uri(path)
            .header(header::CONTENT_TYPE, "application/json");
        if let Some(token) = token {
            builder = builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
        }
        self.send(builder.body(Body::from(serde_json::to_vec(value).unwrap())).unwrap())
            .await
    }

    async fn anon_token(&self) -> String {
        let reply = self
            .send(
                Request::builder()
                    .method("POST")
                    .uri(paths::ANON_SESSIONS)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await;
        reply.json::<AnonSessionResponse>().token
    }

    async fn new_site(&self, token: &str) -> Site {
        self.post(paths::SITES, Some(token), &CreateSiteRequest::default())
            .await
            .json()
    }

    /// 一个昨天就该失效的匿名令牌。
    async fn stale_token(&self) -> String {
        let token = "expired-token-for-tunnel-test";
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
}

fn now() -> i64 {
    OffsetDateTime::now_utc().unix_timestamp()
}

/// 匿名会话 → 建作品 → 要一个令牌 → 用对象存储里的公钥验开它。
#[tokio::test]
async fn anonymous_developer_gets_a_token_the_edge_can_verify() {
    let h = Harness::start().await;
    let token = h.anon_token().await;
    let site = h.new_site(&token).await;

    let issued_at = now();
    let grant: TunnelGrant = h
        .post(
            &paths::site_tunnel(&site.slug),
            Some(&token),
            &serde_json::json!({}),
        )
        .await
        .json();

    assert_eq!(grant.slug, site.slug);
    assert_eq!(grant.url, site.url);
    assert_eq!(
        grant.connect_url,
        format!("ws://{}.localhost:8443{WS_PATH}", site.slug)
    );
    assert_eq!(grant.site_expires_at, site.expires_at);
    assert!(grant.token.starts_with("pt1."), "{}", grant.token);

    let claims: Claims = h
        .edge_key()
        .verify(&grant.token, now())
        .expect("边缘要能用对象存储里的公钥验开这个令牌");
    assert_eq!(claims.v, 1);
    assert_eq!(claims.slug, site.slug);
    assert!(!claims.sub.is_empty(), "用量和撤销要按用户归");
    assert_eq!(claims.title, site.title, "没给作品名就用建作品时那个");
    assert_eq!(claims.developer, "匿名开发者");
    assert!(claims.badge, "匿名和免费档都带角标");
    assert_eq!(claims.gate, GateMode::Once, "默认门禁策略");
    assert!(!claims.isolated);
    assert_eq!(claims.max_players, 50);
    assert_eq!(claims.exp - claims.iat, TOKEN_TTL_SECS);
    assert!(
        (claims.iat - issued_at).abs() <= 5,
        "签发时间要是现在：{} vs {issued_at}",
        claims.iat
    );
    assert!(!claims.jti.is_empty());

    let expires_at = OffsetDateTime::parse(&grant.expires_at, &Rfc3339)
        .unwrap_or_else(|err| panic!("expires_at 要是 RFC 3339：{err}；原文是 {}", grant.expires_at));
    assert_eq!(expires_at.unix_timestamp(), claims.exp);

    // 令牌是签出来的，不是编出来的：改一个字节就验不过。
    let mut forged = grant.token.clone();
    forged.push('x');
    assert!(h.edge_key().verify(&forged, now()).is_err());

    // 再要一个：撤销名单按 jti 记，两次不能是同一个。
    let again: TunnelGrant = h
        .post(
            &paths::site_tunnel(&site.slug),
            Some(&token),
            &serde_json::json!({}),
        )
        .await
        .json();
    let claims2 = h.edge_key().verify(&again.token, now()).unwrap();
    assert_ne!(claims.jti, claims2.jti);
}

/// 请求里给的作品名、门禁策略和 `--isolated` 要原样进令牌：边缘只看令牌，看不到库。
#[tokio::test]
async fn the_request_decides_title_gate_and_isolation() {
    let h = Harness::start().await;
    let token = h.anon_token().await;
    let site = h.new_site(&token).await;

    let grant: TunnelGrant = h
        .post(
            &paths::site_tunnel(&site.slug),
            Some(&token),
            &TunnelRequest {
                title: Some("  小球  ".to_string()),
                gate: GateMode::Always,
                isolated: true,
            },
        )
        .await
        .json();

    let claims = h.edge_key().verify(&grant.token, now()).unwrap();
    assert_eq!(claims.title, "小球", "前后空白要去掉");
    assert_eq!(claims.gate, GateMode::Always);
    assert!(claims.isolated, "Godot 线程导出物要靠这个开 COOP/COEP");
}

#[tokio::test]
async fn the_production_template_gives_wss() {
    let h = Harness::with_template(PROD_TEMPLATE).await;
    let token = h.anon_token().await;
    let site = h.new_site(&token).await;

    let grant: TunnelGrant = h
        .post(
            &paths::site_tunnel(&site.slug),
            Some(&token),
            &serde_json::json!({}),
        )
        .await
        .json();

    assert_eq!(grant.url, format!("https://{}.playtest.run", site.slug));
    assert_eq!(
        grant.connect_url,
        format!("wss://{}.playtest.run{WS_PATH}", site.slug)
    );
}

#[tokio::test]
async fn only_my_own_site_gets_a_token() {
    let h = Harness::start().await;
    let mine = h.anon_token().await;
    let theirs = h.anon_token().await;
    let site = h.new_site(&mine).await;

    h.post(
        &paths::site_tunnel(&site.slug),
        Some(&theirs),
        &serde_json::json!({}),
    )
    .await
    .error(StatusCode::NOT_FOUND, ErrorCode::NotFound);

    h.post(
        &paths::site_tunnel("no-such-slug-11"),
        Some(&mine),
        &serde_json::json!({}),
    )
    .await
    .error(StatusCode::NOT_FOUND, ErrorCode::NotFound);
}

#[tokio::test]
async fn a_token_is_required_and_expiry_says_so() {
    let h = Harness::start().await;
    let token = h.anon_token().await;
    let site = h.new_site(&token).await;

    h.post(&paths::site_tunnel(&site.slug), None, &serde_json::json!({}))
        .await
        .error(StatusCode::UNAUTHORIZED, ErrorCode::Unauthorized);

    let stale = h.stale_token().await;
    let expired = h
        .post(
            &paths::site_tunnel(&site.slug),
            Some(&stale),
            &serde_json::json!({}),
        )
        .await
        .error(StatusCode::UNAUTHORIZED, ErrorCode::TokenExpired);
    assert!(expired.message.contains("新链接"), "{}", expired.message);
}

#[tokio::test]
async fn an_overlong_title_is_refused() {
    let h = Harness::start().await;
    let token = h.anon_token().await;
    let site = h.new_site(&token).await;

    let body = h
        .post(
            &paths::site_tunnel(&site.slug),
            Some(&token),
            &TunnelRequest {
                title: Some("字".repeat(limits::MAX_TITLE_CHARS + 1)),
                ..TunnelRequest::default()
            },
        )
        .await
        .error(StatusCode::BAD_REQUEST, ErrorCode::Invalid);
    assert!(
        body.message.contains(&limits::MAX_TITLE_CHARS.to_string()),
        "报错要说清上限是多少：{}",
        body.message
    );
}

/// 重启不换钥匙：换了的话所有在线的隧道会一起掉线，而边缘只在启动时读一次公钥。
#[tokio::test]
async fn restarting_keeps_the_same_key() {
    let dir = tempfile::tempdir().unwrap();
    let config = Config {
        listen: "127.0.0.1:0".parse().unwrap(),
        data_dir: dir.path().to_path_buf(),
        site_url_template: LOCAL_TEMPLATE.to_string(),
    };

    AppState::from_config(&config).await.unwrap();
    let published = std::fs::read_to_string(
        config.store_root().join(key_files::VERIFYING_KEY_OBJECT),
    )
    .unwrap();

    AppState::from_config(&config).await.unwrap();
    let after_restart = std::fs::read_to_string(
        config.store_root().join(key_files::VERIFYING_KEY_OBJECT),
    )
    .unwrap();
    assert_eq!(published, after_restart);
    assert!(published.ends_with('\n'), "公钥文件要以换行结尾：{published:?}");

    let key_path = dir.path().join(key_files::SIGNING_KEY_FILE);
    assert!(key_path.is_file(), "私钥要落在数据目录里");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&key_path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "私钥不能是同机其他用户读得到的");
    }
}

/// 环境变量里的私钥优先于文件，且不动文件——撤掉变量就回到原来那把。
#[tokio::test]
async fn the_environment_key_wins_over_the_file() {
    let dir = tempfile::tempdir().unwrap();
    let store_root = dir.path().join("store");

    let from_file = tunnel_keys::load_or_create(dir.path(), &store_root, None).unwrap();

    let injected = SigningKey::generate();
    let used =
        tunnel_keys::load_or_create(dir.path(), &store_root, Some(&injected.to_base64())).unwrap();
    assert_eq!(used.verifying_key(), injected.verifying_key());
    assert_ne!(used.verifying_key(), from_file.verifying_key());

    let published =
        std::fs::read_to_string(store_root.join(key_files::VERIFYING_KEY_OBJECT)).unwrap();
    assert_eq!(
        VerifyingKey::from_base64(&published).unwrap(),
        injected.verifying_key(),
        "发布给边缘的公钥要跟着换"
    );

    let again = tunnel_keys::load_or_create(dir.path(), &store_root, None).unwrap();
    assert_eq!(
        again.verifying_key(),
        from_file.verifying_key(),
        "环境变量不该覆盖密钥文件"
    );
}
