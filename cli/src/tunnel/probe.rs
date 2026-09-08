//! 开跑之前看一眼本地那个端口：有没有人在听、听的是什么、这一页有多大。
//!
//! **只在启动时看这一次。** DESIGN §4.3：保活走 WSS 控制通道，不往 `127.0.0.1:<port>` 打探测
//! ——有的隧道每 30 秒往开发者的 dev server 发一个 `HEAD /`，把他自己的日志弄脏。所以这里
//! 发出去的请求全部属于「认引擎」这一件事，之后再不碰本地端口（除了转发玩家的连接）。

use std::time::Duration;

use playtest_common::limits::MIB;
use reqwest::header::{CONTENT_LENGTH, CONTENT_TYPE, HOST};

use crate::output::Finding;
use crate::ui;

/// 连本地端口最多等这么久。都在同一台机器上，两秒还没连上就是没人在听。
const CONNECT_TIMEOUT: Duration = Duration::from_secs(2);

/// 认引擎那几个请求各自最多等这么久。
const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);

/// 首页只读前这么多字节：认引擎和数资源引用要的东西都在 HTML 开头，
/// 而有些框架的首页会塞进一整个 bundle。
const MAX_BODY_BYTES: usize = 64 * 1024;

/// 最多称量这么多个资源。数不完就不给数字，见 [`weigh`]。
const MAX_ASSETS: usize = 32;

/// 超过这么大就该提醒改用上传（DESIGN §4.3 最后一段）。
const HEAVY_BYTES: u64 = 10 * MIB;

/// 算「一个玩家要等多久」用的上行带宽，比特每秒。
///
/// 这是**假设**不是实测：家宽上行普遍 30 Mbps 上下，我们没量过用户那条线。
/// 所以提示语里把这个前提写出来，不假装是测出来的。
const ASSUMED_UPLINK_BPS: f64 = 30_000_000.0;

/// 本地端口上有人在听吗。
pub async fn is_listening(port: u16) -> bool {
    let target = ("127.0.0.1", port);
    matches!(
        tokio::time::timeout(CONNECT_TIMEOUT, tokio::net::TcpStream::connect(target)).await,
        Ok(Ok(_))
    )
}

/// 首页长什么样。拿不到就是各项都空——这不是健康检查，问不出来也照样往下走。
#[derive(Debug, Default, Clone)]
pub struct Page {
    /// 首页的 HTML，截断到 [`MAX_BODY_BYTES`]。不是 HTML 就没有。
    html: Option<String>,
    /// 首页自己有多少字节。响应给了 `Content-Length` 就用它，否则用读到的长度；
    /// 被截断且没有 `Content-Length` 就是不知道。
    self_bytes: Option<u64>,
}

impl Page {
    /// 是 Vite 的开发服务器吗。它注入的那个客户端脚本是最稳的标记。
    pub fn is_vite(&self) -> bool {
        self.html.as_deref().is_some_and(looks_like_vite)
    }

    /// 检测到 Vite 时要说的那句话。
    pub fn vite_hint(&self) -> Option<Finding> {
        self.is_vite().then(|| {
            Finding::note("检测到 Vite").hint(
                "手机上要热更新的话，在 vite.config 里加 server.hmr.clientPort = 443\
                 （不加也能玩，只是改代码后手机那边不会自动刷新）",
            )
        })
    }
}

/// 请求一次首页。只为认引擎，不作为健康探测。
pub async fn look(port: u16) -> Page {
    let Some(http) = local_client() else {
        return Page::default();
    };
    let Ok(response) = http.get(root_url(port)).header(HOST, host(port)).send().await else {
        return Page::default();
    };
    if !response.status().is_success() {
        return Page::default();
    }

    let declared = header_bytes(response.headers().get(CONTENT_LENGTH));
    let is_html = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.to_ascii_lowercase().contains("text/html"));

    let (body, truncated) = read_head(response).await;
    if !is_html {
        return Page::default();
    }
    Page {
        html: Some(String::from_utf8_lossy(&body).into_owned()),
        self_bytes: declared.or((!truncated).then_some(body.len() as u64)),
    }
}

/// 这一页整个下载下来有多少字节。
///
/// 一个都称不准就返回 `None`——宁可不说，也不编一个数字（AGENTS.md 第 4 条）。
/// 判据严一点：只要有一个引用的资源没给 `Content-Length`，总量就一定偏小，那就整个不说。
pub async fn weigh(port: u16, page: &Page) -> Option<u64> {
    let html = page.html.as_deref()?;
    let mut total = page.self_bytes?;
    let refs = asset_refs(html);
    if refs.is_empty() {
        return Some(total);
    }
    let http = local_client()?;

    let asked = refs.iter().map(|path| {
        let http = http.clone();
        let url = format!("http://127.0.0.1:{port}{path}");
        async move {
            let response = http.head(url).header(HOST, host(port)).send().await.ok()?;
            if !response.status().is_success() {
                return None;
            }
            header_bytes(response.headers().get(CONTENT_LENGTH))
        }
    });
    for size in futures_util::future::join_all(asked).await {
        total += size?;
    }
    Some(total)
}

/// 这一页大到该改用上传时说的那句话。DESIGN §4.3：要算出数字，不要只说「可能会慢」。
pub fn heavy_hint(total: u64) -> Option<Finding> {
    if total <= HEAVY_BYTES {
        return None;
    }
    Some(
        Finding::note(format!(
            "这个页面要下载约 {}。隧道走你的电脑上行，按常见家宽 30 Mbps 算一个玩家大约要等 {} 秒，\
             十个人同时进来会更久",
            ui::bytes(total),
            wait_seconds(total)
        ))
        .hint("如果这个目录是构建好的静态导出物，`playtest ./dist` 上传一次更快"),
    )
}

/// 一个玩家等多少秒。往上取整，不给「8.53 秒」这种假精确。
fn wait_seconds(total: u64) -> u64 {
    (total as f64 / (ASSUMED_UPLINK_BPS / 8.0)).ceil() as u64
}

fn looks_like_vite(html: &str) -> bool {
    html.contains("/@vite/client")
}

/// 这一页会顺带下载哪些东西：`<script src>`、`<link href>`，
/// 以及 Godot / Unity 那几种大文件（`.wasm`、`.pck`、`.data`，它们常写在脚本里而不是标签上）。
///
/// 这不是 HTML 解析器，也不必是：数漏了就不给数字。只要同源的（相对路径或以 `/` 开头），
/// 别人家的 CDN 不走我们的隧道。返回的是以 `/` 开头的路径，去重、按出现顺序、最多
/// [`MAX_ASSETS`] 个。
fn asset_refs(html: &str) -> Vec<String> {
    let bytes = html.as_bytes();
    let mut found: Vec<String> = Vec::new();
    let mut i = 0usize;
    while i < bytes.len() {
        let quote = bytes[i];
        if quote != b'"' && quote != b'\'' {
            i += 1;
            continue;
        }
        let start = i + 1;
        let Some(len) = bytes[start..].iter().position(|&b| b == quote) else {
            break;
        };
        let value = &html[start..start + len];
        let named = attribute_before(&html[..i]);
        i = start + len + 1;

        let wanted = matches!(named.as_deref(), Some("src") | Some("href")) || is_big_file(value);
        if !wanted {
            continue;
        }
        let Some(path) = same_origin_path(value) else {
            continue;
        };
        if !found.contains(&path) {
            found.push(path);
        }
        if found.len() >= MAX_ASSETS {
            break;
        }
    }
    found
}

/// 引号前面那个属性名，比如 `src="…"` 里的 `src`。
fn attribute_before(before: &str) -> Option<String> {
    let head = before.trim_end();
    let head = head.strip_suffix('=')?.trim_end();
    let name: String = head
        .chars()
        .rev()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect();
    if name.is_empty() {
        return None;
    }
    Some(name.chars().rev().collect::<String>().to_ascii_lowercase())
}

/// Godot 与 Unity 的大件。这些名字出现在引号里基本就是要下载的东西。
fn is_big_file(value: &str) -> bool {
    let name = value.split(['?', '#']).next().unwrap_or(value);
    [".wasm", ".pck", ".data", ".unityweb", ".symbols.json"]
        .iter()
        .any(|ext| name.to_ascii_lowercase().ends_with(ext))
        || name.to_ascii_lowercase().ends_with(".wasm.br")
        || name.to_ascii_lowercase().ends_with(".data.br")
}

/// 只要走我们隧道的那些：相对路径和以 `/` 开头的绝对路径。
fn same_origin_path(value: &str) -> Option<String> {
    let value = value.trim();
    let value = value.split('#').next().unwrap_or(value);
    if value.is_empty() || value.starts_with("//") || value.contains("://") {
        return None;
    }
    if value.starts_with("data:") || value.starts_with("blob:") || value.starts_with("javascript:") {
        return None;
    }
    if value.starts_with('/') {
        return Some(value.to_string());
    }
    // 我们只取过首页，相对路径就相对根。
    Some(format!("/{}", value.trim_start_matches("./")))
}

fn root_url(port: u16) -> String {
    format!("http://127.0.0.1:{port}/")
}

/// 边缘会把 `Host` 改写成 `localhost:<port>`（DESIGN §4.3），这里照做，
/// 免得 Vite 的 `allowedHosts` 对我们和对玩家给出两种结果。
fn host(port: u16) -> String {
    format!("localhost:{port}")
}

/// 走本机，任何代理设置都只会碍事。
fn local_client() -> Option<reqwest::Client> {
    reqwest::Client::builder()
        .connect_timeout(CONNECT_TIMEOUT)
        .timeout(REQUEST_TIMEOUT)
        .no_proxy()
        .build()
        .ok()
}

fn header_bytes(value: Option<&reqwest::header::HeaderValue>) -> Option<u64> {
    value?.to_str().ok()?.trim().parse().ok()
}

/// 读响应体的前 [`MAX_BODY_BYTES`] 个字节，返回读到的部分和「是不是没读完」。
async fn read_head(mut response: reqwest::Response) -> (Vec<u8>, bool) {
    let mut body = Vec::new();
    while let Ok(Some(chunk)) = response.chunk().await {
        body.extend_from_slice(&chunk);
        if body.len() >= MAX_BODY_BYTES {
            body.truncate(MAX_BODY_BYTES);
            return (body, true);
        }
    }
    (body, false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_vite_page_is_recognised_by_its_injected_client() {
        assert!(looks_like_vite(
            r#"<html><script type="module" src="/@vite/client"></script></html>"#
        ));
        assert!(!looks_like_vite("<html><script src=\"/main.js\"></script></html>"));
    }

    #[test]
    fn the_vite_hint_says_what_to_add_and_that_it_is_optional() {
        let page = Page {
            html: Some("<script src=\"/@vite/client\"></script>".into()),
            self_bytes: Some(10),
        };
        let hint = page.vite_hint().expect("认出 Vite 就该有一句提示");
        assert!(hint.message.contains("Vite"));
        let hint = hint.hint.unwrap();
        assert!(hint.contains("server.hmr.clientPort = 443"), "{hint}");
        assert!(hint.contains("也能玩"), "不加也能玩这件事要说清楚：{hint}");
        assert!(Page::default().vite_hint().is_none());
    }

    #[test]
    fn script_and_link_references_are_counted_once_each() {
        let html = r#"
            <link rel="stylesheet" href="/style.css">
            <link rel="icon" href='./favicon.ico'>
            <script src="/main.js"></script>
            <script src="/main.js"></script>
        "#;
        assert_eq!(
            asset_refs(html),
            ["/style.css", "/favicon.ico", "/main.js"]
        );
    }

    #[test]
    fn engine_payloads_are_counted_even_when_they_sit_inside_a_script() {
        let html = r#"<script>
            const engine = { mainPack: "game.pck", wasm: "/build/game.wasm" };
            createUnityInstance({ dataUrl: "Build/web.data", frameworkUrl: "Build/web.js" });
        </script>"#;
        assert_eq!(
            asset_refs(html),
            ["/game.pck", "/build/game.wasm", "/Build/web.data"],
            "Unity 的 frameworkUrl 不在这几个扩展名里，数不到很正常"
        );
    }

    #[test]
    fn other_peoples_urls_are_not_ours_to_count() {
        // 用 ## 包起来：里面有 "#start"，一个 # 的话字符串会在那里断掉。
        let html = r##"
            <script src="https://cdn.example.com/three.js"></script>
            <script src="//cdn.example.com/two.js"></script>
            <img src="data:image/png;base64,AAAA">
            <a href="#start">开始</a>
            <script src="/ours.js"></script>
        "##;
        assert_eq!(asset_refs(html), ["/ours.js"]);
    }

    #[test]
    fn a_reference_keeps_its_query_because_vite_needs_it() {
        assert_eq!(
            asset_refs(r#"<script src="/main.js?t=1717"></script>"#),
            ["/main.js?t=1717"]
        );
    }

    #[test]
    fn nothing_is_said_about_a_page_that_is_not_heavy() {
        assert!(heavy_hint(0).is_none());
        assert!(heavy_hint(HEAVY_BYTES).is_none());
    }

    #[test]
    fn a_heavy_page_gets_a_number_not_an_adjective() {
        let hint = heavy_hint(32 * MIB).expect("32 MB 该提醒");
        assert!(hint.message.contains("32.0 MB"), "{}", hint.message);
        assert!(hint.message.contains("9 秒"), "{}", hint.message);
        assert!(hint.message.contains("30 Mbps"), "得说清楚这是按什么算的");
        assert!(hint.hint.unwrap().contains("playtest ./dist"));
    }

    #[test]
    fn the_wait_is_rounded_up_so_it_is_never_optimistic() {
        // 30 Mbps = 3.75 MB/s。
        assert_eq!(wait_seconds(3_750_000), 1);
        assert_eq!(wait_seconds(3_750_001), 2);
        assert_eq!(wait_seconds(32 * MIB), 9);
    }
}
