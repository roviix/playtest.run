//! 隧道这一头（DESIGN §4.3）。
//!
//! 开发者敲 `playtest 5173`，CLI 从他的机器上**只出不进**地开一条 WebSocket 到
//! `wss://<slug>.<后缀>/_playtest/tunnel`。边缘验完令牌把这条连接变成 yamux 多路复用通道；
//! 之后每一个玩家请求就是通道上的一条流：边缘把 HTTP/1.1 请求写进去、把响应流回玩家，
//! 收到 101 之后双向拷贝字节，WebSocket 就这么原样过去。
//!
//! 四个文件各管一段：
//!
//! - [`keys`]：验签公钥从哪来。控制面签，边缘只验，不回源（DESIGN §4.5）。
//! - [`handshake`]：握手的每一道校验，以及把连接升级成 yamux。
//! - [`registry`]：谁连着、谁刚走、谁被挤掉。
//! - [`proxy`]：一次玩家请求怎么走完这条路。
//! - [`offline`]：走不通时玩家看到的那几页。
//!
//! 这个模块**不执行任何用户代码，也不改开发者的响应体一个字节**（DESIGN §3.7）：
//! 只补 `game_headers` 那几个「不补游戏就跑不起来」的响应头。

pub mod handshake;
pub mod keys;
pub mod offline;
pub mod proxy;
pub mod registry;

pub use registry::{LastSeen, Session, Tunnels};

use axum::http::{HeaderMap, HeaderValue, Method, StatusCode};
use axum::response::{IntoResponse, Response};
use playtest_common::api::{ErrorBody, ErrorCode};
use playtest_common::manifest::Manifest;

use crate::gate;

/// 握手挂在保留路径的这一段上，用作品自己的 Host。
/// 拼起来就是 [`playtest_common::tunnel::WS_PATH`]，下面有一条测试盯着它俩一致。
pub const HANDSHAKE_TAIL: &str = "tunnel";

/// 隧道模式下门禁页在「版本」那个位置显示的东西。
///
/// 上传路径显示 `v7`，隧道路径没有版本可显示（DESIGN §3.5：隧道只有「在线 / 离线」和会话）。
/// 显示合成清单里那个 `v0` 会让玩家以为自己拿到了一个坏链接。
pub const ONLINE_LABEL: &str = "Live";

/// 隧道模式下这一次请求要不要先出门禁页。
///
/// 上传路径知道请求路径解析出来的是不是 HTML，隧道路径不知道——要问过开发者的机器才知道，
/// 而门禁页存在的意义正是在问之前先拦一下。所以这里拿「浏览器说自己要一个顶层文档」
/// 当替身：它说要文档，我们就给它一页文档。
///
/// 三条硬线一条没松（DESIGN §3.3）：子资源不会走到这里（`Sec-Fetch-Dest` 不是 `document`），
/// 就算走到了，[`gate::should_show`] 里那条资源路径白名单还挡着；判据仍然只有请求语义和
/// 门禁自己种的 cookie，没有任何请求头能绕过它。POST 之类不出门禁页——那是游戏自己的
/// 请求，拦下来等于把玩家的表单体吞掉。
pub fn wants_gate(
    manifest: &Manifest,
    method: &Method,
    path: &str,
    navigation: bool,
    has_cookie: bool,
    from: Option<&str>,
) -> bool {
    (method == Method::GET || method == Method::HEAD)
        && gate::should_show(
            manifest.gate,
            path,
            navigation,
            navigation,
            has_cookie,
            from,
        )
}

/// 握手失败的回答。对端是 CLI 不是浏览器，所以这里是 [`ErrorBody`] 的 JSON 不是 HTML。
pub fn error(status: StatusCode, code: ErrorCode, message: &str) -> Response {
    let body = ErrorBody {
        code,
        message: message.to_string(),
    };
    let json = serde_json::to_string(&body).unwrap_or_else(|_| {
        // ErrorBody 一定序列化得出来；真出了事也不能把 CLI 晾在一个空响应上。
        "{\"code\":\"internal\",\"message\":\"边缘出错了\"}".to_string()
    });
    let mut headers = HeaderMap::new();
    headers.insert(
        "content-type",
        HeaderValue::from_static("application/json; charset=utf-8"),
    );
    headers.insert("cache-control", HeaderValue::from_static("no-store"));
    (status, headers, json).into_response()
}

/// 我们自己渲染的一页 HTML。`isolated` 的作品连错误页都要带隔离头，
/// 否则点了开始之后才隔离等于没隔离。
pub fn page(status: StatusCode, html: String, isolated: bool) -> Response {
    let mut headers = HeaderMap::new();
    headers.insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        "content-type",
        HeaderValue::from_static("text/html; charset=utf-8"),
    );
    headers.insert("cache-control", HeaderValue::from_static("no-store"));
    crate::game_headers::isolation(&mut headers, isolated, false);
    (status, headers, html).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use playtest_common::manifest::GateMode;
    use playtest_common::tunnel::WS_PATH;
    use playtest_common::RESERVED_PATH_PREFIX;

    fn manifest(gate: GateMode) -> Manifest {
        let mut m = registry::synthetic_manifest(
            &playtest_common::tunnel::Claims {
                v: 1,
                slug: "brisk-otter-41".into(),
                sub: "u".into(),
                title: "小球".into(),
                developer: "某某".into(),
                badge: true,
                gate,
                isolated: false,
                max_players: 8,
                hybrid: false,
                iat: 0,
                exp: 0,
                jti: "j".into(),
            },
            time::OffsetDateTime::UNIX_EPOCH,
        );
        m.gate = gate;
        m
    }

    #[test]
    fn the_handshake_path_is_the_one_common_declares() {
        assert_eq!(
            WS_PATH.strip_prefix(RESERVED_PATH_PREFIX),
            Some(HANDSHAKE_TAIL)
        );
    }

    #[test]
    fn only_a_document_navigation_gets_the_gate_page() {
        let m = manifest(GateMode::Once);
        assert!(wants_gate(&m, &Method::GET, "/", true, false, None));
        assert!(wants_gate(&m, &Method::HEAD, "/level/3", true, false, None));

        // 点过开始了，但若显式带了 from=plaza 仍会展示邀请函。
        assert!(!wants_gate(&m, &Method::GET, "/", true, true, None));
        assert!(wants_gate(&m, &Method::GET, "/", true, true, Some("plaza")));
        // 子资源、XHR、WebSocket 升级：浏览器没说要文档。
        assert!(!wants_gate(
            &m,
            &Method::GET,
            "/main.js",
            false,
            false,
            None
        ));
        assert!(!wants_gate(
            &m,
            &Method::GET,
            "/socket.io/",
            false,
            false,
            None
        ));
        // 就算客户端把自己说成导航，长得像资源的路径也不出门禁页。
        assert!(!wants_gate(
            &m,
            &Method::GET,
            "/assets/app-4f2c.js",
            true,
            false,
            None
        ));
        // 游戏自己发的 POST 不能被拦，否则请求体就丢了。
        assert!(!wants_gate(
            &m,
            &Method::POST,
            "/api/score",
            true,
            false,
            None
        ));
        // 开发者选了不出就不出。
        assert!(!wants_gate(
            &manifest(GateMode::Never),
            &Method::GET,
            "/",
            true,
            false,
            None
        ));
    }

    #[test]
    fn handshake_errors_are_machine_readable() {
        let response = error(
            StatusCode::CONFLICT,
            ErrorCode::TunnelReplaced,
            "另一个 playtest 进程已经接管了这个作品",
        );
        assert_eq!(response.status(), StatusCode::CONFLICT);
        assert_eq!(
            response.headers()["content-type"],
            "application/json; charset=utf-8"
        );
        assert_eq!(response.headers()["cache-control"], "no-store");
    }
}
