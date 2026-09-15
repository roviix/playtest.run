//! `GET /_playtest/me`：SDK 唯一需要边缘告诉它的东西。
//!
//! 门禁页种下的会话 cookie 是 `HttpOnly`（见 `app.rs` 的 `cookie()`），页面里的脚本读不到。
//! 这是对的：作品里跑的是别人的任意 JS，会话 cookie 不该被它摸到。所以另开这一个同源端点，
//! 由边缘把「你是谁、在玩第几版、事件往哪发」告诉 SDK，cookie 本身仍然留在浏览器里。
//!
//! 没有会话 cookie 就回 204：玩家还没点过「开始」，或者这个作品把门禁页关了。
//! SDK 看到 204 会退回自己在 `localStorage` 里生成的匿名 id——它只认「这是同一个人的第几次」，
//! 一样不含身份（DESIGN §3.4 不收集玩家身份）。
//!
//! 这一页永远不缓存：会话 id 是一人一份的东西，被任何中间层存下来都是灾难。

use axum::http::{HeaderMap, HeaderValue, Method, StatusCode};
use axum::response::{IntoResponse, Response};
use playtest_common::ingest::Me;
use playtest_common::manifest::Manifest;

use crate::config::Config;

/// 事件往哪发。上线时是控制面的地址，本机是另一个端口上的 api。
pub const API_URL_ENV: &str = "PLAYTEST_API_PUBLIC_URL";

pub fn respond(
    config: &Config,
    manifest: &Manifest,
    sid: Option<&str>,
    method: &Method,
) -> Response {
    if method != Method::GET && method != Method::HEAD {
        let mut headers = base();
        put(&mut headers, "allow", "GET, HEAD");
        put(&mut headers, "content-type", "text/plain; charset=utf-8");
        return (
            StatusCode::METHOD_NOT_ALLOWED,
            headers,
            "This address does not accept this kind of request\n",
        )
            .into_response();
    }

    let mut headers = base();
    // 这是个子资源：`--isolated` 的作品在 COEP 下取它要有 CORP，否则被自己的隔离挡住。
    crate::game_headers::isolation(&mut headers, manifest.isolated, true);

    let Some(sid) = sid else {
        return (StatusCode::NO_CONTENT, headers).into_response();
    };

    let me = Me {
        sid: sid.to_string(),
        slug: manifest.slug.clone(),
        version: manifest.version,
        isolated: manifest.isolated,
        api: api_url(config),
    };
    let body = match serde_json::to_string(&me) {
        Ok(body) => body,
        Err(err) => {
            tracing::warn!(%err, "/_playtest/me 的 JSON 拼不出来");
            return (StatusCode::INTERNAL_SERVER_ERROR, headers).into_response();
        }
    };
    put(
        &mut headers,
        "content-type",
        "application/json; charset=utf-8",
    );
    (StatusCode::OK, headers, body).into_response()
}

fn api_url(config: &Config) -> String {
    match std::env::var(API_URL_ENV) {
        // 明确设了（哪怕是空串）就照它的：空串 = 现在没有公网控制面，SDK 看到空地址会安静地不发。
        Ok(url) => url.trim_end_matches('/').to_string(),
        Err(_) => default_api_url(&config.host_suffix).to_string(),
    }
}

/// 没配的时候的落点。本机对着 KICKOFF §3 的约定；上线时不猜——开发者域名还没指过来的时候
/// 猜出来只会让玩家浏览器对一个不存在的域名报错。上线必须显式设 `PLAYTEST_API_PUBLIC_URL`。
fn default_api_url(host_suffix: &str) -> &'static str {
    if host_suffix == "localhost" || host_suffix.ends_with(".localhost") {
        "http://127.0.0.1:8787"
    } else {
        ""
    }
}

fn base() -> HeaderMap {
    let mut headers = HeaderMap::new();
    put(&mut headers, "cache-control", "no-store");
    put(&mut headers, "x-content-type-options", "nosniff");
    headers
}

fn put(headers: &mut HeaderMap, name: &'static str, value: &str) {
    match HeaderValue::from_str(value) {
        Ok(value) => {
            headers.insert(name, value);
        }
        Err(err) => tracing::warn!(name, %err, "响应头的值不合法，丢掉"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use playtest_common::manifest::{GateMode, Manifest};

    fn manifest(isolated: bool) -> Manifest {
        Manifest {
            schema: playtest_common::manifest::SCHEMA,
            slug: "brisk-otter-41".into(),
            version: 7,
            title: "小球".into(),
            developer: "匿名开发者".into(),
            note: None,
            summary: None,
            cover: None,
            created_at: "2026-09-07T04:00:00Z".into(),
            expires_at: None,
            badge: true,
            gate: GateMode::Once,
            isolated,
            spa: false,
            engine: None,
            kind: playtest_common::manifest::WorkKind::Web,
            entry: None,
            article: None,
            chapters: vec![],
            files: Vec::new(),
        }
    }

    fn config() -> Config {
        Config {
            listen: "127.0.0.1:0".parse().unwrap(),
            data_dir: ".data".into(),
            host_suffix: "localhost".into(),
            public_scheme: "http".into(),
            api_internal_url: None,
            edge_ingest_token: None,
        }
    }

    #[test]
    fn without_a_session_cookie_there_is_nothing_to_say() {
        let response = respond(&config(), &manifest(false), None, &Method::GET);
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        assert_eq!(response.headers()["cache-control"], "no-store");
    }

    #[test]
    fn isolated_sites_get_the_resource_policy_header() {
        let plain = respond(
            &config(),
            &manifest(false),
            Some(&"a1".repeat(16)),
            &Method::GET,
        );
        assert!(plain
            .headers()
            .get("cross-origin-resource-policy")
            .is_none());

        let isolated = respond(
            &config(),
            &manifest(true),
            Some(&"a1".repeat(16)),
            &Method::GET,
        );
        assert_eq!(
            isolated.headers()["cross-origin-resource-policy"],
            "same-origin"
        );
    }

    #[test]
    fn only_reads_are_allowed() {
        let response = respond(&config(), &manifest(false), None, &Method::POST);
        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
        assert_eq!(response.headers()["allow"], "GET, HEAD");
    }

    #[test]
    fn the_api_address_follows_the_suffix() {
        assert_eq!(default_api_url("localhost"), "http://127.0.0.1:8787");
        // 上线不猜控制面域名：没配就是空，SDK 会安静地不发。
        assert_eq!(default_api_url("playtest.run"), "");
    }
}
