//! 广场（DESIGN §3.9）：控制面整理、边缘渲染的那一份 `plaza.json`。
//!
//! 走的是清单同一条路（DESIGN §4.5）：控制面写进对象存储，边缘只读它。控制面挂了，
//! 广场照常能翻，只是人数旧几分钟。这里只有数据的形状；谁在什么时候重写它见 `api`，
//! 怎么画成一页见 `edge`。
//!
//! 2026-09-09 起（方向 B）多了：推广位（`boosted`，DESIGN §3.11）、关注数、名额进度、开发者头像，
//! 以及关注广场的人数。全部带默认值，旧文件照常解析。

use serde::{Deserialize, Serialize};

/// 墙上那一张卡。形状搬到了 [`crate::project`]（REWRITE §2.1）：广场和控制台作品墙
/// 用同一个类型，同一个作品在两个地方才会说同一句话。
pub use crate::project::ProjectCard as PlazaItem;

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

/// 一面空墙。控制面还没写过这份文件、或者它坏了的时候，边缘按这个出——
/// 广场页照常出，只是没有卡片。「空的广场长什么样」只该有一个定义。
impl Default for Plaza {
    fn default() -> Self {
        Self {
            schema: SCHEMA,
            generated_at: String::new(),
            items: Vec::new(),
            club_followers: 0,
        }
    }
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
            note: None,
            engine: None,
            is_game: false,
            version: 1,
            updated_at: "2026-09-08T00:00:00Z".into(),
            expires_at: None,
            cover_hash: None,
            players: 0,
            seeking: false,
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
            "cover_hash",
            "note",
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
