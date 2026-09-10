//! 关注与通知的契约（DESIGN §3.6、§4.10）。
//!
//! 参与方有三个，各自只认这里的形状：
//!
//! - **玩家的浏览器**：在作品的门禁页、SDK 的玩后落点、或根域的广场上填一个邮箱或允许浏览器通知。
//!   它把表单交给**自己所在的域**（作品子域上的 [`edge_paths::FOLLOW`]，根域上的 [`root_paths::FOLLOW`]），
//!   从不直接把邮箱交给另一个域（DESIGN §4.1）。
//! - **边缘**：把登记、确认、退订同步转给控制面（[`routes`]），拿到回答再渲染给玩家。
//!   这是边缘对控制面少数几个「写」方向的同步调用；控制面不在时边缘如实说「现在登记不了」。
//! - **控制面**：存 `players` / `follows` / `notifications`，发确认信，签 `pt_me`（[`crate::ME_COOKIE`]）用的令牌。
//!
//! 身份从行为里长出来：确认信里的链接落在根域的 [`root_paths::ME_CONFIRM`]，边缘拿令牌换一个 `me_token`
//! 种进 `pt_me`；此后这台设备上的「关注」是一下点击。**没有密码、没有注册页。**
//!
//! 开发者永远看不到邮箱，只看到数字（[`crate::live::SiteLive::followers`]）。

use serde::{Deserialize, Serialize};

/// 控制面上的路径（边缘 → 控制面，内网；控制台不用这些）。
pub mod routes {
    /// `POST` [`super::FollowRequest`] → 200 [`super::FollowResponse`]
    pub const FOLLOW: &str = "/v1/follow";
    /// `POST` [`super::ConfirmRequest`] → 200 [`super::ConfirmResponse`]：确认信里的令牌换 `me_token`
    pub const CONFIRM: &str = "/v1/follow/confirm";
    /// `POST` [`super::UnsubscribeRequest`] → 200 [`super::MeView`]：信底那个一键退订
    pub const UNSUBSCRIBE: &str = "/v1/follow/unsubscribe";
    /// `POST` [`super::MeRequest`] → 200 [`super::MeView`]：「我的」那一页要显示的东西
    pub const ME_VIEW: &str = "/v1/me/view";
    /// `POST` [`super::UnfollowRequest`] → 200 [`super::MeView`]：在「我的」里取消一项
    pub const ME_UNFOLLOW: &str = "/v1/me/unfollow";
    /// `POST` [`super::SendLinkRequest`] → 200 [`super::FollowResponse`]：给自己的邮箱再发一条能落到「我的」的链接（换设备用）
    pub const ME_SEND_LINK: &str = "/v1/me/send-link";
}

/// 作品子域上，边缘接玩家表单的路径（在 [`crate::RESERVED_PATH_PREFIX`] 下）。
pub mod edge_paths {
    /// `POST`：门禁页与 SDK 玩后落点的「有新版本时告诉我」。表单字段见 [`super::form`]。
    pub const FOLLOW: &str = "/_playtest/follow";
}

/// 根域（广场所在的那一个主机名）上的路径。
pub mod root_paths {
    /// `POST`：「我的」里留下邮箱就是关注广场。表单字段见 [`super::form`]。
    pub const FOLLOW: &str = "/follow";
    /// `GET`：「我的」（DESIGN §3.10）。
    pub const ME: &str = "/me";
    /// `GET /me/confirm/{token}`：确认信里的链接。成功后种 `pt_me`、303 到 [`ME`]。
    pub const ME_CONFIRM: &str = "/me/confirm/";
    /// `GET /me/unsubscribe/{token}`：每封信底部的一键退订。不问为什么，点了就退。
    pub const ME_UNSUBSCRIBE: &str = "/me/unsubscribe/";
    /// `POST`：「我的」里的动作（取消关注、给自己发链接、关掉浏览器通知）。
    pub const ME_ACTION: &str = "/me/action";
}

/// 玩家表单的字段名。边缘解析表单后拼成 [`FollowRequest`]。
pub mod form {
    /// `site:<slug>` 或 `plaza`，见 [`super::FollowTarget`]。
    pub const TARGET: &str = "target";
    pub const EMAIL: &str = "email";
    /// 浏览器 `PushSubscription.toJSON()` 的 JSON 字符串，见 [`super::PushSubscription`]。
    pub const PUSH: &str = "push";
    /// 从哪一页来的：`gate` / `sdk` / `plaza` / `me`。进 `follows.source`，只用来看哪个入口有效。
    pub const FROM: &str = "from";
    /// 表单提交完回到哪（同源相对路径）。
    pub const TO: &str = "to";
}

/// 一个作品，或广场本身。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FollowTarget {
    Site { slug: String },
    Plaza,
}

impl FollowTarget {
    /// 表单里那个字段的写法：`site:<slug>` / `plaza`。
    pub fn parse(raw: &str) -> Option<Self> {
        let raw = raw.trim();
        if raw == "plaza" {
            return Some(Self::Plaza);
        }
        let slug = raw.strip_prefix("site:")?;
        crate::slug::validate(slug).ok()?;
        Some(Self::Site {
            slug: slug.to_string(),
        })
    }

    pub fn form_value(&self) -> String {
        match self {
            Self::Site { slug } => format!("site:{slug}"),
            Self::Plaza => "plaza".to_string(),
        }
    }
}

/// 浏览器 `PushSubscription.toJSON()` 的形状。原样存、原样用，我们不解读里面的字节。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PushSubscription {
    pub endpoint: String,
    pub keys: PushKeys,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PushKeys {
    pub p256dh: String,
    pub auth: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FollowChannel {
    /// 要双重确认：先发确认信，点了才算关注。
    Email { email: String },
    /// 浏览器已经问过用户了，直接生效。
    Push { subscription: PushSubscription },
    /// 这台设备已经有 `pt_me`：一下点击，不再要邮箱。
    Me { me_token: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FollowRequest {
    pub target: FollowTarget,
    pub channel: FollowChannel,
    /// 见 [`form::FROM`]。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum FollowResponse {
    /// 邮箱路径：确认信已经在路上（或在队列里——邮件服务商不可用时也是这个，玩家不该看到报错）。
    ConfirmSent,
    /// 推送或 `pt_me` 路径：已经生效。
    Subscribed,
    /// 早就关注着了。
    AlreadyFollowing,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfirmRequest {
    pub token: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfirmResponse {
    /// 种进 `pt_me` 的值。长期有效，撤销靠退订。
    pub me_token: String,
    pub me: MeView,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnsubscribeRequest {
    pub token: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeRequest {
    pub me_token: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnfollowRequest {
    pub me_token: String,
    pub target: FollowTarget,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SendLinkRequest {
    pub email: String,
}

/// 「我的」那一页（DESIGN §3.10）：一个抽屉，不是一个 profile。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeView {
    /// 打码显示的邮箱，如 `z***@example.com`；只用浏览器通知、没留邮箱的人没有。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email_masked: Option<String>,
    /// 这个人开了浏览器通知。
    #[serde(default)]
    pub push: bool,
    #[serde(default)]
    pub follows: Vec<FollowView>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FollowView {
    pub target: FollowTarget,
    /// 作品名；广场那一项没有。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// 作品链接；广场那一项没有。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// RFC 3339。
    pub since: String,
}

/// 同一作品给同一个人的通知，两封之间至少隔这么久（DESIGN §3.6：一天发五版只收到一封）。
pub const PER_SITE_NOTICE_INTERVAL_HOURS: i64 = 24;

/// 周报：每周几（ISO，1 = 周一）、几点（Asia/Shanghai，UTC+8）。
pub const DIGEST_ISO_WEEKDAY: u8 = 1;
pub const DIGEST_HOUR: u8 = 9;
pub const DIGEST_UTC_OFFSET_HOURS: i8 = 8;

/// 周报里最多放几个作品。
pub const DIGEST_MAX_ITEMS: usize = 12;

/// 通知与周报里链接上带的来源参数值，见 [`crate::FROM_NOTICE`]。
pub fn notice_url(site_url: &str) -> String {
    format!(
        "{}/?{}={}",
        site_url.trim_end_matches('/'),
        crate::FROM_PARAM,
        crate::FROM_NOTICE
    )
}

/// 邮箱打码：`zhongshangwu@example.com` → `z***@example.com`。
pub fn mask_email(email: &str) -> String {
    match email.split_once('@') {
        Some((user, domain)) => {
            let first = user
                .chars()
                .next()
                .map(|c| c.to_string())
                .unwrap_or_default();
            format!("{first}***@{domain}")
        }
        None => "***".to_string(),
    }
}

/// 邮箱形状的最低检查：有且只有一个 `@`，两边都非空，域名里有点。真正的校验是那封确认信。
pub fn looks_like_email(raw: &str) -> bool {
    let raw = raw.trim();
    if raw.len() > 254 || raw.chars().any(char::is_whitespace) {
        return false;
    }
    match raw.split_once('@') {
        Some((user, domain)) => {
            !user.is_empty()
                && !domain.is_empty()
                && domain.contains('.')
                && !domain.starts_with('.')
                && !domain.ends_with('.')
                && !domain.contains('@')
        }
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_round_trips_through_the_form_value() {
        let site = FollowTarget::parse("site:brisk-otter-41").unwrap();
        assert_eq!(site.form_value(), "site:brisk-otter-41");
        assert_eq!(FollowTarget::parse("plaza"), Some(FollowTarget::Plaza));
        assert_eq!(FollowTarget::parse("site:../etc"), None);
        assert_eq!(FollowTarget::parse("developer:x"), None);
    }

    #[test]
    fn email_check_is_loose_but_not_silly() {
        assert!(looks_like_email("a@b.co"));
        assert!(looks_like_email(" someone@example.com "));
        assert!(!looks_like_email("someone"));
        assert!(!looks_like_email("@example.com"));
        assert!(!looks_like_email("a@b"));
        assert!(!looks_like_email("a b@c.d"));
        assert!(!looks_like_email("a@@b.c"));
    }

    #[test]
    fn masking_keeps_only_the_first_letter_and_the_domain() {
        assert_eq!(mask_email("zhongshangwu@example.com"), "z***@example.com");
        assert_eq!(mask_email("x@y.z"), "x***@y.z");
        assert_eq!(mask_email("garbage"), "***");
    }

    #[test]
    fn notice_links_carry_the_source() {
        assert_eq!(
            notice_url("https://brisk-otter-41.playtest.run"),
            "https://brisk-otter-41.playtest.run/?from=notice"
        );
    }

    #[test]
    fn json_shapes_are_tagged_and_snake_case() {
        let req = FollowRequest {
            target: FollowTarget::Site {
                slug: "brisk-otter-41".into(),
            },
            channel: FollowChannel::Email {
                email: "a@b.co".into(),
            },
            from: Some("gate".into()),
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"kind\":\"site\""));
        assert!(json.contains("\"kind\":\"email\""));
        let back: FollowRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(back, req);

        let resp = serde_json::to_string(&FollowResponse::ConfirmSent).unwrap();
        assert_eq!(resp, "{\"status\":\"confirm_sent\"}");
    }
}
