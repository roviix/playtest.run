//! CLI 与控制面（`api.playtest.sh`）之间的请求与响应体。
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

use crate::manifest::{FileEntry, GateMode};

pub mod routes {
    pub const HEALTH: &str = "/healthz";
    /// `POST` → [`super::AnonSessionResponse`]
    pub const ANON_SESSIONS: &str = "/v1/anon/sessions";
    /// `GET` 列出我的作品 → `Vec<`[`super::Site`]`>`；`POST` 新建 → 200 [`super::Site`]
    pub const SITES: &str = "/v1/sites";
    /// `GET` → [`super::Site`]；`DELETE` → 204
    pub const SITE: &str = "/v1/sites/{slug}";
    /// `POST` → [`super::PrepareUploadResponse`]
    pub const SITE_UPLOADS: &str = "/v1/sites/{slug}/uploads";
    /// `POST` → [`super::CommitUploadResponse`]
    pub const SITE_UPLOAD_COMMIT: &str = "/v1/sites/{slug}/uploads/{upload_id}/commit";
    /// `PUT` 原始字节 → 201（已存在则 200）
    pub const BLOB: &str = "/v1/blobs/{hash}";
    /// `POST` [`crate::tunnel::TunnelRequest`] → 200 [`crate::tunnel::TunnelGrant`]：签一个隧道令牌
    pub const SITE_TUNNEL: &str = "/v1/sites/{slug}/tunnel";

    pub fn site_tunnel(slug: &str) -> String {
        SITE_TUNNEL.replace("{slug}", slug)
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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrepareUploadRequest {
    pub files: Vec<FileEntry>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
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
