//! 一条隧道连接的一生：握手、把边缘送过来的每条流接到本地端口、断了之后说清楚下一步该干什么。
//!
//! 「下一步干什么」全部收在 [`Next`] 里，而且是两个纯函数算出来的（[`handshake_next`]、
//! [`close_next`]）——重连策略是这条路上最容易想岔的地方，得能单独测。

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use playtest_common::api::{ErrorBody, ErrorCode};
use playtest_common::tunnel::io::{handshake_request, Mux, MuxStream, Role, WsByteStream};
use playtest_common::tunnel::{close, TunnelGrant, KEEPALIVE_INTERVAL_SECS};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::http::Response;
use tokio_tungstenite::tungstenite::Error as WsError;
use tokio_tungstenite::{
    connect_async_tls_with_config, Connector, MaybeTlsStream, WebSocketStream,
};

use crate::output;
use crate::tunnel::Stop;

/// 挂掉之后，等边缘把关闭帧走完最多这么久。等不到也照样退出，不让用户按第二次 Ctrl-C。
const CLOSE_GRACE: Duration = Duration::from_secs(2);

/// 边缘那头握过手的 WebSocket。
pub type Wire = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// 连不上或者断了之后该做什么。每一种都带一句给用户看的中文。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Next {
    /// 按退避等一会儿再连，链接不变。
    Retry(String),
    /// 立刻换一张授权再连（令牌过期了，不是网络的问题）。
    NewGrant(String),
    /// 别再连了。再试一百次也是同样的结果，说清楚为什么然后退出。
    Stop(String),
}

/// 一条连接是怎么结束的。
pub enum Ended {
    /// 用户按了 Ctrl-C。
    ByUser,
    /// 对面断了，接下来照 [`Next`] 办。
    Closed(Next),
}

/// 握手。成功就把这条 WebSocket 交出来，失败就说下一步该干什么。
pub async fn open(grant: &TunnelGrant, port: u16) -> Result<Wire, Next> {
    let request = handshake_request(&grant.connect_url, &grant.token, port).map_err(|e| {
        // 地址或令牌本身不对，重试多少次都一样。
        Next::Stop(format!("Can not reach the edge: {e}"))
    })?;

    // Nagle 会把小包攒起来再发，联机消息最怕这个；隧道上多数就是小包。
    let connector = Connector::Rustls(tls_config());
    match connect_async_tls_with_config(request, None, true, Some(connector)).await {
        Ok((ws, _response)) => Ok(ws),
        // 边缘按约定回了一个 HTTP 错误响应，里面有 code 和中文 message。
        Err(WsError::Http(response)) => Err(http_next(&response)),
        Err(e) => Err(Next::Retry(connect_reason(&e))),
    }
}

/// 还没连上就失败时说的那句话。
///
/// tungstenite 的错误是英文的（`IO error: Connection refused (os error 61)`），
/// 直接印出来第一次用的人看不懂；常见的那几种在这里换成人话，剩下的实在没法归类才带上原文。
fn connect_reason(e: &WsError) -> String {
    let WsError::Io(io) = e else {
        return format!("Can not reach the edge ({e})");
    };
    match io.kind() {
        std::io::ErrorKind::ConnectionRefused => {
            "Can not reach the edge: nothing is answering at that address".to_string()
        }
        std::io::ErrorKind::TimedOut => "Timed out reaching the edge".to_string(),
        _ => "Can not reach the edge: no network".to_string(),
    }
}

/// 在这条连接上跑到它结束。
pub async fn serve(wire: Wire, port: u16, stop: &Stop, tally: Arc<Tally>) -> Ended {
    // 保活由这条流自己发（DESIGN §4.3：走 WSS 控制通道，不打扰本地的 dev server）。
    let stream =
        WsByteStream::new(wire).with_keepalive(Duration::from_secs(KEEPALIVE_INTERVAL_SECS));
    // 交给 Mux 之前先留一份遥控器，之后才知道对面是用什么状态码关的。
    let control = stream.control();
    let mut mux = Mux::spawn(stream, Role::Acceptor);

    loop {
        tokio::select! {
            biased;
            _ = stop.wait() => {
                mux.close();
                let _ = tokio::time::timeout(CLOSE_GRACE, mux.closed()).await;
                return Ended::ByUser;
            }
            incoming = mux.accept() => match incoming {
                Some(stream) => {
                    tokio::spawn(forward(stream, port, Arc::clone(&tally)));
                }
                None => return Ended::Closed(close_next(control.close_code())),
            },
        }
    }
}

/// 一个玩家的一次连接：边缘那头是浏览器，这头是开发者的 dev server，中间一个字节不动。
async fn forward(mut stream: MuxStream, port: u16, tally: Arc<Tally>) {
    let Ok(mut local) = TcpStream::connect(("127.0.0.1", port)).await else {
        // dev server 停了或者换了端口。把这条流丢掉，边缘会给玩家一个 502
        // ——比在这里假装成功、让浏览器等到超时要好。
        return;
    };
    let _ = local.set_nodelay(true);

    let _counted = tally.enter();
    if let Ok((up, down)) = tokio::io::copy_bidirectional(&mut stream, &mut local).await {
        tally.add_bytes(up + down);
    }
}

/// 握手被边缘拒绝时该做什么。
///
/// 分三种：换令牌（401）、退出（409 被挤掉，以及别的 4xx）、退避重连（5xx 和其它）。
fn handshake_next(status: u16, body: Option<&ErrorBody>) -> Next {
    let code = body.map(|b| b.code);
    let said = body.map(|b| b.message.as_str()).unwrap_or_default();
    match status {
        // 令牌过期或者边缘不认它。换一张再来，不用退避——这不是网络的问题。
        401 => Next::NewGrant("the token expired".to_string()),
        // 同一个作品来了新的隧道，我们是被挤掉的那个。再连也只会一直得到 409（驱逐名单按
        // 令牌记），所以退出，两个进程才不会互相挤来挤去（DESIGN §4.3）。
        409 if code == Some(ErrorCode::TunnelReplaced) => Next::Stop(
            "Another playtest process took over this project, so this one exits.".to_string(),
        ),
        // 别的 4xx 都是「这次请求本身不对」：作品没了、令牌不是给这个 slug 的、被封了。
        400..=499 => Next::Stop(stop_reason(status, said)),
        _ => Next::Retry(retry_reason(status, said)),
    }
}

fn stop_reason(status: u16, said: &str) -> String {
    if said.is_empty() {
        format!("The edge refused this tunnel (HTTP {status}).")
    } else {
        format!("The server said: {said}")
    }
}

fn retry_reason(status: u16, said: &str) -> String {
    if said.is_empty() {
        format!("The edge is temporarily unavailable (HTTP {status})")
    } else {
        format!("The edge is temporarily unavailable ({said})")
    }
}

/// 连着的时候断了，看边缘给的关闭码决定下一步（[`playtest_common::tunnel::close`]）。
fn close_next(code: Option<u16>) -> Next {
    match code {
        Some(close::REPLACED) => Next::Stop(
            "Another playtest process took over this project, so this one exits.".to_string(),
        ),
        Some(close::REVOKED) => {
            Next::Stop("This project was taken down, so the tunnel stops here.".to_string())
        }
        Some(close::TOKEN_EXPIRED) => Next::NewGrant("the token expired".to_string()),
        _ => Next::Retry("Disconnected from the server".to_string()),
    }
}

fn http_next(response: &Response<Option<Vec<u8>>>) -> Next {
    let status = response.status().as_u16();
    let body = response
        .body()
        .as_deref()
        .and_then(|bytes| serde_json::from_slice::<ErrorBody>(bytes).ok());
    handshake_next(status, body.as_ref())
}

/// TLS：rustls + ring + 内置根证书，整个进程一份。
///
/// 加密后端是 [`crate::client::install_crypto_provider`] 装的那个 ring，
/// 这里只挑根证书。`ws://`（本机开发）走不到这段。
fn tls_config() -> Arc<rustls::ClientConfig> {
    static CONFIG: OnceLock<Arc<rustls::ClientConfig>> = OnceLock::new();
    Arc::clone(CONFIG.get_or_init(|| {
        let mut roots = rustls::RootCertStore::empty();
        roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        Arc::new(
            rustls::ClientConfig::builder()
                .with_root_certificates(roots)
                .with_no_client_auth(),
        )
    }))
}

/// 现在有几个玩家连着、一共接过多少个、转发了多少字节。
#[derive(Debug, Default)]
pub struct Tally {
    active: AtomicUsize,
    total: AtomicU64,
    bytes: AtomicU64,
}

impl Tally {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn total(&self) -> u64 {
        self.total.load(Ordering::Relaxed)
    }

    pub fn bytes(&self) -> u64 {
        self.bytes.load(Ordering::Relaxed)
    }

    fn add_bytes(&self, n: u64) {
        self.bytes.fetch_add(n, Ordering::Relaxed);
    }

    /// 记一个玩家连接进来，返回的东西一丢就算它走了。
    fn enter(self: &Arc<Self>) -> Counted {
        self.total.fetch_add(1, Ordering::Relaxed);
        let now = self.active.fetch_add(1, Ordering::Relaxed) + 1;
        output::report_players(now);
        Counted(Arc::clone(self))
    }
}

/// 活着的时候算一个玩家连接。只在数字变化的那一刻说一句，不每秒刷屏。
struct Counted(Arc<Tally>);

impl Drop for Counted {
    fn drop(&mut self) {
        let now = self.0.active.fetch_sub(1, Ordering::Relaxed) - 1;
        output::report_players(now);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn body(code: ErrorCode, message: &str) -> ErrorBody {
        ErrorBody {
            code,
            message: message.to_string(),
        }
    }

    #[test]
    fn a_refused_connection_is_explained_in_chinese_not_in_errno() {
        let refused = WsError::Io(std::io::Error::from(std::io::ErrorKind::ConnectionRefused));
        let said = connect_reason(&refused);
        assert_eq!(
            said,
            "Can not reach the edge: nothing is answering at that address"
        );
        assert!(!said.contains("error"), "别把英文原文塞给用户：{said}");
    }

    #[test]
    fn being_replaced_is_the_one_case_that_must_not_retry() {
        let replaced = body(ErrorCode::TunnelReplaced, "这个作品已经有别的隧道了");
        let next = handshake_next(409, Some(&replaced));
        assert!(matches!(next, Next::Stop(_)), "{next:?}");
        let Next::Stop(said) = next else {
            unreachable!()
        };
        assert!(said.contains("took over"), "{said}");

        assert_eq!(
            close_next(Some(close::REPLACED)),
            Next::Stop(
                "Another playtest process took over this project, so this one exits.".into()
            )
        );
    }

    #[test]
    fn an_expired_token_is_swapped_not_waited_out() {
        let expired = body(ErrorCode::TokenExpired, "令牌过期了");
        assert!(matches!(
            handshake_next(401, Some(&expired)),
            Next::NewGrant(_)
        ));
        assert!(matches!(
            close_next(Some(close::TOKEN_EXPIRED)),
            Next::NewGrant(_)
        ));
    }

    #[test]
    fn a_taken_down_work_stops_and_says_so() {
        let Next::Stop(said) = close_next(Some(close::REVOKED)) else {
            panic!("下架了不该重连");
        };
        assert!(said.contains("taken down"), "{said}");
    }

    #[test]
    fn anything_else_is_just_a_disconnection() {
        assert_eq!(
            close_next(None),
            Next::Retry("Disconnected from the server".into()),
            "对端没给状态码就直接断了，最常见的一种"
        );
        assert_eq!(
            close_next(Some(close::GOING_AWAY)),
            Next::Retry("Disconnected from the server".into())
        );
        assert_eq!(
            close_next(Some(1006)),
            Next::Retry("Disconnected from the server".into())
        );
    }

    #[test]
    fn the_servers_own_words_are_passed_through_when_it_refuses() {
        let refused = body(ErrorCode::NotFound, "no such project");
        let Next::Stop(said) = handshake_next(404, Some(&refused)) else {
            panic!("404 不该一直重连");
        };
        assert_eq!(said, "The server said: no such project");
    }

    #[test]
    fn a_refusal_without_a_body_still_says_something_useful() {
        let Next::Stop(said) = handshake_next(403, None) else {
            panic!("403 不该重连");
        };
        assert!(said.contains("403"), "{said}");
    }

    #[test]
    fn a_broken_edge_is_worth_waiting_out() {
        assert!(matches!(handshake_next(502, None), Next::Retry(_)));
        assert!(matches!(
            handshake_next(500, Some(&body(ErrorCode::Internal, "内部错误"))),
            Next::Retry(_)
        ));
    }

    #[test]
    fn the_http_error_body_is_read_off_the_wire() {
        let payload = serde_json::to_vec(&body(ErrorCode::TunnelReplaced, "被挤掉了")).unwrap();
        let response = Response::builder().status(409).body(Some(payload)).unwrap();
        assert!(matches!(http_next(&response), Next::Stop(_)));

        // 中间挡了一层网关、回的不是我们的格式时，按状态码办，不能崩。
        let html = Response::builder()
            .status(502)
            .body(Some(b"<html>bad gateway</html>".to_vec()))
            .unwrap();
        assert!(matches!(http_next(&html), Next::Retry(_)));
    }

    #[test]
    fn the_player_count_only_changes_by_one_at_a_time() {
        let tally = Arc::new(Tally::new());
        let first = tally.enter();
        assert_eq!(tally.active.load(Ordering::Relaxed), 1);
        let second = tally.enter();
        assert_eq!(tally.active.load(Ordering::Relaxed), 2);
        drop(first);
        assert_eq!(tally.active.load(Ordering::Relaxed), 1);
        drop(second);
        assert_eq!(tally.active.load(Ordering::Relaxed), 0);
        assert_eq!(tally.total(), 2, "走了的人也算这次接过的连接");
    }
}
