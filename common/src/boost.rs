//! 推广位（DESIGN §3.11）与管理接口（DESIGN §4.5）。
//!
//! 一个作品买（或被赠送）一个时间窗，窗内它出现在广场顶部的推广位上，**永远标「推广」**，
//! 每屏最多 [`MAX_SLOTS`] 张，超出的排队。免费流一个字不动。
//!
//! 购买通道要等境外主体与支付（DESIGN §9）；在那之前只有运营者能通过管理接口赠送，
//! 控制台里「推广」一节如实写「尚未开放」。所以这里先有数据模型和管理接口，没有下单接口——
//! 下单接口等 Stripe 接上再加，形状会是「创建订单 → 支付回调 → 一条 `Pending` 的 [`Boost`]」。

use serde::{Deserialize, Serialize};

/// 广场同一屏上最多几张推广位。两张：看得见、又不至于把免费流推到第二屏。
pub const MAX_SLOTS: usize = 2;

/// 周报里单列的「推广」一段最多几个。
pub const MAX_DIGEST_SLOTS: usize = 2;

/// 管理接口。只在开发者域，`Authorization: Bearer <PLAYTEST_ADMIN_TOKEN>`；令牌没配就整组 404。
pub mod routes {
    /// `GET` → 200 `Vec<`[`super::Boost`]`>`（含排队中的）；`POST` [`super::GrantBoostRequest`] → 200 [`super::Boost`]：赠送一段推广
    pub const BOOSTS: &str = "/admin/boosts";
    /// `POST` [`super::ReviewBoostRequest`] → 200 [`super::Boost`]：人工看过之后放行或拒掉
    pub const BOOST_REVIEW: &str = "/admin/boosts/{id}/review";
    /// `DELETE` → 204：提前结束一段推广（作品被举报撤下时控制面自己也会调这条逻辑）
    pub const BOOST: &str = "/admin/boosts/{id}";
    /// `POST` → 204：手工把一个作品从广场撤下（不删作品，链接照常）；再 `DELETE` 恢复
    pub const PLAZA_HIDE: &str = "/admin/plaza/{slug}/hide";
    /// `GET` → 200 [`super::NotificationQueue`]：通知队列长什么样
    pub const NOTIFICATIONS: &str = "/admin/notifications";

    pub fn boost_review(id: i64) -> String {
        BOOST_REVIEW.replace("{id}", &id.to_string())
    }

    pub fn boost(id: i64) -> String {
        BOOST.replace("{id}", &id.to_string())
    }

    pub fn plaza_hide(slug: &str) -> String {
        PLAZA_HIDE.replace("{slug}", slug)
    }
}

/// 两个 SKU 加一个附加项（DESIGN §6）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BoostKind {
    /// 推广 3 天。
    Days3,
    /// 推广 7 天。
    Days7,
    /// 进本周周报（不占广场推广位）。
    Digest,
}

impl BoostKind {
    /// 广场推广位的时长；周报那一项不占位，是 `None`。
    pub fn days(self) -> Option<u32> {
        match self {
            Self::Days3 => Some(3),
            Self::Days7 => Some(7),
            Self::Digest => None,
        }
    }

    /// 初始定价，美元（DESIGN §6）。支付宝按当日汇率。
    pub fn price_usd(self) -> u32 {
        match self {
            Self::Days3 => 9,
            Self::Days7 => 19,
            Self::Digest => 9,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BoostStatus {
    /// 付了钱（或被赠送），等人工看一眼。
    Pending,
    /// 在推广位上（或已排入本周周报）。
    Live,
    /// 窗口过了。
    Ended,
    /// 人工没放行。付了钱的全额退。
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Boost {
    pub id: i64,
    pub slug: String,
    pub kind: BoostKind,
    pub status: BoostStatus,
    /// 运营者赠送的，不是买的。广场上的标签一样是「推广」，不区分。
    #[serde(default)]
    pub granted: bool,
    /// RFC 3339。`Pending` 时是排到的那一天；周报那一项是那一期发出的时间。
    pub starts_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ends_at: Option<String>,
    pub created_at: String,
    /// 支付订单号；赠送的没有。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrantBoostRequest {
    pub slug: String,
    pub kind: BoostKind,
    /// 不给就是「最早能排上的那一天」。RFC 3339。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub starts_at: Option<String>,
    /// 赠送的默认直接 `Live`（运营者自己就是审核的人）；传 `true` 让它走 `Pending`。
    #[serde(default)]
    pub review: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewBoostRequest {
    pub approve: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotificationQueue {
    pub pending: u32,
    pub sent_24h: u32,
    pub failed_24h: u32,
    /// 进了死信的（DESIGN §4.10：失败三次）。
    pub dead: u32,
    /// 下一期周报什么时候发，RFC 3339。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_digest_at: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds_carry_their_days_and_prices() {
        assert_eq!(BoostKind::Days3.days(), Some(3));
        assert_eq!(BoostKind::Days7.days(), Some(7));
        assert_eq!(BoostKind::Digest.days(), None);
        assert_eq!(BoostKind::Days3.price_usd(), 9);
        assert_eq!(BoostKind::Days7.price_usd(), 19);
    }

    #[test]
    fn json_is_snake_case() {
        let json = serde_json::to_string(&BoostKind::Days3).unwrap();
        assert_eq!(json, "\"days3\"");
        let json = serde_json::to_string(&BoostStatus::Pending).unwrap();
        assert_eq!(json, "\"pending\"");
    }

    #[test]
    fn route_helpers_fill_placeholders() {
        assert_eq!(routes::boost_review(7), "/admin/boosts/7/review");
        assert_eq!(routes::boost(7), "/admin/boosts/7");
        assert_eq!(
            routes::plaza_hide("brisk-otter-41"),
            "/admin/plaza/brisk-otter-41/hide"
        );
    }
}
