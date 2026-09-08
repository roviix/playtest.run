//! playtest 各组件共享的契约。
//!
//! 这里定义的东西同时被 `api`（写）、`edge`（读）、`cli`（发）依赖：
//!
//! - [`manifest`]：一个版本的清单——路径 → 内容哈希，以及门禁页要显示的元信息。
//! - [`store`]：对象存储的目录布局与本机文件系统实现。api 往里写，edge 只读，两边不直接通话。
//! - [`api`]：CLI 与控制面之间的请求 / 响应体。
//! - [`ingest`]：玩家浏览器写进来的东西——SDK 的事件与反馈、边缘补送的第一层事件。
//! - [`results`]：控制台读出来的东西——作品时间线、会话点名册、反馈流。
//! - [`plaza`]：广场那一份 `plaza.json` 的形状——控制面写、边缘渲染（DESIGN §3.8）。
//! - [`hash`]：内容哈希（SHA-256 小写十六进制）。
//! - [`slug`]：slug 的校验与保留名单。
//! - [`limits`]：匿名与免费档的配额常量，CLI 报错和 api 校验用同一份数字。
//! - [`tunnel`]：隧道路径的握手、签名令牌、关闭码；feature `tunnel-io` 再带上 WebSocket ↔ yamux 的字节流适配。
//!
//! 改这里要保持向后兼容（只加字段、加 `#[serde(default)]`），因为三个进程不会同时升级。

pub mod api;
pub mod hash;
pub mod ingest;
pub mod limits;
pub mod manifest;
pub mod plaza;
pub mod results;
pub mod slug;
pub mod store;
pub mod tunnel;

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

/// 匿名链接的有效期。
pub const ANON_LINK_TTL_HOURS: u64 = 24;
