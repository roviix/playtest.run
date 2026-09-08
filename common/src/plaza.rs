//! 广场（DESIGN §3.8）：控制面整理、边缘渲染的那一份 `plaza.json`。
//!
//! 走的是清单同一条路（DESIGN §4.5）：控制面写进对象存储，边缘只读它。控制面挂了，
//! 广场照常能翻，只是人数旧几分钟。这里只有数据的形状；谁在什么时候重写它见 `api`，
//! 怎么画成一页见 `edge`。

use serde::{Deserialize, Serialize};

/// 当前格式版本号。不兼容的改动才加一。
pub const SCHEMA: u32 = 1;

/// 「N 人玩过」看多少天（DESIGN §3.8）：7 天窗口让新作品有机会冒头，总榜只会固化前几名。
pub const PLAYERS_WINDOW_DAYS: i64 = 7;

/// 同一作品在这么多小时内被几个不同会话举报，就自动从广场撤下（DESIGN §3.8）。
pub const REPORTS_TO_HIDE: u32 = 3;
pub const REPORTS_WINDOW_HOURS: i64 = 24;

/// 对象存储里的键。
pub const KEY: &str = "plaza.json";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Plaza {
    pub schema: u32,
    /// RFC 3339，这份是什么时候整理的。边缘页脚写出来，人数旧了看得见。
    pub generated_at: String,
    /// 已经按广场的默认顺序排好：正在找人测的在前，其余按最近更新。边缘不再排。
    pub items: Vec<PlazaItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlazaItem {
    pub slug: String,
    /// 玩家点开的完整链接。
    pub url: String,
    pub title: String,
    pub developer: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    /// 上传时认出来的引擎，小写标识符；认不出来就没有。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub engine: Option<String>,
    /// 说「试玩」还是「体验」（DESIGN §3.3）。控制面按 [`crate::manifest::GAME_ENGINES`] 算好，边缘不再判。
    pub is_game: bool,
    pub version: u32,
    /// 最近一次提交版本的时间，RFC 3339。默认顺序的依据。
    pub updated_at: String,
    /// 匿名作品的到期时间，RFC 3339；登录用户的作品没有。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
    /// 封面的完整地址（作品自己的域下的 `/_playtest/cover`）。没有封面就没有。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cover_url: Option<String>,
    /// [`PLAYERS_WINDOW_DAYS`] 天内点了「开始」的去重人数。
    pub players: u32,
    /// 「正在找人测」。
    #[serde(default)]
    pub seeking: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seek_note: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn optional_fields_stay_out_of_the_json() {
        let item = PlazaItem {
            slug: "brisk-otter-41".into(),
            url: "https://brisk-otter-41.playtest.run".into(),
            title: "小球".into(),
            developer: "匿名开发者".into(),
            summary: None,
            engine: None,
            is_game: false,
            version: 1,
            updated_at: "2026-09-08T00:00:00Z".into(),
            expires_at: None,
            cover_url: None,
            players: 0,
            seeking: false,
            seek_note: None,
        };
        let json = serde_json::to_string(&item).unwrap();
        for absent in ["summary", "engine", "expires_at", "cover_url", "seek_note"] {
            assert!(!json.contains(absent), "{absent}");
        }
        let back: PlazaItem = serde_json::from_str(&json).unwrap();
        assert_eq!(back, item);
    }
}
