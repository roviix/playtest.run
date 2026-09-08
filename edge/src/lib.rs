//! playtest 边缘：玩家唯一接触的进程。
//!
//! 一条 `*.<后缀>` 泛域名进来，边缘做三件事（DESIGN §3.3、§4.2）：
//!
//! 1. 按 Host 找到作品，读对象存储里的清单——不问控制面，控制面挂了链接照常能开；
//! 2. 导航请求先出门禁页（谁邀请你、玩的是第几版、一个「开始」按钮）；
//! 3. 按清单把文件流给玩家，响应头对着引擎导出物调好：`.wasm` 的 MIME、
//!    预压缩产物、跨源隔离、Range。
//!
//! 作品也可以不上传，而是从开发者自己的电脑上直接放出来：[`tunnel`] 那一头
//! 接住 CLI 开过来的 WebSocket，按 Host 找到会话，把玩家的请求送进去（DESIGN §4.3）。
//!
//! 根域是广场（DESIGN §3.8）：[`plaza`] 读控制面写进对象存储的 `plaza.json`，渲染成一页卡片。
//!
//! 这个进程不执行任何用户代码（DESIGN §3.7），也不记玩家 IP（§3.4）。

pub mod app;
pub mod breaker;
pub mod config;
pub mod events;
pub mod game_headers;
pub mod gate;
pub mod host;
pub mod html;
pub mod me;
pub mod pages;
pub mod paths;
pub mod plaza;
pub mod range;
pub mod sdk;
pub mod ship;
pub mod sites;
pub mod tunnel;

pub use app::{router, App};
pub use config::Config;
