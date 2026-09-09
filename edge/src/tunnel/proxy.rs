//! 一次玩家请求怎么走完这条路（DESIGN §4.3）。
//!
//! ```text
//! 玩家 → 边缘 → yamux 开一条流 → HTTP/1.1 写进去 → 开发者的 127.0.0.1:<port>
//!                                                        ↓
//! 玩家 ← 边缘（只补几个响应头）←────── 响应流回来 ───────┘
//! ```
//!
//! 三条不能破的规矩：
//!
//! 1. **响应体一个字节不动**（DESIGN §3.7）。不注入、不改写 URL、不重新压缩。
//!    我们对响应的全部干预就是 [`game_headers::apply`] 那几个头——`.wasm` 的 MIME、
//!    跨源隔离——都是「不补游戏就跑不起来」的那几个。
//! 2. **`Host` 改写成 `localhost:<port>`**。Vite 6 之后默认只放行 localhost，
//!    带着 `xxx.playtest.run` 进去直接 403；Next、webpack-dev-server 同理。
//!    原值放 `X-Forwarded-Host`，开发者要拼绝对链接时用得上。
//! 3. **不加 `X-Forwarded-For`**。我们不收集玩家 IP（DESIGN §3.4），也就没有 IP 可转。
//!
//! 上游回 101 之后这里就不再理解 HTTP 了，两端裸着对拷——socket.io 从 polling
//! 升到 websocket、Vite 的 HMR 都靠这一段。

use std::pin::Pin;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;

use axum::body::Body;
use axum::http::request::Parts;
use axum::http::{header, HeaderMap, HeaderName, HeaderValue, Request, StatusCode};
use axum::response::{IntoResponse, Response};
use bytes::Bytes;
use http_body_util::BodyExt;
use hyper::body::{Body as HttpBody, Frame, Incoming, SizeHint};
use hyper::client::conn::http1::SendRequest;
use hyper_util::rt::TokioIo;
use playtest_common::tunnel::HEADER_FORWARDED_HOST;

use crate::game_headers::{self, Source};
use crate::tunnel::offline;
use crate::tunnel::registry::Session;

/// 开一条 yamux 流最多等这么久。开流只是一个来回，等这么久还没开出来说明连接已经不行了。
const OPEN_TIMEOUT: Duration = Duration::from_secs(10);

/// 上游给出响应头最多等这么久。**只管到响应头**：响应体可以慢慢流，
/// 一个 80 MB 的 wasm 走家宽上行本来就要几分钟。
const HEAD_TIMEOUT: Duration = Duration::from_secs(30);

/// 转发这一次请求时，除了会话之外还要知道的东西。
pub struct Player<'a> {
    /// 玩家地址栏里的那个 Host，原样放进 `X-Forwarded-Host`。
    pub authority: &'a str,
    /// 玩家看到的协议。边缘自己永远只听明文，TLS 在前面的 Caddy 上。
    pub public_scheme: &'a str,
    /// 这是一次顶层文档导航。决定要不要给 CORP（顶层文档不给）。
    pub navigation: bool,
    pub isolated: bool,
}

pub async fn forward(
    session: &Arc<Session>,
    player: Player<'_>,
    mut parts: Parts,
    body: Body,
) -> Response {
    let Some(guard) = Guard::acquire(session) else {
        tracing::info!(
            slug = %session.slug(),
            max_players = session.claims.max_players,
            "同时在场的人到上限了"
        );
        return crate::tunnel::page(
            StatusCode::SERVICE_UNAVAILABLE,
            offline::busy(),
            player.isolated,
        );
    };

    let stream = match tokio::time::timeout(OPEN_TIMEOUT, session.mux.open()).await {
        Ok(Ok(stream)) => stream,
        Ok(Err(err)) => {
            tracing::warn!(slug = %session.slug(), %err, "隧道上开不出流");
            return unreachable(&player);
        }
        Err(_) => {
            tracing::warn!(slug = %session.slug(), "隧道上开一条流等了 {OPEN_TIMEOUT:?} 还没开出来");
            return unreachable(&player);
        }
    };

    let (mut sender, conn) = match hyper::client::conn::http1::handshake(TokioIo::new(stream)).await
    {
        Ok(pair) => pair,
        Err(err) => {
            tracing::warn!(slug = %session.slug(), %err, "隧道流上起不了 HTTP 连接");
            return unreachable(&player);
        }
    };
    // 这条流上的读写全靠这个任务推进；`with_upgrades` 是 101 之后能把裸连接拿回来的前提。
    tokio::spawn(async move {
        if let Err(err) = conn.with_upgrades().await {
            tracing::debug!(%err, "隧道上的这条 HTTP 连接结束得不干净");
        }
    });

    // 玩家那一侧的升级句柄要在这里就拿走：等我们把 101 写回去之后它才会完成，
    // 但那时候 `parts` 已经不在手上了。
    let player_upgrade = parts.extensions.remove::<hyper::upgrade::OnUpgrade>();
    let request = build_request(session, &player, &parts, body);

    let upstream = match tokio::time::timeout(HEAD_TIMEOUT, sender.send_request(request)).await {
        Ok(Ok(response)) => response,
        Ok(Err(err)) => {
            tracing::warn!(slug = %session.slug(), %err, "开发者那边没有把请求答完");
            return unreachable(&player);
        }
        Err(_) => {
            tracing::warn!(slug = %session.slug(), "开发者那边 {HEAD_TIMEOUT:?} 没给响应头");
            return crate::tunnel::page(
                StatusCode::GATEWAY_TIMEOUT,
                offline::timed_out(),
                player.isolated,
            );
        }
    };
    session.requests.fetch_add(1, Ordering::Relaxed);

    if upstream.status() == StatusCode::SWITCHING_PROTOCOLS {
        let Some(player_upgrade) = player_upgrade else {
            // 玩家没要升级，上游却自己 101 了。把这个原样转给浏览器只会让它挂在那里。
            tracing::warn!(slug = %session.slug(), "上游回了 101，但玩家这一侧没有可升级的连接");
            return unreachable(&player);
        };
        return switch_protocols(session, guard, player_upgrade, upstream);
    }

    let path = parts.uri.path().to_string();
    let (mut head, incoming) = upstream.into_parts();
    strip_hop_by_hop(&mut head.headers);
    // 上游是开发者的 dev server，它多半什么都没配。补的只有「不补就跑不起来」的那几个，
    // 别的（包括 `Content-Encoding`）一律它说了算——猜错一个头比少补一个头难查得多。
    game_headers::apply(
        &mut head.headers,
        &path,
        player.isolated,
        Source::Upstream,
        game_headers::Opts {
            resource: !player.navigation,
            ..game_headers::Opts::default()
        },
    );

    let body = Body::new(Streaming {
        inner: incoming,
        session: session.clone(),
        _guard: guard,
        _sender: sender,
    });
    (head.status, head.headers, body).into_response()
}

/// 上游同意升级了。101 和它的头原样回给玩家，两端从此只是两根管子。
fn switch_protocols(
    session: &Arc<Session>,
    guard: Guard,
    player_upgrade: hyper::upgrade::OnUpgrade,
    mut upstream: hyper::Response<Incoming>,
) -> Response {
    // 101 的头一个字都不改：`Sec-WebSocket-Accept` 是上游按玩家的 key 算出来的，
    // `Sec-WebSocket-Protocol` / `Sec-WebSocket-Extensions` 是两端刚谈好的结果。
    let headers = std::mem::take(upstream.headers_mut());
    let upstream_upgrade = hyper::upgrade::on(&mut upstream);
    let session = session.clone();

    tokio::spawn(async move {
        let _guard = guard;
        let (player, upstream) = match tokio::try_join!(player_upgrade, upstream_upgrade) {
            Ok(pair) => pair,
            Err(err) => {
                tracing::debug!(slug = %session.slug(), %err, "101 之后有一端没升级起来");
                return;
            }
        };
        let (mut player, mut upstream) = (TokioIo::new(player), TokioIo::new(upstream));
        match tokio::io::copy_bidirectional(&mut player, &mut upstream).await {
            Ok((to_upstream, to_player)) => {
                session.bytes_in.fetch_add(to_upstream, Ordering::Relaxed);
                session.bytes_out.fetch_add(to_player, Ordering::Relaxed);
            }
            // 玩家关标签页、开发者重启 dev server 都走这里，是常态不是故障。
            Err(err) => tracing::debug!(slug = %session.slug(), %err, "101 之后的双向拷贝断了"),
        }
    });

    (StatusCode::SWITCHING_PROTOCOLS, headers).into_response()
}

fn unreachable(player: &Player<'_>) -> Response {
    crate::tunnel::page(
        StatusCode::BAD_GATEWAY,
        offline::unreachable(),
        player.isolated,
    )
}

fn build_request(
    session: &Arc<Session>,
    player: &Player<'_>,
    parts: &Parts,
    body: Body,
) -> Request<Body> {
    // 方法和 path+query 原样。HTTP/1.1 走 origin-form，本来就只带路径。
    let target = parts
        .uri
        .path_and_query()
        .map(|pq| pq.as_str())
        .unwrap_or("/");
    let mut request = Request::builder()
        .method(parts.method.clone())
        .uri(target)
        .body(counted_body(session, body))
        .expect("方法和路径都是从一个已经解析过的请求上抄来的");

    let headers = request.headers_mut();
    let upgrade = wants_upgrade(&parts.headers);
    for (name, value) in parts.headers.iter() {
        // 同名头可能有好几条（Cookie、Accept-Encoding），`append` 一条都不丢。
        if !is_hop_by_hop(name) && name != header::HOST {
            headers.append(name.clone(), value.clone());
        }
    }
    if upgrade {
        // `Connection` 本身是逐跳头，但只留下 `Upgrade` 而不说 `Connection: upgrade`，
        // 上游就不会把这当成升级请求——socket.io 和 Vite 的 HMR 都会卡在这里。
        headers.insert(header::CONNECTION, HeaderValue::from_static("upgrade"));
    }
    put(
        headers,
        header::HOST.as_str(),
        &format!("localhost:{}", session.local_port),
    );
    put(headers, HEADER_FORWARDED_HOST, player.authority);
    put(headers, "x-forwarded-proto", player.public_scheme);
    // 这里**没有** X-Forwarded-For：玩家 IP 我们不收集，也就没有 IP 可以往开发者那里送。
    request
}

/// 请求体流式往上游送，顺便数一下玩家发过来多少字节。
fn counted_body(session: &Arc<Session>, body: Body) -> Body {
    let session = session.clone();
    Body::new(body.map_frame(move |frame| {
        if let Some(data) = frame.data_ref() {
            session
                .bytes_in
                .fetch_add(data.len() as u64, Ordering::Relaxed);
        }
        frame
    }))
}

/// 响应体，外加几样必须活到「玩家收完最后一个字节」的东西。
///
/// `guard` 是并发名额，`sender` 是上游那条连接的发送端——两样都得跟着响应体走，
/// 提前丢掉，一个会让名额漏回去，另一个会让还在流的响应半路断掉。
struct Streaming {
    inner: Incoming,
    session: Arc<Session>,
    _guard: Guard,
    _sender: SendRequest<Body>,
}

impl HttpBody for Streaming {
    type Data = Bytes;
    type Error = hyper::Error;

    fn poll_frame(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Bytes>, hyper::Error>>> {
        let this = self.get_mut();
        let polled = Pin::new(&mut this.inner).poll_frame(cx);
        if let Poll::Ready(Some(Ok(frame))) = &polled {
            if let Some(data) = frame.data_ref() {
                this.session
                    .bytes_out
                    .fetch_add(data.len() as u64, Ordering::Relaxed);
            }
        }
        polled
    }

    fn is_end_stream(&self) -> bool {
        self.inner.is_end_stream()
    }

    fn size_hint(&self) -> SizeHint {
        self.inner.size_hint()
    }
}

/// 同时能有多少个请求在这条隧道上跑。拿到名额才准开流，随响应一起还回去。
struct Guard(Arc<Session>);

impl Guard {
    fn acquire(session: &Arc<Session>) -> Option<Self> {
        // 档位里写 0 是配置错误，但那会让这个作品一个请求都进不来，比放行一个糟。
        let max = session.claims.max_players.max(1);
        let mut seen = session.open_streams.load(Ordering::Relaxed);
        loop {
            if seen >= max {
                return None;
            }
            // 先读再加会让同时进来的两个玩家都看到「还差一个」，两个都放行。
            match session.open_streams.compare_exchange_weak(
                seen,
                seen + 1,
                Ordering::AcqRel,
                Ordering::Relaxed,
            ) {
                Ok(_) => return Some(Self(session.clone())),
                Err(actual) => seen = actual,
            }
        }
    }
}

impl Drop for Guard {
    fn drop(&mut self) {
        self.0.open_streams.fetch_sub(1, Ordering::AcqRel);
    }
}

/// 只在这一跳有意义的头，不往下一跳转（RFC 9110 §7.6.1）。
///
/// `Upgrade` **不在**这张表里：WebSocket 要靠它过去。`Connection` 在表里，
/// 但升级请求会在上面被重新写成 `Connection: upgrade`。
const HOP_BY_HOP: &[&str] = &[
    "connection",
    "keep-alive",
    "transfer-encoding",
    "te",
    "trailer",
    "trailers",
];

fn is_hop_by_hop(name: &HeaderName) -> bool {
    let name = name.as_str();
    name.starts_with("proxy-") || HOP_BY_HOP.contains(&name)
}

fn strip_hop_by_hop(headers: &mut HeaderMap) {
    let doomed: Vec<HeaderName> = headers
        .keys()
        .filter(|name| is_hop_by_hop(name))
        .cloned()
        .collect();
    for name in doomed {
        headers.remove(name);
    }
}

fn wants_upgrade(headers: &HeaderMap) -> bool {
    headers.contains_key(header::UPGRADE)
        && headers.get_all(header::CONNECTION).iter().any(|raw| {
            raw.to_str().is_ok_and(|line| {
                line.split(',')
                    .any(|item| item.trim().eq_ignore_ascii_case("upgrade"))
            })
        })
}

fn put(headers: &mut HeaderMap, name: &str, value: &str) {
    match (HeaderName::try_from(name), HeaderValue::from_str(value)) {
        (Ok(name), Ok(value)) => {
            headers.insert(name, value);
        }
        _ => tracing::warn!(name, value, "转发时这个请求头的值不合法，丢掉"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use playtest_common::manifest::GateMode;
    use playtest_common::tunnel::io::{ActivityClock, Mux, Role};
    use playtest_common::tunnel::Claims;

    fn session(local_port: u16) -> (Arc<Session>, tokio::io::DuplexStream) {
        let claims = Claims {
            v: 1,
            slug: "brisk-otter-41".into(),
            sub: "u".into(),
            title: "小球".into(),
            developer: "某某".into(),
            badge: true,
            gate: GateMode::Once,
            isolated: false,
            max_players: 2,
            hybrid: false,
            iat: 0,
            exp: 0,
            jti: "j".into(),
        };
        let (ours, peer) = tokio::io::duplex(1024);
        let session = Session::new(
            claims,
            local_port,
            Mux::spawn(ours, Role::Opener),
            ActivityClock::new(),
        );
        (Arc::new(session), peer)
    }

    fn parts_of(request: Request<()>) -> Parts {
        request.into_parts().0
    }

    fn player<'a>() -> Player<'a> {
        Player {
            authority: "brisk-otter-41.localhost:8443",
            public_scheme: "http",
            navigation: true,
            isolated: false,
        }
    }

    #[tokio::test]
    async fn the_host_is_rewritten_and_hop_by_hop_headers_are_dropped() {
        let (session, _peer) = session(5173);
        let incoming = Request::builder()
            .uri("/level/3?hard=1")
            .header("host", "brisk-otter-41.localhost:8443")
            .header("cookie", "pt_gate=1")
            .header("cookie", "pt_sid=abc")
            .header("accept-encoding", "br, gzip")
            .header("connection", "keep-alive")
            .header("keep-alive", "timeout=5")
            .header("transfer-encoding", "chunked")
            .header("te", "trailers")
            .header("trailer", "X-Checksum")
            .header("proxy-authorization", "Basic 偷看")
            .body(())
            .unwrap();

        let out = build_request(&session, &player(), &parts_of(incoming), Body::empty());

        // Vite 6 之后只放行 localhost，Host 必须换掉；原值转给开发者。
        assert_eq!(out.headers()["host"], "localhost:5173");
        assert_eq!(
            out.headers()[HEADER_FORWARDED_HOST],
            "brisk-otter-41.localhost:8443"
        );
        assert_eq!(out.headers()["x-forwarded-proto"], "http");
        // 玩家 IP 我们不收集，所以没有这个头（DESIGN §3.4）。
        assert!(!out.headers().contains_key("x-forwarded-for"));

        // 逐跳头一个都不往下走。
        for gone in [
            "connection",
            "keep-alive",
            "transfer-encoding",
            "te",
            "trailer",
            "proxy-authorization",
        ] {
            assert!(!out.headers().contains_key(gone), "{gone} 不该转过去");
        }

        // 别的原样，同名的一条不少。
        assert_eq!(out.headers()["accept-encoding"], "br, gzip");
        let cookies: Vec<_> = out.headers().get_all("cookie").iter().collect();
        assert_eq!(cookies.len(), 2);

        // 方法和 path+query 原样。
        assert_eq!(out.method(), axum::http::Method::GET);
        assert_eq!(out.uri().path_and_query().unwrap(), "/level/3?hard=1");
    }

    #[tokio::test]
    async fn an_upgrade_request_keeps_the_two_headers_that_make_it_one() {
        let (session, _peer) = session(3000);
        let incoming = Request::builder()
            .uri("/socket.io/?EIO=4&transport=websocket")
            .header("host", "brisk-otter-41.localhost:8443")
            .header("connection", "keep-alive, Upgrade")
            .header("upgrade", "websocket")
            .header("sec-websocket-key", "dGhlIHNhbXBsZSBub25jZQ==")
            .header("sec-websocket-version", "13")
            .body(())
            .unwrap();

        let out = build_request(&session, &player(), &parts_of(incoming), Body::empty());
        assert_eq!(out.headers()["upgrade"], "websocket");
        // 留了 Upgrade 却不说 Connection: upgrade，上游不会当成升级请求。
        assert_eq!(out.headers()["connection"], "upgrade");
        assert_eq!(
            out.headers()["sec-websocket-key"],
            "dGhlIHNhbXBsZSBub25jZQ=="
        );
    }

    #[test]
    fn upstream_hop_by_hop_headers_do_not_reach_the_player() {
        let mut headers = HeaderMap::new();
        headers.insert("content-type", HeaderValue::from_static("text/html"));
        headers.insert("connection", HeaderValue::from_static("keep-alive"));
        headers.insert("keep-alive", HeaderValue::from_static("timeout=5"));
        headers.insert("transfer-encoding", HeaderValue::from_static("chunked"));
        headers.insert("proxy-connection", HeaderValue::from_static("keep-alive"));
        headers.insert("content-encoding", HeaderValue::from_static("gzip"));

        strip_hop_by_hop(&mut headers);

        assert_eq!(headers["content-type"], "text/html");
        // 体是上游压的，这个头一个字都不动（DESIGN §3.7）。
        assert_eq!(headers["content-encoding"], "gzip");
        assert_eq!(headers.len(), 2);
    }

    #[test]
    fn what_counts_as_an_upgrade_request() {
        let mut headers = HeaderMap::new();
        assert!(!wants_upgrade(&headers));
        headers.insert("upgrade", HeaderValue::from_static("websocket"));
        // 光有 Upgrade 不算，`Connection` 得点名它。
        assert!(!wants_upgrade(&headers));
        headers.insert(
            "connection",
            HeaderValue::from_static("keep-alive, Upgrade"),
        );
        assert!(wants_upgrade(&headers));
    }

    #[tokio::test]
    async fn the_concurrency_limit_is_the_tier_quota() {
        let (session, _peer) = session(5173);
        assert_eq!(session.claims.max_players, 2);

        let first = Guard::acquire(&session).expect("第一个玩家进得来");
        let second = Guard::acquire(&session).expect("第二个玩家进得来");
        assert_eq!(session.open_streams.load(Ordering::Relaxed), 2);
        assert!(Guard::acquire(&session).is_none(), "第三个该被挡在外面");

        drop(first);
        assert_eq!(session.open_streams.load(Ordering::Relaxed), 1);
        let third = Guard::acquire(&session).expect("有人走了就该腾出名额");
        drop((second, third));
        assert_eq!(session.open_streams.load(Ordering::Relaxed), 0);
    }
}
