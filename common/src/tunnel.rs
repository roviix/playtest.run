//! 隧道路径的契约（DESIGN §4.3、§4.5）：CLI 把本地端口经一条出站 WSS 接到边缘，边缘按 Host 把玩家请求
//! 送进去。这里放三方都要认的东西：
//!
//! - 握手：CLI 连 `wss://<slug>.<后缀>/_playtest/tunnel`，`Authorization: Bearer <令牌>`，
//!   `X-Playtest-Local-Port: 5173`（边缘据此把 `Host` 改写成 `localhost:5173`）。
//! - 令牌：控制面用 Ed25519 私钥签，边缘只用公钥验、不回源。`pt1.<base64url 声明>.<base64url 签名>`。
//! - 复用：101 之后这条 WebSocket 的每个二进制帧就是 yamux 字节流的一段，边缘开流、CLI 收流。
//! - 关闭码：边缘主动断开时用 4xxx 告诉 CLI 该重连、该换令牌还是该退出。
//! - 挤掉：同一个 slug 来了新隧道，旧连接被关（[`close::REPLACED`]），旧令牌的 `jti` 进边缘的驱逐名单；
//!   拿驱逐过的令牌再握手一律 `409` + [`crate::api::ErrorCode::TunnelReplaced`]，CLI 据此退出而不是重试，
//!   两个进程才不会互相挤来挤去。每个 CLI 进程每次拿到的令牌 `jti` 都不同，换令牌后的旧进程早已退出。
//! - 保活：CLI 每 [`KEEPALIVE_INTERVAL_SECS`] 秒发一个 WebSocket ping（NAT 与公司代理的空闲超时在它那一侧）；
//!   边缘收到任何帧都算活着，[`IDLE_TIMEOUT_SECS`] 秒没动静就当离线。两边都不往 `127.0.0.1:<port>` 打探测。
//!
//! 字节流适配与 yamux 的驾驭在 [`io`]（feature `tunnel-io`），控制面不需要它。

use serde::{Deserialize, Serialize};

use crate::manifest::GateMode;

/// 边缘上的握手路径，在 [`crate::RESERVED_PATH_PREFIX`] 之下，用作品自己的 Host。
pub const WS_PATH: &str = "/_playtest/tunnel";

/// `Sec-WebSocket-Protocol`，版本变了才改。
pub const WS_PROTOCOL: &str = "playtest-tunnel-v1";

/// CLI 在握手里告诉边缘本地端口，边缘据此改写 `Host`（DESIGN §4.3：Vite 只放行 localhost）。
pub const HEADER_LOCAL_PORT: &str = "x-playtest-local-port";

/// 边缘把原始 Host 放在这里，转给开发者的进程。
pub const HEADER_FORWARDED_HOST: &str = "x-forwarded-host";

/// 保活：CLI 每隔这么久发一个 WebSocket ping；边缘这么久没收到任何帧就当离线。
pub const KEEPALIVE_INTERVAL_SECS: u64 = 20;
pub const IDLE_TIMEOUT_SECS: u64 = 60;

/// 重连退避（DESIGN §4.3：1 s → 30 s 指数退避加抖动）。
pub const BACKOFF_MIN_SECS: u64 = 1;
pub const BACKOFF_MAX_SECS: u64 = 30;

/// 令牌有效期。CLI 在到期前向控制面换新，边缘对已建立的连接不因令牌到期而断。
pub const TOKEN_TTL_SECS: i64 = 60 * 60;

/// 边缘主动关闭 WebSocket 时的状态码。
pub mod close {
    /// 同一个 slug 来了新的隧道，旧的被挤掉（DESIGN §4.3）。CLI 退出，不重连。
    pub const REPLACED: u16 = 4001;
    /// 握手时令牌已过期。CLI 换令牌后重连。
    pub const TOKEN_EXPIRED: u16 = 4002;
    /// 作品被撤销或下架。CLI 退出，不重连。
    pub const REVOKED: u16 = 4003;
    /// 边缘要重启或维护。CLI 按退避重连。
    pub const GOING_AWAY: u16 = 1001;
}

/// 令牌里的声明。边缘靠它渲染门禁页、决定响应头，不必再问任何人。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Claims {
    /// 令牌格式版本，现在是 1。
    pub v: u8,
    pub slug: String,
    /// 用户 id，用量与撤销按它归。
    pub sub: String,
    pub title: String,
    pub developer: String,
    #[serde(default)]
    pub badge: bool,
    #[serde(default)]
    pub gate: GateMode,
    #[serde(default)]
    pub isolated: bool,
    /// 同时在线的玩家连接上限（档位配额，DESIGN §6）。
    pub max_players: u32,
    /// 混合模式（DESIGN §4.3，`playtest ./dist --backend 3000`）：上传的清单里有的文件从边缘给，
    /// 清单里没有的路径——不论方法，`/api/x`、WebSocket 都算——才经这条隧道到开发者的机器。
    /// 没有路径约定：清单就是分界线。`false` 是整个作品都走隧道（`playtest 5173`）。
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub hybrid: bool,
    /// 签发与到期，Unix 秒。
    pub iat: i64,
    pub exp: i64,
    /// 随机 id，撤销名单按它记。
    pub jti: String,
}

impl Claims {
    /// 是否在有效期内；`skew` 容忍两边时钟的差。
    pub fn is_valid_at(&self, now: i64, skew: i64) -> bool {
        self.iat - skew <= now && now < self.exp + skew
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum TokenError {
    #[error("令牌格式不对")]
    Malformed,
    #[error("令牌版本 {0} 不认识")]
    UnknownVersion(String),
    #[error("令牌签名不对")]
    BadSignature,
    #[error("令牌已过期")]
    Expired,
    #[error("令牌还没到生效时间")]
    NotYetValid,
}

/// 控制面持有的签名私钥。
#[derive(Clone)]
pub struct SigningKey(ed25519_dalek::SigningKey);

/// 边缘持有的验签公钥。
#[derive(Clone, PartialEq, Eq)]
pub struct VerifyingKey(ed25519_dalek::VerifyingKey);

impl std::fmt::Debug for SigningKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SigningKey(…)")
    }
}

impl std::fmt::Debug for VerifyingKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "VerifyingKey({})", self.to_base64())
    }
}

const TOKEN_PREFIX: &str = "pt1";

fn b64(bytes: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

fn unb64(s: &str) -> Option<Vec<u8>> {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(s)
        .ok()
}

impl SigningKey {
    pub fn generate() -> Self {
        let mut seed = [0u8; 32];
        getrandom::fill(&mut seed).expect("系统随机数不可用");
        Self(ed25519_dalek::SigningKey::from_bytes(&seed))
    }

    pub fn from_base64(s: &str) -> Option<Self> {
        let bytes = unb64(s.trim())?;
        let seed: [u8; 32] = bytes.try_into().ok()?;
        Some(Self(ed25519_dalek::SigningKey::from_bytes(&seed)))
    }

    pub fn to_base64(&self) -> String {
        b64(&self.0.to_bytes())
    }

    pub fn verifying_key(&self) -> VerifyingKey {
        VerifyingKey(self.0.verifying_key())
    }

    /// 签出一个令牌字符串。`claims` 里的时间由调用方填好。
    pub fn sign(&self, claims: &Claims) -> String {
        use ed25519_dalek::Signer;
        let payload = serde_json::to_vec(claims).expect("声明可以序列化");
        let sig = self.0.sign(&payload);
        format!("{TOKEN_PREFIX}.{}.{}", b64(&payload), b64(&sig.to_bytes()))
    }
}

impl VerifyingKey {
    pub fn from_base64(s: &str) -> Option<Self> {
        let bytes = unb64(s.trim())?;
        let arr: [u8; 32] = bytes.try_into().ok()?;
        ed25519_dalek::VerifyingKey::from_bytes(&arr).ok().map(Self)
    }

    pub fn to_base64(&self) -> String {
        b64(self.0.as_bytes())
    }

    /// 验签并检查有效期。`now` 是 Unix 秒。
    pub fn verify(&self, token: &str, now: i64) -> Result<Claims, TokenError> {
        use ed25519_dalek::Verifier;
        let mut parts = token.trim().split('.');
        let (Some(prefix), Some(payload_b64), Some(sig_b64), None) =
            (parts.next(), parts.next(), parts.next(), parts.next())
        else {
            return Err(TokenError::Malformed);
        };
        if prefix != TOKEN_PREFIX {
            return Err(TokenError::UnknownVersion(prefix.to_string()));
        }
        let payload = unb64(payload_b64).ok_or(TokenError::Malformed)?;
        let sig_bytes = unb64(sig_b64).ok_or(TokenError::Malformed)?;
        let sig =
            ed25519_dalek::Signature::from_slice(&sig_bytes).map_err(|_| TokenError::Malformed)?;
        self.0
            .verify(&payload, &sig)
            .map_err(|_| TokenError::BadSignature)?;
        let claims: Claims = serde_json::from_slice(&payload).map_err(|_| TokenError::Malformed)?;
        if claims.v != 1 {
            return Err(TokenError::UnknownVersion(claims.v.to_string()));
        }
        const SKEW: i64 = 60;
        if now >= claims.exp + SKEW {
            return Err(TokenError::Expired);
        }
        if now < claims.iat - SKEW {
            return Err(TokenError::NotYetValid);
        }
        Ok(claims)
    }
}

/// 密钥文件的约定位置。控制面与边缘在一台机器上共用数据目录时零配置：
/// 控制面第一次启动生成私钥写到 `<数据目录>/tunnel-signing.key`（0600），
/// 把公钥写进对象存储 `keys/tunnel.pub`；边缘从对象存储读公钥。多边缘时公钥随对象存储同步。
pub mod key_files {
    pub const SIGNING_KEY_FILE: &str = "tunnel-signing.key";
    /// 相对对象存储根。
    pub const VERIFYING_KEY_OBJECT: &str = "keys/tunnel.pub";
}

/// `POST /v1/projects/{slug}/tunnel` 的请求体。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TunnelRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default)]
    pub gate: GateMode,
    #[serde(default)]
    pub isolated: bool,
    /// 见 [`Claims::hybrid`]。控制面只在作品已有上传过的版本时才签它。
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub hybrid: bool,
}

/// `POST /v1/projects/{slug}/tunnel` 的响应：CLI 拿它去连边缘。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TunnelGrant {
    pub slug: String,
    /// 玩家链接，例如 `https://brisk-otter-41.playtest.run`。
    pub url: String,
    /// 握手地址，例如 `wss://brisk-otter-41.playtest.run/_playtest/tunnel`（本机开发是 `ws://…localhost:8443/…`）。
    pub connect_url: String,
    pub token: String,
    /// RFC 3339，CLI 在此之前换新。
    pub expires_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub site_expires_at: Option<String>,
}

#[cfg(feature = "tunnel-io")]
pub mod io;

#[cfg(test)]
mod tests {
    use super::*;

    fn claims(now: i64) -> Claims {
        Claims {
            v: 1,
            slug: "brisk-otter-41".into(),
            sub: "user-1".into(),
            title: "测试".into(),
            developer: "匿名开发者".into(),
            badge: true,
            gate: GateMode::Once,
            isolated: false,
            max_players: 50,
            hybrid: false,
            iat: now,
            exp: now + TOKEN_TTL_SECS,
            jti: "abc".into(),
        }
    }

    #[test]
    fn sign_then_verify_round_trips() {
        let sk = SigningKey::generate();
        let vk = sk.verifying_key();
        let now = 1_800_000_000;
        let token = sk.sign(&claims(now));
        assert!(token.starts_with("pt1."));
        assert_eq!(vk.verify(&token, now + 10).unwrap(), claims(now));
    }

    #[test]
    fn keys_round_trip_through_base64() {
        let sk = SigningKey::generate();
        let sk2 = SigningKey::from_base64(&sk.to_base64()).unwrap();
        assert_eq!(sk.verifying_key(), sk2.verifying_key());
        let vk = VerifyingKey::from_base64(&sk.verifying_key().to_base64()).unwrap();
        assert_eq!(vk, sk.verifying_key());
        assert!(VerifyingKey::from_base64("not-a-key").is_none());
    }

    #[test]
    fn rejects_tampering_wrong_key_and_expiry() {
        let sk = SigningKey::generate();
        let vk = sk.verifying_key();
        let now = 1_800_000_000;
        let token = sk.sign(&claims(now));

        assert_eq!(
            vk.verify(&token, now + TOKEN_TTL_SECS + 61),
            Err(TokenError::Expired)
        );
        assert_eq!(vk.verify(&token, now - 61), Err(TokenError::NotYetValid));

        let other = SigningKey::generate().verifying_key();
        assert_eq!(other.verify(&token, now), Err(TokenError::BadSignature));

        let mut parts: Vec<&str> = token.split('.').collect();
        let forged = b64(br#"{"v":1,"slug":"admin"}"#);
        parts[1] = &forged;
        assert_eq!(
            vk.verify(&parts.join("."), now),
            Err(TokenError::BadSignature)
        );

        assert_eq!(
            vk.verify("pt0.a.b", now),
            Err(TokenError::UnknownVersion("pt0".into()))
        );
        assert_eq!(vk.verify("garbage", now), Err(TokenError::Malformed));
    }

    /// 旧边缘认不出 `hybrid` 时要把它当整站隧道，而不是拒签或崩掉；没开混合模式的令牌不写这一项。
    #[test]
    fn hybrid_is_optional_on_the_wire_and_off_by_default() {
        let sk = SigningKey::generate();
        let vk = sk.verifying_key();
        let now = 1_800_000_000;

        let plain = sk.sign(&claims(now));
        let payload = plain.split('.').nth(1).unwrap();
        let json = String::from_utf8(unb64(payload).unwrap()).unwrap();
        assert!(!json.contains("hybrid"), "没开就不该占字节：{json}");
        assert!(!vk.verify(&plain, now).unwrap().hybrid);

        let mut on = claims(now);
        on.hybrid = true;
        assert!(vk.verify(&sk.sign(&on), now).unwrap().hybrid);

        let request: TunnelRequest = serde_json::from_str("{}").unwrap();
        assert!(!request.hybrid);
    }
}
