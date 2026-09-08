//! `GET /_playtest/sdk.js`：把 `playtest.js` 从作品自己的域上发出去（DESIGN §4.6）。
//!
//! 为什么由边缘发而不是放 CDN 或控制面：玩家页面「不引第三方脚本」是硬线，同源就没有 CORS、
//! 没有第二个域名要信任；开发者要加的那一行也最短——`<script src="/_playtest/sdk.js"></script>`。
//! 脚本在构建期编进二进制（`sdk/dist/playtest.js`），服务器上不需要 Node。
//!
//! 上传模式的「自动注入」还没做：那要改开发者的 HTML，DESIGN §3.4 说是「可以勾选」，
//! v0.1 先让开发者自己加这一行。

use axum::http::{HeaderMap, HeaderValue, Method, StatusCode};
use axum::response::{IntoResponse, Response};

const SCRIPT: &[u8] = include_bytes!("../../sdk/dist/playtest.js");

/// 内容哈希做 ETag：SDK 换版本，浏览器缓存自然失效。
fn etag() -> String {
    format!("\"{}\"", &playtest_common::hash::hash_bytes(SCRIPT)[..16])
}

pub fn respond(method: &Method, request_headers: &HeaderMap) -> Response {
    let mut headers = HeaderMap::new();
    let put = |h: &mut HeaderMap, k: &'static str, v: &str| {
        if let Ok(value) = HeaderValue::from_str(v) {
            h.insert(k, value);
        }
    };
    put(&mut headers, "content-type", "text/javascript; charset=utf-8");
    put(&mut headers, "x-content-type-options", "nosniff");
    // 内容按哈希变，可以缓存一天；换版本靠 ETag 不一致触发重取。
    put(&mut headers, "cache-control", "public, max-age=86400");
    let tag = etag();
    put(&mut headers, "etag", &tag);

    if method != Method::GET && method != Method::HEAD {
        put(&mut headers, "allow", "GET, HEAD");
        return (StatusCode::METHOD_NOT_ALLOWED, headers).into_response();
    }
    if request_headers
        .get("if-none-match")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.split(',').any(|t| t.trim() == tag))
    {
        return (StatusCode::NOT_MODIFIED, headers).into_response();
    }
    put(&mut headers, "content-length", &SCRIPT.len().to_string());
    if method == Method::HEAD {
        return (StatusCode::OK, headers).into_response();
    }
    (StatusCode::OK, headers, SCRIPT).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn script_is_the_built_sdk() {
        assert!(SCRIPT.len() > 1000);
        assert!(std::str::from_utf8(SCRIPT).unwrap().contains("playtest"));
    }

    #[test]
    fn get_returns_js_and_etag_round_trips_to_304() {
        let first = respond(&Method::GET, &HeaderMap::new());
        assert_eq!(first.status(), StatusCode::OK);
        assert_eq!(
            first.headers().get("content-type").unwrap(),
            "text/javascript; charset=utf-8"
        );
        let tag = first.headers().get("etag").unwrap().clone();

        let mut h = HeaderMap::new();
        h.insert("if-none-match", tag);
        assert_eq!(respond(&Method::GET, &h).status(), StatusCode::NOT_MODIFIED);
        assert_eq!(respond(&Method::POST, &HeaderMap::new()).status(), StatusCode::METHOD_NOT_ALLOWED);
    }
}
