//! 边缘的全部请求处理：按 Host 分流、保留路径、门禁页、按清单出文件。
//!
//! 只有一个 fallback 处理函数，不交给 matchit——作品子域上兜底的那条要先查清单才知道
//! 路径意味着什么。固定的那几条路径在 `router.rs` 那张表里，这里只写每一条落地后做什么。

use std::sync::Arc;

use axum::body::Body;
use axum::extract::{Request, State};
use axum::http::{header, HeaderMap, HeaderValue, Method, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Router;
use percent_encoding::percent_decode_str;
use playtest_common::api::ErrorCode;
use playtest_common::follow::{root_paths, FollowTarget};
use playtest_common::limits::{MAX_ARTICLE_HTML_BYTES, MAX_ARTICLE_SOURCE_BYTES};
use playtest_common::manifest::{GateMode, Manifest, WorkKind};
use playtest_common::store::Store;
use playtest_common::{GATE_COOKIE, ME_COOKIE, SESSION_COOKIE};
use rand::RngCore;
use tokio::io::AsyncSeekExt;
use tower_http::trace::TraceLayer;

use crate::breaker::Breaker;
use crate::capabilities::CapabilitiesCache;
use crate::card::{self, Card, Cover, Renderer, Shape};
use crate::config::Config;
use crate::events::{is_wechat, EventLog, Kind, Visitor};
use crate::follow;
use crate::game_headers::{self, Source};
use crate::gate::{self, GatePage};
use crate::host::{self, HostKind};
use crate::identity;
use crate::live::LiveCache;
use crate::paths::{self, AcceptEncoding, Resolved, Served};
use crate::plaza::{self, PlazaCache};
use crate::range::{self, Range};
use crate::router::{self, reserved_tail, Reserved, Root};
use crate::seo;
use crate::share::SharePage;
use crate::tunnel::{self, Tunnels};
use crate::{breaker, pages, sites::SiteState, sites::SiteStore};
use time::OffsetDateTime;

/// 门禁 cookie 活 24 小时：一次点开在一天内不再重复出现（DESIGN §3.3）。
const GATE_MAX_AGE: u64 = 24 * 60 * 60;
/// 会话 cookie 活 90 天，对齐 DESIGN §3.4 的数据保留期。
const SESSION_MAX_AGE: u64 = 90 * 24 * 60 * 60;
/// 表单体上限。这两个表单只有一个下拉和一个文本框。
const MAX_FORM_BYTES: usize = 16 * 1024;
/// 封面在浏览器里缓存多久。地址带着内容哈希的前几位（控制面拼的 `?v=`），换了封面地址就变，
/// 所以可以放心缓存一天；直接打 `/_playtest/cover` 不带 `?v=` 的也只是最多旧一天。
const COVER_MAX_AGE: u64 = 24 * 60 * 60;
/// `pt_me` 活一年（DESIGN §3.6：换设备是再点一次链接，不是重新注册）。
const ME_MAX_AGE: u64 = 30 * 24 * 60 * 60;
/// 封面超过这么大就不往邀请卡上嵌了——base64 之后还要涨三分之一，
/// 而卡上那一块只有 1080 像素宽，一张 8 MB 的原图在上面看不出区别。
const CARD_COVER_MAX_BYTES: u64 = 8 * 1024 * 1024;

pub struct App {
    pub config: Config,
    pub sites: SiteStore,
    pub blob_cache: Arc<crate::blob_cache::BlobCache>,
    pub events: EventLog,
    pub breaker: Breaker,
    pub traffic: Arc<crate::quota::Ledger>,
    /// 现在连着的隧道（DESIGN §4.3）。和 `sites` 互不知情，谁说了算在 [`site`] 里定。
    pub tunnels: Arc<Tunnels>,
    /// 广场那一份 `plaza.json`（DESIGN §3.9），同样只读对象存储。
    pub plaza: PlazaCache,
    /// 每个作品会变的那些（名额、群、公开反馈、头像）：`sites/<slug>/live.json`。
    pub live: LiveCache,
    /// 控制面现在能做什么：`capabilities.json`。
    pub caps: CapabilitiesCache,
    /// 邀请卡的字体库与渲染缓存（DESIGN §4.9）。整个进程共用一份。
    pub cards: Arc<Renderer>,
}

impl App {
    pub fn new(config: Config) -> Self {
        Self::try_new(config).expect("对象存储配置不合法")
    }

    pub fn try_new(config: Config) -> anyhow::Result<Self> {
        let store = Store::from_config(&config.store_config()?)?;
        let blob_cache = Arc::new(crate::blob_cache::BlobCache::new(
            config.blob_cache_dir(),
            config.blob_cache_max_bytes(),
        ));
        let sites = SiteStore::new(store.clone());
        let plaza = PlazaCache::new(store.clone());
        let live = LiveCache::new(store.clone());
        let caps = CapabilitiesCache::new(store.clone());
        let events = EventLog::new(config.events_path());
        let tunnels = Tunnels::new(&config, store.clone());
        let traffic = crate::quota::Ledger::with_store(config.data_dir.clone(), store);
        Ok(Self {
            config,
            sites,
            blob_cache,
            events,
            breaker: Breaker::new(),
            traffic,
            tunnels,
            plaza,
            live,
            caps,
            cards: card::shared(),
        })
    }
}

pub fn router(app: Arc<App>) -> Router {
    Router::new()
        .fallback(handle)
        .layer(TraceLayer::new_for_http())
        .with_state(app)
}

async fn handle(State(app): State<Arc<App>>, req: Request) -> Response {
    let (parts, body) = req.into_parts();

    // 健康检查在 Host 分流之前：探针带的 Host 通常是 IP，分流会把它判成不存在。
    if let Some((Reserved::Healthz, allow)) =
        reserved_tail(parts.uri.path()).and_then(router::reserved)
    {
        if !allow.permits(&parts.method) {
            return method_not_allowed(allow.header());
        }
        let mut headers = base_headers();
        put(&mut headers, "content-type", "text/plain; charset=utf-8");
        return (StatusCode::OK, headers, "ok\n").into_response();
    }

    let authority = authority_of(&parts);
    match host::classify(&authority, &app.config.host_suffix) {
        HostKind::Root => root(&app, &authority, parts, body).await,
        HostKind::Unknown => page(StatusCode::NOT_FOUND, pages::not_found(), None),
        HostKind::Site(slug) => {
            let head_only = parts.method == Method::HEAD;
            let response = site(&app, &slug, &authority, parts, body).await;
            if !response.status().is_success() {
                return response;
            }
            match app.traffic.meter(&slug).await {
                Ok(meter) => meter.response(response, head_only),
                Err(denied) => denied.response(),
            }
        }
    }
}

/// 根域：广场、关注页、确认与退订（DESIGN §3.9、§3.10）。**只有这几条路径**，
/// 别的一律 404——根域上不放任何用户内容，它是玩家路径里唯一我们说了算的一页。
async fn root(
    app: &App,
    authority: &str,
    parts: axum::http::request::Parts,
    body: Body,
) -> Response {
    if parts.method == Method::POST
        && header_str(&parts.headers, "origin")
            != Some(format!("{}://{authority}", app.config.public_scheme).as_str())
    {
        return page(StatusCode::FORBIDDEN, pages::untrusted_action(), None);
    }
    let me = identity::token(&parts.headers, app.config.public_scheme == "https")
        .filter(|v| is_me_token(v))
        .map(str::to_string);
    let path = parts.uri.path().to_string();
    let Some((door, allow)) = router::root(&path) else {
        return page(StatusCode::NOT_FOUND, pages::not_found(), None);
    };
    if !allow.permits(&parts.method) {
        return method_not_allowed(allow.header());
    }
    let response = match door {
        Root::Plaza => {
            discovery_page(
                app,
                authority,
                parts.uri.query().unwrap_or_default(),
                None,
                false,
                None,
            )
            .await
        }
        Root::Collections => {
            discovery_page(
                app,
                authority,
                parts.uri.query().unwrap_or_default(),
                None,
                true,
                None,
            )
            .await
        }
        Root::Collection(slug) => {
            discovery_page(
                app,
                authority,
                parts.uri.query().unwrap_or_default(),
                Some(slug),
                false,
                me.as_deref(),
            )
            .await
        }
        Root::Project(slug) => project_door(app, authority, slug, parts, body).await,
        Root::Follow => root_follow(app, authority, me.as_deref(), body).await,
        Root::Me => mine(app, authority, me.as_deref()).await,
        Root::MeAction => me_action(app, authority, me.as_deref(), body).await,
        Root::Confirm(token) => confirm(app, authority, token).await,
        Root::Unsubscribe(token) => unsubscribe(app, authority, token).await,
        Root::ServiceWorker => service_worker(parts.method == Method::HEAD),
        Root::Llms => llms_pointer(),
        Root::FaviconSvg => favicon_svg(parts.method == Method::HEAD),
        Root::FaviconIco => favicon_ico(parts.method == Method::HEAD),
        Root::Robots => seo::robots_txt(),
        Root::Sitemap => {
            let plaza = app.plaza.get().await;
            seo::sitemap_xml(&plaza, OffsetDateTime::now_utc())
        }
    };
    response
}

/// 主域作品邀请函（DESIGN §3.1、§3.3）：`playtest.run/p/<slug>`。
/// 承载作品门面、版本告示、名额、一键秒关，以及网页入口 / 文章正文 / 视频播放器。
async fn project_door(
    app: &App,
    authority: &str,
    slug: &str,
    parts: axum::http::request::Parts,
    body: Body,
) -> Response {
    let manifest = match app.sites.resolve(slug).await {
        SiteState::Live(m) => m,
        SiteState::Expired(m) => {
            return page(StatusCode::GONE, pages::gone(), Some((m.isolated, false)));
        }
        SiteState::Unavailable => {
            return page(StatusCode::SERVICE_UNAVAILABLE, pages::unavailable(), None);
        }
        SiteState::Missing => match app.tunnels.get(slug) {
            Some(session) => session.manifest(),
            None => match app.tunnels.last_seen(slug).await {
                Some(seen) => {
                    return tunnel::page(
                        StatusCode::SERVICE_UNAVAILABLE,
                        tunnel::offline::offline(&seen),
                        seen.isolated,
                    );
                }
                None => return page(StatusCode::NOT_FOUND, pages::not_found(), None),
            },
        },
    };

    let scheme = &app.config.public_scheme;
    let suffix = &app.config.host_suffix;
    let port = host::port_of(authority);
    let port_part = port.map(|p| format!(":{p}")).unwrap_or_default();
    let site_url = format!("{scheme}://{slug}.{suffix}{port_part}/");
    let origin = format!("{scheme}://{slug}.{suffix}{port_part}");

    let me_token = identity::token(&parts.headers, app.config.public_scheme == "https")
        .filter(|v| is_me_token(v))
        .map(str::to_string);
    let existing_sid = cookie_value(&parts.headers, SESSION_COOKIE)
        .filter(|value| is_session_id(value))
        .map(str::to_string);
    let sid = existing_sid.clone().unwrap_or_else(new_session_id);

    if parts.method == Method::POST {
        let form = read_form(body).await;
        if field(&form, "action").as_deref() == Some("feedback") || field(&form, "feedback").is_some() {
            let text = field(&form, "feedback").unwrap_or_default().trim().to_string();
            let chapter_id = field(&form, "chapter").map(|c| c.trim().to_string()).filter(|c| !c.is_empty());
            let feedback_text = if let Some(ref cid) = chapter_id {
                format!("[{cid}] {text}")
            } else {
                text.clone()
            };
            let root_origin = format!("{scheme}://{suffix}{port_part}");
            let result = if header_str(&parts.headers, "origin").is_some_and(|origin| origin != root_origin) {
                Err(403)
            } else if text.is_empty() || text.chars().count() > 200 {
                Err(400)
            } else {
                let res = follow::submit_feedback(
                    app.config.api_internal_url.as_deref(),
                    &manifest.slug,
                    &sid,
                    &feedback_text,
                )
                .await;
                if res.is_ok() {
                    app.live.invalidate(&manifest.slug);
                }
                res
            };
            let (status, message) = match result {
                Ok(()) => (StatusCode::OK, "反馈已送达。是否公开由作者决定。"),
                Err(400) => (StatusCode::BAD_REQUEST, "请填写 1–200 字的反馈。"),
                Err(403) => (StatusCode::FORBIDDEN, "请在作品页面提交反馈。"),
                Err(429) => (StatusCode::TOO_MANY_REQUESTS, "本次反馈已达上限，或提交过于频繁，请稍后再试。"),
                Err(_) => (StatusCode::SERVICE_UNAVAILABLE, "反馈未送达，请稍后重试；你的文字仍保留在这里。"),
            };
            let mut headers = base_headers();
            put(&mut headers, "cache-control", "no-store");
            if existing_sid.is_none() {
                append(&mut headers, "set-cookie", &cookie(SESSION_COOKIE, &sid, Some(SESSION_MAX_AGE), app.config.public_scheme == "https", None));
            }
            if header_str(&parts.headers, "accept").is_some_and(|accept| accept.contains("application/json")) {
                return (status, headers, axum::Json(serde_json::json!({"ok": result.is_ok(), "message": message}))).into_response();
            }
            let action = crate::html::esc(parts.uri.path());
            let retry = if result.is_err() {
                format!("<form method=\"post\" action=\"{action}\"><input type=\"hidden\" name=\"action\" value=\"feedback\"><label>你的反馈<textarea name=\"feedback\" maxlength=\"200\">{}</textarea></label><button type=\"submit\">重新发送</button></form>", crate::html::esc(&text))
            } else { String::new() };
            let html = crate::html::shell("作品反馈", "", &format!("<h1>{message}</h1>{retry}<a href=\"{action}#chat-panel\">返回作品</a>"));
            return (status, headers, html).into_response();
        }

        if manifest.kind != WorkKind::Web {
            return method_not_allowed("GET, HEAD, POST");
        }

        let source = gate::known_source(field(&form, gate::field::FROM).as_deref());
        let name = field(&form, gate::field::NAME).and_then(|raw| gate::clean_name(&raw));
        let referer = field(&form, gate::field::REFERER)
            .filter(|f| f.starts_with("http://") || f.starts_with("https://"))
            .unwrap_or_default();
        let ua = header_str(&parts.headers, "user-agent")
            .unwrap_or_default()
            .to_string();
        let visitor = Visitor {
            wechat: is_wechat(&ua),
            sid: sid.clone(),
            ua,
            referer,
            from: source,
            name,
        };
        app.events
            .append(
                Kind::Start,
                &manifest.slug,
                manifest.version,
                &visitor,
                None,
                None,
            )
            .await;

        let mut headers = base_headers();
        put(&mut headers, "location", &site_url);
        put(&mut headers, "cache-control", "no-store");
        if existing_sid.is_none() {
            append(
                &mut headers,
                "set-cookie",
                &cookie(
                    SESSION_COOKIE,
                    &sid,
                    Some(SESSION_MAX_AGE),
                    app.config.public_scheme == "https",
                    None,
                ),
            );
        }
        if header_str(&parts.headers, "accept").is_some_and(|a| a.contains("application/json")) {
            return (
                StatusCode::OK,
                headers,
                axum::Json(serde_json::json!({"ok": true, "location": site_url})),
            )
                .into_response();
        }
        return (StatusCode::SEE_OTHER, headers).into_response();
    }

    let is_chat_feed = parts.uri.query().is_some_and(|q| q.contains("chat=1") || q.contains("chat_feed=1"));
    let is_json_accept = header_str(&parts.headers, "accept").is_some_and(|a| a.contains("application/json"));
    if is_chat_feed || (parts.method == Method::GET && is_json_accept) {
        if is_chat_feed {
            app.live.invalidate(&manifest.slug);
        }
        let live = app.live.get(&manifest.slug).await;
        let messages = live.public_feedback.iter().take(playtest_common::live::PUBLIC_FEEDBACK_ON_GATE).map(|item| {
            let who = item.name.as_deref().map(str::trim).filter(|n| !n.is_empty()).unwrap_or("A tester");
            let avatar = playtest_common::avatar::svg_for_seed(who, Some(28), Some("chat-avatar-svg"));
            let time_str = crate::when::day_time(&item.at).unwrap_or_else(|| "刚刚".to_string());
            serde_json::json!({
                "who": who,
                "text": item.text,
                "version": item.version,
                "time": time_str,
                "avatar": avatar,
            })
        }).collect::<Vec<_>>();
        let mut headers = base_headers();
        put(&mut headers, "cache-control", "no-cache, no-store, must-revalidate");
        return (StatusCode::OK, headers, axum::Json(serde_json::json!({
            "ok": true,
            "count": messages.len(),
            "messages": messages,
        }))).into_response();
    }

    let live = app.live.get(&manifest.slug).await;
    let caps = app.caps.get().await;
    let already_followed = if let Some(token) = me_token.as_deref() {
        match follow::view(app.config.api_internal_url.as_deref(), token).await {
            follow::Mine::View(v) => v.follows.iter().any(|f| match &f.target {
                playtest_common::follow::FollowTarget::Site { slug: s } => s == &manifest.slug,
                _ => false,
            }),
            _ => false,
        }
    } else {
        false
    };

    let root_url = format!("{scheme}://{suffix}{port_part}/");
    let page_url = format!("{scheme}://{suffix}{port_part}{}", parts.uri.path());
    let ua = header_str(&parts.headers, "user-agent").unwrap_or_default();
    let referer = header_str(&parts.headers, "referer").unwrap_or_default();
    let from = gate::known_source(
        parts
            .uri
            .query()
            .and_then(|q| field(q, playtest_common::FROM_PARAM))
            .as_deref(),
    );
    let collection_slug = parts
        .uri
        .query()
        .and_then(|q| field(q, "collection"));
    let plaza = app.plaza.get().await;
    let collection_context = crate::discovery::context(&plaza, slug, collection_slug.as_deref());
    let to_path = format!("{}{slug}", root_paths::PROJECT_PREFIX);

    let is_tunnel = app.tunnels.get(slug).is_some_and(|s| !s.claims.hybrid);
    let version_label = if is_tunnel {
        Some(tunnel::ONLINE_LABEL)
    } else {
        None
    };

    let requested_chapter_id = parts
        .uri
        .query()
        .and_then(|q| field(q, "chapter"));
    let (current_chapter, current_chapter_index) = if manifest.is_serial() {
        if let Some(cid) = requested_chapter_id.as_deref() {
            if let Some((idx, ch)) = manifest.chapters.iter().enumerate().find(|(_, c)| c.id == cid) {
                (Some(ch), Some(idx))
            } else {
                (None, None)
            }
        } else {
            (manifest.chapters.first(), Some(0))
        }
    } else {
        (None, None)
    };

    let article_html = if manifest.kind == WorkKind::Article {
        if manifest.is_serial() {
            if let Some(chapter) = current_chapter {
                match app
                    .sites
                    .store()
                    .get_blob_bytes(&chapter.hash, MAX_ARTICLE_SOURCE_BYTES)
                    .await
                {
                    Ok(Some(bytes)) => {
                        let text = String::from_utf8(bytes).ok();
                        if let Some(content) = text {
                            if content.trim_start().starts_with('<') {
                                Some(content)
                            } else {
                                playtest_common::article::render(&content, &chapter.path, &origin).ok()
                            }
                        } else {
                            None
                        }
                    }
                    _ => None,
                }
            } else if requested_chapter_id.is_some() {
                Some(format!(
                    "<div class=\"chapter-missing\">\
                     <h2>未找到指定章节</h2>\
                     <p>你访问的章节可能已下架或链接有误。</p>\
                     <p><a class=\"media-control\" href=\"/p/{slug}\">返回作品起始页</a></p>\
                     </div>"
                ))
            } else {
                None
            }
        } else {
            let Some(artifact) = manifest.article.as_ref() else {
                tracing::warn!(slug, version = manifest.version, "文章清单缺少展示产物");
                return page(StatusCode::SERVICE_UNAVAILABLE, pages::unavailable(), None);
            };
            match app
                .sites
                .store()
                .get_blob_bytes(&artifact.hash, MAX_ARTICLE_HTML_BYTES)
                .await
            {
                Ok(Some(bytes)) => match String::from_utf8(bytes) {
                    Ok(html) => Some(html),
                    Err(_) => {
                        tracing::warn!(slug, version = manifest.version, "文章展示产物不是 UTF-8");
                        return page(StatusCode::SERVICE_UNAVAILABLE, pages::unavailable(), None);
                    }
                },
                Ok(None) => {
                    tracing::warn!(slug, version = manifest.version, "文章展示产物不存在");
                    return page(StatusCode::SERVICE_UNAVAILABLE, pages::unavailable(), None);
                }
                Err(error) => {
                    tracing::warn!(slug, version = manifest.version, %error, "读取文章展示产物失败");
                    return page(StatusCode::SERVICE_UNAVAILABLE, pages::unavailable(), None);
                }
            }
        }
    } else {
        None
    };

    let nonce = new_nonce();
    let html = GatePage {
        manifest: &manifest,
        live: &live,
        caps: &caps,
        to: &to_path,
        host_suffix: suffix,
        root_url: &root_url,
        page_url: &page_url,
        origin: &origin,
        wechat: is_wechat(ua),
        version_label,
        referer,
        from,
        me_token: me_token.as_deref(),
        already_followed,
        is_root: true,
        nonce: Some(&nonce),
        article_html: article_html.as_deref(),
        current_chapter,
        current_chapter_index,
        collection_context: if collection_context.is_empty() {
            None
        } else {
            Some(&collection_context)
        },
    }
    .render();

    if parts.method == Method::GET {
        let visitor = Visitor {
            wechat: is_wechat(ua),
            sid: sid.clone(),
            ua: ua.to_string(),
            referer: referer.to_string(),
            from,
            name: None,
        };
        app.events
            .append(
                Kind::GateView,
                &manifest.slug,
                manifest.version,
                &visitor,
                None,
                None,
            )
            .await;
    }

    let mut headers = root_headers(app, authority, &nonce);
    if existing_sid.is_none() {
        append(
            &mut headers,
            "set-cookie",
            &cookie(
                SESSION_COOKIE,
                &sid,
                Some(SESSION_MAX_AGE),
                app.config.public_scheme == "https",
                None,
            ),
        );
    }
    (StatusCode::OK, headers, html).into_response()
}

/// 根域上给助手的一张字条：这里是玩家那一侧，开发者那几个文件在另一个域。
///
/// 玩家域上本来只放玩家看的东西（AGENTS 第 7 条）。这是第二个例外，理由和根域介绍页
/// 那个链接一样：一个搜到 `playtest.run` 的助手要能自己找到路，否则它只能照着首页猜——
/// 猜出来的结论就是「这个东西不对口」。它不在任何玩家路径上，玩家不会遇到它。
fn llms_pointer() -> Response {
    let body = format!(
        "# playtest\n\n\
         Build in public. Show the work, not the hype.\n\n\
         This host serves the player side: the works themselves, the gate page, the plaza.\n\
         There is nothing here for you to call.\n\n\
         Everything an assistant needs — what this is, when not to use it, the CLI, the MCP\n\
         server and the HTTP API — is on the developer side:\n\n\
         {api}/llms.txt\n\
         {api}/llms-full.txt\n\
         {api}/skill.md\n\
         {api}/openapi.json\n",
        api = playtest_common::DEVELOPER_API_URL,
    );
    (
        StatusCode::OK,
        [
            (
                header::CONTENT_TYPE,
                HeaderValue::from_static("text/plain; charset=utf-8"),
            ),
            (
                header::CACHE_CONTROL,
                HeaderValue::from_static("public, max-age=3600"),
            ),
        ],
        body,
    )
        .into_response()
}

async fn discovery_page(
    app: &App,
    authority: &str,
    raw_query: &str,
    collection_slug: Option<&str>,
    index: bool,
    me_token: Option<&str>,
) -> Response {
    let plaza = app.plaza.get().await;
    let nonce = new_nonce();
    let view = plaza::View {
        plaza: &plaza,
        now: time::OffsetDateTime::now_utc(),
    };
    let query = crate::discovery::Query::parse(raw_query);
    let html = if let Some(slug) = collection_slug {
        let Some(collection) = plaza
            .collections
            .iter()
            .find(|collection| collection.slug == slug && collection.public && !collection.hidden)
        else {
            return root_html(app, authority, StatusCode::NOT_FOUND, plaza::wrap("合集暂不可用", "", plaza::Here::Collections, "<div class=\"content\"><h1>这个合集暂时不可用</h1><p>可能已停止公开或被移除。</p><a href=\"/collections\">看看其他合集 →</a></div>"));
        };
        let caps = app.caps.get().await;
        let viewer = if let Some(token) = me_token {
            match follow::view(app.config.api_internal_url.as_deref(), token).await {
                follow::Mine::View(view) => Some(view),
                _ => None,
            }
        } else {
            None
        };
        crate::discovery::collection(&view, collection, &query, &caps, viewer.as_ref())
    } else {
        crate::discovery::home(&view, &query, index)
    };
    let html = crate::html::enhance(html, &nonce);
    let mut headers = root_headers(app, authority, &nonce);
    // 这一页对所有人一样，可以短暂公共缓存：内容 30 秒才变一次。
    put(
        &mut headers,
        "cache-control",
        if collection_slug.is_some() {
            "private, no-store"
        } else {
            "public, max-age=30"
        },
    );
    (StatusCode::OK, headers, html).into_response()
}

/// 根域上的关注：可以是某个作品，也可以是广场本身。有 `pt_me` 就是一下点击。
async fn root_follow(app: &App, authority: &str, me: Option<&str>, body: Body) -> Response {
    let raw = read_form(body).await;
    let action = field(&raw, "action").unwrap_or_default();
    if action == "unfollow" {
        let target = field(&raw, playtest_common::follow::form::TARGET)
            .as_deref()
            .and_then(FollowTarget::parse);
        if let (Some(token), Some(target)) = (me, target) {
            follow::unfollow(app.config.api_internal_url.as_deref(), token, target).await;
        }
        let to = field(&raw, "to")
            .map(|t| same_origin_target(&t))
            .unwrap_or_else(|| root_paths::ME.to_string());
        let mut headers = base_headers();
        put(&mut headers, "location", &to);
        put(&mut headers, "cache-control", "no-store");
        return (StatusCode::SEE_OTHER, headers).into_response();
    }

    let submission = match follow::parse(&raw, None, me) {
        Ok(sub) => sub,
        Err(why) => {
            return root_html(
                app,
                authority,
                StatusCode::BAD_REQUEST,
                follow::invalid_page(&why, root_paths::ME),
            )
        }
    };
    let outcome =
        follow::register(app.config.api_internal_url.as_deref(), &submission.request).await;
    let back = if submission.to == "/" {
        root_paths::ME
    } else {
        &submission.to
    };
    if me.is_some()
        && matches!(submission.request.target, FollowTarget::Site { .. })
        && matches!(
            outcome,
            follow::Outcome::Answered(playtest_common::follow::FollowResponse::Subscribed)
                | follow::Outcome::Answered(
                    playtest_common::follow::FollowResponse::AlreadyFollowing
                )
        )
    {
        let mut headers = base_headers();
        put(&mut headers, "location", back);
        put(&mut headers, "cache-control", "no-store");
        return (StatusCode::SEE_OTHER, headers).into_response();
    }
    let (status, html) = follow::result_page(&outcome, &submission, back);
    root_html(app, authority, status, html)
}

/// 关注页（DESIGN §3.10）。没有 `pt_me` 的人看到的是一个邮箱输入，不是一页登录墙。
async fn mine(app: &App, authority: &str, me: Option<&str>) -> Response {
    let caps = app.caps.get().await;
    let nonce = new_nonce();
    let mut stale = false;
    let view = match me {
        Some(token) => match follow::view(app.config.api_internal_url.as_deref(), token).await {
            follow::Mine::View(view) => Some(view),
            // 控制面不认这把钥匙：清掉，按没有处理。不解释，玩家再点一次确认信就是了。
            follow::Mine::Stale => {
                stale = true;
                None
            }
            follow::Mine::Unavailable => {
                let html = crate::plaza::wrap("关注", "", crate::plaza::Here::Mine,
                    "<div class=\"drawer\"><h1>暂时无法读取关注</h1><p>服务连接失败，关注记录没有改变。请稍后重试。</p><a class=\"button\" href=\"/me\">重试</a></div>");
                let html = crate::html::enhance(html, &nonce);
                let mut headers = root_headers(app, authority, &nonce);
                put(&mut headers, "cache-control", "no-store");
                return (StatusCode::SERVICE_UNAVAILABLE, headers, html).into_response();
            }
        },
        None => None,
    };
    let html = follow::me_page(&follow::MePage {
        view: view.as_ref(),
        caps: &caps,
        nonce: &nonce,
    });
    let html = crate::html::enhance(html, &nonce);
    let mut headers = root_headers(app, authority, &nonce);
    put(&mut headers, "cache-control", "no-store");
    if stale {
        append(
            &mut headers,
            "set-cookie",
            &cookie(
                identity::cookie_name(app.config.public_scheme == "https"),
                "",
                Some(0),
                app.config.public_scheme == "https",
                None,
            ),
        );
    }
    (StatusCode::OK, headers, html).into_response()
}

/// 关注页上那三个动作。做完一律 303 回 `/me`——刷新不会重复提交。
async fn me_action(app: &App, authority: &str, me: Option<&str>, body: Body) -> Response {
    let raw = read_form(body).await;
    let action = field(&raw, "action").unwrap_or_default();
    let api = app.config.api_internal_url.as_deref();
    match action.as_str() {
        follow::ACTION_UNFOLLOW => {
            let target = field(&raw, playtest_common::follow::form::TARGET)
                .as_deref()
                .and_then(FollowTarget::parse);
            if let (Some(token), Some(target)) = (me, target) {
                follow::unfollow(api, token, target).await;
            }
        }
        follow::ACTION_SEND_LINK => {
            let email = field(&raw, playtest_common::follow::form::EMAIL).unwrap_or_default();
            if playtest_common::follow::looks_like_email(&email) {
                follow::send_link(api, email.trim()).await;
            }
        }
        // 只有 `pt_me` 的人才有「已开启」那一行可点；没有 `pt_me` 的浏览器要关通知，
        // 直接在浏览器设置里关，我们没有它的身份。
        follow::ACTION_PUSH_OFF => {
            if let Some(token) = me {
                follow::push_off(api, token).await;
            }
        }
        _ => {}
    }
    let mut headers = base_headers();
    put(&mut headers, "location", root_paths::ME);
    put(&mut headers, "cache-control", "no-store");
    let _ = authority;
    (StatusCode::SEE_OTHER, headers).into_response()
}

/// 确认信里那条链接：换到 `pt_me`，种在根域上（host-only，DESIGN §4.1），再 303 到关注页。
async fn confirm(app: &App, authority: &str, token: &str) -> Response {
    let Some(answer) = follow::confirm(app.config.api_internal_url.as_deref(), token).await else {
        let caps = app.caps.get().await;
        return root_html(
            app,
            authority,
            StatusCode::OK,
            follow::confirm_failed_page(&caps),
        );
    };
    let mut headers = base_headers();
    put(&mut headers, "location", root_paths::ME);
    put(&mut headers, "cache-control", "no-store");
    append(
        &mut headers,
        "set-cookie",
        &cookie(
            ME_COOKIE,
            "",
            Some(0),
            app.config.public_scheme == "https",
            Some(&app.config.cookie_domain()),
        ),
    );
    append(
        &mut headers,
        "set-cookie",
        &cookie(
            identity::cookie_name(app.config.public_scheme == "https"),
            &answer.me_token,
            Some(ME_MAX_AGE),
            app.config.public_scheme == "https",
            None,
        ),
    );
    append(
        &mut headers,
        "set-cookie",
        &cookie(
            if app.config.public_scheme == "https" { "__Host-pt_session" } else { "pt_session" },
            &answer.me_token,
            Some(ME_MAX_AGE),
            app.config.public_scheme == "https",
            None,
        ),
    );
    (StatusCode::SEE_OTHER, headers).into_response()
}

/// 每封信底部那个一键退订。点了就退，不问为什么，也不放「再想想」。
async fn unsubscribe(app: &App, authority: &str, token: &str) -> Response {
    let done = follow::unsubscribe(app.config.api_internal_url.as_deref(), token).await;
    let html = follow::unsubscribed_page(done);
    let status = if done {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    let mut response = root_html(app, authority, status, html);
    if done {
        // 退订之后这台设备上那把钥匙也没意义了。
        append(
            response.headers_mut(),
            "set-cookie",
            &cookie(
                identity::cookie_name(app.config.public_scheme == "https"),
                "",
                Some(0),
                app.config.public_scheme == "https",
                None,
            ),
        );
        append(
            response.headers_mut(),
            "set-cookie",
            &cookie(
                if app.config.public_scheme == "https" { "__Host-pt_session" } else { "pt_session" },
                "",
                Some(0),
                app.config.public_scheme == "https",
                None,
            ),
        );
    }
    response
}

/// 根域上我们自己的 Service Worker（只有它能弹通知）。放在保留前缀下面，
/// 作用域是整个根域——它要处理的 `push` 事件和页面在不在开着无关。
fn service_worker(head_only: bool) -> Response {
    let mut headers = base_headers();
    put(&mut headers, "content-type", "application/javascript");
    // 短缓存：改了它要能很快铺开，但也别每次导航都回源。
    put(&mut headers, "cache-control", "public, max-age=600");
    put(&mut headers, "service-worker-allowed", "/");
    if head_only {
        return (StatusCode::OK, headers).into_response();
    }
    (StatusCode::OK, headers, follow::SW_JS).into_response()
}

/// 根域上的矢量 Favicon。全站统一使用官方品牌准星图标。
fn favicon_svg(head_only: bool) -> Response {
    let mut headers = base_headers();
    put(&mut headers, "content-type", "image/svg+xml");
    put(&mut headers, "cache-control", "public, max-age=86400, immutable");
    if head_only {
        return (StatusCode::OK, headers).into_response();
    }
    (StatusCode::OK, headers, crate::html::FAVICON_SVG).into_response()
}

/// 根域上的点阵 Favicon 兜底。
fn favicon_ico(head_only: bool) -> Response {
    let mut headers = base_headers();
    put(&mut headers, "content-type", "image/x-icon");
    put(&mut headers, "cache-control", "public, max-age=86400, immutable");
    if head_only {
        return (StatusCode::OK, headers).into_response();
    }
    (StatusCode::OK, headers, crate::html::FAVICON_ICO).into_response()
}

/// 根域上一页 HTML 的标准答法。
fn root_html(app: &App, authority: &str, status: StatusCode, html: String) -> Response {
    let nonce = new_nonce();
    let mut headers = root_headers(app, authority, &nonce);
    put(&mut headers, "cache-control", "no-store");
    (status, headers, html).into_response()
}

/// 根域的响应头。我们自己的页面、没有用户脚本，所以能把 CSP 锁死。
fn root_headers(app: &App, authority: &str, nonce: &str) -> HeaderMap {
    let mut headers = base_headers();
    put(&mut headers, "content-type", "text/html; charset=utf-8");
    let port = host::port_of(authority)
        .map(|p| format!(":{p}"))
        .unwrap_or_default();
    let img_src = format!(
        "{}://*.{}{port}",
        app.config.public_scheme, app.config.host_suffix
    );
    // 图只从各作品自己的子域来，外加一个例外：GitHub 的头像域。开发者的头像是
    // 「一张脸比一个 ID 更像真人」那一层（DESIGN §3.9），而 GitHub 登录拿到的地址
    // 就在这个域上。只放这一个来源，不放通配。
    // 脚本只放行带这个 nonce 的那几段；`connect-src` 给「用浏览器通知」那一下 fetch；
    // `worker-src` 给我们自己的 Service Worker；`form-action 'self'` 给关注表单。
    put(
        &mut headers,
        "content-security-policy",
        &format!(
            "default-src 'none'; img-src {img_src} https://avatars.githubusercontent.com; media-src {img_src}; \
style-src 'unsafe-inline'; script-src 'nonce-{nonce}'; connect-src 'self'; worker-src 'self'; \
base-uri 'none'; form-action 'self'; frame-ancestors 'none'"
        ),
    );
    put(
        &mut headers,
        "referrer-policy",
        "strict-origin-when-cross-origin",
    );
    headers
}

/// `pt_me` 的形态。它是控制面签的，边缘不解读；这一道只挡明显不是它发的东西
/// （控制字符会把 Set-Cookie 头拆断）。
fn is_me_token(value: &str) -> bool {
    (8..=256).contains(&value.len()) && value.bytes().all(|b| b.is_ascii_graphic())
}

/// CSP 的 nonce：16 字节随机数的十六进制，每个响应一个。
fn new_nonce() -> String {
    new_session_id()
}

async fn site(
    app: &App,
    slug: &str,
    authority: &str,
    mut parts: axum::http::request::Parts,
    body: Body,
) -> Response {
    let tail = reserved_tail(parts.uri.path()).map(str::to_string);

    // 握手不看作品状态：开发者要连上来的那一刻，这个 slug 多半还什么都没有。
    if tail
        .as_deref()
        .and_then(router::reserved)
        .map(|(door, _)| door)
        == Some(Reserved::Handshake)
    {
        return tunnel::handshake::respond(&app.tunnels, slug, &mut parts).await;
    }

    // 整作品隧道在线时它说了算：开发者机器上跑着的才是此刻最新的东西。
    // 混合模式的隧道（`claims.hybrid`）不在这里接：它只管上传清单里没有的路径，
    // 分界线由下面 `serve_file` 查完清单之后再定——门禁页、保留路径、静态文件都按上传的版本走。
    if let Some(session) = app.tunnels.get(slug).filter(|s| !s.claims.hybrid) {
        let manifest = session.manifest();
        let mut ctx = Ctx::new(&parts, authority, slug, &app.config);
        ctx.version_label = Some(tunnel::ONLINE_LABEL);
        let ctx = &ctx;
        return match tail {
            Some(tail) => reserved(app, &manifest, ctx, &tail, parts, body).await,
            None => {
                if ctx.from.is_some() && ctx.navigation {
                    let target = format!("{}p/{}", ctx.root_url, slug);
                    let mut headers = base_headers();
                    put(&mut headers, "location", &target);
                    put(&mut headers, "cache-control", "no-store");
                    return (StatusCode::SEE_OTHER, headers).into_response();
                }
                tunnel::proxy::forward(
                    &session,
                    tunnel::proxy::Player {
                        authority,
                        public_scheme: &app.config.public_scheme,
                        navigation: ctx.navigation,
                        isolated: manifest.isolated,
                        ledger: Some(&app.traffic),
                    },
                    parts,
                    body,
                )
                .await
            }
        };
    }

    let manifest = match app.sites.resolve(slug).await {
        SiteState::Live(m) => m,
        SiteState::Expired(m) => {
            return page(StatusCode::GONE, pages::gone(), Some((m.isolated, false)))
        }
        SiteState::Unavailable => {
            return page(StatusCode::SERVICE_UNAVAILABLE, pages::unavailable(), None)
        }
        // 走到这里说明隧道不在线。一个作品可以既上传过又开过隧道，上面的 `Live` 分支
        // 因此排在离线页前面：给玩家一个能玩的旧版本，比给他一页「他不在线」有用。
        SiteState::Missing => match app.tunnels.last_seen(slug).await {
            Some(seen) => {
                return tunnel::page(
                    StatusCode::SERVICE_UNAVAILABLE,
                    tunnel::offline::offline(&seen),
                    seen.isolated,
                )
            }
            None => return page(StatusCode::NOT_FOUND, pages::not_found(), None),
        },
    };

    let ctx = Ctx::new(&parts, authority, slug, &app.config);
    if manifest.kind != WorkKind::Web && parts.uri.path() == "/" && ctx.navigation {
        let target = format!("{}p/{}", ctx.root_url, manifest.slug);
        let mut headers = base_headers();
        put(&mut headers, "location", &target);
        put(&mut headers, "cache-control", "no-store");
        return (StatusCode::SEE_OTHER, headers).into_response();
    }
    match tail {
        Some(tail) => reserved(app, &manifest, &ctx, &tail, parts, body).await,
        None => serve_file(app, slug, authority, &manifest, &ctx, parts, body).await,
    }
}

/// 上传的清单里没有这条路径。混合模式（DESIGN §4.3）下这就是后端的地界：隧道在线就转发
/// ——不论方法，`POST /api/x`、WebSocket 升级都在内；隧道不在线就说清楚「后端不在线」，
/// 回 JSON 而不是一页 HTML（对端是游戏里的 `fetch`，给它 HTML 只会得到一个看不懂的解析错）。
/// 从没开过混合模式的作品，清单里没有就是 404。
async fn beyond_manifest(
    app: &App,
    slug: &str,
    authority: &str,
    manifest: &Manifest,
    ctx: &Ctx,
    parts: axum::http::request::Parts,
    body: Body,
) -> Response {
    if let Some(session) = app.tunnels.get(slug).filter(|s| s.claims.hybrid) {
        return tunnel::proxy::forward(
            &session,
            tunnel::proxy::Player {
                authority,
                public_scheme: &app.config.public_scheme,
                navigation: ctx.navigation,
                isolated: manifest.isolated,
                ledger: Some(&app.traffic),
            },
            parts,
            body,
        )
        .await;
    }
    if app
        .tunnels
        .last_seen(slug)
        .await
        .is_some_and(|seen| seen.hybrid)
    {
        return tunnel::error(
            StatusCode::SERVICE_UNAVAILABLE,
            ErrorCode::BackendOffline,
            "The developer machine is currently offline: static pages load, but the backend is unreachable.",
        );
    }
    if parts.method != Method::GET && parts.method != Method::HEAD {
        return method_not_allowed("GET, HEAD");
    }
    page(
        StatusCode::NOT_FOUND,
        pages::file_not_found(),
        Some((manifest.isolated, false)),
    )
}

/// 请求里跟着走、每个分支都要用的那些东西。
#[allow(dead_code)]
struct Ctx {
    /// `scheme://<后缀>[:端口]/`，角标链到这里。
    root_url: String,
    /// 这一页自己的地址，给 og:url。
    page_url: String,
    /// 这个作品的源 `scheme://<slug>.<后缀>[:端口]`，封面的绝对地址接在后面。
    origin: String,
    visitor: Visitor,
    /// 已有的会话 cookie，形态合法才认。
    sid: Option<String>,
    has_gate_cookie: bool,
    navigation: bool,
    accept_encoding: AcceptEncoding,
    /// 版本那个位置显示什么。上传路径是 `None`（显示 `v7`），隧道路径是「在线」——
    /// 隧道没有版本这个概念（DESIGN §3.5）。门禁页和邀请卡都照它写。
    version_label: Option<&'static str>,
    /// 链接上的 `?from=`，只认 `card` / `notice`（DESIGN §3.5）。
    from: Option<&'static str>,
    /// 共享的设备身份凭证（pt_me），用于一键免登录秒关注与识别已关注状态。
    me_token: Option<String>,
}

impl Ctx {
    fn new(
        parts: &axum::http::request::Parts,
        authority: &str,
        slug: &str,
        config: &Config,
    ) -> Self {
        let headers = &parts.headers;
        let ua = header_str(headers, "user-agent")
            .unwrap_or_default()
            .to_string();
        let referer = header_str(headers, "referer")
            .unwrap_or_default()
            .to_string();
        let sid = cookie_value(headers, SESSION_COOKIE)
            .filter(|s| is_session_id(s))
            .map(str::to_string);
        let port = host::port_of(authority);
        let suffix = &config.host_suffix;
        let scheme = &config.public_scheme;
        let port_part = port.map(|p| format!(":{p}")).unwrap_or_default();
        let from = gate::known_source(
            parts
                .uri
                .query()
                .and_then(|q| field(q, playtest_common::FROM_PARAM))
                .as_deref(),
        );
        Self {
            root_url: format!("{scheme}://{suffix}{port_part}/"),
            page_url: format!("{scheme}://{slug}.{suffix}{port_part}{}", parts.uri.path()),
            origin: format!("{scheme}://{slug}.{suffix}{port_part}"),
            visitor: Visitor {
                wechat: is_wechat(&ua),
                sid: sid.clone().unwrap_or_default(),
                ua,
                referer,
                from,
                name: None,
            },
            sid,
            has_gate_cookie: cookie_value(headers, GATE_COOKIE).is_some(),
            navigation: gate::is_navigation(
                header_str(headers, "sec-fetch-dest"),
                header_str(headers, "accept"),
            ),
            accept_encoding: paths::parse_accept_encoding(header_str(headers, "accept-encoding")),
            version_label: None,
            from,
            me_token: None,
        }
    }
}

// ---------------------------------------------------------------- 保留路径

async fn reserved(
    app: &App,
    manifest: &Manifest,
    ctx: &Ctx,
    tail: &str,
    parts: axum::http::request::Parts,
    body: Body,
) -> Response {
    let isolated = Some((manifest.isolated, false));
    let Some((door, allow)) = router::reserved(tail) else {
        return page(StatusCode::NOT_FOUND, pages::file_not_found(), isolated);
    };
    if !allow.permits(&parts.method) {
        return method_not_allowed(allow.header());
    }
    match door {
        // 这两条在 Host 分流之前、查清单之前就接走了；走到这里说明有人拿它们当作品路径。
        Reserved::Healthz | Reserved::Handshake => {
            page(StatusCode::NOT_FOUND, pages::file_not_found(), isolated)
        }
        Reserved::Start => start(app, manifest, ctx, parts, body).await,
        Reserved::Report if parts.method == Method::POST => {
            let form = read_form(body).await;
            let reason = field(&form, "reason").unwrap_or_default();
            let detail = field(&form, "detail").unwrap_or_default();
            app.events
                .append(
                    Kind::Report,
                    &manifest.slug,
                    manifest.version,
                    &ctx.visitor,
                    Some(&reason),
                    Some(&detail),
                )
                .await;
            page(StatusCode::OK, pages::report_done(), isolated)
        }
        Reserved::Report => page(StatusCode::OK, pages::report_form(), isolated),
        Reserved::Me => {
            crate::me::respond(&app.config, manifest, ctx.sid.as_deref(), &parts.method)
        }
        Reserved::Sdk => crate::sdk::respond(&parts.method, &parts.headers),
        Reserved::Cover => cover(app, manifest, &parts).await,
        Reserved::Card(shape) => card_png(app, manifest, ctx, &parts, shape).await,
        Reserved::Share => {
            let live = app.live.get(&manifest.slug).await;
            match (SharePage {
                manifest,
                live: &live,
                origin: &ctx.origin,
            })
            .render()
            {
                Some(html) => page(StatusCode::OK, html, isolated),
                // 不公开的作品没有这一页。一页「你没有权限」等于告诉别人它存在。
                None => page(StatusCode::NOT_FOUND, pages::not_found(), isolated),
            }
        }
        Reserved::Follow => site_follow(app, manifest, ctx, body).await,
        Reserved::Invite => {
            let target = format!("{}p/{}", ctx.root_url, manifest.slug);
            let mut headers = base_headers();
            put(&mut headers, "location", &target);
            put(&mut headers, "cache-control", "no-store");
            (StatusCode::SEE_OTHER, headers).into_response()
        }
    }
}

/// 作品子域上的关注更新（DESIGN §3.3、§3.6、§4.1）：
/// 已有 pt_me 凭证的设备一键免邮箱关注；未认证设备输入邮箱首次绑定。
async fn site_follow(app: &App, manifest: &Manifest, ctx: &Ctx, body: Body) -> Response {
    let isolated = Some((manifest.isolated, false));
    let invite_target = format!("{}p/{}", ctx.root_url, manifest.slug);
    let raw = read_form(body).await;
    let action = field(&raw, "action").unwrap_or_default();
    if action == "unfollow" {
        if let Some(token) = ctx.me_token.as_deref() {
            let target = FollowTarget::Site {
                slug: manifest.slug.clone(),
            };
            follow::unfollow(app.config.api_internal_url.as_deref(), token, target).await;
        }
        let mut headers = base_headers();
        put(&mut headers, "location", &invite_target);
        put(&mut headers, "cache-control", "no-store");
        return (StatusCode::SEE_OTHER, headers).into_response();
    }

    let submission = match follow::parse(&raw, Some(&manifest.slug), ctx.me_token.as_deref()) {
        Ok(sub) => sub,
        Err(why) => {
            return page(
                StatusCode::BAD_REQUEST,
                follow::invalid_page(&why, &invite_target),
                isolated,
            )
        }
    };
    let outcome =
        follow::register(app.config.api_internal_url.as_deref(), &submission.request).await;
    if ctx.me_token.is_some()
        && matches!(
            outcome,
            follow::Outcome::Answered(playtest_common::follow::FollowResponse::Subscribed)
                | follow::Outcome::Answered(
                    playtest_common::follow::FollowResponse::AlreadyFollowing
                )
        )
    {
        let mut headers = base_headers();
        put(&mut headers, "location", &invite_target);
        put(&mut headers, "cache-control", "no-store");
        return (StatusCode::SEE_OTHER, headers).into_response();
    }
    let (status, html) = follow::result_page(&outcome, &submission, &invite_target);
    page(status, html, isolated)
}

/// 邀请卡（DESIGN §3.4、§4.9）。和封面一样：它是一张图，永远不会拿到 HTML，
/// 也走这个 slug 的每小时熔断。
async fn card_png(
    app: &App,
    manifest: &Manifest,
    ctx: &Ctx,
    parts: &axum::http::request::Parts,
    shape: Shape,
) -> Response {
    let verdict = app
        .breaker
        .check(&manifest.slug, breaker::limit_for(manifest));
    if !verdict.allowed {
        let mut headers = base_headers();
        put(
            &mut headers,
            "retry-after",
            &verdict.retry_after.to_string(),
        );
        return (StatusCode::TOO_MANY_REQUESTS, headers).into_response();
    }

    let live = app.live.get(&manifest.slug).await;
    let cover_bytes = cover_for_card(app, manifest).await;
    let card = Card {
        manifest,
        live: &live,
        origin: &ctx.origin,
        host_suffix: &app.config.host_suffix,
        cover: cover_bytes.as_ref().map(|(mime, bytes)| Cover {
            mime,
            bytes: bytes.as_slice(),
        }),
        version_label: ctx.version_label,
        shape,
    };
    let etag = card.etag();

    let mut headers = base_headers();
    put(&mut headers, "content-type", "image/png");
    put(&mut headers, "etag", &etag);
    put(
        &mut headers,
        "cache-control",
        &format!("public, max-age={}", card::MAX_AGE),
    );
    // 广场和抓分享卡片的机器人从别的源来拿它。
    put(&mut headers, "access-control-allow-origin", "*");
    security(&mut headers, manifest.isolated, true);
    if matches_etag(parts.headers.get("if-none-match"), etag.trim_matches('"')) {
        return (StatusCode::NOT_MODIFIED, headers).into_response();
    }

    let png = match app.cards.cached(&etag) {
        Some(hit) => hit,
        None => {
            // 光栅化是纯 CPU 的几十毫秒，不能占着 tokio 的工作线程。
            let svg = card.svg();
            let renderer = app.cards.clone();
            let key = etag.clone();
            match tokio::task::spawn_blocking(move || renderer.render_svg(key, &svg, shape)).await {
                Ok(Ok(png)) => png,
                Ok(Err(err)) => {
                    tracing::warn!(slug = %manifest.slug, %err, "邀请卡渲染失败");
                    return (StatusCode::INTERNAL_SERVER_ERROR, base_headers()).into_response();
                }
                Err(err) => {
                    tracing::warn!(slug = %manifest.slug, %err, "渲染邀请卡的线程没回来");
                    return (StatusCode::INTERNAL_SERVER_ERROR, base_headers()).into_response();
                }
            }
        }
    };
    put(&mut headers, "content-length", &png.len().to_string());
    if parts.method == Method::HEAD {
        return (StatusCode::OK, headers).into_response();
    }
    app.breaker.record(&manifest.slug, png.len() as u64);
    (StatusCode::OK, headers, Body::from(png)).into_response()
}

/// 卡上那张封面的字节。读不到就当没有封面——卡照常出，只是换成字卡。
async fn cover_for_card(app: &App, manifest: &Manifest) -> Option<(String, Vec<u8>)> {
    let cover = manifest.cover.as_ref()?;
    if cover.size > CARD_COVER_MAX_BYTES {
        return None;
    }
    match app
        .sites
        .store()
        .get_blob_bytes(&cover.hash, CARD_COVER_MAX_BYTES)
        .await
    {
        Ok(Some(bytes)) => Some((cover.mime.clone(), bytes)),
        Ok(None) => {
            tracing::warn!(slug = %manifest.slug, hash = %cover.hash, "邀请卡上的封面不存在，改用字卡");
            None
        }
        Err(error) => {
            tracing::warn!(slug = %manifest.slug, %error, "邀请卡上的封面读不到，改用字卡");
            None
        }
    }
}

/// 封面（DESIGN §3.3）：清单里单独的那条引用，从 blob 出，按清单里记的类型给。
///
/// 它是一张图，永远不会拿到 HTML：没有封面就是一个裸 404——广场卡片和分享抓取器
/// 只认状态码，一页「找不到」的 HTML 对它们是噪音。同样走这个 slug 的每小时熔断。
async fn cover(app: &App, manifest: &Manifest, parts: &axum::http::request::Parts) -> Response {
    let Some(cover) = &manifest.cover else {
        return (StatusCode::NOT_FOUND, base_headers()).into_response();
    };
    let verdict = app
        .breaker
        .check(&manifest.slug, breaker::limit_for(manifest));
    if !verdict.allowed {
        let mut headers = base_headers();
        put(
            &mut headers,
            "retry-after",
            &verdict.retry_after.to_string(),
        );
        return (StatusCode::TOO_MANY_REQUESTS, headers).into_response();
    }
    let total = match app.sites.store().blob_size(&cover.hash).await {
        Ok(Some(size)) => size,
        Ok(None) => {
            tracing::warn!(slug = %manifest.slug, hash = %cover.hash, "封面的 blob 不存在");
            return (StatusCode::NOT_FOUND, base_headers()).into_response();
        }
        Err(error) => {
            tracing::warn!(slug = %manifest.slug, %error, "封面的 blob 读不到");
            return (StatusCode::INTERNAL_SERVER_ERROR, base_headers()).into_response();
        }
    };

    let mut headers = base_headers();
    put(&mut headers, "content-type", &cover.mime);
    put(&mut headers, "content-length", &total.to_string());
    put(&mut headers, "etag", &format!("\"{}\"", cover.hash));
    put(
        &mut headers,
        "cache-control",
        &format!("public, max-age={COVER_MAX_AGE}"),
    );
    // 广场在根域上，封面在作品的子域上：跨源的 <img> 不需要 CORS，但抓分享卡片的机器人
    // 有时候会带 Origin 来要，给它就是。
    put(&mut headers, "access-control-allow-origin", "*");
    security(&mut headers, manifest.isolated, true);
    if matches_etag(parts.headers.get("if-none-match"), &cover.hash) {
        return (StatusCode::NOT_MODIFIED, headers).into_response();
    }
    if parts.method == Method::HEAD {
        return (StatusCode::OK, headers).into_response();
    }
    let object = match app.sites.store().get_blob(&cover.hash, None).await {
        Ok(Some(object)) => object,
        Ok(None) => return (StatusCode::NOT_FOUND, base_headers()).into_response(),
        Err(error) => {
            tracing::warn!(slug = %manifest.slug, %error, "封面的 blob 流式读取失败");
            return (StatusCode::INTERNAL_SERVER_ERROR, base_headers()).into_response();
        }
    };
    app.breaker.record(&manifest.slug, total);
    (
        StatusCode::OK,
        headers,
        Body::from_stream(object.into_stream()),
    )
        .into_response()
}

/// 「开始」这一下：种门禁 cookie 与会话 cookie，记一条 `start`，回到玩家原来要去的地方。
async fn start(
    app: &App,
    manifest: &Manifest,
    ctx: &Ctx,
    parts: axum::http::request::Parts,
    body: Body,
) -> Response {
    let form = read_form(body).await;
    let target = field(&form, gate::field::TO)
        .or_else(|| parts.uri.query().and_then(|q| field(q, gate::field::TO)))
        .unwrap_or_default();
    let location = same_origin_target(&target);
    // 玩家真正从哪来：门禁页把自己收到的 Referer 放在表单里带过来。这一下 POST 的 Referer
    // 永远是门禁页自己，没有信息量。只留一个 URL 形态的值，别的当没有。
    let came_from = field(&form, gate::field::REFERER)
        .filter(|f| f.starts_with("http://") || f.starts_with("https://"))
        .unwrap_or_default();
    // 链接上带的来源比 Referer 干净：扫卡的人多半在微信里，UA 会说「微信」，
    // 但开发者想知道的是「这个人是我发出去的卡带来的」（DESIGN §3.5）。
    let source = gate::known_source(field(&form, gate::field::FROM).as_deref());
    // 留名是自愿的，不填就是不填——没有名字的会话在点名册里显示设备和时间（DESIGN §3.3）。
    let name = field(&form, gate::field::NAME).and_then(|raw| gate::clean_name(&raw));

    let secure = app.config.public_scheme == "https";
    let mut headers = base_headers();
    put(&mut headers, "location", &location);
    put(&mut headers, "cache-control", "no-store");
    security(&mut headers, manifest.isolated, false);

    // Always 是「每次都出」，所以门禁 cookie 只活一个浏览器会话；
    // Once 是「24 小时内不再出」，给足 24 小时。
    let gate_max_age = match manifest.gate {
        GateMode::Always => None,
        _ => Some(GATE_MAX_AGE),
    };
    append(
        &mut headers,
        "set-cookie",
        &cookie(GATE_COOKIE, "1", gate_max_age, secure, None),
    );

    let sid = match &ctx.sid {
        Some(existing) => existing.clone(),
        None => {
            let fresh = new_session_id();
            append(
                &mut headers,
                "set-cookie",
                &cookie(SESSION_COOKIE, &fresh, Some(SESSION_MAX_AGE), secure, None),
            );
            fresh
        }
    };

    let visitor = Visitor {
        sid,
        referer: came_from,
        from: source,
        name,
        ..ctx.visitor.clone()
    };
    app.events
        .append(
            Kind::Start,
            &manifest.slug,
            manifest.version,
            &visitor,
            None,
            None,
        )
        .await;

    (StatusCode::SEE_OTHER, headers).into_response()
}

// ---------------------------------------------------------------- 静态分发

async fn serve_file(
    app: &App,
    slug: &str,
    authority: &str,
    manifest: &Manifest,
    ctx: &Ctx,
    parts: axum::http::request::Parts,
    body: Body,
) -> Response {
    // 熔断判定在取文件之前，也在门禁页之前：这一小时的额度用完了就一个字节都不出
    // （DESIGN §4.8）。`/_playtest/report` 走的是另一条路，举报入口任何时候都开着。
    let verdict = app
        .breaker
        .check(&manifest.slug, breaker::limit_for(manifest));
    if !verdict.allowed {
        return tripped(app, manifest, ctx, verdict).await;
    }
    let isolated = Some((manifest.isolated, false));
    // 连路径都不成形的（`..`、空段）不是后端的地界，是有人在试探。
    let Some(norm) = paths::normalize(parts.uri.path()) else {
        return page(StatusCode::NOT_FOUND, pages::file_not_found(), isolated);
    };
    // 文章原稿作为可回滚的版本原件保存，但阅读路径只公开安全渲染产物与显式配图。
    // 不能因为原稿也在清单里，就让猜到文件名的人从作品子域下载 Markdown。
    if manifest.kind == WorkKind::Article
        && manifest.entry.as_deref() == Some(norm.candidate.as_str())
    {
        return page(StatusCode::NOT_FOUND, pages::file_not_found(), isolated);
    }

    // 先查清单再看方法：清单里没有的路径在混合模式下归后端，POST 也要过去。
    match paths::resolve(manifest, &norm, ctx.accept_encoding, ctx.navigation) {
        Resolved::NotFound => {
            beyond_manifest(app, slug, authority, manifest, ctx, parts, body).await
        }
        _ if parts.method != Method::GET && parts.method != Method::HEAD => {
            method_not_allowed("GET, HEAD")
        }
        Resolved::Redirect(location) => {
            let mut headers = base_headers();
            put(&mut headers, "location", &location);
            put(&mut headers, "cache-control", "no-store");
            put(&mut headers, "content-type", "text/html; charset=utf-8");
            security(&mut headers, manifest.isolated, false);
            let body = format!(
                "<!doctype html><meta charset=\"utf-8\"><title>已移动</title><a href=\"{}\">{}</a>\n",
                crate::html::esc(&location),
                crate::html::esc(&location)
            );
            (StatusCode::MOVED_PERMANENTLY, headers, body).into_response()
        }
        Resolved::File(served) => {
            if ctx.from.is_some() && ctx.navigation && served.is_html {
                let target = format!("{}p/{}", ctx.root_url, manifest.slug);
                let mut headers = base_headers();
                put(&mut headers, "location", &target);
                put(&mut headers, "cache-control", "no-store");
                return (StatusCode::SEE_OTHER, headers).into_response();
            }
            blob(app, manifest, ctx, &parts, &served).await
        }
    }
}

async fn blob(
    app: &App,
    manifest: &Manifest,
    ctx: &Ctx,
    parts: &axum::http::request::Parts,
    served: &Served,
) -> Response {
    let total = match app.sites.store().blob_size(&served.entry.hash).await {
        Ok(Some(size)) => size,
        Ok(None) => {
            tracing::warn!(hash = %served.entry.hash, "清单指向的 blob 不存在");
            return page(
                StatusCode::INTERNAL_SERVER_ERROR,
                pages::broken(),
                Some((manifest.isolated, false)),
            );
        }
        Err(error) => {
            tracing::warn!(hash = %served.entry.hash, %error, "blob 读不到大小");
            return page(
                StatusCode::INTERNAL_SERVER_ERROR,
                pages::broken(),
                Some((manifest.isolated, false)),
            );
        }
    };

    let mut headers = base_headers();
    // 对游戏有感的那几个头全在这里补，隧道路径回来的响应调同一个函数（DESIGN §4.3）。
    // 传清单里那条路径而不是请求路径：`/` 和 `/sub/` 身上没有扩展名，
    // 决定 MIME 的是它们解析到的 `index.html`。
    game_headers::apply(
        &mut headers,
        &served.entry.path,
        manifest.isolated,
        Source::Ours,
        game_headers::Opts {
            resource: !served.is_html,
            encoding: served.encoding,
            vary_encoding: served.vary_encoding,
            ranges: true,
        },
    );
    put(&mut headers, "etag", &format!("\"{}\"", served.entry.hash));
    // 引擎导出物的文件名不带哈希（`Build/game.wasm` 每一版都叫这个名字），
    // 给玩家 URL 长缓存会让玩过 v7 的人拿到一半旧一半新的文件。DESIGN §4.2 里
    // 「对象不可变、长缓存」说的是边缘到对象存储那一层，不是这一层。
    put(
        &mut headers,
        "cache-control",
        if served.is_html {
            "no-cache"
        } else {
            "public, max-age=0, must-revalidate"
        },
    );
    if matches_etag(parts.headers.get("if-none-match"), &served.entry.hash) {
        return (StatusCode::NOT_MODIFIED, headers).into_response();
    }

    let wanted = range::parse(header_str(&parts.headers, "range"), total);
    if wanted == Range::Unsatisfiable {
        put(&mut headers, "content-range", &format!("bytes */{total}"));
        put(&mut headers, "content-type", "text/plain; charset=utf-8");
        return (
            StatusCode::RANGE_NOT_SATISFIABLE,
            headers,
            "请求的范围超出了这个文件\n",
        )
            .into_response();
    }

    let (status, start, length) = match wanted {
        Range::Part { start, end } => {
            put(
                &mut headers,
                "content-range",
                &format!("bytes {start}-{end}/{total}"),
            );
            (StatusCode::PARTIAL_CONTENT, start, end - start + 1)
        }
        _ => (StatusCode::OK, 0, total),
    };
    put(&mut headers, "content-length", &length.to_string());

    if served.is_html && ctx.navigation && parts.method == Method::GET {
        app.events
            .append(
                Kind::HtmlView,
                &manifest.slug,
                manifest.version,
                &ctx.visitor,
                None,
                None,
            )
            .await;
    }

    if parts.method == Method::HEAD {
        return (status, headers).into_response();
    }
    // 记账放在真的要发字节的时候：304、416、HEAD 都不算。
    app.breaker.record(&manifest.slug, length);
    let object_range = (length != total).then_some(start..start + length);

    // 优先尝试本地持久文件（本机 Store 目录或已落盘的本地 Blob 缓存）
    let local_path = if let Ok(path) = app.sites.store().blob_path(&served.entry.hash) {
        if path.is_file() {
            Some(path)
        } else {
            None
        }
    } else {
        app.blob_cache.get(&served.entry.hash)
    };

    if let Some(path) = local_path {
        let body = match tokio::fs::File::open(&path).await {
            Ok(mut file) => {
                if start > 0 {
                    if let Err(err) = file.seek(std::io::SeekFrom::Start(start)).await {
                        tracing::warn!(%err, path = %path.display(), "seek 本地 blob 失败");
                        return page(
                            StatusCode::INTERNAL_SERVER_ERROR,
                            pages::broken(),
                            Some((manifest.isolated, false)),
                        );
                    }
                }
                let stream =
                    tokio_util::io::ReaderStream::new(tokio::io::AsyncReadExt::take(file, length));
                Body::from_stream(stream)
            }
            Err(err) => {
                tracing::warn!(%err, path = %path.display(), "打开本地 blob 失败");
                return page(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    pages::broken(),
                    Some((manifest.isolated, false)),
                );
            }
        };
        return (status, headers, body).into_response();
    }

    // 本地未命中时向远端对象存储（如私有 S3）发起读取
    let object = match app
        .sites
        .store()
        .get_blob(&served.entry.hash, object_range.clone())
        .await
    {
        Ok(Some(object)) => object,
        Ok(None) => {
            tracing::warn!(hash = %served.entry.hash, "准备流式读取时 blob 不见了");
            return page(
                StatusCode::INTERNAL_SERVER_ERROR,
                pages::broken(),
                Some((manifest.isolated, false)),
            );
        }
        Err(error) => {
            tracing::warn!(hash = %served.entry.hash, %error, "blob 流式读取失败");
            return page(
                StatusCode::INTERNAL_SERVER_ERROR,
                pages::broken(),
                Some((manifest.isolated, false)),
            );
        }
    };

    // 若为完整读取（非局部 Range），边向客户端流式输出边缓存至本地磁盘；
    // 局部 Range 读取直接流式写出，避免不必要地把大文件全量落盘。
    let body = if object_range.is_none() {
        app.blob_cache
            .stream_and_cache(&served.entry.hash, total, object.into_stream())
    } else {
        Body::from_stream(object.into_stream())
    };

    (status, headers, body).into_response()
}

// ---------------------------------------------------------------- 熔断

/// 这一小时的额度用完了（DESIGN §4.8）。**导航和子资源分开答**，理由同门禁页的硬线：
/// 给一个 `.wasm` 回 200 或者一页 HTML，浏览器不报网络错，加载器会崩在一个看不懂的地方。
///
/// 两边都用 429 不用 503：503 是「这个服务挂了」，会让探针、CDN 和搜索引擎当成故障，
/// 而这里坏的只是一个 slug 的一小时，别的作品好好的。429 带 `Retry-After` 正好说清
/// 「你等一会儿再来」，也是唯一一个客户端普遍认得的「稍后再试」。
async fn tripped(app: &App, manifest: &Manifest, ctx: &Ctx, verdict: breaker::Verdict) -> Response {
    if verdict.first_trip {
        tracing::warn!(
            slug = %manifest.slug,
            bytes = verdict.total,
            "这一小时的流量用完了，暂时关掉这个作品"
        );
        app.events
            .breaker_trip(
                &manifest.slug,
                manifest.version,
                &ctx.visitor,
                verdict.total,
                breaker::limit_for(manifest),
            )
            .await;
    }

    let mut headers = base_headers();
    put(&mut headers, "cache-control", "no-store");
    put(
        &mut headers,
        "retry-after",
        &verdict.retry_after.to_string(),
    );
    if !ctx.navigation {
        // 子资源：状态码就是全部，一个字节的 HTML 也不给。
        return (StatusCode::TOO_MANY_REQUESTS, headers).into_response();
    }
    put(&mut headers, "content-type", "text/html; charset=utf-8");
    security(&mut headers, manifest.isolated, false);
    (StatusCode::TOO_MANY_REQUESTS, headers, pages::over_quota()).into_response()
}

// ---------------------------------------------------------------- 响应零件

fn base_headers() -> HeaderMap {
    let mut headers = HeaderMap::new();
    // 不加 CSP：作品里什么脚本都可能有，一条策略就能把别人的游戏弄坏（DESIGN §4.2）。
    put(&mut headers, "x-content-type-options", "nosniff");
    headers
}

/// 我们自己渲染的响应没有文件路径，但同样在这个作品的源下，隔离头必须一致。
fn security(headers: &mut HeaderMap, isolated: bool, resource: bool) {
    game_headers::isolation(headers, isolated, resource);
}

/// 我们自己渲染的一页。`isolation` 是 `(isolated, 是不是资源)`，知道清单时才传。
fn page(status: StatusCode, html: String, isolation: Option<(bool, bool)>) -> Response {
    let mut headers = base_headers();
    put(&mut headers, "content-type", "text/html; charset=utf-8");
    put(&mut headers, "cache-control", "no-store");
    if let Some((isolated, resource)) = isolation {
        security(&mut headers, isolated, resource);
    }
    (status, headers, html).into_response()
}

fn method_not_allowed(allow: &str) -> Response {
    let mut headers = base_headers();
    put(&mut headers, "allow", allow);
    put(&mut headers, "content-type", "text/plain; charset=utf-8");
    (
        StatusCode::METHOD_NOT_ALLOWED,
        headers,
        "这个地址不接受这种请求\n",
    )
        .into_response()
}

fn put(headers: &mut HeaderMap, name: &'static str, value: &str) {
    match HeaderValue::from_str(value) {
        Ok(v) => {
            headers.insert(name, v);
        }
        Err(err) => tracing::warn!(name, value, %err, "响应头的值不合法，丢掉"),
    }
}

fn append(headers: &mut HeaderMap, name: &'static str, value: &str) {
    match HeaderValue::from_str(value) {
        Ok(v) => {
            headers.append(name, v);
        }
        Err(err) => tracing::warn!(name, %err, "响应头的值不合法，丢掉"),
    }
}

fn header_str<'a>(headers: &'a HeaderMap, name: &'static str) -> Option<&'a str> {
    headers.get(name).and_then(|v| v.to_str().ok())
}

fn authority_of(parts: &axum::http::request::Parts) -> String {
    // HTTP/2 把权威信息放在 :authority 里，HTTP/1.1 放在 Host 头里。
    parts
        .uri
        .authority()
        .map(|a| a.as_str().to_string())
        .or_else(|| header_str(&parts.headers, "host").map(str::to_string))
        .unwrap_or_default()
}

fn cookie_value<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    for raw in headers.get_all("cookie") {
        let Ok(line) = raw.to_str() else { continue };
        for item in line.split(';') {
            let item = item.trim();
            let Some(rest) = item.strip_prefix(name) else {
                continue;
            };
            let Some(value) = rest.strip_prefix('=') else {
                continue;
            };
            return Some(value);
        }
    }
    None
}

fn cookie(
    name: &str,
    value: &str,
    max_age: Option<u64>,
    secure: bool,
    domain: Option<&str>,
) -> String {
    let mut out = format!("{name}={value}; Path=/; SameSite=Lax; HttpOnly");
    if let Some(domain) = domain {
        out.push_str(&format!("; Domain={domain}"));
    }
    if let Some(age) = max_age {
        out.push_str(&format!("; Max-Age={age}"));
    }
    if secure {
        out.push_str("; Secure");
    }
    out
}

/// 会话 id：16 字节随机数的十六进制。只用来把同一个人的几次打开串起来，
/// 不含任何身份信息（DESIGN §3.4 不收集玩家身份）。
fn new_session_id() -> String {
    let mut bytes = [0u8; 16];
    rand::rng().fill_bytes(&mut bytes);
    let mut out = String::with_capacity(32);
    for b in bytes {
        out.push_str(&format!("{b:02x}"));
    }
    out
}

fn is_session_id(value: &str) -> bool {
    value.len() == 32 && value.bytes().all(|b| b.is_ascii_hexdigit())
}

/// 只接受以单个 `/` 开头的同源相对路径。`//evil.com` 和 `/\evil.com` 在浏览器里
/// 都会被当成绝对地址，是开放重定向；非 ASCII 或空白说明这不是我们发出去的值。
pub(crate) fn same_origin_target(raw: &str) -> String {
    let fallback = "/".to_string();
    if !raw.starts_with('/') {
        return fallback;
    }
    if matches!(raw.as_bytes().get(1), Some(b'/') | Some(b'\\')) {
        return fallback;
    }
    if !raw.bytes().all(|b| (0x21..=0x7e).contains(&b)) {
        return fallback;
    }
    raw.to_string()
}

fn matches_etag(header: Option<&HeaderValue>, hash: &str) -> bool {
    let Some(Ok(raw)) = header.map(|v| v.to_str()) else {
        return false;
    };
    let quoted = format!("\"{hash}\"");
    raw.split(',').any(|candidate| {
        let candidate = candidate.trim();
        candidate == "*"
            || candidate == quoted
            || candidate.strip_prefix("W/").is_some_and(|c| c == quoted)
    })
}

async fn read_form(body: Body) -> String {
    match axum::body::to_bytes(body, MAX_FORM_BYTES).await {
        Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
        Err(err) => {
            tracing::debug!(%err, "表单体读不完，按空处理");
            String::new()
        }
    }
}

/// `application/x-www-form-urlencoded` 只有这么点规则，没必要为它加一个依赖。
pub(crate) fn field(raw: &str, key: &str) -> Option<String> {
    raw.split('&').filter(|p| !p.is_empty()).find_map(|pair| {
        let (name, value) = pair.split_once('=').unwrap_or((pair, ""));
        (decode_component(name) == key).then(|| decode_component(value))
    })
}

fn decode_component(raw: &str) -> String {
    let spaced = raw.replace('+', " ");
    percent_decode_str(&spaced).decode_utf8_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 这里按尾巴分派，契约里给的是整条路径。两边写岔了，卡和分享页就会 404 得莫名其妙。

    #[test]
    fn redirect_targets_must_be_same_origin() {
        assert_eq!(same_origin_target("/"), "/");
        assert_eq!(same_origin_target("/level/3?x=1"), "/level/3?x=1");
        assert_eq!(same_origin_target("//evil.example"), "/");
        assert_eq!(same_origin_target("/\\evil.example"), "/");
        assert_eq!(same_origin_target("https://evil.example"), "/");
        assert_eq!(same_origin_target(""), "/");
        assert_eq!(same_origin_target("/a b"), "/");
        assert_eq!(same_origin_target("/中文"), "/");
        assert_eq!(same_origin_target("/%E4%B8%AD"), "/%E4%B8%AD");
    }

    #[test]
    fn cookie_lookup_needs_an_exact_name() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "cookie",
            HeaderValue::from_static("a=1; pt_gate=1; pt_sid=abc"),
        );
        assert_eq!(cookie_value(&headers, GATE_COOKIE), Some("1"));
        assert_eq!(cookie_value(&headers, SESSION_COOKIE), Some("abc"));
        assert_eq!(cookie_value(&headers, "pt_g"), None);
        assert_eq!(cookie_value(&headers, "nope"), None);
    }

    #[test]
    fn cookie_attributes() {
        assert_eq!(
            cookie("pt_gate", "1", Some(86400), false, None),
            "pt_gate=1; Path=/; SameSite=Lax; HttpOnly; Max-Age=86400"
        );
        assert_eq!(
            cookie("pt_gate", "1", None, true, None),
            "pt_gate=1; Path=/; SameSite=Lax; HttpOnly; Secure"
        );
        assert_eq!(
            cookie("pt_me", "token123", Some(86400), true, Some(".playtest.run")),
            "pt_me=token123; Path=/; SameSite=Lax; HttpOnly; Domain=.playtest.run; Max-Age=86400; Secure"
        );
    }

    #[test]
    fn session_ids_look_right() {
        let id = new_session_id();
        assert_eq!(id.len(), 32);
        assert!(is_session_id(&id));
        assert_ne!(id, new_session_id());
        assert!(!is_session_id("../../etc"));
        assert!(!is_session_id(""));
    }

    #[test]
    fn etag_matching_handles_lists_and_weak_forms() {
        let hash = "a".repeat(64);
        let quoted = format!("\"{hash}\"");
        assert!(matches_etag(
            Some(&HeaderValue::from_str(&quoted).unwrap()),
            &hash
        ));
        assert!(matches_etag(Some(&HeaderValue::from_static("*")), &hash));
        assert!(matches_etag(
            Some(&HeaderValue::from_str(&format!("\"other\", W/{quoted}")).unwrap()),
            &hash
        ));
        assert!(!matches_etag(
            Some(&HeaderValue::from_static("\"other\"")),
            &hash
        ));
        assert!(!matches_etag(None, &hash));
    }

    #[test]
    fn form_fields_are_decoded() {
        assert_eq!(field("to=%2Flevel%2F3", "to").as_deref(), Some("/level/3"));
        assert_eq!(
            field("reason=other&detail=a+b", "detail").as_deref(),
            Some("a b")
        );
        assert_eq!(field("reason=other", "detail"), None);
        assert_eq!(field("flag&to=%2F", "to").as_deref(), Some("/"));
    }
}
