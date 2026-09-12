//! CLI 与控制面（[`crate::DEVELOPER_API_URL`]）之间的请求与响应体。
//!
//! 路径常量与类型放一起，CLI 和 api 各自引用，改一处两边同时变。
//! 鉴权：`Authorization: Bearer <token>`；匿名令牌由 [`routes::ANON_SESSIONS`] 签发。
//!
//! 上传一个版本的四步（DESIGN §4.2）：
//!
//! 1. `POST /v1/projects`（首次）拿到 slug；
//! 2. `POST /v1/projects/{slug}/uploads` 送全部文件的路径、哈希、大小，拿回缺哪些哈希；
//! 3. `PUT /v1/blobs/{hash}` 只传缺的（可并行、可重试，服务端校验哈希）；
//! 4. `POST /v1/projects/{slug}/uploads/{upload_id}/commit` 提交，拿到 `vN` 与链接。

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::manifest::{Cover, FileEntry, GateMode};

pub mod routes {
    pub const HEALTH: &str = "/healthz";

    // ---- 给 AI 助手读的（REWRITE §3.2）。控制面自己出，不是静态文件——
    // 端点改了文档跟着改，一份写在别处的 API 文档一定会漂。
    /// 一屏说清「这是什么、什么时候用、什么时候别用」。
    pub const LLMS_TXT: &str = "/llms.txt";
    /// 加上完整端点表、配额与失败形态。
    pub const LLMS_FULL_TXT: &str = "/llms-full.txt";
    /// 可以直接装进 Claude / Cursor 的 skill 文件。
    pub const SKILL_MD: &str = "/skill.md";
    pub const OPENAPI_JSON: &str = "/openapi.json";
    pub const AGENT_JSON: &str = "/.well-known/agent.json";
    /// `POST` → [`super::AnonSessionResponse`]
    pub const ANON_SESSIONS: &str = "/v1/anon/sessions";
    /// `GET` 列出我的作品 → `Vec<`[`super::Site`]`>`；`POST` 新建 → 200 [`super::Site`]
    pub const SITES: &str = "/v1/projects";
    /// `GET` → [`super::Site`]；`PATCH` [`super::UpdateSiteRequest`] → 200 [`super::Site`]；`DELETE` → 204
    pub const SITE: &str = "/v1/projects/{slug}";
    /// `POST` → [`super::PrepareUploadResponse`]
    pub const SITE_UPLOADS: &str = "/v1/projects/{slug}/uploads";
    /// `POST` → [`super::CommitUploadResponse`]
    pub const SITE_UPLOAD_COMMIT: &str = "/v1/projects/{slug}/uploads/{upload_id}/commit";
    /// `PUT` 原始字节 → 201（已存在则 200）
    pub const BLOB: &str = "/v1/blobs/{hash}";
    /// `POST` [`crate::tunnel::TunnelRequest`] → 200 [`crate::tunnel::TunnelGrant`]：签一个隧道令牌
    pub const SITE_TUNNEL: &str = "/v1/projects/{slug}/tunnel";
    /// `GET` → 200 [`super::VersionList`]：这个作品发过的每一版，新的在前
    pub const SITE_VERSIONS: &str = "/v1/projects/{slug}/versions";
    /// `POST` → 200 [`super::Site`]：把「当前版本」指针指回某一版（回滚，DESIGN §3.5）。清单不可变，指针一动玩家立刻看到
    pub const SITE_VERSION_ACTIVATE: &str = "/v1/projects/{slug}/versions/{version}/activate";
    /// `GET` → 200 [`super::VersionFiles`]：这一版里到底有哪些文件。
    pub const SITE_VERSION_FILES: &str = "/v1/projects/{slug}/versions/{version}/files";

    // ---- 登录（DESIGN §3.2：`playtest login`，GitHub，一次之后不再问） ----
    //
    // 两条路进同一个账号：终端走 GitHub 的设备码流程（不用回调端口，也不用 client secret），
    // 控制台走网页授权码流程。两条路的最后一步都可以顺带带上手里的匿名令牌，
    // 那个匿名身份下的作品会一起归到账号里、不再 24 小时后失效。

    /// `POST` → 200 [`super::DeviceLoginStart`]：向 GitHub 要一个设备码。控制面代为请求，CLI 不用知道 client_id。
    pub const LOGIN_DEVICE_START: &str = "/v1/login/github/device";
    /// `POST` [`super::DeviceLoginPoll`] → 200 [`super::LoginPollResponse`]。可带 `Authorization: Bearer <匿名令牌>`。
    pub const LOGIN_DEVICE_POLL: &str = "/v1/login/github/device/poll";
    /// `GET` → 302 到 GitHub 的授权页。浏览器直接访问；回来时 GitHub 把 `code` 和 `state` 挂在控制台地址上。
    pub const LOGIN_WEB_START: &str = "/v1/login/github/start";
    /// `POST` [`super::WebLoginExchange`] → 200 [`super::LoginResponse`]。可带 `Authorization: Bearer <匿名令牌>`。
    pub const LOGIN_WEB_EXCHANGE: &str = "/v1/login/github/exchange";
    /// `GET` → 200 [`super::Me`]：这个令牌是谁。
    pub const ME: &str = "/v1/me";
    pub const ME_TOKEN: &str = "/v1/me/token";

    pub fn site_version_files(slug: &str, version: u32) -> String {
        SITE_VERSION_FILES
            .replace("{slug}", slug)
            .replace("{version}", &version.to_string())
    }

    pub fn site_tunnel(slug: &str) -> String {
        SITE_TUNNEL.replace("{slug}", slug)
    }

    pub fn site_versions(slug: &str) -> String {
        SITE_VERSIONS.replace("{slug}", slug)
    }

    pub fn site_version_activate(slug: &str, version: u32) -> String {
        SITE_VERSION_ACTIVATE
            .replace("{slug}", slug)
            .replace("{version}", &version.to_string())
    }

    pub fn site(slug: &str) -> String {
        SITE.replace("{slug}", slug)
    }

    pub fn site_uploads(slug: &str) -> String {
        SITE_UPLOADS.replace("{slug}", slug)
    }

    pub fn site_upload_commit(slug: &str, upload_id: &str) -> String {
        SITE_UPLOAD_COMMIT
            .replace("{slug}", slug)
            .replace("{upload_id}", upload_id)
    }

    pub fn blob(hash: &str) -> String {
        BLOB.replace("{hash}", hash)
    }
}

/// 所有非 2xx 响应的体。`code` 给程序判断，`message` 给人看（中文，第一次用的人看得懂）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ErrorBody {
    pub code: ErrorCode,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    Unauthorized,
    TokenExpired,
    NotFound,
    /// 请求体不合法，`message` 说明哪一项。
    Invalid,
    /// 超出档位配额（体积、文件数、作品数），`message` 说明数字。体积与文件数用 413，作品数用 403。
    QuotaExceeded,
    /// 上传的字节哈希与 URL 里的哈希不一致。
    HashMismatch,
    /// 提交时还有哈希没传上来，`message` 列出前几个。
    BlobsMissing,
    /// slug 被占或保留。
    SlugUnavailable,
    /// 这条隧道令牌对应的会话已被同一作品更新的隧道挤掉（DESIGN §4.3「新的挤掉旧的」）；
    /// 拿着它重连会一直得到这个码，CLI 应退出而不是重试。
    TunnelReplaced,
    /// 混合模式（`playtest ./dist --backend 3000`）里走隧道的那几条路径，在开发者的电脑
    /// 不在线时由边缘回这个码（503）。页面本身照常能开——静态文件是上传过的。
    /// 对端是游戏里的 `fetch`，不是浏览器导航，所以是 JSON 不是一页 HTML。
    BackendOffline,
    /// 这个控制面没配 GitHub 登录（自托管、或本机开发没设 client_id）。`message` 说明匿名链接照常能用。
    LoginUnavailable,
    /// GitHub 那边没走完：设备码过期、用户在授权页点了拒绝、授权码用过了或 state 对不上。
    /// 重新 `playtest login` 一次即可；`message` 会说是哪一种。
    LoginFailed,
    Internal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct AnonSessionResponse {
    pub token: String,
    /// RFC 3339。到期后令牌与它创建的作品一起失效。
    pub expires_at: String,
}

/// GitHub 设备码流程的第一步：CLI 把 `user_code` 和 `verification_uri` 打给人看，然后拿 `device_code` 轮询。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct DeviceLoginStart {
    pub device_code: String,
    /// 给人在浏览器里输入的那串，形如 `WDJB-MJHT`。
    pub user_code: String,
    /// 一般是 `https://github.com/login/device`。
    pub verification_uri: String,
    /// 这个码还能用几秒。
    pub expires_in: u32,
    /// 两次轮询之间至少隔几秒；GitHub 说 `slow_down` 时控制面会把它加大。
    pub interval: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct DeviceLoginPoll {
    pub device_code: String,
}

/// 轮询的结果。`pending` 继续等；`ok` 里就是登录结果。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum LoginPollResponse {
    /// 人还没在浏览器里输完码。`interval` 是下一次至少隔几秒。
    Pending {
        interval: u32,
    },
    Ok(LoginResponse),
}

/// 登录成功。`token` 长期有效，之后所有请求都带它。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct LoginResponse {
    pub token: String,
    /// GitHub 用户名（不带 @）。
    pub login: String,
    /// 玩家在门禁页和广场上看到的名字：GitHub 上的显示名，没有就是用户名。
    pub display_name: String,
    /// 这次顺带归入账号的匿名作品数（请求带了匿名令牌才会大于 0）。
    pub migrated_sites: u32,
}

/// 网页授权码流程的最后一步：控制台把 GitHub 回传的 `code` 与 `state` 交给控制面换令牌。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct WebLoginExchange {
    pub code: String,
    pub state: String,
}

/// `GET /v1/me`。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Me {
    /// `anon` 或 `github`。
    pub kind: String,
    pub display_name: String,
    /// GitHub 用户名；匿名没有。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub login: Option<String>,
    /// 匿名身份的到期时间；登录用户没有。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
    /// GitHub 头像；匿名没有。广场卡片与门禁页上显示（DESIGN §3.9）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct CreateSiteRequest {
    /// 想要的 slug；匿名用户忽略此项，随机分配。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,
    /// 作品名；没给就在上传时用目录名。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Site {
    pub slug: String,
    /// 玩家点开的完整链接，例如 `https://brisk-otter-41.playtest.run`。
    pub url: String,
    pub title: String,
    /// 还没上传过版本时为 `None`。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_version: Option<u32>,
    pub created_at: String,
    /// 匿名作品的到期时间。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
    /// 广场上的状态（DESIGN §3.8）。旧控制面不返回这一段，按「不公开」解析。
    #[serde(default)]
    pub listing: Listing,
}

/// 一个作品在广场（DESIGN §3.8）上的状态。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Listing {
    /// 开发者勾了「放到广场上」。默认不公开。
    #[serde(default)]
    pub public: bool,
    /// 「正在找人测」。只在 `public` 时有意义。
    #[serde(default)]
    pub seeking: bool,
    /// 想让来的人重点看什么，最多 [`crate::limits::MAX_SEEK_NOTE_CHARS`] 字。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seek_note: Option<String>,
    /// 一句话介绍，最多 [`crate::limits::MAX_SUMMARY_CHARS`] 字。随最新版本的清单走。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    /// 被举报到阈值、或我们手工撤下了：`public` 仍是开发者的意愿，但广场上不出现。
    /// 控制台要把这件事告诉开发者，不能让他以为自己在广场上。
    #[serde(default)]
    pub hidden: bool,
    /// 最新版本有没有封面。
    #[serde(default)]
    pub has_cover: bool,
    /// 想找几位试玩者（`--seats`，DESIGN §3.3）；`joined` 是留了名字的人数。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seats: Option<u32>,
    #[serde(default)]
    pub joined: u32,
    /// 关注这个作品的人数（DESIGN §3.6）。开发者只看到数字。
    #[serde(default)]
    pub followers: u32,
    /// 开发者的群（`--community`）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub community_url: Option<String>,
    /// 「让玩家看到彼此的反馈」（DESIGN §3.5）。
    #[serde(default)]
    pub feedback_public: bool,
    /// 推广状态（DESIGN §3.11）：`None` 没有；否则是当前或排队中的那一段。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub boost: Option<crate::boost::Boost>,
}

/// `PATCH /v1/projects/{slug}`：只改带了的字段。
/// `seek_note` / `community_url` 传空字符串表示清掉；`seats` 传 0 表示清掉。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct UpdateSiteRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub public: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seeking: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seek_note: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seats: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub community_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub feedback_public: Option<bool>,
}

/// 一个已发布的版本。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct VersionInfo {
    pub version: u32,
    pub created_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    pub file_count: u32,
    pub total_bytes: u64,
    /// 玩家现在看到的就是这一版。
    pub current: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct VersionList {
    pub slug: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_version: Option<u32>,
    /// 新的在前。
    pub versions: Vec<VersionInfo>,
}

/// 一个版本里的文件清单（`GET /v1/projects/{slug}/versions/{version}/files`）。
///
/// 只有清单和哈希，没有字节：文件本身在作品自己的域上按路径取就行。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct VersionFiles {
    pub slug: String,
    pub version: u32,
    /// 玩家现在看到的就是这一版。
    pub current: bool,
    pub total_bytes: u64,
    /// 按路径排序，和清单里一致。
    pub files: Vec<VersionFile>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct VersionFile {
    pub path: String,
    pub size: u64,
    /// 内容哈希（SHA-256 小写十六进制）。同样的哈希就是同样的字节，不用再传一遍。
    pub hash: String,
    /// 直接能打开的地址。
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PrepareUploadRequest {
    pub files: Vec<FileEntry>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// 一句话介绍（DESIGN §3.8）。没给就沿用这个作品上一版的。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    /// 封面。CLI 先把它当普通 blob 传上来（`PUT /v1/blobs/{hash}`），提交时服务端检查它在不在。
    /// 没给就沿用上一版的封面。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cover: Option<Cover>,
    #[serde(default)]
    pub gate: GateMode,
    #[serde(default)]
    pub isolated: bool,
    #[serde(default)]
    pub spa: bool,
    /// CLI 上传时认出来的引擎，见 [`crate::manifest::Manifest::engine`]。认不出来就没有。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub engine: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PrepareUploadResponse {
    pub upload_id: String,
    /// 服务端没有的哈希，CLI 只传这些。去重后、按清单顺序。
    pub missing: Vec<String>,
    /// 这次要传的字节数之和，给进度条用。
    pub missing_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct CommitUploadResponse {
    pub slug: String,
    pub version: u32,
    pub url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn route_helpers_fill_placeholders() {
        assert_eq!(
            routes::site("brisk-otter-41"),
            "/v1/projects/brisk-otter-41"
        );
        assert_eq!(
            routes::site_upload_commit("a", "u1"),
            "/v1/projects/a/uploads/u1/commit"
        );
        assert_eq!(routes::blob("ff"), "/v1/blobs/ff");
    }

    #[test]
    fn error_code_is_snake_case() {
        let body = ErrorBody {
            code: ErrorCode::QuotaExceeded,
            message: "太大了".into(),
        };
        let json = serde_json::to_string(&body).unwrap();
        assert!(json.contains("\"quota_exceeded\""));
    }
}
