//! 浏览器通知（DESIGN §3.6 的第二条渠道）。
//!
//! 为什么不用 `web-push` crate：它 0.11 版无条件依赖 `ece`，而 `ece` 的默认后端是
//! C 的 openssl；crate 没有开关能换成纯 Rust。这个仓库从第一天起就不引 openssl
//! （见 `reqwest` / `rustls` 的特性选择），所以这里按 RFC 8291（aes128gcm）和
//! RFC 8292（VAPID）自己拼，用的是 RustCrypto 那几个纯 Rust 的库。
//!
//! 加密这一段有 RFC 8291 §5 的官方测试向量兜底，见本文件底部的测试。

use aes_gcm::aead::{Aead, Payload};
use aes_gcm::{Aes128Gcm, KeyInit, Nonce};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use hkdf::Hkdf;
use p256::elliptic_curve::sec1::ToEncodedPoint;
use p256::{PublicKey, SecretKey};
use sha2::Sha256;

use super::mailer::{SendError, SendResult};

/// 推送服务允许的最大有效载荷。我们的正文是三行字，离这个上限远得很。
const MAX_RECORD_SIZE: u32 = 4096;
/// 推送服务替我们保留多久。一天：更久也没意义，玩家隔天再看到「出新版本了」已经不是通知了。
const TTL_SECONDS: u32 = 86_400;
/// VAPID 断言的有效期。RFC 8292 建议不超过 24 小时。
const JWT_TTL_SECONDS: i64 = 12 * 3600;

/// 玩家浏览器给的那一份订阅，边缘原样转给我们。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Subscription {
    pub endpoint: String,
    pub keys: SubscriptionKeys,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SubscriptionKeys {
    pub p256dh: String,
    pub auth: String,
}

/// 我们这一端的 VAPID 身份。公钥要发布在 `capabilities.json` 里，浏览器订阅时要用它。
#[derive(Clone)]
pub struct Vapid {
    secret: SecretKey,
    public_b64: String,
    subject: String,
}

impl Vapid {
    pub fn generate() -> Self {
        Self::from_secret(
            SecretKey::random(&mut p256::elliptic_curve::rand_core::OsRng),
            None,
        )
    }

    fn from_secret(secret: SecretKey, subject: Option<String>) -> Self {
        let public_b64 = URL_SAFE_NO_PAD.encode(secret.public_key().to_encoded_point(false));
        Self {
            secret,
            public_b64,
            // 没配就写一个我们自己的地址：推送服务要的是「出事找谁」，留空有些实现会拒。
            subject: subject.unwrap_or_else(|| "mailto:notice@playtest.run".to_string()),
        }
    }

    /// 私钥的 32 字节标量，base64url。只写进数据目录里的 `vapid.json`，不进日志。
    pub fn secret_b64(&self) -> String {
        URL_SAFE_NO_PAD.encode(self.secret.to_bytes())
    }

    pub fn public_b64(&self) -> &str {
        &self.public_b64
    }

    pub fn from_secret_b64(secret_b64: &str, subject: Option<String>) -> anyhow::Result<Self> {
        let bytes = URL_SAFE_NO_PAD
            .decode(secret_b64.trim())
            .map_err(|_| anyhow::anyhow!("vapid.json 里的私钥不是 base64url"))?;
        let secret = SecretKey::from_slice(&bytes)
            .map_err(|_| anyhow::anyhow!("vapid.json 里的私钥不是一把 P-256 私钥"))?;
        Ok(Self::from_secret(secret, subject))
    }

    pub fn with_subject(self, subject: Option<String>) -> Self {
        match subject {
            Some(s) => Self { subject: s, ..self },
            None => self,
        }
    }

    /// RFC 8292 的 `Authorization: vapid t=<JWT>, k=<公钥>`。
    fn authorization(&self, audience: &str, now: i64) -> anyhow::Result<String> {
        use p256::ecdsa::signature::Signer;

        let header = URL_SAFE_NO_PAD.encode(br#"{"typ":"JWT","alg":"ES256"}"#);
        let claims = serde_json::json!({
            "aud": audience,
            "exp": now + JWT_TTL_SECONDS,
            "sub": self.subject,
        });
        let claims = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&claims)?);
        let signing_input = format!("{header}.{claims}");
        let signing_key = p256::ecdsa::SigningKey::from(&self.secret);
        let signature: p256::ecdsa::Signature = signing_key.sign(signing_input.as_bytes());
        let signature = URL_SAFE_NO_PAD.encode(signature.to_bytes());
        Ok(format!(
            "vapid t={signing_input}.{signature}, k={}",
            self.public_b64
        ))
    }
}

/// 推送服务的域，JWT 的 `aud` 要写它。
fn audience(endpoint: &str) -> anyhow::Result<String> {
    let rest = endpoint
        .split_once("://")
        .ok_or_else(|| anyhow::anyhow!("推送地址不像一个网址"))?;
    let host = rest.1.split('/').next().unwrap_or_default();
    if host.is_empty() {
        anyhow::bail!("推送地址里没有域名");
    }
    Ok(format!("{}://{host}", rest.0))
}

/// RFC 8291 §3.4 的那一段：拿订阅公钥、认证密钥和一次性的临时密钥，把正文封成 aes128gcm 的记录。
///
/// `ephemeral` 和 `salt` 作为参数传进来而不是在函数里摇，是为了能用 RFC 的测试向量验它。
fn encrypt(
    ua_public: &[u8],
    auth_secret: &[u8],
    plaintext: &[u8],
    ephemeral: &SecretKey,
    salt: &[u8; 16],
) -> anyhow::Result<Vec<u8>> {
    let ua_public_key = PublicKey::from_sec1_bytes(ua_public)
        .map_err(|_| anyhow::anyhow!("订阅里的 p256dh 不是一把 P-256 公钥"))?;
    let as_public = ephemeral.public_key().to_encoded_point(false);
    let as_public = as_public.as_bytes();

    let shared =
        p256::ecdh::diffie_hellman(ephemeral.to_nonzero_scalar(), ua_public_key.as_affine());

    // 第一次 HKDF：把 ECDH 结果和两边的公钥揉成输入密钥材料（RFC 8291 §3.3）。
    let mut info = Vec::with_capacity(14 + 65 + 65);
    info.extend_from_slice(b"WebPush: info\0");
    info.extend_from_slice(ua_public);
    info.extend_from_slice(as_public);
    let mut ikm = [0u8; 32];
    Hkdf::<Sha256>::new(Some(auth_secret), shared.raw_secret_bytes())
        .expand(&info, &mut ikm)
        .map_err(|_| anyhow::anyhow!("推算密钥失败"))?;

    // 第二次 HKDF：RFC 8188 的内容加密密钥与 nonce。
    let hkdf = Hkdf::<Sha256>::new(Some(salt), &ikm);
    let mut cek = [0u8; 16];
    hkdf.expand(b"Content-Encoding: aes128gcm\0", &mut cek)
        .map_err(|_| anyhow::anyhow!("推算内容密钥失败"))?;
    let mut nonce = [0u8; 12];
    hkdf.expand(b"Content-Encoding: nonce\0", &mut nonce)
        .map_err(|_| anyhow::anyhow!("推算 nonce 失败"))?;

    // RFC 8188 的填充：正文后面跟一个 0x02 表示「这是最后一条记录」。
    let mut padded = plaintext.to_vec();
    padded.push(0x02);
    let ciphertext = Aes128Gcm::new_from_slice(&cek)
        .map_err(|_| anyhow::anyhow!("内容密钥长度不对"))?
        .encrypt(
            Nonce::from_slice(&nonce),
            Payload {
                msg: &padded,
                aad: &[],
            },
        )
        .map_err(|_| anyhow::anyhow!("加密失败"))?;

    // RFC 8188 §2 的记录头：盐、记录大小、公钥长度、公钥，然后是密文。
    let mut body = Vec::with_capacity(21 + as_public.len() + ciphertext.len());
    body.extend_from_slice(salt);
    body.extend_from_slice(&MAX_RECORD_SIZE.to_be_bytes());
    body.push(as_public.len() as u8);
    body.extend_from_slice(as_public);
    body.extend_from_slice(&ciphertext);
    Ok(body)
}

/// 玩家浏览器收到的那一小段 JSON，service worker 照它弹通知。
#[derive(serde::Serialize)]
struct PushPayload<'a> {
    title: &'a str,
    body: &'a str,
    url: &'a str,
}

/// 送一条推送。`Ok(true)` 是送到了；`Ok(false)` 是这个订阅已经没了（410 / 404），
/// 调用方该把它从库里删掉。
pub async fn send(
    http: &reqwest::Client,
    vapid: &Vapid,
    subscription: &Subscription,
    title: &str,
    body: &str,
    url: &str,
) -> Result<bool, SendError> {
    let payload = serde_json::to_vec(&PushPayload { title, body, url })
        .map_err(|e| SendError::Permanent(format!("通知内容拼不出来：{e}")))?;

    let ua_public = URL_SAFE_NO_PAD
        .decode(subscription.keys.p256dh.trim())
        .map_err(|_| SendError::Permanent("订阅里的 p256dh 不是 base64url".to_string()))?;
    let auth_secret = URL_SAFE_NO_PAD
        .decode(subscription.keys.auth.trim())
        .map_err(|_| SendError::Permanent("订阅里的 auth 不是 base64url".to_string()))?;

    let ephemeral = SecretKey::random(&mut p256::elliptic_curve::rand_core::OsRng);
    let mut salt = [0u8; 16];
    rand::RngCore::fill_bytes(&mut rand::rng(), &mut salt);
    let encrypted = encrypt(&ua_public, &auth_secret, &payload, &ephemeral, &salt)
        .map_err(|e| SendError::Permanent(e.to_string()))?;

    let audience =
        audience(&subscription.endpoint).map_err(|e| SendError::Permanent(e.to_string()))?;
    let authorization = vapid
        .authorization(&audience, crate::clock::now().unix_timestamp())
        .map_err(|e| SendError::Permanent(format!("签 VAPID 断言失败：{e}")))?;

    let response = http
        .post(&subscription.endpoint)
        .header("authorization", authorization)
        .header("content-encoding", "aes128gcm")
        .header("content-type", "application/octet-stream")
        .header("ttl", TTL_SECONDS.to_string())
        .header("urgency", "normal")
        .body(encrypted)
        .send()
        .await
        .map_err(|e| SendError::Retry(format!("连不上推送服务：{e}")))?;

    let status = response.status();
    if status == 404 || status == 410 {
        return Ok(false);
    }
    if status.is_success() {
        return Ok(true);
    }
    let detail = response.text().await.unwrap_or_default();
    let message = format!("推送服务回了 {status}：{}", detail.trim());
    if status.is_client_error() && status.as_u16() != 429 {
        Err(SendError::Permanent(message))
    } else {
        Err(SendError::Retry(message))
    }
}

/// 队列里存的是订阅的 JSON 原文；发之前解回来。
pub fn parse_subscription(json: &str) -> Option<Subscription> {
    serde_json::from_str(json).ok()
}

/// 让 `SendResult` 和推送那条路的返回值能拼在一起。
pub fn as_send_result(outcome: Result<bool, SendError>) -> (SendResult, bool) {
    match outcome {
        Ok(true) => (Ok(()), true),
        Ok(false) => (Ok(()), false),
        Err(e) => (Err(e), true),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// RFC 8291 §5 的官方例子：给定的临时密钥和盐，加密结果必须一字节不差。
    /// 这一条挂了就说明加密拼错了，真机上任何浏览器都解不开。
    #[test]
    fn matches_rfc8291_example() {
        let plaintext = b"When I grow up, I want to be a watermelon";
        let ua_public = URL_SAFE_NO_PAD
            .decode("BCVxsr7N_eNgVRqvHtD0zTZsEc6-VV-JvLexhqUzORcxaOzi6-AYWXvTBHm4bjyPjs7Vd8pZGH6SRpkNtoIAiw4")
            .unwrap();
        let auth = URL_SAFE_NO_PAD.decode("BTBZMqHH6r4Tts7J_aSIgg").unwrap();
        let as_secret = URL_SAFE_NO_PAD
            .decode("yfWPiYE-n46HLnH0KqZOF1fJJU3MYrct3AELtAQ-oRw")
            .unwrap();
        let salt: [u8; 16] = URL_SAFE_NO_PAD
            .decode("DGv6ra1nlYgDCS1FRnbzlw")
            .unwrap()
            .try_into()
            .unwrap();

        let ephemeral = SecretKey::from_slice(&as_secret).unwrap();
        let body = encrypt(&ua_public, &auth, plaintext, &ephemeral, &salt).unwrap();

        let expected = URL_SAFE_NO_PAD
            .decode(concat!(
                "DGv6ra1nlYgDCS1FRnbzlwAAEABBBP4z9KsN6nGRTbVYI_c7VJSPQTBtkgcy27ml",
                "mlMoZIIgDll6e3vCYLocInmYWAmS6TlzAC8wEqKK6PBru3jl7A_yl95bQpu6cVPT",
                "pK4Mqgkf1CXztLVBSt2Ks3oZwbuwXPXLWyouBWLVWGNWQexSgSxsj_Qulcy4a-fN"
            ))
            .unwrap();
        assert_eq!(body, expected);
    }

    #[test]
    fn audience_is_scheme_and_host() {
        assert_eq!(
            audience("https://fcm.googleapis.com/fcm/send/abc").unwrap(),
            "https://fcm.googleapis.com"
        );
        assert!(audience("不是网址").is_err());
    }

    #[test]
    fn vapid_survives_a_round_trip() {
        let vapid = Vapid::generate();
        let again = Vapid::from_secret_b64(&vapid.secret_b64(), None).unwrap();
        assert_eq!(vapid.public_b64(), again.public_b64());
        // 公钥是 65 字节的未压缩点，base64url 之后 87 个字符——浏览器就认这个形状。
        assert_eq!(vapid.public_b64().len(), 87);
    }

    #[test]
    fn authorization_has_both_parts() {
        let vapid = Vapid::generate().with_subject(Some("mailto:hi@example.com".to_string()));
        let header = vapid
            .authorization("https://push.example.com", 1_757_000_000)
            .unwrap();
        assert!(header.starts_with("vapid t="), "{header}");
        assert!(header.contains(", k="), "{header}");
    }
}
