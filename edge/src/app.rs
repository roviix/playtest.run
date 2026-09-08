//! 边缘的全部请求处理：按 Host 分流、保留路径、门禁页、按清单出文件。
//!
//! 只有一个 fallback 处理函数，没有路由表——路径要先经过清单才知道意味着什么，
//! 交给 matchit 反而要把同一段逻辑拆成两处。

use std::io::SeekFrom;
use std::sync::Arc;

use axum::body::Body;
use axum::extract::{Request, State};
use axum::http::{HeaderMap, HeaderValue, Method, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Router;
use percent_encoding::percent_decode_str;
use playtest_common::manifest::{GateMode, Manifest};
use playtest_common::store::FsStore;
use playtest_common::{GATE_COOKIE, RESERVED_PATH_PREFIX, SESSION_COOKIE};
use rand::RngCore;
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use tokio_util::io::ReaderStream;
use tower_http::trace::TraceLayer;

use crate::breaker::Breaker;
use crate::config::Config;
use crate::events::{is_wechat, EventLog, Kind, Visitor};
use crate::game_headers::{self, Source};
use crate::gate::{self, GatePage};
use crate::host::{self, HostKind};
use crate::paths::{self, AcceptEncoding, Resolved, Served};
use crate::plaza::{self, PlazaCache};
use crate::range::{self, Range};
use crate::tunnel::{self, Tunnels};
use crate::{breaker, pages, sites::SiteState, sites::SiteStore};

/// 门禁 cookie 活 24 小时：一次点开在一天内不再重复出现（DESIGN §3.3）。
const GATE_MAX_AGE: u64 = 24 * 60 * 60;
/// 会话 cookie 活 90 天，对齐 DESIGN §3.4 的数据保留期。
const SESSION_MAX_AGE: u64 = 90 * 24 * 60 * 60;
/// 表单体上限。这两个表单只有一个下拉和一个文本框。
const MAX_FORM_BYTES: usize = 16 * 1024;
/// 封面在浏览器里缓存多久。地址带着内容哈希的前几位（控制面拼的 `?v=`），换了封面地址就变，
/// 所以可以放心缓存一天；直接打 `/_playtest/cover` 不带 `?v=` 的也只是最多旧一天。
const COVER_MAX_AGE: u64 = 24 * 60 * 60;

pub struct App {
    pub config: Config,
    pub sites: SiteStore,
    pub events: EventLog,
    pub breaker: Breaker,
    /// 现在连着的隧道（DESIGN §4.3）。和 `sites` 互不知情，谁说了算在 [`site`] 里定。
    pub tunnels: Arc<Tunnels>,
    /// 广场那一份 `plaza.json`（DESIGN §3.8），同样只读对象存储。
    pub plaza: PlazaCache,
}

impl App {
    pub fn new(config: Config) -> Self {
        let store = FsStore::new(config.store_root());
        let sites = SiteStore::new(store.clone());
        let plaza = PlazaCache::new(store);
        let events = EventLog::new(config.events_path());
        let tunnels = Tunnels::new(&config);
        Self {
            config,
            sites,
            events,
            breaker: Breaker::new(),
            tunnels,
            plaza,
        }
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
    if reserved_tail(parts.uri.path()) == Some("healthz") {
        return if parts.method == Method::GET || parts.method == Method::HEAD {
            let mut headers = base_headers();
            put(&mut headers, "content-type", "text/plain; charset=utf-8");
            (StatusCode::OK, headers, "ok\n").into_response()
        } else {
            method_not_allowed("GET, HEAD")
        };
    }

    let authority = authority_of(&parts);
    match host::classify(&authority, &app.config.host_suffix) {
        HostKind::Root => root(&app, &parts.method, parts.uri.path(), &authority).await,
        HostKind::Unknown => page(StatusCode::NOT_FOUND, pages::not_found(), None),
        HostKind::Site(slug) => site(&app, &slug, &authority, parts, body).await,
    }
}

/// 根域就是广场（DESIGN §3.8）。
async fn root(app: &App, method: &Method, path: &str, authority: &str) -> Response {
    if method != Method::GET && method != Method::HEAD {
        return method_not_allowed("GET, HEAD");
    }
    if path != "/" {
        return page(StatusCode::NOT_FOUND, pages::not_found(), None);
    }
    let plaza = app.plaza.get().await;
    let nonce = new_nonce();
    let html = plaza::render(&plaza::View {
        plaza: &plaza,
        host_suffix: &app.config.host_suffix,
        nonce: &nonce,
        now: time::OffsetDateTime::now_utc(),
    });

    let mut headers = base_headers();
    put(&mut headers, "content-type", "text/html; charset=utf-8");
    // 这一页可以短暂公共缓存：内容 30 秒才变一次，前面的 Caddy 或浏览器多拿一次是白拿。
    put(&mut headers, "cache-control", "public, max-age=30");
    // 我们自己的页面，没有用户脚本，所以能锁死：脚本只放行带这个 nonce 的那段，
    // 图只从各作品自己的子域来，别的一律不许（DESIGN §3.8「长相」）。
    let port = host::port_of(authority)
        .map(|p| format!(":{p}"))
        .unwrap_or_default();
    let img_src = format!(
        "{}://*.{}{port}",
        app.config.public_scheme, app.config.host_suffix
    );
    put(
        &mut headers,
        "content-security-policy",
        &format!(
            "default-src 'none'; img-src {img_src}; style-src 'unsafe-inline'; \
script-src 'nonce-{nonce}'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'"
        ),
    );
    put(
        &mut headers,
        "referrer-policy",
        "strict-origin-when-cross-origin",
    );
    (StatusCode::OK, headers, html).into_response()
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
    if tail.as_deref() == Some(tunnel::HANDSHAKE_TAIL) {
        return tunnel::handshake::respond(&app.tunnels, slug, &mut parts).await;
    }

    // 隧道在线时它说了算：开发者机器上跑着的才是此刻最新的东西。
    if let Some(session) = app.tunnels.get(slug) {
        let manifest = session.manifest();
        let ctx = Ctx::new(&parts, authority, slug, &app.config);
        return match tail {
            Some(tail) => reserved(app, &manifest, &ctx, &tail, parts, body).await,
            None if tunnel::wants_gate(
                &manifest,
                &parts.method,
                parts.uri.path(),
                ctx.navigation,
                ctx.has_gate_cookie,
            ) =>
            {
                gate_page(app, &manifest, &ctx, &parts, Some(tunnel::ONLINE_LABEL)).await
            }
            None => {
                tunnel::proxy::forward(
                    &session,
                    tunnel::proxy::Player {
                        authority,
                        public_scheme: &app.config.public_scheme,
                        navigation: ctx.navigation,
                        isolated: manifest.isolated,
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
    match tail {
        Some(tail) => reserved(app, &manifest, &ctx, &tail, parts, body).await,
        None => serve_file(app, &manifest, &ctx, &parts).await,
    }
}

/// 请求里跟着走、每个分支都要用的那些东西。
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
        Self {
            root_url: format!("{scheme}://{suffix}{port_part}/"),
            page_url: format!("{scheme}://{slug}.{suffix}{port_part}{}", parts.uri.path()),
            origin: format!("{scheme}://{slug}.{suffix}{port_part}"),
            visitor: Visitor {
                wechat: is_wechat(&ua),
                sid: sid.clone().unwrap_or_default(),
                ua,
                referer,
            },
            sid,
            has_gate_cookie: cookie_value(headers, GATE_COOKIE).is_some(),
            navigation: gate::is_navigation(
                header_str(headers, "sec-fetch-dest"),
                header_str(headers, "accept"),
            ),
            accept_encoding: paths::parse_accept_encoding(header_str(headers, "accept-encoding")),
        }
    }
}

// ---------------------------------------------------------------- 保留路径

/// `/_playtest/...` 后面那一段。作品目录里就算有同名文件也永远取不到这里。
fn reserved_tail(path: &str) -> Option<&str> {
    if path == RESERVED_PATH_PREFIX.trim_end_matches('/') {
        return Some("");
    }
    path.strip_prefix(RESERVED_PATH_PREFIX)
}

async fn reserved(
    app: &App,
    manifest: &Manifest,
    ctx: &Ctx,
    tail: &str,
    parts: axum::http::request::Parts,
    body: Body,
) -> Response {
    let isolated = Some((manifest.isolated, false));
    match tail {
        "start" if parts.method == Method::POST => start(app, manifest, ctx, parts, body).await,
        "start" => method_not_allowed("POST"),
        "report" if parts.method == Method::GET || parts.method == Method::HEAD => {
            page(StatusCode::OK, pages::report_form(), isolated)
        }
        "report" if parts.method == Method::POST => {
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
        "report" => method_not_allowed("GET, POST"),
        "me" => crate::me::respond(&app.config, manifest, ctx.sid.as_deref(), &parts.method),
        "sdk.js" => crate::sdk::respond(&parts.method, &parts.headers),
        "cover" if parts.method == Method::GET || parts.method == Method::HEAD => {
            cover(app, manifest, &parts).await
        }
        "cover" => method_not_allowed("GET, HEAD"),
        _ => page(StatusCode::NOT_FOUND, pages::file_not_found(), isolated),
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
    let Ok(path) = app.sites.store().blob_path(&cover.hash) else {
        return (StatusCode::NOT_FOUND, base_headers()).into_response();
    };
    let file = match tokio::fs::File::open(&path).await {
        Ok(f) => f,
        Err(err) => {
            tracing::warn!(slug = %manifest.slug, %err, "封面的 blob 打不开");
            return (StatusCode::NOT_FOUND, base_headers()).into_response();
        }
    };
    let total = file.metadata().await.map(|m| m.len()).unwrap_or(0);

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
    app.breaker.record(&manifest.slug, total);
    (
        StatusCode::OK,
        headers,
        Body::from_stream(ReaderStream::new(file)),
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
    let target = field(&form, "to")
        .or_else(|| parts.uri.query().and_then(|q| field(q, "to")))
        .unwrap_or_default();
    let location = same_origin_target(&target);
    // 玩家真正从哪来：门禁页把自己收到的 Referer 放在表单里带过来。这一下 POST 的 Referer
    // 永远是门禁页自己，没有信息量。只留一个 URL 形态的值，别的当没有。
    let came_from = field(&form, "from")
        .filter(|f| f.starts_with("http://") || f.starts_with("https://"))
        .unwrap_or_default();

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
        &cookie(GATE_COOKIE, "1", gate_max_age, secure),
    );

    let sid = match &ctx.sid {
        Some(existing) => existing.clone(),
        None => {
            let fresh = new_session_id();
            append(
                &mut headers,
                "set-cookie",
                &cookie(SESSION_COOKIE, &fresh, Some(SESSION_MAX_AGE), secure),
            );
            fresh
        }
    };

    let visitor = Visitor {
        sid,
        referer: came_from,
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
    manifest: &Manifest,
    ctx: &Ctx,
    parts: &axum::http::request::Parts,
) -> Response {
    if parts.method != Method::GET && parts.method != Method::HEAD {
        return method_not_allowed("GET, HEAD");
    }
    // 熔断判定在取文件之前，也在门禁页之前：这一小时的额度用完了就一个字节都不出
    // （DESIGN §4.8）。`/_playtest/report` 走的是另一条路，举报入口任何时候都开着。
    let verdict = app
        .breaker
        .check(&manifest.slug, breaker::limit_for(manifest));
    if !verdict.allowed {
        return tripped(app, manifest, ctx, verdict).await;
    }
    let isolated = Some((manifest.isolated, false));
    let Some(norm) = paths::normalize(parts.uri.path()) else {
        return page(StatusCode::NOT_FOUND, pages::file_not_found(), isolated);
    };

    match paths::resolve(manifest, &norm, ctx.accept_encoding, ctx.navigation) {
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
        Resolved::NotFound => page(StatusCode::NOT_FOUND, pages::file_not_found(), isolated),
        Resolved::File(served) => {
            if gate::should_show(
                manifest.gate,
                parts.uri.path(),
                served.is_html,
                ctx.navigation,
                ctx.has_gate_cookie,
            ) {
                gate_page(app, manifest, ctx, parts, None).await
            } else {
                blob(app, manifest, ctx, parts, &served).await
            }
        }
    }
}

/// `version_label` 是版本那个位置显示什么：上传路径传 `None`（显示 `v7`），
/// 隧道路径传「在线」——隧道没有版本这个概念（DESIGN §3.5）。
async fn gate_page(
    app: &App,
    manifest: &Manifest,
    ctx: &Ctx,
    parts: &axum::http::request::Parts,
    version_label: Option<&str>,
) -> Response {
    let to = same_origin_target(
        parts
            .uri
            .path_and_query()
            .map(|pq| pq.as_str())
            .unwrap_or("/"),
    );
    let html = GatePage {
        manifest,
        to: &to,
        host_suffix: &app.config.host_suffix,
        root_url: &ctx.root_url,
        page_url: &ctx.page_url,
        origin: &ctx.origin,
        wechat: ctx.visitor.wechat,
        version_label,
        referer: &ctx.visitor.referer,
    }
    .render();

    if parts.method == Method::GET {
        app.events
            .append(
                Kind::GateView,
                &manifest.slug,
                manifest.version,
                &ctx.visitor,
                None,
                None,
            )
            .await;
    }
    page(StatusCode::OK, html, Some((manifest.isolated, false)))
}

async fn blob(
    app: &App,
    manifest: &Manifest,
    ctx: &Ctx,
    parts: &axum::http::request::Parts,
    served: &Served,
) -> Response {
    let Ok(path) = app.sites.store().blob_path(&served.entry.hash) else {
        tracing::warn!(hash = %served.entry.hash, "清单里的哈希形态不对");
        return page(
            StatusCode::INTERNAL_SERVER_ERROR,
            pages::broken(),
            Some((manifest.isolated, false)),
        );
    };
    let mut file = match tokio::fs::File::open(&path).await {
        Ok(f) => f,
        Err(err) => {
            tracing::warn!(path = %path.display(), %err, "清单指向的 blob 打不开");
            return page(
                StatusCode::INTERNAL_SERVER_ERROR,
                pages::broken(),
                Some((manifest.isolated, false)),
            );
        }
    };
    let total = match file.metadata().await {
        Ok(meta) => meta.len(),
        Err(err) => {
            tracing::warn!(path = %path.display(), %err, "blob 读不到大小");
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
    if start > 0 {
        if let Err(err) = file.seek(SeekFrom::Start(start)).await {
            tracing::warn!(path = %path.display(), %err, "定位 Range 起点失败");
            return page(
                StatusCode::INTERNAL_SERVER_ERROR,
                pages::broken(),
                Some((manifest.isolated, false)),
            );
        }
    }
    // 流式回给玩家：一个 80 MB 的 Unity `.data` 不该先进我们的内存。
    let stream = ReaderStream::new(file.take(length));
    (status, headers, Body::from_stream(stream)).into_response()
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

fn cookie(name: &str, value: &str, max_age: Option<u64>, secure: bool) -> String {
    let mut out = format!("{name}={value}; Path=/; SameSite=Lax; HttpOnly");
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
fn same_origin_target(raw: &str) -> String {
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
fn field(raw: &str, key: &str) -> Option<String> {
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

    #[test]
    fn reserved_prefix_is_never_a_site_file() {
        assert_eq!(reserved_tail("/_playtest/start"), Some("start"));
        assert_eq!(reserved_tail("/_playtest/"), Some(""));
        assert_eq!(reserved_tail("/_playtest"), Some(""));
        assert_eq!(reserved_tail("/_playtest/deep/thing"), Some("deep/thing"));
        assert_eq!(reserved_tail("/index.html"), None);
        assert_eq!(reserved_tail("/_playtestx/start"), None);
    }

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
            cookie("pt_gate", "1", Some(86400), false),
            "pt_gate=1; Path=/; SameSite=Lax; HttpOnly; Max-Age=86400"
        );
        assert_eq!(
            cookie("pt_gate", "1", None, true),
            "pt_gate=1; Path=/; SameSite=Lax; HttpOnly; Secure"
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
