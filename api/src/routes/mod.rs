//! 路由表。路径全部取自 [`playtest_common::api::routes`]，改契约时这里跟着变。

pub mod blobs;
pub mod events;
pub mod feedback;
pub mod results;
pub mod sessions;
pub mod sites;
pub mod tunnel;
pub mod uploads;

use axum::extract::{FromRequest, Request};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, patch, post, put};
use axum::{middleware, Extension, Json, Router};
use playtest_common::api::{routes as paths, ErrorBody, ErrorCode};
use playtest_common::ingest::routes as ingest_paths;
use playtest_common::limits;
use playtest_common::results::routes as result_paths;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::trace::TraceLayer;

use crate::error::{ApiError, INTERNAL_MESSAGE};
use crate::state::AppState;

pub fn app(state: AppState) -> Router {
    Router::new()
        .route(paths::HEALTH, get(health))
        .route(paths::ANON_SESSIONS, post(sessions::create))
        .route(paths::SITES, get(sites::list).post(sites::create))
        .route(paths::SITE, get(sites::show).delete(sites::remove))
        .route(paths::SITE_UPLOADS, post(uploads::prepare))
        .route(paths::SITE_UPLOAD_COMMIT, post(uploads::commit))
        .route(paths::SITE_TUNNEL, post(tunnel::grant))
        .route(result_paths::SITE_RESULTS, get(results::timeline))
        .route(result_paths::SITE_VERSION_SESSIONS, get(results::sessions))
        .route(result_paths::SITE_FEEDBACK, get(results::feedback))
        .route(
            result_paths::SITE_FEEDBACK_ITEM,
            patch(results::update_feedback),
        )
        // 玩家的浏览器直连这三个：不带令牌，只认 Origin 和令牌桶（见 events.rs）。
        .route(
            ingest_paths::EVENTS,
            post(events::from_sdk).options(events::preflight),
        )
        .route(
            ingest_paths::EDGE,
            post(events::from_edge).options(events::preflight),
        )
        .route(
            ingest_paths::FEEDBACK,
            post(feedback::submit).options(events::preflight),
        )
        .route(
            paths::BLOB,
            // 上传是流式的，请求体从头到尾不进内存，所以体积上限要在层里挡，
            // 不能靠那些「先收完再解析」的提取器。
            put(blobs::upload).layer(RequestBodyLimitLayer::new(limits::MAX_FILE_BYTES as usize)),
        )
        .fallback(unknown_route)
        // 写入端点的令牌桶。一个 app() 一份，进程内存里（见 events.rs）。
        .layer(Extension(events::Limiter::new()))
        .layer(middleware::map_response(as_error_body))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

async fn health() -> &'static str {
    "ok"
}

async fn unknown_route() -> ApiError {
    ApiError::not_found("没有这个地址。请确认 CLI 和控制面的版本对得上。")
}

/// 兜底：把中间层产生的裸响应（方法不对、体积超限）也补成 [`ErrorBody`]，
/// 客户端才能只写一套解析逻辑。
async fn as_error_body(response: Response) -> Response {
    if response.status().is_success() || is_json(&response) {
        return response;
    }
    let status = response.status();
    let (code, message) = describe(status);
    (status, Json(ErrorBody { code, message })).into_response()
}

fn is_json(response: &Response) -> bool {
    response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.starts_with("application/json"))
}

fn describe(status: StatusCode) -> (ErrorCode, String) {
    match status {
        StatusCode::METHOD_NOT_ALLOWED => (
            ErrorCode::Invalid,
            "这个地址不接受这种请求方法。".to_string(),
        ),
        StatusCode::PAYLOAD_TOO_LARGE => (ErrorCode::QuotaExceeded, too_large_message()),
        StatusCode::UNSUPPORTED_MEDIA_TYPE => (
            ErrorCode::Invalid,
            "请求内容的类型不对，这个地址要 application/json。".to_string(),
        ),
        StatusCode::NOT_FOUND => (
            ErrorCode::NotFound,
            "没有这个地址。请确认 CLI 和控制面的版本对得上。".to_string(),
        ),
        _ if status.is_server_error() => (ErrorCode::Internal, INTERNAL_MESSAGE.to_string()),
        _ => (ErrorCode::Invalid, "这个请求我们处理不了。".to_string()),
    }
}

pub fn too_large_message() -> String {
    format!(
        "单个文件最多 {} MB，这个超了。",
        limits::MAX_FILE_BYTES / limits::MIB
    )
}

/// 和 [`axum::Json`] 一样，只是把解析失败也换成中文的 [`ErrorBody`]。
pub struct JsonBody<T>(pub T);

impl<T, S> FromRequest<S> for JsonBody<T>
where
    T: serde::de::DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request(request: Request, state: &S) -> Result<Self, Self::Rejection> {
        match Json::<T>::from_request(request, state).await {
            Ok(Json(value)) => Ok(Self(value)),
            Err(rejection) => {
                // 原文是英文，只进日志；给人看的是下面那句。
                tracing::debug!(rejection = %rejection, "请求体解析失败");
                Err(ApiError::invalid(
                    "请求内容不是我们认识的格式：可能不是合法的 JSON，也可能少了必填字段。请确认 CLI 和控制面的版本对得上。",
                ))
            }
        }
    }
}
