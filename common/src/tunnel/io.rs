//! WebSocket ↔ yamux 的字节流适配与连接驾驭（feature `tunnel-io`，DESIGN §4.3）。
//!
//! 隧道两端共用这一层，上面的策略各自写：
//!
//! - [`WsByteStream`] 把一条 WebSocket 当字节流用——每个 Binary 帧是流里的一段，Ping 自动回 Pong，
//!   Close 或断开就是 EOF。两端底层类型不同（边缘是 axum 升级出来的连接，CLI 是 TLS 或明文 TCP），
//!   所以对底层泛型。它还能自己发保活 Ping（[`WsByteStream::with_keepalive`]）。
//! - [`WsControl`] 是这条流的遥控器，交给 [`Mux`] 之前先克隆一份留在手上：之后想带状态码关掉它
//!   （边缘挤掉旧隧道）、或者想知道对端是用什么状态码关的（CLI 区分被挤掉还是普通断线），都靠它。
//! - [`Mux`] 在那条字节流上跑 yamux：边缘 [`Role::Opener`] 每个玩家连接开一条流，
//!   CLI [`Role::Acceptor`] 收到流就去连 `127.0.0.1:<port>`。
//! - [`handshake_request`] 给 CLI 拼一个 `tokio_tungstenite::connect_async` 能直接用的握手请求。
//!
//! 这一层只管字节。令牌怎么签、怎么验在 [`super`]；按 Host 找会话、改写 `Host`、补游戏响应头
//! 是边缘的事；重连退避是 CLI 的事。

mod mux;
mod ws;

pub use mux::{Mux, MuxError, MuxStream, Role};
pub use ws::{ActivityClock, WsByteStream, WsControl, MAX_FRAME_BYTES};

use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::{header, HeaderName, HeaderValue, Request, Uri};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum HandshakeError {
    #[error("握手地址不对（{0}），应该像 wss://brisk-otter-41.playtest.run/_playtest/tunnel")]
    BadUrl(String),
    #[error("令牌里有不能放进 HTTP 头的字符")]
    BadToken,
}

/// 拼出 CLI 连边缘用的握手请求。
///
/// `connect_url` 用 [`crate::tunnel::TunnelGrant::connect_url`] 那一份。
/// `local_port` 是开发者本机的端口，边缘据此把 `Host` 改写成 `localhost:<port>`。
pub fn handshake_request(
    connect_url: &str,
    token: &str,
    local_port: u16,
) -> Result<Request<()>, HandshakeError> {
    let uri: Uri = connect_url
        .parse()
        .map_err(|e| HandshakeError::BadUrl(format!("{connect_url}：{e}")))?;
    match uri.scheme_str() {
        Some("ws") | Some("wss") => {}
        other => {
            return Err(HandshakeError::BadUrl(format!(
                "协议是 {}，只认 ws 和 wss",
                other.unwrap_or("空")
            )))
        }
    }

    // Host / Upgrade / Sec-WebSocket-Key 这几个头必须一字不差，交给 tungstenite 自己生成，
    // 我们只往上补 playtest 自己的。
    let mut request = uri
        .into_client_request()
        .map_err(|e| HandshakeError::BadUrl(e.to_string()))?;
    let headers = request.headers_mut();
    headers.insert(
        header::AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {token}")).map_err(|_| HandshakeError::BadToken)?,
    );
    headers.insert(
        HeaderName::from_static(crate::tunnel::HEADER_LOCAL_PORT),
        HeaderValue::from(local_port),
    );
    headers.insert(
        header::SEC_WEBSOCKET_PROTOCOL,
        HeaderValue::from_static(crate::tunnel::WS_PROTOCOL),
    );
    Ok(request)
}

#[cfg(test)]
mod test_support {
    use super::WsByteStream;
    use tokio::io::DuplexStream;
    use tokio_tungstenite::tungstenite::protocol::Role as WsRole;
    use tokio_tungstenite::WebSocketStream;

    /// 用一对内存管道把两个 WebSocket 端点直接接起来，不用真的监听端口。
    /// 一端 Client 一端 Server，掩码规则才对得上。
    pub(super) async fn raw_pair() -> (WebSocketStream<DuplexStream>, WebSocketStream<DuplexStream>)
    {
        let (a, b) = tokio::io::duplex(256 * 1024);
        (
            WebSocketStream::from_raw_socket(a, WsRole::Client, None).await,
            WebSocketStream::from_raw_socket(b, WsRole::Server, None).await,
        )
    }

    pub(super) async fn ws_pair() -> (WsByteStream<DuplexStream>, WsByteStream<DuplexStream>) {
        let (a, b) = raw_pair().await;
        (WsByteStream::new(a), WsByteStream::new(b))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tunnel;

    #[test]
    fn handshake_request_carries_token_port_and_protocol() {
        let url = format!("wss://brisk-otter-41.playtest.run{}", tunnel::WS_PATH);
        let request = handshake_request(&url, "pt1.claims.sig", 5173).unwrap();
        let headers = request.headers();

        assert_eq!(headers[header::AUTHORIZATION], "Bearer pt1.claims.sig");
        assert_eq!(headers[tunnel::HEADER_LOCAL_PORT], "5173");
        assert_eq!(headers[header::SEC_WEBSOCKET_PROTOCOL], tunnel::WS_PROTOCOL);

        // 这几个是 tungstenite 自己要用的，少一个 connect_async 就不认。
        assert_eq!(headers[header::HOST], "brisk-otter-41.playtest.run");
        assert_eq!(headers[header::UPGRADE], "websocket");
        assert_eq!(headers[header::CONNECTION], "Upgrade");
        assert_eq!(headers[header::SEC_WEBSOCKET_VERSION], "13");
        assert!(headers.contains_key(header::SEC_WEBSOCKET_KEY));

        assert_eq!(request.uri().path(), tunnel::WS_PATH);
    }

    #[test]
    fn handshake_request_rejects_urls_that_are_not_websockets() {
        for bad in ["https://x.playtest.run/_playtest/tunnel", "不是地址", "/只有路径"] {
            assert!(
                matches!(handshake_request(bad, "t", 1), Err(HandshakeError::BadUrl(_))),
                "{bad} 应该被拒"
            );
        }
    }

    #[test]
    fn handshake_request_rejects_a_token_with_control_characters() {
        let url = format!("ws://127.0.0.1:8443{}", tunnel::WS_PATH);
        assert_eq!(
            handshake_request(&url, "pt1.bad\ntoken", 5173).unwrap_err(),
            HandshakeError::BadToken
        );
    }
}
