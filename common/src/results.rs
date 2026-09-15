//! 结果层读出来的东西：控制台（开发者域名下的 `/console/`）问，控制面答。
//!
//! 写进去的那一侧在 [`crate::ingest`]（玩家的浏览器和边缘往里塞），这边是**开发者带令牌来看**，
//! 所以两组的形状要求正好相反：那边要收得紧，这边要给得够，够到能拼出 DESIGN §3.4 的那段话。
//!
//! 三个端点（[`routes`]）：
//!
//! - [`routes::SITE_RESULTS`]：作品时间线，每版一条 [`VersionResults`]，版本倒序；
//! - [`routes::SITE_VERSION_SESSIONS`]：一版的点名册，每人一行 [`SessionRow`]；
//! - [`routes::SITE_FEEDBACK`]：反馈流 [`FeedbackItem`]，[`routes::SITE_FEEDBACK_ITEM`] 改状态。
//!
//! **只有计数和中位数，没有比例、没有平均值**（DESIGN §3.4）。5–50 个人身上「37% 流失」
//! 既算不准也没法行动，「3 个人在加载时走了」可以——那是三个具体的人，点开就能看见是谁。
//! 同理这里一个字段都不为画图准备：控制台不画图表（DESIGN §3.7）。
//!
//! 那段话本身不在这里生成。这一层只给数，句子由控制台拼——同一组数字在手机上、
//! 在将来的 `--json` 里、在 `playtest mcp` 的回答里要说成不同的话，把措辞钉死在服务端
//! 只会让每一处都得先把句子拆回数字。

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

fn nullable_u32(_gen: &mut schemars::generate::SchemaGenerator) -> schemars::Schema {
    let serde_json::Value::Object(map) = serde_json::json!({
        "type": ["integer", "null"],
        "minimum": 0,
        "format": "uint32"
    }) else {
        unreachable!()
    };
    schemars::Schema::from(map)
}

pub mod routes {
    /// `GET` → [`super::SiteResults`]
    pub const SITE_RESULTS: &str = "/v1/projects/{slug}/results";
    /// `GET`（`?sort=dwell|time`）→ [`super::VersionSessions`]
    pub const SITE_VERSION_SESSIONS: &str = "/v1/projects/{slug}/versions/{version}/sessions";
    /// `GET`（`?version=N`、`?status=new|seen|done`）→ [`super::FeedbackList`]
    pub const SITE_FEEDBACK: &str = "/v1/projects/{slug}/feedback";
    /// `PATCH` [`super::UpdateFeedbackRequest`] → [`super::FeedbackItem`]
    pub const SITE_FEEDBACK_ITEM: &str = "/v1/projects/{slug}/feedback/{id}";

    pub fn site_results(slug: &str) -> String {
        SITE_RESULTS.replace("{slug}", slug)
    }

    pub fn site_version_sessions(slug: &str, version: u32) -> String {
        SITE_VERSION_SESSIONS
            .replace("{slug}", slug)
            .replace("{version}", &version.to_string())
    }

    pub fn site_feedback(slug: &str) -> String {
        SITE_FEEDBACK.replace("{slug}", slug)
    }

    pub fn site_feedback_item(slug: &str, id: i64) -> String {
        SITE_FEEDBACK_ITEM
            .replace("{slug}", slug)
            .replace("{id}", &id.to_string())
    }
}

/// 「玩了 5 分钟以上」的那条线（DESIGN §3.4 的示例句）。
pub const LONG_PLAY_SECONDS: u32 = 300;

/// 一版最多列几条错误。列多了就成了错误列表页，这里要的是「最该先看的那一两条」。
pub const TOP_ERRORS: usize = 3;

/// 展开一个会话时最多带几条事件。一次正常的试玩不会有这么多，
/// 有这么多的多半是循环里在打点，截断之后 [`SessionRow::more_events`] 会说明。
pub const MAX_EVENTS_PER_SESSION: usize = 100;

/// 作品时间线：一屏能看完的全部（DESIGN §3.4「点开是这一版的会话列表」的上一层）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct SiteResults {
    pub slug: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_version: Option<u32>,
    /// 版本倒序，最新的在最前面。
    pub versions: Vec<VersionResults>,
    /// 关注这个作品的人数（DESIGN §3.6）——「下一版发出去他们会收到通知」那一句的数字。
    #[serde(default)]
    pub followers: u32,
}

/// 一个版本的全部数字。控制台按这些拼那段话，数为 0 的句子不说。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct VersionResults {
    pub version: u32,
    /// 这一版上传的时间。只在会话里见过、版本表里没有的版本是 `None`。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    /// 上传时附的那句「这版改了什么」（DESIGN §3.5）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// 多少人打开。一次打开就是一个会话。
    pub opened: u32,
    /// 多少人真的进到游戏：点过「开始」，或者 SDK 报过 `load`。
    pub entered: u32,
    /// L7：点了「开始」却没等到首帧的人数——门禁到首帧之间掉的那几个（DESIGN §3.4）。
    ///
    /// `None` 表示**这一版我们不知道**：没有任何会话报过首帧，多半是没接 SDK。
    /// 不知道就说不知道，不拿 0 冒充「一个都没掉」（AGENTS 第 4 条）。
    #[schemars(schema_with = "nullable_u32")]
    pub dropped_before_first_frame: Option<u32>,
    /// 回头再来一次的人数。
    pub returned: u32,
    /// 停留秒数的中位数，见 [`median_seconds`]。没有会话时是 `None`。
    #[schemars(schema_with = "nullable_u32")]
    pub dwell_median_s: Option<u32>,
    /// 停留超过 [`LONG_PLAY_SECONDS`] 的人数。
    pub played_5min_plus: u32,
    pub errors: ErrorSummary,
    /// 边缘报的 `resource_fail` 条数：404、下到一半断了、MIME 不对。
    pub load_failures: u32,
    pub feedback_count: u32,
    /// 这一版第一次和最后一次被打开的时间。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_at: Option<String>,
    /// 来自哪里，按人数降序（DESIGN §3.5「来自：邀请卡 4 · 广场 2 · 微信 2」）。
    /// 键是 [`crate::ingest::source`] 里的值。0 的不列。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sources: Vec<SourceTally>,
    /// 留了名字的人数。
    #[serde(default)]
    pub named: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct SourceTally {
    pub kind: String,
    pub count: u32,
}

/// 错误按 fingerprint 归堆之后的样子。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ErrorSummary {
    /// 去重之后有几种。
    pub distinct: u32,
    /// 一共撞了几次。
    pub total: u32,
    /// 撞得最多的几条，最多 [`TOP_ERRORS`] 条。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub top: Vec<ErrorTally>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ErrorTally {
    /// SDK 报上来的 `name`，通常是「错误类型 + 出错的那一行」。
    pub fingerprint: String,
    pub count: u32,
}

/// 点名册的排法。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum RosterSort {
    /// 停留最短的排最前面——排在最前面的人就是你要看的人（DESIGN §3.4）。默认。
    #[default]
    Dwell,
    /// 最近打开的排最前面。
    Time,
}

impl std::str::FromStr for RosterSort {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "dwell" => Ok(Self::Dwell),
            "time" => Ok(Self::Time),
            other => Err(format!(
                "The roster can only be sorted by dwell (shortest stay first) or time (most recent first), not {other}"
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct VersionSessions {
    pub slug: String,
    pub version: u32,
    pub sort: RosterSort,
    pub sessions: Vec<SessionRow>,
}

/// 点名册里的一个人。每一列都要能回答「我下一步该看谁」，回答不了的列不加。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SessionRow {
    pub id: String,
    /// 打开的时间。
    pub at: String,
    /// 点「开始」时留的名字（DESIGN §3.3 第 5 条）。没留就没有，点名册显示会话 id 的头几位。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub browser: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub os: Option<String>,
    pub wechat: bool,
    /// [`crate::ingest::source`] 里的一个：card / notice / plaza / wechat / discord / direct / other，尽力而为。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub referrer_kind: Option<String>,
    /// 点过门禁页的「开始」。
    pub started: bool,
    /// 报过首帧。要 SDK，没接的作品这一列永远是 false。
    pub first_frame: bool,
    /// 进到游戏了没。
    pub entered: bool,
    /// 停留秒数：最后一次看见减第一次看见。
    pub dwell_s: u32,
    /// 最后一次输入距「进入」多久。没接 SDK、或者一次都没动过是 `None`。
    #[schemars(required)]
    pub last_input_after_s: Option<u32>,
    /// 玩到哪：最后一个自定义事件的名字。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reached: Option<String>,
    pub errors: u32,
    /// 留了几句话。
    pub feedback: u32,
    pub is_return: bool,
    /// 展开这一行看到的：这个会话的事件与错误，时间正序，最多 [`MAX_EVENTS_PER_SESSION`] 条。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub events: Vec<SessionEvent>,
    /// 事件多到被截断了。
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub more_events: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SessionEvent {
    pub ts: String,
    /// edge / sdk。
    pub source: String,
    /// [`crate::ingest::kind`] 或 [`crate::ingest::edge_kind`] 里的一个。
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum FeedbackStatus {
    /// 还没看。
    New,
    /// 看过了。
    Seen,
    /// 处理完了。
    Done,
}

impl FeedbackStatus {
    pub fn as_db(self) -> &'static str {
        match self {
            Self::New => "new",
            Self::Seen => "seen",
            Self::Done => "done",
        }
    }

    /// 库里出现没见过的字面量时当作「还没看」：一条反馈宁可多看一次，也不该因为
    /// 状态列脏了就从流里消失。
    pub fn from_db(value: &str) -> Self {
        match value {
            "seen" => Self::Seen,
            "done" => Self::Done,
            _ => Self::New,
        }
    }
}

impl std::str::FromStr for FeedbackStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "new" => Ok(Self::New),
            "seen" => Ok(Self::Seen),
            "done" => Ok(Self::Done),
            other => Err(format!(
                "Feedback status has to be new, seen or done, not {other}"
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct FeedbackList {
    pub slug: String,
    /// 时间倒序，最新的在最前面。
    pub items: Vec<FeedbackItem>,
}

/// 一条反馈：一句话 + 让这句话能被定位的上下文（DESIGN §3.4）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct FeedbackItem {
    pub id: i64,
    pub session_id: String,
    pub version: u32,
    pub ts: String,
    pub text: String,
    /// 进入多久说的这句话。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seconds_in: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub browser: Option<String>,
    /// 截图 v0.2 才做，现在永远是 `None`（DESIGN §3.5 说的「附截图」还没实现）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub screenshot_hash: Option<String>,
    pub status: FeedbackStatus,
    /// 说这句话的人在门禁页留的名字（DESIGN §3.5）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// 这一条在门禁页上公开着（作品开了公开反馈、且开发者没把它单独藏起来）。
    #[serde(default)]
    pub public: bool,
}

/// `PATCH /v1/projects/{slug}/feedback/{id}`：只改带了的字段。
/// `public: Some(false)` 是「把这一条藏起来」——作品级的开关在 [`crate::api::UpdateSiteRequest::feedback_public`]。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct UpdateFeedbackRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<FeedbackStatus>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub public: Option<bool>,
}

/// 停留秒数的中位数。
///
/// 偶数个的时候取**偏小的那一个**，不取两个的平均：这一层里出现的每一个数字都该是
/// 某一个真实会话的秒数，点名册里能找到那一行。平均数在 5–50 个人身上还容易被
/// 一个开着标签页去吃饭的人拽走（DESIGN §3.4 只给中位数和逐条明细）。
pub fn median_seconds(values: &mut [u32]) -> Option<u32> {
    if values.is_empty() {
        return None;
    }
    values.sort_unstable();
    Some(values[(values.len() - 1) / 2])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn route_helpers_fill_placeholders() {
        assert_eq!(
            routes::site_results("brisk-otter-41"),
            "/v1/projects/brisk-otter-41/results"
        );
        assert_eq!(
            routes::site_version_sessions("brisk-otter-41", 7),
            "/v1/projects/brisk-otter-41/versions/7/sessions"
        );
        assert_eq!(
            routes::site_feedback_item("brisk-otter-41", 12),
            "/v1/projects/brisk-otter-41/feedback/12"
        );
    }

    #[test]
    fn median_is_always_somebody_real() {
        assert_eq!(median_seconds(&mut []), None);
        assert_eq!(median_seconds(&mut [42]), Some(42));
        assert_eq!(median_seconds(&mut [10, 20, 30]), Some(20));
        // 偶数个：取偏小的那个，不是 15。
        assert_eq!(median_seconds(&mut [10, 20]), Some(10));
        assert_eq!(median_seconds(&mut [40, 10, 30, 20]), Some(20));
        // 顺序无关。
        assert_eq!(median_seconds(&mut [30, 10, 20]), Some(20));
    }

    #[test]
    fn sort_and_status_parse_from_query_strings() {
        assert_eq!("dwell".parse::<RosterSort>().unwrap(), RosterSort::Dwell);
        assert_eq!("time".parse::<RosterSort>().unwrap(), RosterSort::Time);
        assert!("按停留".parse::<RosterSort>().is_err());

        assert_eq!(
            "done".parse::<FeedbackStatus>().unwrap(),
            FeedbackStatus::Done
        );
        assert!("closed".parse::<FeedbackStatus>().is_err());
        assert_eq!(FeedbackStatus::from_db("坏了"), FeedbackStatus::New);
    }

    #[test]
    fn unknown_l7_is_null_not_zero() {
        let json = serde_json::to_value(VersionResults {
            version: 7,
            created_at: None,
            note: None,
            opened: 3,
            entered: 3,
            dropped_before_first_frame: None,
            returned: 0,
            dwell_median_s: Some(12),
            played_5min_plus: 0,
            errors: ErrorSummary::default(),
            load_failures: 0,
            feedback_count: 0,
            first_at: None,
            last_at: None,
            sources: vec![],
            named: 0,
        })
        .unwrap();
        assert!(json["dropped_before_first_frame"].is_null());
        // 一个比例、一个平均值都不该出现在这一层。
        assert!(json.get("rate").is_none());
        assert!(json.get("dwell_avg_s").is_none());
    }
}
