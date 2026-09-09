//! CLI 与控制面（[`crate::DEVELOPER_API_URL`]）之间的请求与响应体。
//!
//! 路径常量与类型放一起，CLI 和 api 各自引用，改一处两边同时变。
//! 鉴权：`Authorization: Bearer <token>`；匿名令牌由 [`routes::ANON_SESSIONS`] 签发。
//!
//! 上传一个版本的四步（DESIGN §4.2）：
//!
//! 1. `POST /v1/sites`（首次）拿到 slug；
//! 2. `POST /v1/sites/{slug}/uploads` 送全部文件的路径、哈希、大小，拿回缺哪些哈希；
//! 3. `PUT /v1/blobs/{hash}` 只传缺的（可并行、可重试，服务端校验哈希）；
//! 4. `POST /v1/sites/{slug}/uploads/{upload_id}/commit` 提交，拿到 `vN` 与链接。

use serde::{Deserialize, Serialize};

use crate::manifest::{Cover, FileEntry, GateMode};

pub mod routes {
    pub const HEALTH: &str = "/healthz";
    /// `POST` → [`super::AnonSessionResponse`]
    pub const ANON_SESSIONS: &str = "/v1/anon/sessions";
    /// `GET` 列出我的作品 → `Vec<`[`super::Site`]`>`；`POST` 新建 → 200 [`super::Site`]
    pub const SITES: &str = "/v1/sites";
    /// `GET` → [`super::Site`]；`PATCH` [`super::UpdateSiteRequest`] → 200 [`super::Site`]；`DELETE` → 204
    pub const SITE: &str = "/v1/sites/{slug}";
    /// `POST` → [`super::PrepareUploadResponse`]
    pub const SITE_UPLOADS: &str = "/v1/sites/{slug}/uploads";
    /// `POST` → [`super::CommitUploadResponse`]
    pub const SITE_UPLOAD_COMMIT: &str = "/v1/sites/{slug}/uploads/{upload_id}/commit";
    /// `PUT` 原始字节 → 201（已存在则 200）
    pub const BLOB: &str = "/v1/blobs/{hash}";
    /// `POST` [`crate::tunnel::TunnelRequest`] → 200 [`crate::tunnel::TunnelGrant`]：签一个隧道令牌
    pub const SITE_TUNNEL: &str = "/v1/sites/{slug}/tunnel";
    /// `GET` → 200 [`super::VersionList`]：这个作品发过的每一版，新的在前
    pub const SITE_VERSIONS: &str = "/v1/sites/{slug}/versions";
    /// `POST` → 200 [`super::Site`]：把「当前版本」指针指回某一版（回滚，DESIGN §3.5）。清单不可变，指针一动玩家立刻看到
    pub const SITE_VERSION_ACTIVATE: &str = "/v1/sites/{slug}/versions/{version}/activate";

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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorBody {
    pub code: ErrorCode,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
    Internal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnonSessionResponse {
    pub token: String,
    /// RFC 3339。到期后令牌与它创建的作品一起失效。
    pub expires_at: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateSiteRequest {
    /// 想要的 slug；匿名用户忽略此项，随机分配。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,
    /// 作品名；没给就在上传时用目录名。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
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
}

/// `PATCH /v1/sites/{slug}`：只改带了的字段。`seek_note` 传空字符串表示清掉。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateSiteRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub public: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seeking: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seek_note: Option<String>,
}

/// 一个已发布的版本。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionList {
    pub slug: String,
    pub current_version: Option<u32>,
    /// 新的在前。
    pub versions: Vec<VersionInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrepareUploadResponse {
    pub upload_id: String,
    /// 服务端没有的哈希，CLI 只传这些。去重后、按清单顺序。
    pub missing: Vec<String>,
    /// 这次要传的字节数之和，给进度条用。
    pub missing_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
        assert_eq!(routes::site("brisk-otter-41"), "/v1/sites/brisk-otter-41");
        assert_eq!(
            routes::site_upload_commit("a", "u1"),
            "/v1/sites/a/uploads/u1/commit"
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
