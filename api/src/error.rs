//! 失败怎么变成响应。
//!
//! 两种失败分得很开：[`ApiError::Public`] 是能直接给人看的（配额、路径不合法、令牌过期），
//! `message` 就是终端里会出现的那句话；[`ApiError::Internal`] 是我们自己的问题，
//! 客户端只会看到一句「服务器出错了」，真正的原因进日志。

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use playtest_common::api::{ErrorBody, ErrorCode};

/// 500 对外只说这一句。把内部错误原文吐给客户端等于把库结构和路径也一起吐出去。
pub const INTERNAL_MESSAGE: &str = "服务器出错了，请稍后重试";

pub type ApiResult<T> = Result<T, ApiError>;

#[derive(Debug)]
pub enum ApiError {
    Public {
        status: StatusCode,
        code: ErrorCode,
        message: String,
    },
    Internal(anyhow::Error),
}

impl ApiError {
    pub fn public(status: StatusCode, code: ErrorCode, message: impl Into<String>) -> Self {
        Self::Public {
            status,
            code,
            message: message.into(),
        }
    }

    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self::public(StatusCode::UNAUTHORIZED, ErrorCode::Unauthorized, message)
    }

    pub fn token_expired(message: impl Into<String>) -> Self {
        Self::public(StatusCode::UNAUTHORIZED, ErrorCode::TokenExpired, message)
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::public(StatusCode::NOT_FOUND, ErrorCode::NotFound, message)
    }

    pub fn invalid(message: impl Into<String>) -> Self {
        Self::public(StatusCode::BAD_REQUEST, ErrorCode::Invalid, message)
    }

    /// 数量类配额（作品个数）。403：请求本身没问题，是这个档位不让做。
    pub fn quota(message: impl Into<String>) -> Self {
        Self::public(StatusCode::FORBIDDEN, ErrorCode::QuotaExceeded, message)
    }

    /// 体积类配额（单文件、单版本、文件个数）。413 让 CLI 不用读 message 就知道是「太大了」。
    pub fn too_large(message: impl Into<String>) -> Self {
        Self::public(
            StatusCode::PAYLOAD_TOO_LARGE,
            ErrorCode::QuotaExceeded,
            message,
        )
    }

    pub fn hash_mismatch(message: impl Into<String>) -> Self {
        Self::public(StatusCode::BAD_REQUEST, ErrorCode::HashMismatch, message)
    }

    pub fn blobs_missing(message: impl Into<String>) -> Self {
        Self::public(StatusCode::CONFLICT, ErrorCode::BlobsMissing, message)
    }

    pub fn slug_unavailable(message: impl Into<String>) -> Self {
        Self::public(StatusCode::CONFLICT, ErrorCode::SlugUnavailable, message)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, body) = match self {
            ApiError::Public {
                status,
                code,
                message,
            } => (status, ErrorBody { code, message }),
            ApiError::Internal(err) => {
                tracing::error!(error = format!("{err:#}"), "这个请求没能处理完");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    ErrorBody {
                        code: ErrorCode::Internal,
                        message: INTERNAL_MESSAGE.to_string(),
                    },
                )
            }
        };
        (status, Json(body)).into_response()
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(err: anyhow::Error) -> Self {
        Self::Internal(err)
    }
}

impl From<rusqlite::Error> for ApiError {
    fn from(err: rusqlite::Error) -> Self {
        Self::Internal(anyhow::Error::new(err).context("读写数据库失败"))
    }
}

impl From<playtest_common::store::StoreError> for ApiError {
    fn from(err: playtest_common::store::StoreError) -> Self {
        Self::Internal(anyhow::Error::new(err).context("读写对象存储失败"))
    }
}

impl From<std::io::Error> for ApiError {
    fn from(err: std::io::Error) -> Self {
        Self::Internal(anyhow::Error::new(err).context("读写文件失败"))
    }
}

impl From<serde_json::Error> for ApiError {
    fn from(err: serde_json::Error) -> Self {
        Self::Internal(anyhow::Error::new(err).context("JSON 编解码失败"))
    }
}
