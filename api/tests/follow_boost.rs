//! 关注、通知、推广（DESIGN §3.6、§3.10、§3.11、§4.10）。
//!
//! 关注那一组路由的调用方是**边缘**，不是开发者的 CLI，也不是玩家的浏览器：
//! 玩家把邮箱交给自己所在的那个域，边缘再内网转过来。所以这里既没有开发者令牌，
//! 也没有 Origin 检查——每条断言问的都是「谁都能调的时候会怎么样」。
//!
//! 推广那一组反过来：只有一个运营者，靠一个环境变量里的令牌，没配就整组不存在。
//!
//! GitHub 用一个本机的假服务器顶替（同 `login.rs`）：推广要求作品有主人，
//! 匿名作品的链接 24 小时就没了，推广位上不能放这种东西。

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use axum::body::{Body, Bytes};
use axum::extract::State;
use axum::http::{header, Request, StatusCode};
use axum::routing::{get, post};
use axum::{Json, Router};
use playtest_api::config::GitHubApp;
use playtest_api::{app, boosts, clock, db, live, notify, plaza, AppState, Config};
use playtest_common::api::{
    routes as paths, CommitUploadResponse, CreateSiteRequest, DeviceLoginPoll, DeviceLoginStart,
    ErrorBody, ErrorCode, LoginPollResponse, Me, PrepareUploadRequest, PrepareUploadResponse, Site,
    UpdateSiteRequest,
};
use playtest_common::boost::{
    routes as admin_paths, Boost, BoostKind, BoostStatus, GrantBoostRequest, NotificationQueue,
    ReviewBoostRequest, MAX_SLOTS,
};
use playtest_common::follow::{
    routes as follow_paths, ConfirmRequest, ConfirmResponse, FollowChannel, FollowRequest,
    FollowResponse, FollowTarget, MeRequest, MeView, PushKeys, PushSubscription, SendLinkRequest,
    UnfollowRequest, UnsubscribeRequest,
};
use playtest_common::hash;
use playtest_common::manifest::{FileEntry, GateMode};
use playtest_common::plaza::Plaza;
use serde::de::DeserializeOwned;
use serde::Serialize;
use tower::ServiceExt;

const INDEX_HTML: &[u8] = b"<!doctype html><meta charset=utf-8><canvas id=game></canvas>";
const ADMIN_TOKEN: &str = "admin-token-for-tests";
const AVATAR: &str = "https://avatars.example.com/u/4242";

// ---------------------------------------------------------------- 假 GitHub

async fn fake_device_code() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "device_code": "dev-code-1",
        "user_code": "WDJB-MJHT",
        "verification_uri": "https://github.com/login/device",
        "expires_in": 899,
        "interval": 0
    }))
}

async fn fake_access_token(
    State(polls): State<Arc<AtomicUsize>>,
    Json(_body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    polls.fetch_add(1, Ordering::SeqCst);
    Json(serde_json::json!({ "access_token": "gho_fake", "token_type": "bearer" }))
}

async fn fake_user() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "id": 4242,
        "login": "octo",
        "name": "Octo Cat",
        "avatar_url": AVATAR,
    }))
}

async fn start_fake_github() -> String {
    let router = Router::new()
        .route("/login/device/code", post(fake_device_code))
        .route("/login/oauth/access_token", post(fake_access_token))
        .route("/user", get(fake_user))
        .with_state(Arc::new(AtomicUsize::new(0)));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    format!("http://{addr}")
}

// ---------------------------------------------------------------- 控制面

struct Harness {
    router: Router,
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
    /// `admin` 为假就是这台机器没配管理令牌：`/admin/*` 整组不注册。
    async fn start(admin: bool) -> Self {
        let base = start_fake_github().await;
        let dir = tempfile::tempdir().unwrap();
        let config = Config {
            listen: "127.0.0.1:0".parse().unwrap(),
            data_dir: dir.path().to_path_buf(),
            site_url_template: "http://{slug}.localhost:8443".to_string(),
            github: Some(GitHubApp {
                client_id: "Iv1.fake".to_string(),
                client_secret: Some("secret".to_string()),
                console_url: "http://localhost:5273/".to_string(),
                web_base: base.clone(),
                api_base: base,
            }),
            admin_token: admin.then(|| ADMIN_TOKEN.to_string()),
            ..Config::default()
        };
        let state = AppState::from_config(&config).await.unwrap();
        Self {
            router: app(state.clone()),
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

    /// 边缘替玩家转过来的一条。带上玩家的 IP，限速的两把桶要用它。
    async fn edge_sends<T: Serialize>(&self, path: &str, ip: &str, value: &T) -> Reply {
        self.send(
            Request::builder()
                .method("POST")
                .uri(path)
                .header(header::CONTENT_TYPE, "application/json")
                .header("x-forwarded-for", ip)
                .body(Body::from(serde_json::to_vec(value).unwrap()))
                .unwrap(),
        )
        .await
    }

    /// 登录一个 GitHub 账号，返回开发者令牌。
    async fn login(&self) -> String {
        let start: DeviceLoginStart = self
            .call(
                "POST",
                paths::LOGIN_DEVICE_START,
                None,
                Some(&serde_json::json!({})),
            )
            .await
            .json();
        let polled: LoginPollResponse = self
            .call(
                "POST",
                paths::LOGIN_DEVICE_POLL,
                None,
                Some(&DeviceLoginPoll {
                    device_code: start.device_code,
                }),
            )
            .await
            .json();
        match polled {
            LoginPollResponse::Ok(login) => login.token,
            other => panic!("该登录成功的：{other:?}"),
        }
    }

    /// 一个有主人、发过一版、在广场上的作品。
    async fn site(&self, token: &str) -> String {
        let site: Site = self
            .call(
                "POST",
                paths::SITES,
                Some(token),
                Some(&CreateSiteRequest::default()),
            )
            .await
            .json();
        self.publish(token, &site.slug, None).await;
        self.call(
            "PATCH",
            &paths::site(&site.slug),
            Some(token),
            Some(&UpdateSiteRequest {
                public: Some(true),
                ..Default::default()
            }),
        )
        .await
        .json::<Site>();
        site.slug
    }

    async fn publish(&self, token: &str, slug: &str, note: Option<&str>) -> u32 {
        let file = FileEntry {
            path: "index.html".to_string(),
            hash: hash::hash_bytes(INDEX_HTML),
            size: INDEX_HTML.len() as u64,
        };
        let prepared: PrepareUploadResponse = self
            .call(
                "POST",
                &paths::site_uploads(slug),
                Some(token),
                Some(&PrepareUploadRequest {
                    files: vec![file.clone()],
                    title: Some("小球".to_string()),
                    note: note.map(str::to_string),
                    summary: Some("一个滚来滚去的小球".to_string()),
                    cover: None,
                    gate: GateMode::Once,
                    isolated: false,
                    spa: false,
                    engine: None,
                }),
            )
            .await
            .json();
        for missing in &prepared.missing {
            let put = Request::builder()
                .method("PUT")
                .uri(paths::blob(missing))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::from(INDEX_HTML))
                .unwrap();
            assert!(self.send(put).await.status.is_success());
        }
        let committed: CommitUploadResponse = self
            .call(
                "POST",
                &paths::site_upload_commit(slug, &prepared.upload_id),
                Some(token),
                Some(&serde_json::json!({})),
            )
            .await
            .json();
        committed.version
    }

    async fn admin<T: Serialize>(&self, method: &str, path: &str, value: Option<&T>) -> Reply {
        self.call(method, path, Some(ADMIN_TOKEN), value).await
    }

    /// 队列里最近那一封（按 id 倒序）。
    async fn last_notice(&self) -> Option<(String, String, String, String)> {
        let conn = self.state.db().lock().await;
        conn.query_row(
            "SELECT kind, subject, body, status FROM notifications ORDER BY id DESC LIMIT 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .ok()
    }

    /// 最近那一封信要把人送到哪。正文是开发者写的话，链接单独一列——
    /// 邮件由 `notify::render` 把两者拼起来，推送则把链接放进负载，所以来源要在这一列上。
    async fn last_notice_url(&self) -> Option<String> {
        let conn = self.state.db().lock().await;
        conn.query_row(
            "SELECT url FROM notifications ORDER BY id DESC LIMIT 1",
            [],
            |row| row.get(0),
        )
        .ok()
        .flatten()
    }

    async fn queued(&self, kind: &str) -> i64 {
        let conn = self.state.db().lock().await;
        conn.query_row(
            "SELECT COUNT(*) FROM notifications WHERE kind = ?1",
            [kind],
            |row| row.get(0),
        )
        .unwrap()
    }

    /// 确认信里那个令牌。信里只有链接，所以从链接尾巴上取。
    async fn confirm_token(&self) -> String {
        let (_, _, body, _) = self.last_notice().await.expect("该有一封确认信");
        body.split_whitespace()
            .find(|w| w.contains("/me/confirm/"))
            .and_then(|w| w.rsplit('/').next())
            .expect("确认信里该有链接")
            .to_string()
    }

    async fn follow_email(&self, target: FollowTarget, email: &str, ip: &str) -> Reply {
        self.edge_sends(
            follow_paths::FOLLOW,
            ip,
            &FollowRequest {
                target,
                channel: FollowChannel::Email {
                    email: email.to_string(),
                },
                from: Some("gate".to_string()),
            },
        )
        .await
    }

    /// 走完「留邮箱 → 收信 → 点确认」，返回 me_token。
    async fn confirmed(&self, target: FollowTarget, email: &str, ip: &str) -> String {
        let sent: FollowResponse = self.follow_email(target, email, ip).await.json();
        assert_eq!(sent, FollowResponse::ConfirmSent);
        let token = self.confirm_token().await;
        let confirmed: ConfirmResponse = self
            .edge_sends(follow_paths::CONFIRM, ip, &ConfirmRequest { token })
            .await
            .json();
        confirmed.me_token
    }

    async fn plaza(&self) -> Plaza {
        self.state
            .store()
            .get_plaza()
            .await
            .unwrap()
            .expect("控制面该写过 plaza.json 了")
    }
}

fn push_subscription(endpoint: &str) -> PushSubscription {
    PushSubscription {
        endpoint: endpoint.to_string(),
        keys: PushKeys {
            // 形状对就行：这两个值只在真发推送时才被解读。
            p256dh: "BCVxsr7N_eNgVRqvHtD0zTZsEc6-VV-JvLexhqUzORcx7aMRIGtUwSDIPMlPPFRHfMbEIhcJ_KGXlJgN9dRJ0Fc"
                .to_string(),
            auth: "BTBZMqHH6r4Tts7J_aSIgg".to_string(),
        },
    }
}

// ---------------------------------------------------------------- 关注

#[tokio::test]
async fn an_email_only_counts_after_the_link_is_clicked() {
    let h = Harness::start(false).await;
    let token = h.login().await;
    let slug = h.site(&token).await;

    let sent: FollowResponse = h
        .follow_email(
            FollowTarget::Site { slug: slug.clone() },
            " Someone@Example.COM ",
            "203.0.113.7",
        )
        .await
        .json();
    assert_eq!(sent, FollowResponse::ConfirmSent);

    // 还没点：一个关注都没有，开发者看到的还是 0。
    {
        let conn = h.state.db().lock().await;
        assert_eq!(db::followers_count(&conn, &slug).unwrap(), 0);
    }
    let (kind, subject, body, status) = h.last_notice().await.expect("该有一封确认信");
    assert_eq!(kind, notify::KIND_CONFIRM);
    assert!(
        subject.contains("确认关注") && subject.contains("小球"),
        "{subject}"
    );
    assert!(body.contains("忽略这封信"), "{body}");
    assert_eq!(status, "pending");

    // 点了才算。
    let confirm_token = h.confirm_token().await;
    let confirmed: ConfirmResponse = h
        .edge_sends(
            follow_paths::CONFIRM,
            "203.0.113.7",
            &ConfirmRequest {
                token: confirm_token.clone(),
            },
        )
        .await
        .json();
    assert!(!confirmed.me_token.is_empty());
    assert_eq!(
        confirmed.me.email_masked.as_deref(),
        Some("s***@example.com"),
        "邮箱只以打码的样子回去"
    );
    assert_eq!(confirmed.me.follows.len(), 1);
    assert_eq!(
        confirmed.me.follows[0].target,
        FollowTarget::Site { slug: slug.clone() }
    );
    assert_eq!(confirmed.me.follows[0].title.as_deref(), Some("小球"));

    {
        let conn = h.state.db().lock().await;
        assert_eq!(db::followers_count(&conn, &slug).unwrap(), 1);
    }
    assert_eq!(
        h.state
            .store()
            .get_live(&slug)
            .await
            .unwrap()
            .unwrap()
            .followers,
        1,
        "门禁页上的关注数当场就该变"
    );

    // 同一个令牌用第二次不行。
    h.edge_sends(
        follow_paths::CONFIRM,
        "203.0.113.7",
        &ConfirmRequest {
            token: confirm_token,
        },
    )
    .await
    .error(StatusCode::NOT_FOUND, ErrorCode::NotFound);
}

#[tokio::test]
async fn the_me_page_lists_unfollows_and_unsubscribes() {
    let h = Harness::start(false).await;
    let token = h.login().await;
    let slug = h.site(&token).await;
    let me_token = h
        .confirmed(
            FollowTarget::Site { slug: slug.clone() },
            "someone@example.com",
            "203.0.113.7",
        )
        .await;

    // 有了 me_token 之后，再关注别的东西是一下点击。
    let added: FollowResponse = h
        .edge_sends(
            follow_paths::FOLLOW,
            "203.0.113.7",
            &FollowRequest {
                target: FollowTarget::Plaza,
                channel: FollowChannel::Me {
                    me_token: me_token.clone(),
                },
                from: Some("plaza".to_string()),
            },
        )
        .await
        .json();
    assert_eq!(added, FollowResponse::Subscribed);
    let again: FollowResponse = h
        .edge_sends(
            follow_paths::FOLLOW,
            "203.0.113.7",
            &FollowRequest {
                target: FollowTarget::Plaza,
                channel: FollowChannel::Me {
                    me_token: me_token.clone(),
                },
                from: None,
            },
        )
        .await
        .json();
    assert_eq!(again, FollowResponse::AlreadyFollowing);

    let view: MeView = h
        .edge_sends(
            follow_paths::ME_VIEW,
            "203.0.113.7",
            &MeRequest {
                me_token: me_token.clone(),
            },
        )
        .await
        .json();
    assert_eq!(view.follows.len(), 2);
    assert!(!view.push, "这个人没开浏览器通知");

    // 取消一项。
    let after: MeView = h
        .edge_sends(
            follow_paths::ME_UNFOLLOW,
            "203.0.113.7",
            &UnfollowRequest {
                me_token: me_token.clone(),
                target: FollowTarget::Site { slug: slug.clone() },
            },
        )
        .await
        .json();
    assert_eq!(after.follows.len(), 1);
    assert_eq!(after.follows[0].target, FollowTarget::Plaza);
    assert_eq!(
        h.state
            .store()
            .get_live(&slug)
            .await
            .unwrap()
            .unwrap()
            .followers,
        0
    );

    // 退订：全清，而且那把 me_token 也跟着失效。
    let unsubscribe_token = {
        let conn = h.state.db().lock().await;
        conn.query_row("SELECT unsubscribe_token FROM players", [], |row| {
            row.get::<_, String>(0)
        })
        .unwrap()
    };
    let empty: MeView = h
        .edge_sends(
            follow_paths::UNSUBSCRIBE,
            "203.0.113.7",
            &UnsubscribeRequest {
                token: unsubscribe_token.clone(),
            },
        )
        .await
        .json();
    assert!(empty.follows.is_empty());
    h.edge_sends(
        follow_paths::ME_VIEW,
        "203.0.113.7",
        &MeRequest { me_token },
    )
    .await
    .error(StatusCode::UNAUTHORIZED, ErrorCode::Unauthorized);

    // 点第二次也不报错——多半就是点了两下。
    h.edge_sends(
        follow_paths::UNSUBSCRIBE,
        "203.0.113.7",
        &UnsubscribeRequest {
            token: unsubscribe_token,
        },
    )
    .await
    .json::<MeView>();
}

#[tokio::test]
async fn a_browser_subscription_takes_effect_at_once() {
    let h = Harness::start(false).await;
    let token = h.login().await;
    let slug = h.site(&token).await;

    let subscribed: FollowResponse = h
        .edge_sends(
            follow_paths::FOLLOW,
            "203.0.113.8",
            &FollowRequest {
                target: FollowTarget::Site { slug: slug.clone() },
                channel: FollowChannel::Push {
                    subscription: push_subscription("https://push.example.com/abc"),
                },
                from: Some("gate".to_string()),
            },
        )
        .await
        .json();
    assert_eq!(
        subscribed,
        FollowResponse::Subscribed,
        "浏览器已经问过本人了，不用再发一封确认信"
    );
    assert_eq!(h.queued(notify::KIND_CONFIRM).await, 0);
    assert_eq!(
        h.state
            .store()
            .get_live(&slug)
            .await
            .unwrap()
            .unwrap()
            .followers,
        1
    );

    let again: FollowResponse = h
        .edge_sends(
            follow_paths::FOLLOW,
            "203.0.113.8",
            &FollowRequest {
                target: FollowTarget::Site { slug },
                channel: FollowChannel::Push {
                    subscription: push_subscription("https://push.example.com/abc"),
                },
                from: None,
            },
        )
        .await
        .json();
    assert_eq!(again, FollowResponse::AlreadyFollowing);
}

#[tokio::test]
async fn turning_browser_notifications_off_keeps_the_follows() {
    let h = Harness::start(false).await;
    let token = h.login().await;
    let slug = h.site(&token).await;
    let me_token = h
        .confirmed(
            FollowTarget::Site { slug: slug.clone() },
            "someone@example.com",
            "203.0.113.7",
        )
        .await;
    // 这个人后来在「我的」里也开了浏览器通知。
    {
        let conn = h.state.db().lock().await;
        conn.execute(
            "UPDATE players SET push_subscription = '{}', push_endpoint = 'https://push.example.com/x'",
            [],
        )
        .unwrap();
    }
    let before: MeView = h
        .edge_sends(
            follow_paths::ME_VIEW,
            "203.0.113.7",
            &MeRequest {
                me_token: me_token.clone(),
            },
        )
        .await
        .json();
    assert!(before.push);

    let after: MeView = h
        .edge_sends(
            follow_paths::ME_PUSH_OFF,
            "203.0.113.7",
            &MeRequest {
                me_token: me_token.clone(),
            },
        )
        .await
        .json();
    assert!(!after.push, "推送订阅清掉了");
    assert_eq!(after.follows.len(), 1, "关注还在，通知改走邮箱");
    assert_eq!(after.email_masked.as_deref(), Some("s***@example.com"));

    // 伪造的钥匙：401，边缘据此清 cookie。
    h.edge_sends(
        follow_paths::ME_PUSH_OFF,
        "203.0.113.7",
        &MeRequest {
            me_token: "nope".to_string(),
        },
    )
    .await
    .error(StatusCode::UNAUTHORIZED, ErrorCode::Unauthorized);
}

#[tokio::test]
async fn an_already_confirmed_person_still_gets_a_letter_when_they_type_an_email() {
    let h = Harness::start(false).await;
    let token = h.login().await;
    let first = h.site(&token).await;
    let second = h.site(&token).await;
    h.confirmed(
        FollowTarget::Site {
            slug: first.clone(),
        },
        "someone@example.com",
        "203.0.113.7",
    )
    .await;

    // 谁都能替别人填一个邮箱，所以这一下也要确认——否则这就是一台「替人加关注」的机器。
    let sent: FollowResponse = h
        .follow_email(
            FollowTarget::Site {
                slug: second.clone(),
            },
            "someone@example.com",
            "198.51.100.3",
        )
        .await
        .json();
    assert_eq!(sent, FollowResponse::ConfirmSent);
    {
        let conn = h.state.db().lock().await;
        assert_eq!(db::followers_count(&conn, &second).unwrap(), 0);
    }
}

#[tokio::test]
async fn send_link_never_says_whether_the_email_is_known() {
    let h = Harness::start(false).await;
    let token = h.login().await;
    let slug = h.site(&token).await;
    h.confirmed(
        FollowTarget::Site { slug },
        "someone@example.com",
        "203.0.113.7",
    )
    .await;

    let known: FollowResponse = h
        .edge_sends(
            follow_paths::ME_SEND_LINK,
            "198.51.100.4",
            &SendLinkRequest {
                email: "someone@example.com".to_string(),
            },
        )
        .await
        .json();
    let unknown: FollowResponse = h
        .edge_sends(
            follow_paths::ME_SEND_LINK,
            "198.51.100.5",
            &SendLinkRequest {
                email: "nobody@example.com".to_string(),
            },
        )
        .await
        .json();
    assert_eq!(known, unknown, "回答一模一样，问不出这个邮箱在不在库里");
    assert_eq!(
        h.queued(notify::KIND_SEND_LINK).await,
        1,
        "只有认识的那个真发了一封"
    );

    // 写歪的邮箱当场就说。
    let body = h
        .edge_sends(
            follow_paths::ME_SEND_LINK,
            "198.51.100.6",
            &SendLinkRequest {
                email: "someone".to_string(),
            },
        )
        .await
        .error(StatusCode::BAD_REQUEST, ErrorCode::Invalid);
    assert!(body.message.contains("邮箱"), "{}", body.message);
}

#[tokio::test]
async fn hammering_one_mailbox_runs_out_of_tokens() {
    let h = Harness::start(false).await;
    let token = h.login().await;
    let slug = h.site(&token).await;

    // 被轰的是别人的信箱，所以邮箱那把桶比 IP 那把紧。换 IP 也躲不掉。
    let mut refused = 0;
    for i in 0..80 {
        let reply = h
            .follow_email(
                FollowTarget::Site { slug: slug.clone() },
                "victim@example.com",
                &format!("198.51.100.{}", i % 200),
            )
            .await;
        if reply.status == StatusCode::TOO_MANY_REQUESTS {
            refused += 1;
        }
    }
    assert!(refused > 0, "同一个邮箱被反复轰要挡下来");
}

#[tokio::test]
async fn following_something_that_is_not_there_is_a_404() {
    let h = Harness::start(false).await;
    h.follow_email(
        FollowTarget::Site {
            slug: "no-such-site-11".to_string(),
        },
        "someone@example.com",
        "203.0.113.7",
    )
    .await
    .error(StatusCode::NOT_FOUND, ErrorCode::NotFound);

    // 边缘据 401 把 pt_me 那个 cookie 清掉。
    h.edge_sends(
        follow_paths::ME_VIEW,
        "203.0.113.7",
        &MeRequest {
            me_token: "not-a-real-token".to_string(),
        },
    )
    .await
    .error(StatusCode::UNAUTHORIZED, ErrorCode::Unauthorized);
}

// ---------------------------------------------------------------- 通知

#[tokio::test]
async fn a_new_version_tells_the_followers_once_a_day() {
    let h = Harness::start(false).await;
    let token = h.login().await;
    let slug = h.site(&token).await;
    h.confirmed(
        FollowTarget::Site { slug: slug.clone() },
        "someone@example.com",
        "203.0.113.7",
    )
    .await;

    h.publish(&token, &slug, Some("改了第三关的跳跃判定")).await;
    assert_eq!(h.queued(notify::KIND_SITE_VERSION).await, 1);
    let (_, subject, body, _) = h.last_notice().await.unwrap();
    assert!(
        subject.contains("小球") && subject.contains("v2"),
        "{subject}"
    );
    assert!(body.contains("改了第三关的跳跃判定"), "{body}");
    let url = h.last_notice_url().await.expect("信里要有去处");
    assert!(url.contains("from=notice"), "链接要带来源：{url}");

    // 一天连发几版，收件箱里还是一封——把它改成最新那一版。
    h.publish(&token, &slug, Some("又改了一点")).await;
    h.publish(&token, &slug, None).await;
    assert_eq!(
        h.queued(notify::KIND_SITE_VERSION).await,
        1,
        "24 小时内合并成一封"
    );
    let (_, subject, body, _) = h.last_notice().await.unwrap();
    assert!(subject.contains("v4"), "合并后是最新那一版：{subject}");
    assert!(
        body.contains("开发者发了新版本"),
        "没写 note 就用这句：{body}"
    );
}

#[tokio::test]
async fn the_log_mailer_sends_and_the_letter_carries_a_way_out() {
    let h = Harness::start(false).await;
    let token = h.login().await;
    let slug = h.site(&token).await;
    h.confirmed(
        FollowTarget::Site { slug },
        "someone@example.com",
        "203.0.113.7",
    )
    .await;

    // 默认那档 mailer 把整封信打进日志，队列这一头照常走完。
    let sent = notify::worker::run_once(&h.state).await;
    assert_eq!(sent, 1);
    let (_, _, _, status) = h.last_notice().await.unwrap();
    assert_eq!(status, "sent");

    // 每封信底下都要有退订的那一行（DESIGN §4.8）。
    let row = {
        let conn = h.state.db().lock().await;
        db::due_notifications(&conn, "9999-01-01T00:00:00Z", 10).unwrap()
    };
    assert!(row.is_empty(), "发完了队列里就该空了");
}

#[tokio::test]
async fn every_letter_carries_the_unsubscribe_line() {
    let h = Harness::start(false).await;
    let token = h.login().await;
    let slug = h.site(&token).await;
    h.confirmed(
        FollowTarget::Site { slug: slug.clone() },
        "someone@example.com",
        "203.0.113.7",
    )
    .await;
    h.publish(&token, &slug, Some("新版本")).await;

    let rows = {
        let conn = h.state.db().lock().await;
        db::due_notifications(&conn, &clock::now_string(), 10).unwrap()
    };
    let letter = notify::render(h.state.notify(), &rows[0]);
    assert!(letter.contains("一键退订"), "{letter}");
    assert!(letter.contains("/me/unsubscribe/"), "{letter}");
}

#[tokio::test]
async fn the_weekly_digest_only_goes_out_when_there_is_something_new() {
    let h = Harness::start(false).await;
    let token = h.login().await;
    let slug = h.site(&token).await;
    h.confirmed(FollowTarget::Plaza, "someone@example.com", "203.0.113.7")
        .await;

    // 时间是参数不是「现在」：这一期什么时候发得能在测试里指定，否则只有周一才跑得过。
    let now = clock::now();
    let queued = {
        let conn = h.state.db().lock().await;
        notify::digest::enqueue(&conn, now, |slug| format!("http://{slug}.localhost:8443")).unwrap()
    };
    assert_eq!(queued, 1);
    let (kind, subject, body, _) = h.last_notice().await.unwrap();
    assert_eq!(kind, notify::KIND_DIGEST);
    assert!(subject.contains(&clock::format(now)[..10]), "{subject}");
    assert!(body.contains("小球") && body.contains("Octo Cat"), "{body}");
    assert!(body.contains("一个滚来滚去的小球"), "{body}");
    assert!(body.contains("from=notice"), "{body}");

    // 周报底下还多一行「管理关注」。
    let rows = {
        let conn = h.state.db().lock().await;
        db::due_notifications(&conn, &clock::format(now), 10).unwrap()
    };
    let letter = notify::render(h.state.notify(), rows.last().unwrap());
    assert!(letter.contains("管理关注"), "{letter}");

    // 过几周之后那个作品不再算「新」，这一期就不发。
    let later = now + time::Duration::days(30);
    let none = {
        let conn = h.state.db().lock().await;
        notify::digest::compose(&conn, later, |slug| format!("http://{slug}.localhost:8443"))
            .unwrap()
    };
    assert!(
        none.is_none(),
        "没有新作品那周不发——「这周没什么」也是一封信，收多了人就退订了"
    );
    let _ = slug;
}

// ---------------------------------------------------------------- 推广

#[tokio::test]
async fn without_a_token_the_admin_routes_do_not_exist() {
    let h = Harness::start(false).await;
    // 探测不出这台机器有没有管理接口：看到的是 404 而不是 401。
    h.call::<()>("GET", admin_paths::BOOSTS, None, None)
        .await
        .error(StatusCode::NOT_FOUND, ErrorCode::NotFound);
    h.call::<()>("GET", admin_paths::BOOSTS, Some(ADMIN_TOKEN), None)
        .await
        .error(StatusCode::NOT_FOUND, ErrorCode::NotFound);
}

#[tokio::test]
async fn a_wrong_admin_token_is_401() {
    let h = Harness::start(true).await;
    h.call::<()>("GET", admin_paths::BOOSTS, Some("nope"), None)
        .await
        .error(StatusCode::UNAUTHORIZED, ErrorCode::Unauthorized);
    h.call::<()>("GET", admin_paths::BOOSTS, None, None)
        .await
        .error(StatusCode::UNAUTHORIZED, ErrorCode::Unauthorized);
    h.admin::<()>("GET", admin_paths::BOOSTS, None)
        .await
        .json::<Vec<Boost>>();
}

#[tokio::test]
async fn a_granted_boost_goes_live_and_lands_on_top_of_the_plaza() {
    let h = Harness::start(true).await;
    let token = h.login().await;
    let plain = h.site(&token).await;
    let promoted = h.site(&token).await;

    let boost: Boost = h
        .admin(
            "POST",
            admin_paths::BOOSTS,
            Some(&GrantBoostRequest {
                slug: promoted.clone(),
                kind: BoostKind::Days3,
                starts_at: None,
                review: false,
            }),
        )
        .await
        .json();
    assert_eq!(boost.status, BoostStatus::Live, "赠送的默认直接上");
    assert!(boost.granted);
    assert!(boost.ends_at.is_some());

    // 广场上它在最前面，而且明说是推广。
    let plaza = h.plaza().await;
    assert_eq!(plaza.items.len(), 2);
    assert_eq!(plaza.items[0].slug, promoted);
    assert!(plaza.items[0].boosted);
    assert_eq!(plaza.items[1].slug, plain);
    assert!(!plaza.items[1].boosted);
    assert_eq!(
        plaza.items[0].avatar_url.as_deref(),
        Some(AVATAR),
        "卡片上要有开发者头像"
    );

    // 开发者自己也看得到这一段（控制台「推广」那一节）。
    let site: Site = h
        .call::<()>("GET", &paths::site(&promoted), Some(&token), None)
        .await
        .json();
    assert_eq!(
        site.listing.boost.map(|b| b.status),
        Some(BoostStatus::Live)
    );

    // 提前结束：推广位上就没有它了。
    let ended = h
        .admin::<()>("DELETE", &admin_paths::boost(boost.id), None)
        .await;
    assert_eq!(ended.status, StatusCode::NO_CONTENT);
    assert!(h.plaza().await.items.iter().all(|i| !i.boosted));
    let all: Vec<Boost> = h.admin::<()>("GET", admin_paths::BOOSTS, None).await.json();
    assert_eq!(all[0].status, BoostStatus::Ended);
}

#[tokio::test]
async fn an_anonymous_site_cannot_be_promoted() {
    let h = Harness::start(true).await;
    let anon: playtest_common::api::AnonSessionResponse = h
        .call::<()>("POST", paths::ANON_SESSIONS, None, None)
        .await
        .json();
    let slug = h.site(&anon.token).await;

    let body = h
        .admin(
            "POST",
            admin_paths::BOOSTS,
            Some(&GrantBoostRequest {
                slug,
                kind: BoostKind::Days3,
                starts_at: None,
                review: false,
            }),
        )
        .await
        .error(StatusCode::BAD_REQUEST, ErrorCode::Invalid);
    assert!(body.message.contains("匿名"), "{}", body.message);
}

#[tokio::test]
async fn the_third_boost_queues_behind_the_first_two() {
    let h = Harness::start(true).await;
    let token = h.login().await;
    let mut slugs = Vec::new();
    for _ in 0..MAX_SLOTS + 1 {
        slugs.push(h.site(&token).await);
    }

    let mut granted = Vec::new();
    for slug in &slugs {
        granted.push(
            h.admin(
                "POST",
                admin_paths::BOOSTS,
                Some(&GrantBoostRequest {
                    slug: slug.clone(),
                    kind: BoostKind::Days3,
                    starts_at: None,
                    review: false,
                }),
            )
            .await
            .json::<Boost>(),
        );
    }

    let live: Vec<&Boost> = granted
        .iter()
        .filter(|b| b.status == BoostStatus::Live)
        .collect();
    assert_eq!(live.len(), MAX_SLOTS, "同一时刻只有这么多位子");
    let queued = granted.last().unwrap();
    assert_eq!(queued.status, BoostStatus::Pending);
    assert!(
        clock::parse(&queued.starts_at).unwrap() > clock::now(),
        "排队的那一个要排到别人下来之后"
    );

    // 广场上只有在位的那几个带「推广」。
    let plaza = h.plaza().await;
    assert_eq!(plaza.items.iter().filter(|i| i.boosted).count(), MAX_SLOTS);
}

#[tokio::test]
async fn a_boost_that_needs_review_waits_and_can_be_turned_down() {
    let h = Harness::start(true).await;
    let token = h.login().await;
    let slug = h.site(&token).await;

    let boost: Boost = h
        .admin(
            "POST",
            admin_paths::BOOSTS,
            Some(&GrantBoostRequest {
                slug: slug.clone(),
                kind: BoostKind::Days7,
                starts_at: None,
                review: true,
            }),
        )
        .await
        .json();
    assert_eq!(boost.status, BoostStatus::Pending);
    assert!(!h.plaza().await.items[0].boosted, "没放行就还不在推广位上");

    let rejected: Boost = h
        .admin(
            "POST",
            &admin_paths::boost_review(boost.id),
            Some(&ReviewBoostRequest {
                approve: false,
                reason: Some("素材里有别人的美术".to_string()),
            }),
        )
        .await
        .json();
    assert_eq!(rejected.status, BoostStatus::Rejected);

    // 换一个，放行。
    let second: Boost = h
        .admin(
            "POST",
            admin_paths::BOOSTS,
            Some(&GrantBoostRequest {
                slug: slug.clone(),
                kind: BoostKind::Days3,
                starts_at: None,
                review: true,
            }),
        )
        .await
        .json();
    let approved: Boost = h
        .admin(
            "POST",
            &admin_paths::boost_review(second.id),
            Some(&ReviewBoostRequest {
                approve: true,
                reason: None,
            }),
        )
        .await
        .json();
    assert_eq!(approved.status, BoostStatus::Live);
    assert!(h.plaza().await.items[0].boosted);
}

#[tokio::test]
async fn a_boost_ends_on_its_own_when_the_window_passes() {
    let h = Harness::start(true).await;
    let token = h.login().await;
    let slug = h.site(&token).await;
    let boost: Boost = h
        .admin(
            "POST",
            admin_paths::BOOSTS,
            Some(&GrantBoostRequest {
                slug,
                kind: BoostKind::Days3,
                starts_at: None,
                review: false,
            }),
        )
        .await
        .json();

    // 时间推到窗口之后，扫地的那一趟把它下掉。
    let after = clock::parse(boost.ends_at.as_deref().unwrap()).unwrap() + time::Duration::hours(1);
    let changed = {
        let conn = h.state.db().lock().await;
        boosts::advance(&conn, &clock::format(after)).unwrap()
    };
    assert!(changed);
    plaza::publish(&h.state).await;
    assert!(!h.plaza().await.items[0].boosted);
}

#[tokio::test]
async fn taking_a_site_off_the_plaza_by_hand_also_ends_its_boost() {
    let h = Harness::start(true).await;
    let token = h.login().await;
    let slug = h.site(&token).await;
    h.admin(
        "POST",
        admin_paths::BOOSTS,
        Some(&GrantBoostRequest {
            slug: slug.clone(),
            kind: BoostKind::Days3,
            starts_at: None,
            review: false,
        }),
    )
    .await
    .json::<Boost>();

    h.admin::<()>("POST", &admin_paths::plaza_hide(&slug), None)
        .await;
    assert!(h.plaza().await.items.is_empty(), "撤下之后广场上就没有了");
    let all: Vec<Boost> = h.admin::<()>("GET", admin_paths::BOOSTS, None).await.json();
    assert_eq!(all[0].status, BoostStatus::Ended, "撤下了就不该还在卖");

    // 复核之后放回去。
    h.admin::<()>("DELETE", &admin_paths::plaza_hide(&slug), None)
        .await;
    assert_eq!(h.plaza().await.items.len(), 1);
    assert!(!h.plaza().await.items[0].boosted, "推广不会跟着恢复");
}

#[tokio::test]
async fn the_queue_view_says_how_much_is_waiting() {
    let h = Harness::start(true).await;
    let token = h.login().await;
    let slug = h.site(&token).await;
    h.follow_email(
        FollowTarget::Site { slug },
        "someone@example.com",
        "203.0.113.7",
    )
    .await
    .json::<FollowResponse>();

    let queue: NotificationQueue = h
        .admin::<()>("GET", admin_paths::NOTIFICATIONS, None)
        .await
        .json();
    assert_eq!(queue.pending, 1);
    assert_eq!(queue.sent_24h, 0);
    assert!(
        queue.next_digest_at.is_some(),
        "下一期周报什么时候发要看得到"
    );

    notify::worker::run_once(&h.state).await;
    let queue: NotificationQueue = h
        .admin::<()>("GET", admin_paths::NOTIFICATIONS, None)
        .await
        .json();
    assert_eq!(queue.pending, 0);
    assert_eq!(queue.sent_24h, 1);
}

// ---------------------------------------------------------------- 头像与 live.json

#[tokio::test]
async fn the_avatar_follows_the_login_all_the_way_to_the_gate() {
    let h = Harness::start(false).await;
    let token = h.login().await;
    let slug = h.site(&token).await;

    let me: Me = h
        .call::<()>("GET", paths::ME, Some(&token), None)
        .await
        .json();
    assert_eq!(me.avatar_url.as_deref(), Some(AVATAR));

    live::publish(&h.state, &slug).await;
    let live = h.state.store().get_live(&slug).await.unwrap().unwrap();
    assert_eq!(live.avatar_url.as_deref(), Some(AVATAR));
    assert!(live.listed);
}

#[tokio::test]
async fn the_five_minute_sweep_only_touches_sites_with_recent_activity() {
    let h = Harness::start(false).await;
    let token = h.login().await;
    let slug = h.site(&token).await;
    let touched = live::refresh_active(&h.state).await.unwrap();
    assert!(touched >= 1, "刚发过版本的作品要在这一轮里");
    assert!(h.state.store().get_live(&slug).await.unwrap().is_some());
}
