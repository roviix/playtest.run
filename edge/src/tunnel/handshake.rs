//! `GET /_playtest/tunnel`：CLI 把一条出站 WebSocket 变成这个作品的隧道。
//!
//! 不走 axum 的 `ws` feature，自己拼 101。理由是这条握手要做的事和「收发消息」无关：
//! 校验顺序要能一条条说清楚、失败要回 [`ErrorBody`] 的 JSON 而不是一个裸状态码、
//! 升级之后拿到的是**一条字节流**（喂给 yamux），不是一个消息流。自己做这三件事都更直接。
//!
//! 校验从「这像不像一次 WebSocket 握手」开始，一路到「这个令牌现在还算不算数」，
//! 每一步的失败都有自己的码，CLI 据此决定是换令牌重连、退避重连，还是干脆退出：
//!
//! | 情况 | 回什么 |
//! |---|---|
//! | 不是 WebSocket 握手 / 协议名不对 / 端口不对 | 400 `invalid` |
//! | 没带令牌、验不过、令牌不是给这个 slug 的 | 401 `unauthorized` |
//! | 令牌过期 | 401 `token_expired`，CLI 换一个再来 |
//! | 这个 `jti` 被挤掉过 | 409 `tunnel_replaced`，CLI 退出，不重试 |
//! | 边缘手上没有验签公钥 | 503 `internal`，这是我们的问题不是 CLI 的 |

use std::sync::Arc;

use axum::body::Body;
use axum::http::request::Parts;
use axum::http::{header, HeaderMap, Method, Response, StatusCode};
use hyper_util::rt::TokioIo;
use playtest_common::api::ErrorCode;
use playtest_common::tunnel::io::{Mux, Role, WsByteStream};
use playtest_common::tunnel::{Claims, TokenError, HEADER_LOCAL_PORT, WS_PROTOCOL};
use time::OffsetDateTime;
use tokio_tungstenite::tungstenite::handshake::derive_accept_key;
use tokio_tungstenite::tungstenite::protocol::Role as WsRole;
use tokio_tungstenite::WebSocketStream;

use crate::tunnel::keys;
use crate::tunnel::registry::{Session, Tunnels};

/// `slug` 是 Host 里那一段，令牌必须和它对得上——不然拿着 A 的令牌就能占住 B 的域名。
pub async fn respond(
    tunnels: &Arc<Tunnels>,
    slug: &str,
    parts: &mut Parts,
) -> axum::response::Response {
    let headers = &parts.headers;

    if parts.method != Method::GET
        || !has_token(headers, header::UPGRADE.as_str(), "websocket")
        || value(headers, header::SEC_WEBSOCKET_VERSION.as_str()) != Some("13")
        || !headers.contains_key(header::SEC_WEBSOCKET_KEY)
    {
        return invalid(
            "这个地址只接受 WebSocket 握手：GET、Upgrade: websocket、Sec-WebSocket-Version: 13、Sec-WebSocket-Key",
        );
    }
    if !has_token(
        headers,
        header::SEC_WEBSOCKET_PROTOCOL.as_str(),
        WS_PROTOCOL,
    ) {
        return invalid(&format!("握手要带 Sec-WebSocket-Protocol: {WS_PROTOCOL}"));
    }

    let Some(token) = bearer(headers) else {
        return unauthorized("缺少 Authorization: Bearer <令牌>");
    };

    // 公钥可能在边缘启动之后才出现（同一台机器上 api 后起），所以每次握手都问一次。
    let Some(key) = tunnels.keys().current().await else {
        tracing::warn!(
            path = %tunnels.keys().path().display(),
            "没有隧道验签公钥：环境变量 {} 没设，对象存储里那份也读不到，所有握手都会被拒",
            keys::ENV
        );
        return crate::tunnel::error(
            StatusCode::SERVICE_UNAVAILABLE,
            ErrorCode::Internal,
            "边缘还没拿到验签公钥",
        );
    };

    let now = OffsetDateTime::now_utc().unix_timestamp();
    let claims = match key.verify(token, now) {
        Ok(claims) => claims,
        Err(TokenError::Expired) => {
            return crate::tunnel::error(
                StatusCode::UNAUTHORIZED,
                ErrorCode::TokenExpired,
                "令牌已经过期，换一个新的再连",
            )
        }
        Err(err) => {
            tracing::debug!(slug, %err, "隧道令牌验不过");
            return unauthorized("令牌验不过");
        }
    };

    if claims.slug != slug {
        tracing::warn!(slug, token_slug = %claims.slug, "令牌里的 slug 和 Host 对不上");
        return unauthorized(&format!("这个令牌不是给 {slug} 的"));
    }
    if tunnels.is_evicted(&claims.jti) {
        return crate::tunnel::error(
            StatusCode::CONFLICT,
            ErrorCode::TunnelReplaced,
            "另一个 playtest 进程已经接管了这个作品，这个令牌不会再被接受；\
             关掉那一个，或者换一个新令牌再连",
        );
    }

    let Some(local_port) = local_port(headers) else {
        return invalid(&format!(
            "{HEADER_LOCAL_PORT} 缺了，或者它不是 1 到 65535 之间的端口号"
        ));
    };

    // hyper 只在 HTTP/1.1 且带 `Upgrade` 头时才把这个扩展塞进来。拿走它就是接下这条连接。
    let Some(on_upgrade) = parts.extensions.remove::<hyper::upgrade::OnUpgrade>() else {
        return invalid("这条连接升级不了：要 HTTP/1.1，并且带上 Connection: Upgrade");
    };
    let accept = match headers.get(header::SEC_WEBSOCKET_KEY) {
        Some(key) => derive_accept_key(key.as_bytes()),
        None => return invalid("缺少 Sec-WebSocket-Key"),
    };

    spawn_session(tunnels.clone(), claims, local_port, on_upgrade);

    Response::builder()
        .status(StatusCode::SWITCHING_PROTOCOLS)
        .header(header::CONNECTION, "Upgrade")
        .header(header::UPGRADE, "websocket")
        .header(header::SEC_WEBSOCKET_ACCEPT, accept)
        .header(header::SEC_WEBSOCKET_PROTOCOL, WS_PROTOCOL)
        .body(Body::empty())
        .expect("101 的头都是常量，拼得出来")
}

/// 101 写出去之后 hyper 才会把连接交出来，所以这一段只能在另一个任务里等。
fn spawn_session(
    tunnels: Arc<Tunnels>,
    claims: Claims,
    local_port: u16,
    on_upgrade: hyper::upgrade::OnUpgrade,
) {
    tokio::spawn(async move {
        let upgraded = match on_upgrade.await {
            Ok(upgraded) => upgraded,
            Err(err) => {
                tracing::warn!(slug = %claims.slug, %err, "隧道连接升级失败");
                return;
            }
        };
        // 握手是我们自己做的，所以这里从「已经升级完的裸连接」接手，
        // 角色写死 Server——掩码规则和帧的方向都由它决定。
        let ws =
            WebSocketStream::from_raw_socket(TokioIo::new(upgraded), WsRole::Server, None).await;
        let stream = WsByteStream::new(ws);
        // 活动时钟与控制句柄要先复制出来：`stream` 马上就被 yamux 拿走，再也摸不到了。
        let activity = stream.activity();
        let control = stream.control();
        // 边缘是开流的那一头：每个玩家请求一条流（`common` 的 `Role` 文档）。
        let mux = Mux::spawn(stream, Role::Opener);
        tunnels.register(Arc::new(
            Session::new(claims, local_port, mux, activity).with_control(control),
        ));
    });
}

fn invalid(message: &str) -> axum::response::Response {
    crate::tunnel::error(StatusCode::BAD_REQUEST, ErrorCode::Invalid, message)
}

fn unauthorized(message: &str) -> axum::response::Response {
    crate::tunnel::error(StatusCode::UNAUTHORIZED, ErrorCode::Unauthorized, message)
}

fn value<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers.get(name).and_then(|v| v.to_str().ok())
}

/// 这个头的值里有没有这个词。`Upgrade` 和 `Sec-WebSocket-Protocol` 都可以是逗号分隔的列表，
/// 而且可以重复出现，所以逐个值再逐段看。词本身大小写不敏感（RFC 6455）。
fn has_token(headers: &HeaderMap, name: &str, wanted: &str) -> bool {
    headers.get_all(name).iter().any(|raw| {
        raw.to_str().is_ok_and(|line| {
            line.split(',')
                .any(|item| item.trim().eq_ignore_ascii_case(wanted))
        })
    })
}

fn bearer(headers: &HeaderMap) -> Option<&str> {
    let raw = value(headers, header::AUTHORIZATION.as_str())?;
    let (scheme, token) = raw.split_once(' ')?;
    if !scheme.eq_ignore_ascii_case("Bearer") {
        return None;
    }
    let token = token.trim();
    (!token.is_empty()).then_some(token)
}

fn local_port(headers: &HeaderMap) -> Option<u16> {
    let raw = value(headers, HEADER_LOCAL_PORT)?.trim();
    // 0 不是一个能连的端口，`parse::<u16>()` 却收下它。
    raw.parse::<u16>().ok().filter(|p| *p > 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    fn headers(pairs: &[(&str, &str)]) -> HeaderMap {
        let mut headers = HeaderMap::new();
        for (name, value) in pairs {
            headers.append(
                axum::http::HeaderName::from_bytes(name.as_bytes()).unwrap(),
                HeaderValue::from_str(value).unwrap(),
            );
        }
        headers
    }

    #[test]
    fn tokens_in_a_header_are_matched_case_insensitively_across_a_list() {
        let h = headers(&[("upgrade", "WebSocket")]);
        assert!(has_token(&h, "upgrade", "websocket"));

        // 列表形式，以及同名头出现两次。
        let h = headers(&[
            ("sec-websocket-protocol", "chat, playtest-tunnel-v1"),
            ("sec-websocket-protocol", "superchat"),
        ]);
        assert!(has_token(&h, "sec-websocket-protocol", WS_PROTOCOL));
        assert!(!has_token(
            &h,
            "sec-websocket-protocol",
            "playtest-tunnel-v2"
        ));

        // 子串不算。
        assert!(!has_token(
            &headers(&[("upgrade", "websockets-plus")]),
            "upgrade",
            "websocket"
        ));
        assert!(!has_token(&HeaderMap::new(), "upgrade", "websocket"));
    }

    #[test]
    fn only_a_bearer_token_counts() {
        assert_eq!(
            bearer(&headers(&[("authorization", "Bearer pt1.a.b")])),
            Some("pt1.a.b")
        );
        // 方案名大小写不敏感（RFC 7235）。
        assert_eq!(
            bearer(&headers(&[("authorization", "bearer pt1.a.b")])),
            Some("pt1.a.b")
        );
        assert_eq!(bearer(&headers(&[("authorization", "Basic abc")])), None);
        assert_eq!(bearer(&headers(&[("authorization", "Bearer   ")])), None);
        assert_eq!(bearer(&headers(&[("authorization", "pt1.a.b")])), None);
        assert_eq!(bearer(&HeaderMap::new()), None);
    }

    #[test]
    fn the_local_port_must_be_a_port() {
        assert_eq!(
            local_port(&headers(&[(HEADER_LOCAL_PORT, "5173")])),
            Some(5173)
        );
        assert_eq!(
            local_port(&headers(&[(HEADER_LOCAL_PORT, " 80 ")])),
            Some(80)
        );
        assert_eq!(
            local_port(&headers(&[(HEADER_LOCAL_PORT, "65535")])),
            Some(65535)
        );
        for bad in ["0", "65536", "-1", "5173x", "", "八千"] {
            assert_eq!(
                local_port(&headers(&[(HEADER_LOCAL_PORT, bad)])),
                None,
                "{bad}"
            );
        }
        assert_eq!(local_port(&HeaderMap::new()), None);
    }
}
