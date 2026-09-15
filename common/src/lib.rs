//! playtest 各组件共享的契约。
//!
//! 这里定义的东西同时被 `api`（写）、`edge`（读）、`cli`（发）依赖：
//!
//! - [`manifest`]：一个版本的清单——路径 → 内容哈希，以及门禁页要显示的元信息。
//! - [`store`]：对象存储布局与文件系统 / S3 实现。api 往里写，edge 只读，两边不直接通话。
//! - [`api`]：CLI 与控制面之间的请求 / 响应体。
//! - [`ingest`]：玩家浏览器写进来的东西——SDK 的事件与反馈、边缘补送的第一层事件。
//! - [`results`]：控制台读出来的东西——作品时间线、会话点名册、反馈流。
//! - [`project`]：**一个原语**——作品的六组属性，以及它的两个投影（卡、`live.json`）。REWRITE §2.1。
//! - [`plan`]：档位与它带来的上限；卖的就是这张表里的行。REWRITE §4.1。
//! - [`plaza`]：广场那一份 `plaza.json` 的形状——控制面写、边缘渲染（DESIGN §3.9）。
//! - [`live`]：一个作品会变的那些（名额、关注数、群、公开反馈）——`sites/<slug>/live.json`（DESIGN §4.5）。
//! - [`capabilities`]：控制面现在能做什么（邮件、Web Push）——`capabilities.json`（DESIGN §4.5）。
//! - [`follow`]：关注与通知的契约——玩家表单、边缘转发、「我的」（DESIGN §3.6、§3.10）。
//! - [`boost`]：推广位的数据模型与管理接口（DESIGN §3.11）。
//! - [`hash`]：内容哈希（SHA-256 小写十六进制）。
//! - [`slug`]：slug 的校验与保留名单。
//! - [`limits`]：匿名与免费档的配额常量，CLI 报错和 api 校验用同一份数字。
//! - [`tunnel`]：隧道路径的握手、签名令牌、关闭码；feature `tunnel-io` 再带上 WebSocket ↔ yamux 的字节流适配。
//!
//! 改这里要保持向后兼容（只加字段、加 `#[serde(default)]`），因为三个进程不会同时升级。

pub mod api;
pub mod article;
pub mod avatar;
pub mod boost;
pub mod capabilities;
pub mod collection;
pub mod contract;
pub mod follow;
pub mod hash;
pub mod ingest;
pub mod limits;
pub mod live;
pub mod manifest;
pub mod plan;
pub mod plaza;
pub mod project;
pub mod quota;
pub mod results;
pub mod slug;
pub mod store;
pub mod tunnel;
pub mod video;
pub mod wording;

pub const DEVELOPER_HOST: &str = "playtest.run";

/// 控制面 API 的公网地址。控制台挂在同一主机的 `/console/` 下。
pub const DEVELOPER_API_URL: &str = "https://playtest.run";

/// 边缘保留给自己的路径前缀。门禁页的「开始」、举报、事件上报都在这下面，
/// 作品目录里如果有同名路径会被遮住——文档里写明。
pub const RESERVED_PATH_PREFIX: &str = "/_playtest/";

/// 门禁页 cookie：存在即表示「这个浏览器 24 小时内已经点过开始」。
pub const GATE_COOKIE: &str = "pt_gate";

/// 会话 cookie：从门禁页开始算「这一次打开」，只在本站，不跨站。
pub const SESSION_COOKIE: &str = "pt_sid";

/// 根域上的玩家钥匙（DESIGN §3.6、§4.1）：host-only，只种在广场所在的那一个主机名上，
/// 子域读不到也种不进来。它不是登录——只开「我的」这个抽屉，撤销靠退订。
pub const ME_COOKIE: &str = "pt_me";

/// 匿名链接的有效期。
pub const ANON_LINK_TTL_HOURS: u64 = 24;

// ---- 邀请卡与分享（DESIGN §3.4） ----

/// 竖版邀请卡（1080×1350），发群里用。边缘按当前版本渲染，PNG。
pub const CARD_PATH: &str = "/_playtest/card.png";
/// 横版邀请卡（1200×630），没有封面时做 `og:image`。
pub const CARD_WIDE_PATH: &str = "/_playtest/card-wide.png";
/// 分享页：只放那张卡、「保存图片」和「复制链接」。只有公开的作品有。
pub const SHARE_PATH: &str = "/_playtest/share";

/// 封面。在作品自己的域下，走那个 slug 的每小时熔断——一屏十几张封面也是流量。
/// 没有封面时是裸 404，不出 HTML（它是子资源，门禁页的硬线管着这件事）。
pub const COVER_PATH: &str = "/_playtest/cover";

pub const CARD_WIDTH: u32 = 1080;
pub const CARD_HEIGHT: u32 = 1350;
pub const CARD_WIDE_WIDTH: u32 = 1200;
pub const CARD_WIDE_HEIGHT: u32 = 630;

fn is_ip_host(host: &str) -> bool {
    let hostname = host.split(':').next().unwrap_or(host);
    hostname.parse::<std::net::IpAddr>().is_ok()
}

/// 从主域邀请函地址提取/还原作品子域地址（例如：`https://playtest.run/p/slug` → `https://slug.playtest.run`）。
pub fn site_url_from_door(door_url: &str) -> String {
    let Some((scheme, rest)) = door_url.split_once("://") else {
        return door_url.to_string();
    };
    let host_and_path = rest.trim_end_matches('/');
    let (host, path) = match host_and_path.split_once('/') {
        Some((h, p)) => (h, p),
        None => (host_and_path, ""),
    };
    if let Some(slug) = path.strip_prefix("p/").or_else(|| path.strip_prefix("p")) {
        let clean_slug = slug.trim_matches('/');
        if !clean_slug.is_empty() {
            if is_ip_host(host) {
                return format!("{scheme}://{host}");
            }
            return format!("{scheme}://{clean_slug}.{host}");
        }
    }
    door_url.to_string()
}

/// 邀请卡的完整地址。支持传入子域链接或主域邀请函链接。
pub fn card_url(site_or_door: &str) -> String {
    let site = site_url_from_door(site_or_door);
    format!("{}{}", site.trim_end_matches('/'), CARD_PATH)
}

/// 横版邀请卡的完整地址。支持传入子域链接或主域邀请函链接。
pub fn card_wide_url(site_or_door: &str) -> String {
    let site = site_url_from_door(site_or_door);
    format!("{}{}", site.trim_end_matches('/'), CARD_WIDE_PATH)
}

// ---- 来源（DESIGN §3.5「来自哪里」） ----

/// 作品链接上表示「从哪来」的查询参数名。门禁页把它放进「开始」表单带过去。
pub const FROM_PARAM: &str = "from";
/// 扫邀请卡上的二维码来的。
pub const FROM_CARD: &str = "card";
/// 从关注通知或周报里点进来的。
pub const FROM_NOTICE: &str = "notice";
/// 从广场那面墙上点进来的。三个环各带来了几个人，开发者据此知道（REWRITE §3.4「结果」）。
pub const FROM_PLAZA: &str = "plaza";
pub const FROM_COLLECTION: &str = "collection";

/// 从作品子域地址（如 `https://brisk-otter-41.playtest.run` 或 `http://brisk-otter-41.localhost:8443`）
/// 计算出根域地址（如 `https://playtest.run/` 或 `http://localhost:8443/`）。
pub fn root_url_from_site(site_url: &str) -> String {
    let Some((scheme, rest)) = site_url.split_once("://") else {
        return site_url.to_string();
    };
    let host_and_path = rest.trim_end_matches('/');
    let host = match host_and_path.split_once('/') {
        Some((host, _path)) => host,
        None => host_and_path,
    };
    if is_ip_host(host) {
        return format!("{scheme}://{host}/");
    }
    let (hostname, port_suffix) = match host.split_once(':') {
        Some((h, p)) => (h, format!(":{p}")),
        None => (host, String::new()),
    };
    let parts: Vec<&str> = hostname.split('.').collect();
    let root_hostname = if hostname == "playtest.run" || hostname == "localhost" {
        hostname
    } else if hostname.ends_with(".playtest.run") && parts.len() >= 3 {
        &hostname[parts[0].len() + 1..]
    } else if hostname.ends_with(".localhost") && parts.len() >= 2 {
        &hostname[parts[0].len() + 1..]
    } else if parts.len() > 2 {
        &hostname[parts[0].len() + 1..]
    } else {
        hostname
    };
    format!("{scheme}://{root_hostname}{port_suffix}/")
}

/// 作品主域邀请函完整链接（Front Door，DESIGN §3.1 与 §3.3）。
/// 例如：`https://brisk-otter-41.playtest.run` + `brisk-otter-41` → `https://playtest.run/p/brisk-otter-41`
/// 本机开发：`http://brisk-otter-41.localhost:8443` + `brisk-otter-41` → `http://localhost:8443/p/brisk-otter-41`
pub fn door_url(site_url: &str, slug: &str) -> String {
    if site_url.contains("/p/") {
        return site_url.to_string();
    }
    let root = root_url_from_site(site_url);
    format!("{}p/{slug}", root)
}

/// 邀请卡二维码里的地址：主域邀请函加 `?from=card`（DESIGN §3.4）。
pub fn card_qr_url(door_url: &str) -> String {
    let base = door_url.trim_end_matches('/');
    if base.contains('?') {
        format!("{base}&{FROM_PARAM}={FROM_CARD}")
    } else {
        format!("{base}?{FROM_PARAM}={FROM_CARD}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn card_and_qr_urls_hang_off_the_site_url() {
        assert_eq!(
            card_url("https://brisk-otter-41.playtest.run/"),
            "https://brisk-otter-41.playtest.run/_playtest/card.png"
        );
        assert_eq!(
            root_url_from_site("https://brisk-otter-41.playtest.run"),
            "https://playtest.run/"
        );
        assert_eq!(
            root_url_from_site("https://playtest.run/p/merry-eel-78"),
            "https://playtest.run/"
        );
        assert_eq!(
            root_url_from_site("https://playtest.run/"),
            "https://playtest.run/"
        );
        assert_eq!(
            root_url_from_site("http://brisk-otter-41.localhost:8443"),
            "http://localhost:8443/"
        );
        assert_eq!(
            root_url_from_site("http://localhost:8443/p/brisk-otter-41"),
            "http://localhost:8443/"
        );
        assert_eq!(
            door_url("https://brisk-otter-41.playtest.run", "brisk-otter-41"),
            "https://playtest.run/p/brisk-otter-41"
        );
        assert_eq!(
            door_url("http://brisk-otter-41.localhost:8443", "brisk-otter-41"),
            "http://localhost:8443/p/brisk-otter-41"
        );
        assert_eq!(
            site_url_from_door("https://playtest.run/p/brisk-otter-41"),
            "https://brisk-otter-41.playtest.run"
        );
        assert_eq!(
            site_url_from_door("http://localhost:8443/p/brisk-otter-41"),
            "http://brisk-otter-41.localhost:8443"
        );
        assert_eq!(
            card_url("https://playtest.run/p/brisk-otter-41"),
            "https://brisk-otter-41.playtest.run/_playtest/card.png"
        );
        assert_eq!(
            card_qr_url("https://playtest.run/p/brisk-otter-41"),
            "https://playtest.run/p/brisk-otter-41?from=card"
        );
    }
}
