//! `Authorization: Bearer <token>`。
//!
//! 令牌明文不落库，库里只有 sha256(token)。匿名令牌 24 小时到期（DESIGN §3.2），
//! 过期的说法要让第一次用的人一看就知道下一步做什么，所以这里的三句话是端点行为的一部分。

use axum::extract::FromRequestParts;
use axum::http::header::AUTHORIZATION;
use axum::http::request::Parts;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use playtest_common::hash;
use rand::RngCore;

use crate::clock;
use crate::db;
use crate::error::ApiError;
use crate::state::AppState;

pub const ANON_DISPLAY_NAME: &str = "匿名开发者";

const NO_TOKEN: &str = "这个请求没带令牌。第一次用直接运行 playtest，它会自动申请一个 24 小时的匿名链接。";
const BAD_TOKEN: &str = "这个令牌我们不认识。重新运行 playtest 会拿到一个新的链接。";
const EXPIRED: &str = "匿名链接的 24 小时已到，这个令牌和它创建的作品都失效了。重新运行 playtest 会拿到一个新链接。";

/// 令牌的字节数。32 字节的随机量，base64url 之后是 43 个字符，能整行复制粘贴。
const TOKEN_BYTES: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserKind {
    /// 24 小时匿名链接。
    Anon,
    /// GitHub 登录。v0.1 还没有登录入口，这一支目前进不来。
    GitHub,
}

impl UserKind {
    pub fn as_db(self) -> &'static str {
        match self {
            Self::Anon => "anon",
            Self::GitHub => "github",
        }
    }

    fn from_db(s: &str) -> Self {
        match s {
            "github" => Self::GitHub,
            _ => Self::Anon,
        }
    }

    pub fn is_anon(self) -> bool {
        self == Self::Anon
    }
}

/// 通过鉴权的调用方。
#[derive(Debug, Clone)]
pub struct Caller {
    pub user_id: String,
    pub kind: UserKind,
    pub display_name: String,
    /// 这个人的到期时间，作品跟着它走。
    pub expires_at: Option<String>,
}

impl FromRequestParts<AppState> for Caller {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let token = bearer_token(parts).ok_or_else(|| ApiError::unauthorized(NO_TOKEN))?;
        let token_hash = hash::hash_bytes(token.as_bytes());

        let owner = {
            let conn = state.db().lock().await;
            db::find_token_owner(&conn, &token_hash)?
        };
        let owner = owner.ok_or_else(|| ApiError::unauthorized(BAD_TOKEN))?;

        let expired = owner
            .token_expires_at
            .as_deref()
            .is_some_and(clock::is_expired)
            || owner
                .user_expires_at
                .as_deref()
                .is_some_and(clock::is_expired);
        if expired {
            return Err(ApiError::token_expired(EXPIRED));
        }

        Ok(Caller {
            user_id: owner.user_id,
            kind: UserKind::from_db(&owner.kind),
            display_name: owner.display_name,
            expires_at: owner.user_expires_at,
        })
    }
}

fn bearer_token(parts: &Parts) -> Option<String> {
    let raw = parts.headers.get(AUTHORIZATION)?.to_str().ok()?;
    let (scheme, value) = raw.split_once(' ')?;
    if !scheme.eq_ignore_ascii_case("bearer") {
        return None;
    }
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

/// 新令牌的明文。只在签发那一次返回给客户端，之后谁都拿不回来。
pub fn new_token() -> String {
    let mut bytes = [0u8; TOKEN_BYTES];
    rand::rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::Request;

    fn parts_with(header: &str) -> Parts {
        Request::builder()
            .header(AUTHORIZATION, header)
            .body(())
            .unwrap()
            .into_parts()
            .0
    }

    #[test]
    fn reads_bearer_case_insensitively() {
        assert_eq!(bearer_token(&parts_with("Bearer abc")).as_deref(), Some("abc"));
        assert_eq!(bearer_token(&parts_with("bearer abc")).as_deref(), Some("abc"));
        assert_eq!(bearer_token(&parts_with("Basic abc")), None);
        assert_eq!(bearer_token(&parts_with("Bearer   ")), None);
    }

    #[test]
    fn tokens_are_url_safe_and_unique() {
        let a = new_token();
        let b = new_token();
        assert_ne!(a, b);
        assert!(a.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'), "{a}");
    }
}
