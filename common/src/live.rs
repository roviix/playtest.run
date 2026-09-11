//! `projects/<slug>/live.json`：一个作品**会变的那些**。
//!
//! 形状搬到了 [`crate::project`]（REWRITE §2.1「一个原语」）——它和门禁页、广场卡、
//! 控制台看到的是同一份事实的不同投影，各写一份就会各自漂。这里只留名字和键，
//! 让「读 live.json」这件事仍然有一个自己的模块名。

pub use crate::project::{
    live_key as key, ProjectLive as SiteLive, PublicNote as PublicFeedbackItem,
    LIVE_SCHEMA as SCHEMA, PUBLIC_NOTES_ON_GATE as PUBLIC_FEEDBACK_ON_GATE,
};
