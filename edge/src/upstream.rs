//! 边缘往控制面写的那一次 HTTP。
//!
//! 边缘对控制面**只有写方向**的调用（DESIGN §4.1）：事件批量上报、关注登记、确认、退订。
//! 读永远走对象存储——控制面挂了，作品照常能玩、广场照常能翻。
//!
//! 这里只有「连一次、发一个 JSON、拿回状态码和字节」这一件事。上面那两个调用方的**语义**
//! 不一样，所以它们没有合并：`follow` 那边玩家在页面前等着，超时要短、要解析回答、
//! 要能区分「控制面说不」和「控制面不在」；`ship` 那边是后台批量，失败了下一轮再来、
//! 不关心响应体。不一样的是这些，不是拼 URI 和 TCP 握手——那两段以前一字不差地写了两遍。

use std::time::Duration;

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::Request;
use hyper_util::rt::TokioIo;

/// 内网一次 JSON POST 的结果。
#[derive(Debug)]
pub struct Answer {
    pub status: u16,
    pub body: Bytes,
}

impl Answer {
    pub fn ok(&self) -> bool {
        (200..300).contains(&self.status)
    }

    /// 出错时给人看的那一段。截短：控制面的错误页可能很长，日志里不需要全文。
    pub fn snippet(&self) -> String {
        String::from_utf8_lossy(&self.body)
            .chars()
            .take(200)
            .collect()
    }
}

/// 往控制面发一个 JSON，拿回状态码和响应体。
///
/// 只允许 `http://`：控制面在同一台机器或内网上（compose 里就是隔壁容器）。允许 https
/// 就要在边缘里带一套 TLS 客户端和证书校验，而这条链路根本不过公网。
pub async fn post_json<B: serde::Serialize>(
    api_base: &str,
    path: &str,
    body: &B,
    timeout: Duration,
) -> anyhow::Result<Answer> {
    post_json_with_bearer(api_base, path, body, timeout, None).await
}

/// 带内部服务凭据的 JSON POST。令牌只放请求头，不进入 URL 或日志。
pub async fn post_json_bearer<B: serde::Serialize>(
    api_base: &str,
    path: &str,
    body: &B,
    timeout: Duration,
    token: &str,
) -> anyhow::Result<Answer> {
    post_json_with_bearer(api_base, path, body, timeout, Some(token)).await
}

async fn post_json_with_bearer<B: serde::Serialize>(
    api_base: &str,
    path: &str,
    body: &B,
    timeout: Duration,
    token: Option<&str>,
) -> anyhow::Result<Answer> {
    let url: hyper::Uri = format!("{}{path}", api_base.trim_end_matches('/')).parse()?;
    if url.scheme_str() != Some("http") {
        anyhow::bail!("控制面地址只支持 http://（同机或内网）");
    }
    let host = url
        .host()
        .ok_or_else(|| anyhow::anyhow!("控制面地址没有主机名"))?;
    let port = url.port_u16().unwrap_or(80);
    let payload = serde_json::to_vec(body)?;

    // 整个往返一个超时，包含连接、握手、发、收。分成几段各自计时，最坏情况会是它们的和。
    tokio::time::timeout(timeout, async {
        let stream = tokio::net::TcpStream::connect((host, port)).await?;
        let (mut sender, conn) =
            hyper::client::conn::http1::handshake(TokioIo::new(stream)).await?;
        // 连接驱动跑在后台；这次往返结束就没人用它了，让它自己收场。
        tokio::spawn(async move {
            let _ = conn.await;
        });
        let mut request = Request::post(url.path())
            .header("host", format!("{host}:{port}"))
            .header("content-type", "application/json")
            .header(
                "user-agent",
                concat!("playtest-edge/", env!("CARGO_PKG_VERSION")),
            );
        if let Some(token) = token {
            request = request.header("authorization", format!("Bearer {token}"));
        }
        let request = request.body(Full::new(Bytes::from(payload)))?;
        let response = sender.send_request(request).await?;
        let status = response.status().as_u16();
        let body = response.into_body().collect().await?.to_bytes();
        Ok::<_, anyhow::Error>(Answer { status, body })
    })
    .await
    .map_err(|_| anyhow::anyhow!("控制面没有在 {} 秒内答话", timeout.as_secs()))?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn https_is_refused_because_this_link_never_leaves_the_box() {
        let err = post_json("https://example.com", "/v1/x", &(), Duration::from_secs(1))
            .await
            .unwrap_err();
        assert!(err.to_string().contains("http://"), "{err}");
    }

    #[tokio::test]
    async fn a_control_plane_that_is_not_there_times_out_with_a_number_in_the_message() {
        // 保留地址，连不上也不会挂到别人机器上。
        let err = post_json(
            "http://192.0.2.1:8787",
            "/v1/x",
            &(),
            Duration::from_millis(150),
        )
        .await
        .unwrap_err();
        assert!(!err.to_string().is_empty());
    }

    #[test]
    fn a_long_error_page_is_cut_down_before_it_reaches_the_log() {
        let answer = Answer {
            status: 500,
            body: Bytes::from("x".repeat(1000)),
        };
        assert!(!answer.ok());
        assert_eq!(answer.snippet().chars().count(), 200);
    }

    #[test]
    fn two_hundreds_are_ok_and_nothing_else_is() {
        for status in [200, 201, 204, 299] {
            assert!(Answer {
                status,
                body: Bytes::new()
            }
            .ok());
        }
        for status in [199, 300, 400, 500] {
            assert!(!Answer {
                status,
                body: Bytes::new()
            }
            .ok());
        }
    }
}
