//! playtest 各组件共享的契约。
//!
//! 这里定义的东西同时被 `api`（写）、`edge`（读）、`cli`（发）依赖：
//!
//! - [`manifest`]：一个版本的清单——路径 → 内容哈希，以及门禁页要显示的元信息。
//! - [`store`]：对象存储的目录布局与本机文件系统实现。api 往里写，edge 只读，两边不直接通话。
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
pub mod boost;
pub mod capabilities;
pub mod follow;
pub mod hash;
pub mod ingest;
pub mod limits;
pub mod live;
pub mod manifest;
pub mod plan;
pub mod plaza;
pub mod project;
pub mod results;
pub mod slug;
pub mod store;
pub mod tunnel;
pub mod wording;

/// 开发者这一侧的域名：控制面、控制台、登录、文档都在这里（DESIGN §4.1，AGENTS 第 7 条）。
/// 玩家路径上唯一允许出现它的地方是根域介绍页的「开发者从这里开始」。
/// 为什么是 roviix 的子域而不是独立域：隔离 cookie 与「品牌不陪葬」只要求「另一个可注册域」，
/// `roviix.com` 已满足；roviix 是制作者，playtest 是它的产品（KICKOFF §1 2b）。
pub const DEVELOPER_HOST: &str = "playtest.roviix.com";

/// 控制面 API 的公网地址。控制台挂在同一主机的 `/console/` 下。
pub const DEVELOPER_API_URL: &str = "https://playtest.roviix.com";

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

/// 邀请卡的完整地址。`site_url` 是 `https://<slug>.playtest.run`。
pub fn card_url(site_url: &str) -> String {
    format!("{}{}", site_url.trim_end_matches('/'), CARD_PATH)
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

/// 邀请卡二维码里的地址：作品链接加 `?from=card`。
pub fn card_qr_url(site_url: &str) -> String {
    format!(
        "{}/?{}={}",
        site_url.trim_end_matches('/'),
        FROM_PARAM,
        FROM_CARD
    )
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
            card_qr_url("https://brisk-otter-41.playtest.run"),
            "https://brisk-otter-41.playtest.run/?from=card"
        );
    }
}
