//! 一个作品**会变的那些**（DESIGN §4.5 的 `sites/<slug>/live.json`）。
//!
//! 清单不可变、每版一份；但门禁页上还有几样东西随时在变，又不值得为它们发一个版本：
//! 名额与已加入人数、关注数、开发者的群链接、反馈是否公开与最近几条公开反馈、开发者头像。
//! 控制面在这些变化时重写这一份，边缘只读、短缓存——控制面挂了门禁页照常出，只是数字旧几分钟。
//!
//! 没有这份文件的作品（老作品、控制面还没来得及写）一律按 [`SiteLive::default`] 解析：
//! 没名额、没人关注、没有群、反馈不公开——门禁页上对应的那几行不出现，不报错。

use serde::{Deserialize, Serialize};

pub const SCHEMA: u32 = 1;

/// 门禁页上最多显示几条公开反馈（DESIGN §3.3 第 7 条）。
pub const PUBLIC_FEEDBACK_ON_GATE: usize = 3;

pub fn key(slug: &str) -> String {
    format!("sites/{slug}/live.json")
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SiteLive {
    #[serde(default)]
    pub schema: u32,
    #[serde(default)]
    pub slug: String,
    /// RFC 3339，这份是什么时候整理的。
    #[serde(default)]
    pub generated_at: String,
    /// 开发者想找几位试玩者（`--seats`）。没设就没有这一行。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seats: Option<u32>,
    /// 已加入的人数 = 点「开始」时留了名字的去重会话数（DESIGN §3.3 第 4 条）。
    #[serde(default)]
    pub joined: u32,
    /// 关注这个作品的人数。开发者看到的是这个数字，不是名单（DESIGN §3.6）。
    #[serde(default)]
    pub followers: u32,
    /// 开发者的群（`--community`）：QQ 群、微信群二维码页、Discord、TG，去哪是他的事。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub community_url: Option<String>,
    /// 开发者开了「让玩家看到彼此的反馈」。
    #[serde(default)]
    pub feedback_public: bool,
    /// 公开的反馈里最近的几条，最多 [`PUBLIC_FEEDBACK_ON_GATE`] 条，新的在前。`feedback_public` 为假时为空。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub public_feedback: Vec<PublicFeedbackItem>,
    /// 开发者头像（GitHub 登录的开发者才有）。广场卡片与门禁页用。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
    /// 现在在广场上（开发者公开了、且没被撤下）。门禁页的「分享」只给这样的作品（DESIGN §3.4）。
    #[serde(default)]
    pub listed: bool,
    /// 「正在找人测」（只在 `listed` 时有意义）。
    #[serde(default)]
    pub seeking: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicFeedbackItem {
    /// 留下的名字；没留就显示「一位试玩者」，由渲染方决定，这里是 `None`。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub text: String,
    pub version: u32,
    /// RFC 3339。
    pub at: String,
}

impl SiteLive {
    pub fn empty(slug: &str) -> Self {
        Self {
            schema: SCHEMA,
            slug: slug.to_string(),
            ..Self::default()
        }
    }

    /// 名额到齐了没有（DESIGN §3.3：到齐之后不拦人，只如实说）。
    pub fn seats_full(&self) -> bool {
        matches!(self.seats, Some(n) if n > 0 && self.joined >= n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_reads_as_nothing_to_show() {
        let live: SiteLive = serde_json::from_str("{}").unwrap();
        assert_eq!(live.seats, None);
        assert_eq!(live.joined, 0);
        assert_eq!(live.followers, 0);
        assert!(!live.feedback_public);
        assert!(live.public_feedback.is_empty());
        assert!(!live.seats_full());
    }

    #[test]
    fn seats_full_only_when_a_target_is_set_and_met() {
        let mut live = SiteLive::empty("brisk-otter-41");
        assert!(!live.seats_full());
        live.seats = Some(10);
        live.joined = 9;
        assert!(!live.seats_full());
        live.joined = 10;
        assert!(live.seats_full());
        live.seats = Some(0);
        assert!(!live.seats_full());
    }

    #[test]
    fn key_lives_next_to_the_manifests() {
        assert_eq!(key("brisk-otter-41"), "sites/brisk-otter-41/live.json");
    }
}
