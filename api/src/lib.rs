//! playtest 控制面。
//!
//! 三类客户端里 v0.1 只接 CLI（DESIGN §4.5）：`playtest ./dist` 走的是
//! 「问缺哪些哈希 → 只传缺的 → 提交清单」这条路，端点定义在 [`playtest_common::api::routes`]。
//!
//! 两处刻意的分工：
//!
//! - **清单和「当前版本」指针不进数据库**，写进对象存储。边缘只读对象存储，
//!   控制面挂了已经发出去的链接照常能开。
//! - **文件内容不经过数据库，也不整包进内存**：`PUT /v1/blobs/{hash}` 边收边写临时文件、
//!   边算 SHA-256，哈希对上才分段提交进对象存储，数据库里只留一行「这个哈希有了」。

pub mod account;
pub mod auth;
pub mod boosts;
pub mod capabilities;
pub mod clock;
pub mod collections;
pub mod config;
pub mod db;
pub mod device;
pub mod error;
pub mod live;
pub mod notify;
pub mod plaza;
pub mod project;
pub mod quota;
pub mod routes;
pub mod scheduler;
pub mod state;
pub mod sweeper;
pub mod tunnel_keys;
pub mod words;

pub use config::Config;
pub use routes::app;
pub use state::AppState;
