//! 广场（DESIGN §3.9）：控制面整理、边缘渲染的那一份 `plaza.json`。
//!
//! 走的是清单同一条路（DESIGN §4.5）：控制面写进对象存储，边缘只读它。控制面挂了，
//! 广场照常能翻，只是人数旧几分钟。这里只有数据的形状；谁在什么时候重写它见 `api`，
//! 怎么画成一页见 `edge`。
//!
//! 2026-09-09 起（方向 B）多了：推广位（`boosted`，DESIGN §3.11）、关注数、名额进度、开发者头像，
//! 以及关注广场的人数。全部带默认值，旧文件照常解析。

use serde::{Deserialize, Serialize};

/// 当前格式版本号。不兼容的改动才加一；2026-09-09 的字段全是加法，仍是 1。
pub const SCHEMA: u32 = 1;

/// 「N 人玩过」看多少天（DESIGN §3.9）。只当卡上的一件事实，不决定顺序。
pub const PLAYERS_WINDOW_DAYS: i64 = 7;

/// 同一作品在这么多小时内被几个不同会话举报，就自动从广场撤下（DESIGN §3.9）。
pub const REPORTS_TO_HIDE: u32 = 3;
pub const REPORTS_WINDOW_HOURS: i64 = 24;

/// 对象存储里的键。
pub const KEY: &str = "plaza.json";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Plaza {
    pub schema: u32,
    /// RFC 3339，这份是什么时候整理的。排查用，页面上不显示。
    pub generated_at: String,
    /// 已经按广场的默认顺序排好：推广中的在最前（最多 [`crate::boost::MAX_SLOTS`] 个），
    /// 然后正在找人测的，其余按最近更新。边缘按这个顺序铺一面网格，不再分段。
    pub items: Vec<PlazaItem>,
    /// 关注广场本身的人数（DESIGN §3.6 的周报收件人）。页面上不写，给周报用。
    #[serde(default)]
    pub club_followers: u32,
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
    /// 开发者想找几位试玩者（`--seats`）；`joined` 是留了名字的人数（DESIGN §3.3）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seats: Option<u32>,
    #[serde(default)]
    pub joined: u32,
    /// 关注这个作品的人数。0 就不说。
    #[serde(default)]
    pub followers: u32,
    /// 开发者头像（GitHub 登录的开发者才有）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
    /// 在推广位上（DESIGN §3.11）。边缘渲染时永远标「推广」。
    #[serde(default)]
    pub boosted: bool,
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
            seats: None,
            joined: 0,
            followers: 0,
            avatar_url: None,
            boosted: false,
        };
        let json = serde_json::to_string(&item).unwrap();
        for absent in [
            "summary",
            "engine",
            "expires_at",
            "cover_url",
            "seek_note",
            "seats",
            "avatar_url",
        ] {
            assert!(!json.contains(absent), "{absent}");
        }
        let back: PlazaItem = serde_json::from_str(&json).unwrap();
        assert_eq!(back, item);
    }

    #[test]
    fn a_plaza_json_from_before_the_club_still_parses() {
        let old = r#"{"schema":1,"generated_at":"2026-09-08T03:57:00Z","items":[{
            "slug":"brisk-otter-41","url":"https://brisk-otter-41.playtest.run","title":"小球",
            "developer":"某某","is_game":true,"version":7,"updated_at":"2026-09-08T03:00:00Z","players":12}]}"#;
        let plaza: Plaza = serde_json::from_str(old).unwrap();
        assert_eq!(plaza.club_followers, 0);
        let item = &plaza.items[0];
        assert!(!item.boosted);
        assert_eq!(item.followers, 0);
        assert_eq!(item.seats, None);
    }
}
