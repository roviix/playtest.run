//! 玩家浏览器写进来的东西：SDK 的事件与反馈，以及边缘补送的第一层事件。
//!
//! 和 [`crate::api`] 那一组的区别是**谁在调**：那边是开发者的 CLI，带令牌；这边是玩家的浏览器，
//! 不带任何身份（DESIGN §3.4 不收集玩家身份），所以形状要收得很紧——
//! 类型是白名单、条数有上限、版本号一律由服务端解析，客户端说什么都不算。
//!
//! 三个端点（[`routes`]）：
//!
//! - [`routes::EVENTS`]：SDK 的 `error` / `load` / `event` / `input`，一批最多 [`MAX_EVENTS_PER_BATCH`] 条；
//! - [`routes::FEEDBACK`]：玩家写的一句话，每个会话最多 [`MAX_FEEDBACK_PER_SESSION`] 条；
//! - [`routes::EDGE`]：边缘 `edge/src/events.rs` 现在写在本地 JSONL 里的那种行，批量补送。
//!
//! 会话 id 来自门禁页种下的 `pt_sid`（[`crate::SESSION_COOKIE`]）。它是 `HttpOnly` 的，
//! 页面里的脚本读不到，所以边缘另开一个同源端点 [`ME_PATH`] 把它告诉 SDK。

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub mod routes {
    /// `POST` [`super::EventBatch`] → [`super::Accepted`]
    pub const EVENTS: &str = "/v1/ingest/events";
    /// `POST` [`super::FeedbackRequest`] → [`super::FeedbackAccepted`]
    pub const FEEDBACK: &str = "/v1/ingest/feedback";
    /// `POST` [`super::EdgeBatch`] → [`super::Accepted`]
    pub const EDGE: &str = "/v1/ingest/edge";
}

/// 边缘的同源端点：SDK 从这里知道自己是谁、往哪发。没有会话 cookie 时回 204。
pub const ME_PATH: &str = "/_playtest/me";

/// 一批最多几条。SDK 是 1 秒合并或 20 条就发，50 是给退出时补发留的余量。
pub const MAX_EVENTS_PER_BATCH: usize = 50;

/// 自定义事件名 / 错误 fingerprint 的长度上限。
pub const MAX_NAME_CHARS: usize = 200;

/// 一条事件 `data` 序列化后的字节上限。错误堆栈占大头，SDK 那边截到 2 KB。
pub const MAX_DATA_BYTES: usize = 4096;

/// 一条反馈的字数上限。
pub const MAX_FEEDBACK_CHARS: usize = 2000;

/// 一个会话最多能提几条反馈。防的是刷，不是防说话——玩家想再说一句还有两次机会。
pub const MAX_FEEDBACK_PER_SESSION: u32 = 3;

/// SDK 能报的事件类型。不在这张表里的一律拒绝：这个端点不鉴权，能落进库的形状必须是闭集。
pub mod kind {
    /// 加载分阶段完成，`data.ms` 是「打开到首帧」的毫秒数。
    pub const LOAD: &str = "load";
    /// 最后一次输入。不是每次输入都发，退出时补一条（DESIGN §3.4）。
    pub const INPUT: &str = "input";
    /// JS 错误或未处理的 Promise 拒绝，`name` 是 fingerprint。
    pub const ERROR: &str = "error";
    /// 开发者自己打的点：`playtest.event("level_done", {level: 3})`。
    pub const EVENT: &str = "event";

    pub const ALL: [&str; 4] = [LOAD, INPUT, ERROR, EVENT];

    pub fn known(kind: &str) -> bool {
        ALL.contains(&kind)
    }
}

/// 边缘报的类型。`breaker_trip` / `resource_fail` 现在还没有产生方，先把名字定下来。
pub mod edge_kind {
    pub const GATE_VIEW: &str = "gate_view";
    pub const START: &str = "start";
    pub const HTML_VIEW: &str = "html_view";
    pub const REPORT: &str = "report";
    pub const BREAKER_TRIP: &str = "breaker_trip";
    pub const RESOURCE_FAIL: &str = "resource_fail";

    pub const ALL: [&str; 6] = [
        GATE_VIEW,
        START,
        HTML_VIEW,
        REPORT,
        BREAKER_TRIP,
        RESOURCE_FAIL,
    ];

    pub fn known(kind: &str) -> bool {
        ALL.contains(&kind)
    }
}

/// 边缘对 SDK 的自我介绍（[`ME_PATH`]）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Me {
    /// 门禁页种下的会话 id。
    pub sid: String,
    pub slug: String,
    pub version: u32,
    /// 作品开了跨源隔离。SDK 知道之后不去碰会被 COEP 挡下的东西。
    pub isolated: bool,
    /// 事件往哪发，例如 [`crate::DEVELOPER_API_URL`]。
    pub api: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct EventBatch {
    /// 会话 id，来自 [`Me::sid`]；SDK 拿不到时是它自己在 localStorage 里生成的匿名 id。
    pub session: String,
    pub slug: String,
    #[serde(default)]
    pub events: Vec<Event>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Event {
    /// 客户端的 RFC 3339 时间。客户端的钟不可信，服务端只在它落在合理窗口里时采信。
    pub ts: String,
    /// [`kind`] 里的一个。
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// 边缘 JSONL 里的一行，字段名和 `edge/src/events.rs` 写出来的一致。
///
/// **没有 IP 字段**，和那边一样：DESIGN §3.4 不收集精确位置，IP 是最容易顺手记下的那一样。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct EdgeEvent {
    pub ts: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub slug: String,
    pub version: u32,
    pub sid: String,
    #[serde(default)]
    pub ua: String,
    #[serde(default)]
    pub referer: String,
    #[serde(default)]
    pub wechat: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    /// 作品链接上的 `?from=`（[`crate::FROM_PARAM`]），门禁页放进「开始」表单带过来的。
    /// 只认 [`source::CARD`] 与 [`source::NOTICE`]，别的值当没有。比 Referer 可信，优先用它（[`source_kind`]）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
    /// 点「开始」时留的名字（DESIGN §3.3 第 5 条），只在 `start` 事件上；边缘已按 [`crate::limits::MAX_PLAYER_NAME_CHARS`] 截过。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// 「来自哪里」的全部取值（DESIGN §3.5）。点名册与时间线里的 `referrer_kind` / `sources` 用这些词。
pub mod source {
    /// 扫邀请卡二维码。
    pub const CARD: &str = "card";
    /// 从关注通知或周报点进来。
    pub const NOTICE: &str = "notice";
    /// 从广场点进来。
    pub const PLAZA: &str = "plaza";
    pub const COLLECTION: &str = "collection";
    pub const WECHAT: &str = "wechat";
    pub const DISCORD: &str = "discord";
    pub const DIRECT: &str = "direct";
    pub const OTHER: &str = "other";

    /// 控制台显示用的中文。
    pub fn label(kind: &str) -> &'static str {
        match kind {
            CARD => "邀请卡",
            NOTICE => "通知",
            PLAZA => "广场",
            COLLECTION => "合集",
            WECHAT => "微信",
            DISCORD => "Discord",
            DIRECT => "直接打开",
            _ => "其它",
        }
    }
}

/// 来源判定的完整版：`from` 参数（卡、通知）优先于 Referer 与 UA。
///
/// 为什么 `from` 优先：扫卡的人多半在微信里，UA 会说 `wechat`，但开发者想知道的是「这个人是我发出去的卡带来的」，
/// 微信只是他扫码的地方；通知同理。`from` 是我们自己放上去的，比 Referer 干净。
pub fn source_kind(
    from: Option<&str>,
    referer: &str,
    wechat: bool,
    plaza_host: &str,
) -> &'static str {
    match from.map(str::trim) {
        Some(source::CARD) => source::CARD,
        Some(source::NOTICE) => source::NOTICE,
        Some(source::COLLECTION) => source::COLLECTION,
        _ => referrer_kind(referer, wechat, plaza_host),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct EdgeBatch {
    #[serde(default)]
    pub events: Vec<EdgeEvent>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct FeedbackRequest {
    pub session: String,
    pub slug: String,
    pub text: String,
    /// 玩家进来多少秒了。SDK 自己算，用来在点名册里显示「进入 47 秒」。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seconds_in: Option<u32>,
}

/// 收下了几条。被形状挡掉的不算在里面，但整批不会因为一条坏的就全退。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Accepted {
    pub accepted: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct FeedbackAccepted {
    /// 这个会话还能再提几条。
    pub remaining: u32,
}

/// 会话 id 的形态：32 个十六进制字符（边缘的 `new_session_id`，SDK 的匿名回退也照这个长）。
pub fn is_session_id(value: &str) -> bool {
    value.len() == 32 && value.bytes().all(|b| b.is_ascii_hexdigit())
}

/// 从 UA 粗分出来的三样。够回答「这个人用什么打开的」就行，不做设备指纹。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Client {
    /// phone / tablet / desktop
    pub device: &'static str,
    /// chrome / safari / firefox / wechat / other
    pub browser: &'static str,
    /// ios / android / windows / macos / linux / other
    pub os: &'static str,
}

/// 微信内置浏览器。X5 内核的能力缺口都从这里判断（DESIGN §5），
/// 边缘 `events.rs` 里那一份判断的是同一个词。
pub fn is_wechat_ua(ua: &str) -> bool {
    ua.contains("MicroMessenger")
}

pub fn classify_ua(ua: &str) -> Client {
    Client {
        device: device_of(ua),
        browser: browser_of(ua),
        os: os_of(ua),
    }
}

fn os_of(ua: &str) -> &'static str {
    if ua.contains("Android") {
        "android"
    } else if ua.contains("iPhone") || ua.contains("iPad") || ua.contains("iPod") {
        "ios"
    } else if ua.contains("Windows") {
        "windows"
    } else if ua.contains("Mac OS X") || ua.contains("Macintosh") {
        "macos"
    } else if ua.contains("Linux") || ua.contains("X11") {
        "linux"
    } else {
        "other"
    }
}

fn device_of(ua: &str) -> &'static str {
    if ua.contains("iPad") || (ua.contains("Android") && !ua.contains("Mobile")) {
        "tablet"
    } else if ua.contains("Mobile") || ua.contains("iPhone") || ua.contains("iPod") {
        "phone"
    } else {
        "desktop"
    }
}

/// 只分五类。Edge、Opera、UC、QQ 都归 other——它们的内核已经在上面那三类里，
/// 点名册要回答的是「这一版在谁的浏览器上坏了」，再细分只会让 5–50 行的表更难读。
fn browser_of(ua: &str) -> &'static str {
    if is_wechat_ua(ua) {
        "wechat"
    } else if ua.contains("Firefox") || ua.contains("FxiOS") {
        "firefox"
    } else if ua.contains("Edg/")
        || ua.contains("OPR/")
        || ua.contains("UCBrowser")
        || ua.contains("QQBrowser")
    {
        "other"
    } else if ua.contains("CriOS") || ua.contains("Chrome") || ua.contains("Chromium") {
        "chrome"
    } else if ua.contains("Safari") {
        "safari"
    } else {
        "other"
    }
}

/// 来自哪里。尽力而为：referrer 本身脏，微信常常一个字都不发，所以 UA 里认出微信就以它为准。
///
/// `plaza_host` 是广场所在的根域（如 `playtest.run`，本机是 `localhost`），从那里点进来的
/// 记成 `plaza`——开发者据此知道广场有没有真的给他带来人（DESIGN §3.8、§8 T9）。
pub fn referrer_kind(referer: &str, wechat: bool, plaza_host: &str) -> &'static str {
    if wechat {
        return "wechat";
    }
    let host = host_of(referer);
    if host.is_empty() {
        "direct"
    } else if !plaza_host.is_empty() && host.eq_ignore_ascii_case(plaza_host) {
        "plaza"
    } else if host.contains("discord") {
        "discord"
    } else if host.contains("weixin") || host.contains("wechat") || host.ends_with("qq.com") {
        "wechat"
    } else {
        "other"
    }
}

/// Referer 是作品自己的某一页（门禁页 → 303 → 作品，或作品内部跳转）。
/// 这种「来源」不含玩家从哪来的信息，调用方应当当作不知道，而不是记成「其它」。
pub fn is_self_referral(referer: &str, slug: &str) -> bool {
    let host = host_of(referer).to_ascii_lowercase();
    host.starts_with(&format!("{slug}.")) || host == slug
}

/// `https://host:443/path` → `host`。解析不出来就当没有。
pub fn host_of(url: &str) -> &str {
    let rest = url
        .split_once("://")
        .map(|(_, rest)| rest)
        .unwrap_or_default();
    let rest = rest.split('/').next().unwrap_or_default();
    let rest = rest.rsplit_once('@').map(|(_, h)| h).unwrap_or(rest);
    rest.split(':').next().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_ids_match_what_the_edge_hands_out() {
        assert!(is_session_id(&"a1".repeat(16)));
        assert!(!is_session_id("../../etc"));
        assert!(!is_session_id(""));
        assert!(!is_session_id(&"a".repeat(31)));
    }

    #[test]
    fn wechat_wins_over_everything_else() {
        let ua = "Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) AppleWebKit/605.1.15 \
                  (KHTML, like Gecko) Mobile/15E148 MicroMessenger/8.0.49(0x18003128) NetType/WIFI";
        let c = classify_ua(ua);
        assert_eq!(c.browser, "wechat");
        assert_eq!(c.os, "ios");
        assert_eq!(c.device, "phone");
    }

    #[test]
    fn coarse_buckets_cover_the_common_four() {
        let android = classify_ua(
            "Mozilla/5.0 (Linux; Android 14; Pixel 8) AppleWebKit/537.36 (KHTML, like Gecko) \
             Chrome/140.0.0.0 Mobile Safari/537.36",
        );
        assert_eq!(
            (android.device, android.browser, android.os),
            ("phone", "chrome", "android")
        );

        let mac = classify_ua(
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 \
             (KHTML, like Gecko) Version/18.0 Safari/605.1.15",
        );
        assert_eq!(
            (mac.device, mac.browser, mac.os),
            ("desktop", "safari", "macos")
        );

        let ipad = classify_ua(
            "Mozilla/5.0 (iPad; CPU OS 17_0 like Mac OS X) AppleWebKit/605.1.15 \
             (KHTML, like Gecko) Version/17.0 Mobile/15E148 Safari/604.1",
        );
        assert_eq!(
            (ipad.device, ipad.browser, ipad.os),
            ("tablet", "safari", "ios")
        );

        let win = classify_ua(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) \
             Chrome/140.0.0.0 Safari/537.36 Edg/140.0.0.0",
        );
        assert_eq!(
            (win.device, win.browser, win.os),
            ("desktop", "other", "windows")
        );

        let nothing = classify_ua("curl/8.7.1");
        assert_eq!(
            (nothing.device, nothing.browser, nothing.os),
            ("desktop", "other", "other")
        );
    }

    #[test]
    fn referrer_is_best_effort() {
        const ROOT: &str = "playtest.run";
        assert_eq!(referrer_kind("", false, ROOT), "direct");
        assert_eq!(referrer_kind("", true, ROOT), "wechat");
        assert_eq!(
            referrer_kind("https://discord.com/channels/1/2", false, ROOT),
            "discord"
        );
        assert_eq!(
            referrer_kind("https://mp.weixin.qq.com/s/abc", false, ROOT),
            "wechat"
        );
        assert_eq!(
            referrer_kind("https://news.ycombinator.com/", false, ROOT),
            "other"
        );
        assert_eq!(referrer_kind("垃圾", false, ROOT), "direct");
        // 从广场点进来的（DESIGN §3.8）：Referer 是根域本身。子域不算——那是作品自己。
        assert_eq!(referrer_kind("https://playtest.run/", false, ROOT), "plaza");
        assert_eq!(
            referrer_kind("https://PLAYTEST.run/?f=seeking", false, ROOT),
            "plaza"
        );
        assert_eq!(
            referrer_kind("http://localhost:8443/", false, "localhost"),
            "plaza"
        );
        assert_eq!(
            referrer_kind("https://other.playtest.run/", false, ROOT),
            "other"
        );
        assert_eq!(referrer_kind("https://playtest.run/", false, ""), "other");
        assert!(is_self_referral(
            "https://brisk-otter-41.playtest.run/",
            "brisk-otter-41"
        ));
        assert!(is_self_referral(
            "http://brisk-otter-41.localhost:8443/game",
            "brisk-otter-41"
        ));
        assert!(!is_self_referral(
            "https://discord.com/channels/1/2",
            "brisk-otter-41"
        ));
        assert!(!is_self_referral(
            "https://brisk-otter-411.playtest.run/",
            "brisk-otter-41"
        ));
    }

    #[test]
    fn from_param_beats_referer_and_wechat() {
        const ROOT: &str = "playtest.run";
        assert_eq!(source_kind(Some("card"), "", true, ROOT), "card");
        assert_eq!(
            source_kind(Some("notice"), "https://playtest.run/", false, ROOT),
            "notice"
        );
        // 不认识的值当没有，退回 Referer 的判断。
        assert_eq!(
            source_kind(Some("tiktok"), "https://playtest.run/", false, ROOT),
            "plaza"
        );
        assert_eq!(source_kind(None, "", true, ROOT), "wechat");
        assert_eq!(source::label("card"), "邀请卡");
        assert_eq!(source::label("whatever"), "其它");
    }

    #[test]
    fn edge_lines_keep_the_jsonl_field_names() {
        let line = r#"{"ts":"2026-09-07T04:00:00Z","type":"gate_view","slug":"brisk-otter-41",
                       "version":7,"sid":"abc","ua":"curl/8","referer":"","wechat":false}"#;
        let event: EdgeEvent = serde_json::from_str(line).unwrap();
        assert_eq!(event.kind, edge_kind::GATE_VIEW);
        assert_eq!(event.version, 7);
        assert!(event.reason.is_none());

        let back = serde_json::to_value(&event).unwrap();
        assert_eq!(back["type"], "gate_view");
        assert!(back.get("ip").is_none(), "IP 连字段都不该有");
    }

    #[test]
    fn only_known_kinds_get_through() {
        assert!(kind::known("load"));
        assert!(!kind::known("pageview"));
        assert!(edge_kind::known("start"));
        assert!(!edge_kind::known("start;DROP"));
    }
}
