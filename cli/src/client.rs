//! 和控制面说话。路由和请求体全部来自 `playtest_common::api`。

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use futures_util::TryStreamExt;
use playtest_common::api::{
    routes, AnonSessionResponse, CommitUploadResponse, CreateSiteRequest, DeviceLoginPoll,
    DeviceLoginStart, ErrorBody, ErrorCode, LoginPollResponse, Me, PrepareUploadRequest,
    PrepareUploadResponse, Site, UpdateSiteRequest, VersionFiles, VersionList,
};
use playtest_common::tunnel::{TunnelGrant, TunnelRequest};
use reqwest::header::AUTHORIZATION;
use reqwest::{Method, StatusCode};
use tokio_util::io::ReaderStream;

/// 建立连接最多等这么久。连上以后不再设总时限——大文件本来就慢。
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

const UPLOAD_CHUNK_BYTES: usize = 64 * 1024;

/// 每个请求都带上，服务端日志里能看出是哪一版 CLI。
const USER_AGENT: &str = concat!("playtest/", env!("CARGO_PKG_VERSION"));

/// 一个不认路由、也不带令牌的 HTTP 客户端。
///
/// 邀请卡在玩家那一侧的域名上（`https://<slug>.playtest.run/_playtest/card.png`），不在控制面：
/// 那边既不需要、也不该收到开发者的令牌（AGENTS 第 7 条，两个域名不混）。所以拿卡不走 [`Client`]，
/// 只共用 User-Agent 和「多久算连不上」这套设置。等多久由调用方定——发布之后顺手取一张卡，
/// 不该为它把一次成功的发布拖上几十秒。
pub fn new_http(connect: Duration, total: Duration) -> reqwest::Result<reqwest::Client> {
    reqwest::Client::builder()
        .connect_timeout(connect)
        .timeout(total)
        .user_agent(USER_AGENT)
        .build()
}

/// 上传时每写出去一段字节就叫一次，用来推进度条。
pub type OnBytes = Arc<dyn Fn(u64) + Send + Sync>;

/// 装 TLS 的加密后端。必须在建第一个 [`Client`] 之前调用：reqwest 的 rustls-no-provider
/// 没有内置后端，缺了它建 Client 时直接 panic。
///
/// 选 ring 而不是 reqwest 默认的 aws-lc-rs：不需要 C 工具链，构建产物也小得多。
pub fn install_crypto_provider() {
    // 装过一次就够了，重复调用不算错。
    let _ = rustls::crypto::ring::default_provider().install_default();
}

#[derive(Debug)]
pub enum Error {
    /// 连不上、连接断了、TLS 握手失败之类。
    Transport { api: String, source: reqwest::Error },
    /// 服务器按约定回了一个错误，`message` 是中文，直接给用户看。
    Server { status: u16, body: ErrorBody },
    /// 非 2xx，但内容不是我们约定的格式——多半中间挡了一层代理或网关。
    Unexpected { status: u16, text: String },
    /// 本机这边的问题，比如要传的文件读不了。
    Local {
        path: PathBuf,
        source: std::io::Error,
    },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Transport { api, source } => {
                let reason = if source.is_timeout() {
                    format!("等了 {} 秒没连上", CONNECT_TIMEOUT.as_secs())
                } else {
                    root_cause(source)
                };
                write!(
                    f,
                    "连不上服务器（{api}）：{reason}。检查网络，或用 --api 指定地址。"
                )
            }
            Error::Server { body, .. } => write!(f, "服务器说：{}", body.message),
            Error::Unexpected { status, text } => {
                let mut preview: String = text.chars().take(120).collect();
                if text.chars().count() > 120 {
                    preview.push('…');
                }
                write!(
                    f,
                    "服务器返回了看不懂的内容（HTTP {status}）：{preview}。地址可能不是 playtest 控制面。"
                )
            }
            Error::Local { path, source } => write!(f, "读不了 {}：{source}", path.display()),
        }
    }
}

impl std::error::Error for Error {}

impl Error {
    pub fn code(&self) -> Option<ErrorCode> {
        match self {
            Error::Server { body, .. } => Some(body.code),
            _ => None,
        }
    }

    /// 再试一次有可能成功吗。
    pub fn worth_retrying(&self) -> bool {
        match self {
            Error::Transport { .. } => true,
            Error::Server { status, .. } | Error::Unexpected { status, .. } => *status >= 500,
            Error::Local { .. } => false,
        }
    }

    /// 记住的那个作品已经不认我们了：令牌过期，或者作品本身没了。
    pub fn means_anonymous_link_gone(&self) -> bool {
        self.means_token_gone() || matches!(self, Error::Server { status, .. } if *status == 404)
    }

    /// 是令牌本身不行了（过期或服务器不认），而不是作品没了。
    /// 两者的处理不同：前者要换匿名身份，后者只换作品，否则之前的作品会跟着旧身份一起丢。
    pub fn means_token_gone(&self) -> bool {
        matches!(
            self,
            Error::Server { status: 401, body }
                if matches!(body.code, ErrorCode::TokenExpired | ErrorCode::Unauthorized)
        )
    }
}

pub type Result<T> = std::result::Result<T, Error>;

pub struct Client {
    http: reqwest::Client,
    base: String,
    token: Option<String>,
}

impl Client {
    pub fn new(base: &str) -> anyhow::Result<Self> {
        let http = reqwest::Client::builder()
            .connect_timeout(CONNECT_TIMEOUT)
            .user_agent(USER_AGENT)
            .build()?;
        Ok(Self {
            http,
            base: base.trim_end_matches('/').to_string(),
            token: None,
        })
    }

    pub fn set_token(&mut self, token: Option<String>) {
        self.token = token;
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base, path)
    }

    fn request(&self, method: Method, path: &str) -> reqwest::RequestBuilder {
        let builder = self.http.request(method, self.url(path));
        match &self.token {
            Some(token) => builder.header(AUTHORIZATION, format!("Bearer {token}")),
            None => builder,
        }
    }

    pub async fn anon_session(&self) -> Result<AnonSessionResponse> {
        let response = self
            .http
            .post(self.url(routes::ANON_SESSIONS))
            .send()
            .await
            .map_err(|e| self.transport(e))?;
        self.read_json(response).await
    }

    /// 设备码登录的第一步（`playtest login`）。不带令牌。
    pub async fn device_login_start(&self) -> Result<DeviceLoginStart> {
        let response = self
            .http
            .post(self.url(routes::LOGIN_DEVICE_START))
            .json(&serde_json::json!({}))
            .send()
            .await
            .map_err(|e| self.transport(e))?;
        self.read_json(response).await
    }

    /// 轮询。带着手里的匿名令牌去，成功时那个身份下的作品会一起归到账号里。
    pub async fn device_login_poll(&self, device_code: &str) -> Result<LoginPollResponse> {
        let response = self
            .request(Method::POST, routes::LOGIN_DEVICE_POLL)
            .json(&DeviceLoginPoll {
                device_code: device_code.to_string(),
            })
            .send()
            .await
            .map_err(|e| self.transport(e))?;
        self.read_json(response).await
    }

    pub async fn me(&self) -> Result<Me> {
        let response = self
            .request(Method::GET, routes::ME)
            .send()
            .await
            .map_err(|e| self.transport(e))?;
        self.read_json(response).await
    }

    pub async fn create_site(&self, request: &CreateSiteRequest) -> Result<Site> {
        let response = self
            .request(Method::POST, routes::SITES)
            .json(request)
            .send()
            .await
            .map_err(|e| self.transport(e))?;
        self.read_json(response).await
    }

    pub async fn list_sites(&self) -> Result<Vec<Site>> {
        let response = self
            .request(Method::GET, routes::SITES)
            .send()
            .await
            .map_err(|e| self.transport(e))?;
        self.read_json(response).await
    }

    pub async fn get_site(&self, slug: &str) -> Result<Site> {
        let response = self
            .request(Method::GET, &routes::site(slug))
            .send()
            .await
            .map_err(|e| self.transport(e))?;
        self.read_json(response).await
    }

    pub async fn delete_site(&self, slug: &str) -> Result<()> {
        let response = self
            .request(Method::DELETE, &routes::site(slug))
            .send()
            .await
            .map_err(|e| self.transport(e))?;
        if response.status().is_success() {
            return Ok(());
        }
        Err(self.read_error(response).await)
    }

    /// 改广场上的状态（DESIGN §3.8）：公开、求测、想让人看什么。
    pub async fn update_site(&self, slug: &str, request: &UpdateSiteRequest) -> Result<Site> {
        let response = self
            .request(Method::PATCH, &routes::site(slug))
            .json(request)
            .send()
            .await
            .map_err(|e| self.transport(e))?;
        self.read_json(response).await
    }

    /// 发过的每一版，新的在前。
    pub async fn list_versions(&self, slug: &str) -> Result<VersionList> {
        let response = self
            .request(Method::GET, &routes::site_versions(slug))
            .send()
            .await
            .map_err(|e| self.transport(e))?;
        self.read_json(response).await
    }

    /// 某一版里到底有哪些文件。
    pub async fn version_files(&self, slug: &str, version: u32) -> Result<VersionFiles> {
        let response = self
            .request(Method::GET, &routes::site_version_files(slug, version))
            .send()
            .await
            .map_err(|e| self.transport(e))?;
        self.read_json(response).await
    }

    /// 让玩家看到的换回某一版（DESIGN §3.5）。
    pub async fn activate_version(&self, slug: &str, version: u32) -> Result<Site> {
        let response = self
            .request(Method::POST, &routes::site_version_activate(slug, version))
            .json(&serde_json::json!({}))
            .send()
            .await
            .map_err(|e| self.transport(e))?;
        self.read_json(response).await
    }

    pub async fn prepare_upload(
        &self,
        slug: &str,
        request: &PrepareUploadRequest,
    ) -> Result<PrepareUploadResponse> {
        let response = self
            .request(Method::POST, &routes::site_uploads(slug))
            .json(request)
            .send()
            .await
            .map_err(|e| self.transport(e))?;
        self.read_json(response).await
    }

    pub async fn commit_upload(&self, slug: &str, upload_id: &str) -> Result<CommitUploadResponse> {
        let response = self
            .request(Method::POST, &routes::site_upload_commit(slug, upload_id))
            .send()
            .await
            .map_err(|e| self.transport(e))?;
        self.read_json(response).await
    }

    /// 要一张隧道授权：握手地址、短期令牌、玩家链接（DESIGN §4.3）。
    /// 令牌一小时到期，长跑的隧道要在到期前再来一次。
    pub async fn tunnel_grant(&self, slug: &str, request: &TunnelRequest) -> Result<TunnelGrant> {
        let response = self
            .request(Method::POST, &routes::site_tunnel(slug))
            .json(request)
            .send()
            .await
            .map_err(|e| self.transport(e))?;
        self.read_json(response).await
    }

    /// 传一个对象。请求体从文件流式读出来，不整个装进内存。
    pub async fn put_blob(&self, hash: &str, source: &Path, on_bytes: OnBytes) -> Result<()> {
        let file = tokio::fs::File::open(source)
            .await
            .map_err(|source_err| Error::Local {
                path: source.to_path_buf(),
                source: source_err,
            })?;
        let stream =
            ReaderStream::with_capacity(file, UPLOAD_CHUNK_BYTES).inspect_ok(move |chunk| {
                on_bytes(chunk.len() as u64);
            });
        let response = self
            .request(Method::PUT, &routes::blob(hash))
            .body(reqwest::Body::wrap_stream(stream))
            .send()
            .await
            .map_err(|e| self.transport(e))?;
        if response.status().is_success() {
            return Ok(());
        }
        Err(self.read_error(response).await)
    }

    fn transport(&self, source: reqwest::Error) -> Error {
        Error::Transport {
            api: self.base.clone(),
            source,
        }
    }

    async fn read_json<T: serde::de::DeserializeOwned>(
        &self,
        response: reqwest::Response,
    ) -> Result<T> {
        if !response.status().is_success() {
            return Err(self.read_error(response).await);
        }
        let status = response.status().as_u16();
        let bytes = response.bytes().await.map_err(|e| self.transport(e))?;
        serde_json::from_slice(&bytes).map_err(|_| Error::Unexpected {
            status,
            text: String::from_utf8_lossy(&bytes).into_owned(),
        })
    }

    async fn read_error(&self, response: reqwest::Response) -> Error {
        let status = response.status();
        let bytes = match response.bytes().await {
            Ok(bytes) => bytes,
            Err(e) => return self.transport(e),
        };
        match serde_json::from_slice::<ErrorBody>(&bytes) {
            Ok(body) => Error::Server {
                status: status.as_u16(),
                body,
            },
            Err(_) => Error::Unexpected {
                status: status.as_u16(),
                text: describe_status(status, &bytes),
            },
        }
    }
}

/// reqwest 最外层那句「error sending request for url …」只是重复了我们已经说过的地址，
/// 有用的是链条最里面那一条，比如「Connection refused (os error 61)」。
fn root_cause(e: &reqwest::Error) -> String {
    let mut deepest: &dyn std::error::Error = e;
    while let Some(inner) = deepest.source() {
        deepest = inner;
    }
    deepest.to_string()
}

fn describe_status(status: StatusCode, bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes).trim().to_string();
    if text.is_empty() {
        status.canonical_reason().unwrap_or("没有内容").to_string()
    } else {
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn server(status: u16, code: ErrorCode) -> Error {
        Error::Server {
            status,
            body: ErrorBody {
                code,
                message: "这个版本太大了".into(),
            },
        }
    }

    #[test]
    fn server_messages_are_shown_verbatim() {
        assert_eq!(
            server(413, ErrorCode::QuotaExceeded).to_string(),
            "服务器说：这个版本太大了"
        );
    }

    #[test]
    fn only_server_faults_are_retried() {
        assert!(server(500, ErrorCode::Internal).worth_retrying());
        assert!(!server(400, ErrorCode::HashMismatch).worth_retrying());
        assert!(!server(404, ErrorCode::NotFound).worth_retrying());
    }

    #[test]
    fn expired_token_and_missing_site_both_mean_start_over() {
        assert!(server(401, ErrorCode::TokenExpired).means_anonymous_link_gone());
        assert!(server(401, ErrorCode::Unauthorized).means_anonymous_link_gone());
        assert!(server(404, ErrorCode::NotFound).means_anonymous_link_gone());
        assert!(!server(400, ErrorCode::Invalid).means_anonymous_link_gone());
    }

    #[test]
    fn urls_do_not_double_up_slashes() {
        install_crypto_provider();
        let client = Client::new("http://127.0.0.1:8787/").unwrap();
        assert_eq!(client.url(routes::SITES), "http://127.0.0.1:8787/v1/sites");
    }
}
