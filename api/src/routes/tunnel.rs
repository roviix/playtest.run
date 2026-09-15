//! 隧道令牌：`playtest 5173` 之前的那一步（DESIGN §4.3、§4.5）。
//!
//! CLI 拿着这里签出来的短期令牌去连边缘，边缘只用公钥验签、不回源问我们——
//! 所以令牌里要带够边缘渲染门禁页和判配额的全部东西，签完这一刻控制面就可以不在了。
//! 令牌本身不落库也不进日志，撤销靠有效期（一小时）；撤销名单是后面的事。

use axum::extract::{Path, State};
use axum::Json;
use playtest_common::tunnel::{Claims, TunnelGrant, TunnelRequest, TOKEN_TTL_SECS, WS_PATH};
use time::Duration;
use uuid::Uuid;

use crate::auth::Caller;
use crate::clock;
use crate::db;
use crate::error::{ApiError, ApiResult};
use crate::routes::sites::NO_SUCH_SITE;
use crate::routes::uploads::clean_title;
use crate::routes::JsonBody;
use crate::state::AppState;

/// 一个 slug 同时能有多少个玩家连着（DESIGN §6 免费档「每 slug 并发 50 人」）。
/// v0.1 只有匿名和免费两档，数字一样，所以先放在这里；出现第二个数字时再进 `common/src/limits.rs`。
const MAX_PLAYERS: u32 = 50;

pub async fn grant(
    State(state): State<AppState>,
    caller: Caller,
    Path(slug): Path<String>,
    JsonBody(request): JsonBody<TunnelRequest>,
) -> ApiResult<Json<TunnelGrant>> {
    let site = {
        let conn = state.db().lock().await;
        db::find_live_site(&conn, &slug, &caller.user_id)?
    }
    .ok_or_else(|| ApiError::not_found(NO_SUCH_SITE))?;

    // 隧道模式没有上传，作品名只能从这次请求来；没给就用建作品时那个。
    let title = clean_title(request.title.as_deref())?.unwrap_or_else(|| site.title.clone());
    // 混合模式的分界线是上传的清单：没有清单就没有分界线，玩家点开首页只会看到 404。
    if request.hybrid && site.current_version.is_none() {
        return Err(ApiError::invalid(
            "Hybrid mode needs an uploaded version first: files in the directory are served from the edge, and only paths that are not in it reach your backend. Run playtest ./dist once.",
        ));
    }

    let issued_at = clock::now();
    let expires_at = token_expiry(issued_at, site.expires_at.as_deref());
    if expires_at <= issued_at {
        return Err(ApiError::token_expired(
            "The 24 hours of this anonymous project are up, so no new tunnel can open. Run playtest again to get a new link.",
        ));
    }
    let claims = Claims {
        v: 1,
        slug: site.slug.clone(),
        sub: caller.user_id.clone(),
        title,
        developer: caller.display_name.clone(),
        // v0.1 只有匿名和免费档，两档都带角标（DESIGN §3.3）。
        badge: true,
        gate: request.gate,
        isolated: request.isolated,
        max_players: MAX_PLAYERS,
        hybrid: request.hybrid,
        iat: issued_at.unix_timestamp(),
        exp: expires_at.unix_timestamp(),
        jti: Uuid::new_v4().to_string(),
    };
    let token = state.tunnel_key().sign(&claims);

    let url = state.site_url(&site.slug);
    let connect_url = connect_url(&url);

    tracing::info!(slug = %site.slug, jti = %claims.jti, exp = claims.exp, "签发了一个隧道令牌");
    Ok(Json(TunnelGrant {
        slug: site.slug,
        url,
        connect_url,
        token,
        expires_at: clock::format(expires_at),
        site_expires_at: site.expires_at,
    }))
}

/// 令牌活不过作品：匿名作品还剩十分钟时签出的令牌也只剩十分钟，否则边缘会在作品到期后
/// 继续认它一小时（边缘只验签不回源，DESIGN §4.5）。`site_expires_at` 解析不了就当没有到期。
fn token_expiry(
    issued_at: time::OffsetDateTime,
    site_expires_at: Option<&str>,
) -> time::OffsetDateTime {
    let token_exp = issued_at + Duration::seconds(TOKEN_TTL_SECS);
    match site_expires_at.and_then(clock::parse) {
        Some(site_exp) if site_exp < token_exp => site_exp,
        _ => token_exp,
    }
}

/// 握手地址和玩家链接是同一个 Host，只换协议——边缘就是按 Host 找隧道会话的。
/// 认不出协议就原样留着：宁可 CLI 报「这个地址连不上」，也不要我们猜一个错的出来。
fn connect_url(site_url: &str) -> String {
    let base = if let Some(rest) = site_url.strip_prefix("https://") {
        format!("wss://{rest}")
    } else if let Some(rest) = site_url.strip_prefix("http://") {
        format!("ws://{rest}")
    } else {
        site_url.to_string()
    };
    format!("{}{WS_PATH}", base.trim_end_matches('/'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connect_url_only_swaps_the_scheme() {
        assert_eq!(
            connect_url("https://brisk-otter-41.playtest.run"),
            "wss://brisk-otter-41.playtest.run/_playtest/tunnel"
        );
        assert_eq!(
            connect_url("http://brisk-otter-41.localhost:8443/"),
            "ws://brisk-otter-41.localhost:8443/_playtest/tunnel"
        );
        assert_eq!(
            connect_url("brisk-otter-41.example"),
            "brisk-otter-41.example/_playtest/tunnel"
        );
    }

    #[test]
    fn token_never_outlives_the_site() {
        let issued = clock::parse("2026-09-08T12:00:00Z").unwrap();
        let full = issued + Duration::seconds(TOKEN_TTL_SECS);

        assert_eq!(token_expiry(issued, None), full);
        assert_eq!(token_expiry(issued, Some("2026-09-09T00:00:00Z")), full);
        assert_eq!(
            token_expiry(issued, Some("2026-09-08T12:10:00Z")),
            clock::parse("2026-09-08T12:10:00Z").unwrap()
        );
        assert!(token_expiry(issued, Some("2026-09-08T11:00:00Z")) <= issued);
        assert_eq!(token_expiry(issued, Some("不是时间")), full);
    }
}
