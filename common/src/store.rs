//! 作品对象的存储契约，以及文件系统 / S3 两种实现。
//!
//! 布局在两种后端里完全相同：
//!
//! ```text
//! blobs/<hash 前两位>/<hash>              文件内容，不可变，按内容哈希去重
//! sites/<slug>/manifests/<version>.json   一个版本的清单，不可变
//! sites/<slug>/current.json               指向当前版本的指针，回滚就是改它
//! sites/<slug>/live.json                  名额、关注数与公开反馈的读取快照
//! sites/<slug>/policy.json                边缘执行的额度策略
//! ```
//!
//! api 写、edge 读。本机默认使用文件系统；生产显式选择 S3 后，配置或连通性有问题就
//! 启动失败，绝不悄悄写回本地盘。上传请求仍先落本机临时文件并校验 SHA-256，再以固定
//! 大小分段写入对象存储；玩家读取则从对象存储流式返回，Range 不经过整包内存。

use std::path::{Path as FsPath, PathBuf};
use std::sync::Arc;

use futures_util::TryStreamExt;
use object_store::aws::AmazonS3Builder;
use object_store::local::LocalFileSystem;
use object_store::path::Path;
use object_store::prefix::PrefixStore;
use object_store::{
    ClientOptions, GetOptions, GetRange, GetResult, ObjectStore, ObjectStoreExt, RetryConfig,
    WriteMultipart,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tokio::io::AsyncReadExt;

use crate::hash::is_valid_hex;
use crate::manifest::Manifest;
use crate::slug;

pub const BACKEND_ENV: &str = "PLAYTEST_STORAGE_BACKEND";
pub const S3_BUCKET_ENV: &str = "PLAYTEST_S3_BUCKET";
pub const S3_REGION_ENV: &str = "PLAYTEST_S3_REGION";
pub const S3_ENDPOINT_ENV: &str = "PLAYTEST_S3_ENDPOINT";
pub const S3_PREFIX_ENV: &str = "PLAYTEST_S3_PREFIX";
pub const S3_ACCESS_KEY_ENV: &str = "PLAYTEST_S3_ACCESS_KEY_ID";
pub const S3_SECRET_KEY_ENV: &str = "PLAYTEST_S3_SECRET_ACCESS_KEY";
pub const S3_SESSION_TOKEN_ENV: &str = "PLAYTEST_S3_SESSION_TOKEN";

/// S3 的非最后一段至少要 5 MiB；8 MiB 留一点兼容余量，且最多同时飞四段（32 MiB）。
const MULTIPART_CHUNK: usize = 8 * 1024 * 1024;
const MULTIPART_CONCURRENCY: usize = 4;
/// 清单、指针与读取快照都应很小。对象存储里若被误放了大文件，不能整包吃进进程内存。
const MAX_JSON_BYTES: u64 = 4 * 1024 * 1024;

/// `sites/<slug>/current.json` 的内容。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Current {
    pub version: u32,
    /// RFC 3339，最近一次改指针的时间（上传或回滚）。
    pub updated_at: String,
}

pub fn blob_key(hash: &str) -> String {
    format!("blobs/{}/{}", &hash[..2], hash)
}

pub fn manifest_key(slug: &str, version: u32) -> String {
    format!("sites/{slug}/manifests/{version}.json")
}

pub fn current_key(slug: &str) -> String {
    format!("sites/{slug}/current.json")
}

#[derive(Debug, Clone)]
pub enum StoreConfig {
    Filesystem {
        root: PathBuf,
    },
    S3 {
        bucket: String,
        region: String,
        endpoint: Option<String>,
        prefix: Option<String>,
        /// 上传校验完成前使用的本机临时目录，不是持久化源站。
        temp_dir: PathBuf,
    },
}

impl StoreConfig {
    /// 从环境变量选择后端。默认 `fs` 只为本机和测试；部署文件会显式写 `s3`。
    pub fn from_env(local_root: PathBuf, temp_dir: PathBuf) -> anyhow::Result<Self> {
        let backend = env_opt(BACKEND_ENV).unwrap_or_else(|| "fs".to_string());
        match backend.trim().to_ascii_lowercase().as_str() {
            "fs" | "filesystem" => Ok(Self::Filesystem { root: local_root }),
            "s3" => {
                let bucket = required_env(S3_BUCKET_ENV)?;
                let region = required_env(S3_REGION_ENV)?;
                let endpoint = env_opt(S3_ENDPOINT_ENV);
                if let Some(endpoint) = &endpoint {
                    let url = reqwest::Url::parse(endpoint).map_err(|error| {
                        anyhow::anyhow!("{S3_ENDPOINT_ENV} 不是完整 URL：{error}")
                    })?;
                    anyhow::ensure!(
                        matches!(url.scheme(), "https" | "http")
                            && url.host_str().is_some()
                            && url.username().is_empty()
                            && url.password().is_none()
                            && url.path() == "/"
                            && url.query().is_none()
                            && url.fragment().is_none(),
                        "{S3_ENDPOINT_ENV} 必须是无账号、路径、查询参数或片段的 http(s) 根地址；凭据用 {S3_ACCESS_KEY_ENV} / {S3_SECRET_KEY_ENV}"
                    );
                }
                let prefix = env_opt(S3_PREFIX_ENV)
                    .map(|raw| raw.trim_matches('/').to_string())
                    .filter(|raw| !raw.is_empty());
                if let Some(prefix) = &prefix {
                    Path::parse(prefix).map_err(|error| {
                        anyhow::anyhow!("{S3_PREFIX_ENV} 不是安全的对象键前缀：{error}")
                    })?;
                }
                let access_key = env_opt(S3_ACCESS_KEY_ENV);
                let secret_key = env_opt(S3_SECRET_KEY_ENV);
                anyhow::ensure!(
                    access_key.is_some() == secret_key.is_some(),
                    "{S3_ACCESS_KEY_ENV} 与 {S3_SECRET_KEY_ENV} 必须一起设置；都不设时使用实例 / 容器角色"
                );
                anyhow::ensure!(
                    env_opt(S3_SESSION_TOKEN_ENV).is_none() || access_key.is_some(),
                    "{S3_SESSION_TOKEN_ENV} 不能单独设置"
                );
                Ok(Self::S3 {
                    bucket,
                    region,
                    endpoint,
                    prefix,
                    temp_dir,
                })
            }
            other => anyhow::bail!(
                "{BACKEND_ENV} 只接受 fs 或 s3，现在是「{other}」；生产要用 S3 时必须显式写 s3"
            ),
        }
    }

    pub fn describe(&self) -> String {
        match self {
            Self::Filesystem { root } => format!("本机文件系统 {}", root.display()),
            Self::S3 {
                bucket,
                region,
                endpoint,
                prefix,
                ..
            } => {
                let endpoint = endpoint.as_deref().unwrap_or("AWS S3");
                let prefix = prefix
                    .as_deref()
                    .map(|value| format!("/{value}"))
                    .unwrap_or_default();
                format!("S3 {endpoint} bucket={bucket}{prefix} region={region}")
            }
        }
    }
}

fn env_opt(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn required_env(key: &str) -> anyhow::Result<String> {
    env_opt(key).ok_or_else(|| anyhow::anyhow!("{BACKEND_ENV}=s3 时必须设置 {key}"))
}

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("哈希格式不对：{0}")]
    BadHash(String),
    #[error("slug 不合法：{0}")]
    BadSlug(String),
    #[error("对象内容不是合法 JSON：{key}：{source}")]
    BadJson {
        key: String,
        #[source]
        source: serde_json::Error,
    },
    #[error("对象大得不合理：{key} 有 {size} 字节，上限是 {limit}")]
    TooLarge { key: String, size: u64, limit: u64 },
    #[error("读写对象存储失败：{key}：{source}")]
    Object {
        key: String,
        #[source]
        source: object_store::Error,
    },
    #[error("读写上传临时文件失败：{key}：{source}")]
    Io {
        key: String,
        #[source]
        source: std::io::Error,
    },
    #[error("{0}")]
    Unsupported(String),
}

/// 文件系统和 S3 共用的对象存储。克隆只克隆句柄，不复制客户端或连接池。
#[derive(Clone)]
pub struct Store {
    inner: Arc<dyn ObjectStore>,
    temp_dir: PathBuf,
    local_root: Option<PathBuf>,
    description: Arc<str>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MigrationReport {
    pub objects: usize,
    pub copied: usize,
    pub skipped: usize,
    pub bytes: u64,
}

impl std::fmt::Debug for Store {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Store")
            .field("backend", &self.description)
            .field("temp_dir", &self.temp_dir)
            .finish_non_exhaustive()
    }
}

/// 兼容已有调用者；这个名字只表示用 [`Store::new`] 建的是文件系统后端。
pub type FsStore = Store;

impl Store {
    /// 本机文件系统后端。`root` 不存在会创建；配置错误在这里属于启动时致命错误。
    pub fn new(root: impl Into<PathBuf>) -> Self {
        let root = root.into();
        Self::from_config(&StoreConfig::Filesystem { root }).expect("本机对象存储目录建不起来")
    }

    pub fn from_config(config: &StoreConfig) -> anyhow::Result<Self> {
        match config {
            StoreConfig::Filesystem { root } => {
                std::fs::create_dir_all(root).map_err(|error| {
                    anyhow::anyhow!("建不了对象存储目录 {}：{error}", root.display())
                })?;
                let backend = LocalFileSystem::new_with_prefix(root).map_err(|error| {
                    anyhow::anyhow!("对象存储目录 {} 不能使用：{error}", root.display())
                })?;
                Ok(Self {
                    inner: Arc::new(backend),
                    temp_dir: root.join("tmp"),
                    local_root: Some(root.clone()),
                    description: StoreConfig::Filesystem { root: root.clone() }
                        .describe()
                        .into(),
                })
            }
            StoreConfig::S3 {
                bucket,
                region,
                endpoint,
                prefix,
                temp_dir,
            } => {
                std::fs::create_dir_all(temp_dir).map_err(|error| {
                    anyhow::anyhow!("建不了上传临时目录 {}：{error}", temp_dir.display())
                })?;
                // object_store 使用 rustls-no-provider；全仓共用 ring，不再额外链接 AWS-LC。
                let _ = rustls::crypto::ring::default_provider().install_default();
                let mut builder = AmazonS3Builder::from_env()
                    .with_bucket_name(bucket)
                    .with_region(region)
                    .with_client_options(
                        ClientOptions::default()
                            .with_connect_timeout(std::time::Duration::from_secs(5))
                            .with_timeout(std::time::Duration::from_secs(30)),
                    )
                    .with_retry(RetryConfig {
                        max_retries: 3,
                        ..Default::default()
                    });
                if let (Some(access_key), Some(secret_key)) =
                    (env_opt(S3_ACCESS_KEY_ENV), env_opt(S3_SECRET_KEY_ENV))
                {
                    builder = builder
                        .with_access_key_id(access_key)
                        .with_secret_access_key(secret_key);
                    if let Some(token) = env_opt(S3_SESSION_TOKEN_ENV) {
                        builder = builder.with_token(token);
                    }
                }
                if let Some(endpoint) = endpoint {
                    builder = builder
                        .with_endpoint(endpoint)
                        .with_allow_http(endpoint.starts_with("http://"))
                        // 自定义 endpoint 默认使用 path-style；AWS 原生地址也接受这一形态。
                        .with_virtual_hosted_style_request(false);
                }
                let backend = builder
                    .build()
                    .map_err(|error| anyhow::anyhow!("S3 客户端配置不完整：{error}"))?;
                let inner: Arc<dyn ObjectStore> = match prefix {
                    Some(prefix) => Arc::new(PrefixStore::new(backend, Path::parse(prefix)?)),
                    None => Arc::new(backend),
                };
                Ok(Self {
                    inner,
                    temp_dir: temp_dir.clone(),
                    local_root: None,
                    description: config.describe().into(),
                })
            }
        }
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn is_s3(&self) -> bool {
        self.local_root.is_none()
    }

    /// 只供兼容测试和本地诊断；业务读取不能依赖路径，因为 S3 没有本地路径。
    pub fn root(&self) -> Option<&FsPath> {
        self.local_root.as_deref()
    }

    /// 只供验证文件系统布局；S3 调用者必须走 [`Self::get_blob`]。
    pub fn blob_path(&self, hash: &str) -> Result<PathBuf, StoreError> {
        Self::check_hash(hash)?;
        let root = self.local_root.as_ref().ok_or_else(|| {
            StoreError::Unsupported("S3 对象没有本地路径；请走流式读取接口".to_string())
        })?;
        Ok(root.join(blob_key(hash)))
    }

    fn object(key: &str, source: object_store::Error) -> StoreError {
        StoreError::Object {
            key: key.to_string(),
            source,
        }
    }

    fn io(key: &str, source: std::io::Error) -> StoreError {
        StoreError::Io {
            key: key.to_string(),
            source,
        }
    }

    fn check_hash(hash: &str) -> Result<(), StoreError> {
        if is_valid_hex(hash) {
            Ok(())
        } else {
            Err(StoreError::BadHash(hash.to_string()))
        }
    }

    fn check_slug(value: &str) -> Result<(), StoreError> {
        slug::validate(value).map_err(|_| StoreError::BadSlug(value.to_string()))
    }

    fn path(key: &str) -> Result<Path, StoreError> {
        Path::parse(key)
            .map_err(|source| Self::object(key, object_store::Error::InvalidPath { source }))
    }

    pub async fn probe_read(&self) -> Result<(), StoreError> {
        let key = crate::capabilities::KEY;
        match self.inner.head(&Self::path(key)?).await {
            Ok(_) | Err(object_store::Error::NotFound { .. }) => Ok(()),
            Err(error) => Err(Self::object(key, error)),
        }
    }

    /// API 启动时验证写、读、删三种权限。探针名不含秘密，删除失败也会明确阻止启动。
    pub async fn probe_write(&self) -> Result<(), StoreError> {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|value| value.as_nanos())
            .unwrap_or_default();
        let key = format!("system/probes/{}-{nanos}", std::process::id());
        let payload = b"playtest-storage-probe";
        self.put_bytes(&key, payload).await?;
        let checked = self.get_small(&key, 1024).await;
        match checked {
            Ok(Some(bytes)) if bytes == payload => self.remove_key(&key).await,
            Ok(_) => {
                let _ = self.remove_key(&key).await;
                Err(StoreError::Unsupported(format!(
                    "对象存储写入 {key} 后没有读回相同内容"
                )))
            }
            Err(error) => {
                let _ = self.remove_key(&key).await;
                Err(error)
            }
        }
    }

    /// 把一份停止写入的旧存储复制到新存储，并逐对象计算内容哈希验证。
    ///
    /// blob 与版本清单不可变：目标已有但内容不同就中止，不覆盖。current/live/policy 等快照
    /// 可以在重跑时覆盖，但仍会在写后重新读取校验。当前指针最后复制，目标不会先暴露半套作品。
    pub async fn migrate_to(&self, destination: &Store) -> Result<MigrationReport, StoreError> {
        let mut objects = self.inner.list(None);
        let mut metas = Vec::new();
        while let Some(meta) = objects
            .try_next()
            .await
            .map_err(|error| Self::object("/", error))?
        {
            let key = meta.location.to_string();
            if key.starts_with("tmp/")
                || key.starts_with("upload-tmp/")
                || key.starts_with("system/probes/")
                || key.split('/').any(|part| part.starts_with('.'))
            {
                continue;
            }
            metas.push(meta);
        }
        metas.sort_by_key(|meta| migration_rank(meta.location.as_ref()));

        let mut report = MigrationReport::default();
        for meta in metas {
            let key = meta.location.to_string();
            let (source_size, source_hash) = self.checksum(&key).await?.ok_or_else(|| {
                StoreError::Unsupported(format!("迁移时源对象 {key} 消失了，请停止写入后重跑"))
            })?;
            if let Some(expected) = blob_hash_from_key(&key) {
                if source_hash != expected {
                    return Err(StoreError::Unsupported(format!(
                        "源 blob {key} 的内容哈希是 {source_hash}，与对象键不一致；不能把损坏对象迁过去"
                    )));
                }
            }

            if let Some((destination_size, destination_hash)) = destination.checksum(&key).await? {
                if destination_size == source_size && destination_hash == source_hash {
                    report.objects += 1;
                    report.skipped += 1;
                    report.bytes = report.bytes.saturating_add(source_size);
                    continue;
                }
                if immutable_key(&key) {
                    return Err(StoreError::Unsupported(format!(
                        "目标里已有内容不同的不可变对象 {key}；为防止覆盖另一份作品，迁移已停止"
                    )));
                }
            }

            self.copy_key_to(&key, destination).await?;
            let verified = destination.checksum(&key).await?;
            if verified != Some((source_size, source_hash.clone())) {
                return Err(StoreError::Unsupported(format!(
                    "目标对象 {key} 写后校验不一致；迁移已停止，旧存储没有改动"
                )));
            }
            report.objects += 1;
            report.copied += 1;
            report.bytes = report.bytes.saturating_add(source_size);
        }
        Ok(report)
    }

    pub async fn has_blob(&self, hash: &str) -> Result<bool, StoreError> {
        Ok(self.blob_size(hash).await?.is_some())
    }

    pub async fn blob_size(&self, hash: &str) -> Result<Option<u64>, StoreError> {
        Self::check_hash(hash)?;
        self.head_key(&blob_key(hash)).await
    }

    pub async fn get_blob(
        &self,
        hash: &str,
        range: Option<std::ops::Range<u64>>,
    ) -> Result<Option<GetResult>, StoreError> {
        Self::check_hash(hash)?;
        let key = blob_key(hash);
        let path = Self::path(&key)?;
        let options = GetOptions {
            range: range.map(GetRange::Bounded),
            ..GetOptions::default()
        };
        match self.inner.get_opts(&path, options).await {
            Ok(result) => Ok(Some(result)),
            Err(object_store::Error::NotFound { .. }) => Ok(None),
            Err(error) => Err(Self::object(&key, error)),
        }
    }

    pub async fn get_blob_bytes(
        &self,
        hash: &str,
        limit: u64,
    ) -> Result<Option<Vec<u8>>, StoreError> {
        Self::check_hash(hash)?;
        let key = blob_key(hash);
        let Some(result) = self.get_key(&key).await? else {
            return Ok(None);
        };
        if result.meta.size > limit {
            return Err(StoreError::TooLarge {
                key,
                size: result.meta.size,
                limit,
            });
        }
        result
            .bytes()
            .await
            .map(|bytes| Some(bytes.to_vec()))
            .map_err(|error| Self::object(&blob_key(hash), error))
    }

    /// 小对象直接写；对象存储的 `put` 对外是原子可见的。
    pub async fn put_blob(&self, hash: &str, data: &[u8]) -> Result<(), StoreError> {
        Self::check_hash(hash)?;
        self.put_bytes(&blob_key(hash), data).await
    }

    pub fn tmp_dir(&self) -> PathBuf {
        self.temp_dir.clone()
    }

    pub async fn new_tmp_path(&self) -> Result<PathBuf, StoreError> {
        static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

        tokio::fs::create_dir_all(&self.temp_dir)
            .await
            .map_err(|error| Self::io("upload-tmp", error))?;
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|value| value.as_nanos())
            .unwrap_or_default();
        let seq = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Ok(self
            .temp_dir
            .join(format!("{}-{nanos}-{seq}.part", std::process::id())))
    }

    /// 把已校验 SHA-256 的本机临时文件，以有界内存、固定分段写进对象存储。
    pub async fn put_blob_from_path(&self, hash: &str, tmp: &FsPath) -> Result<(), StoreError> {
        Self::check_hash(hash)?;
        let key = blob_key(hash);
        let path = Self::path(&key)?;
        let mut file = tokio::fs::File::open(tmp)
            .await
            .map_err(|error| Self::io(&tmp.display().to_string(), error))?;
        let size = file
            .metadata()
            .await
            .map_err(|error| Self::io(&tmp.display().to_string(), error))?
            .len();
        if size == 0 {
            return self.put_bytes(&key, &[]).await;
        }

        let upload = self
            .inner
            .put_multipart(&path)
            .await
            .map_err(|error| Self::object(&key, error))?;
        let mut writer = WriteMultipart::new_with_chunk_size(upload, MULTIPART_CHUNK);
        let mut buffer = vec![0u8; 1024 * 1024];
        loop {
            let read = match file.read(&mut buffer).await {
                Ok(0) => break,
                Ok(read) => read,
                Err(error) => {
                    let _ = writer.abort().await;
                    return Err(Self::io(&tmp.display().to_string(), error));
                }
            };
            if let Err(error) = writer.wait_for_capacity(MULTIPART_CONCURRENCY).await {
                let _ = writer.abort().await;
                return Err(Self::object(&key, error));
            }
            writer.write(&buffer[..read]);
        }
        writer
            .finish()
            .await
            .map(|_| ())
            .map_err(|error| Self::object(&key, error))
    }

    pub async fn put_manifest(&self, manifest: &Manifest) -> Result<(), StoreError> {
        Self::check_slug(&manifest.slug)?;
        self.write_json(&manifest_key(&manifest.slug, manifest.version), manifest)
            .await
    }

    pub async fn get_manifest(
        &self,
        slug: &str,
        version: u32,
    ) -> Result<Option<Manifest>, StoreError> {
        Self::check_slug(slug)?;
        self.read_json(&manifest_key(slug, version)).await
    }

    pub async fn set_current(&self, slug: &str, current: &Current) -> Result<(), StoreError> {
        Self::check_slug(slug)?;
        self.write_json(&current_key(slug), current).await
    }

    pub async fn get_current(&self, slug: &str) -> Result<Option<Current>, StoreError> {
        Self::check_slug(slug)?;
        self.read_json(&current_key(slug)).await
    }

    pub async fn put_plaza(&self, plaza: &crate::plaza::Plaza) -> Result<(), StoreError> {
        self.write_json(crate::plaza::KEY, plaza).await
    }

    pub async fn get_plaza(&self) -> Result<Option<crate::plaza::Plaza>, StoreError> {
        self.read_json(crate::plaza::KEY).await
    }

    pub async fn put_live(&self, live: &crate::live::SiteLive) -> Result<(), StoreError> {
        Self::check_slug(&live.slug)?;
        self.write_json(&crate::live::key(&live.slug), live).await
    }

    pub async fn get_live(&self, slug: &str) -> Result<Option<crate::live::SiteLive>, StoreError> {
        Self::check_slug(slug)?;
        self.read_json(&crate::live::key(slug)).await
    }

    pub async fn put_policy(
        &self,
        slug: &str,
        policy: &crate::quota::Policy,
    ) -> Result<(), StoreError> {
        Self::check_slug(slug)?;
        self.write_json(&crate::quota::key(slug), policy).await
    }

    pub async fn get_policy(&self, slug: &str) -> Result<Option<crate::quota::Policy>, StoreError> {
        Self::check_slug(slug)?;
        self.read_json(&crate::quota::key(slug)).await
    }

    pub async fn put_capabilities(
        &self,
        capabilities: &crate::capabilities::Capabilities,
    ) -> Result<(), StoreError> {
        self.write_json(crate::capabilities::KEY, capabilities)
            .await
    }

    pub async fn get_capabilities(
        &self,
    ) -> Result<Option<crate::capabilities::Capabilities>, StoreError> {
        self.read_json(crate::capabilities::KEY).await
    }

    /// 公钥等少数非 JSON 控制对象走这个入口；key 必须是代码内常量，不接受玩家输入。
    pub async fn put_bytes(&self, key: &str, data: &[u8]) -> Result<(), StoreError> {
        let path = Self::path(key)?;
        self.inner
            .put(&path, data.to_vec().into())
            .await
            .map(|_| ())
            .map_err(|error| Self::object(key, error))
    }

    pub async fn get_small(&self, key: &str, limit: u64) -> Result<Option<Vec<u8>>, StoreError> {
        let Some(result) = self.get_key(key).await? else {
            return Ok(None);
        };
        if result.meta.size > limit {
            return Err(StoreError::TooLarge {
                key: key.to_string(),
                size: result.meta.size,
                limit,
            });
        }
        result
            .bytes()
            .await
            .map(|bytes| Some(bytes.to_vec()))
            .map_err(|error| Self::object(key, error))
    }

    /// 删除一个作品的全部控制对象。blob 跨作品去重，不在这里删。
    pub async fn remove_site(&self, slug: &str) -> Result<(), StoreError> {
        Self::check_slug(slug)?;
        let prefix_key = format!("sites/{slug}");
        let prefix = Self::path(&prefix_key)?;
        // 先固定清单再删，避免 S3 的分页游标因为页内对象被删而跳过后续键。
        let locations = self
            .inner
            .list(Some(&prefix))
            .map_ok(|meta| meta.location)
            .try_collect::<Vec<_>>()
            .await
            .map_err(|error| Self::object(&prefix_key, error))?;
        for location in locations {
            let key = location.to_string();
            match self.inner.delete(&location).await {
                Ok(()) | Err(object_store::Error::NotFound { .. }) => {}
                Err(error) => return Err(Self::object(&key, error)),
            }
        }
        Ok(())
    }

    /// 所有仍存在清单引用的哈希。任何清单读不出或解析失败都会让整轮回收失败。
    pub async fn referenced_hashes(&self) -> Result<std::collections::HashSet<String>, StoreError> {
        let mut out = std::collections::HashSet::new();
        let prefix_key = "sites";
        let prefix = Self::path(prefix_key)?;
        let mut objects = self.inner.list(Some(&prefix));
        while let Some(meta) = objects
            .try_next()
            .await
            .map_err(|error| Self::object(prefix_key, error))?
        {
            let key = meta.location.to_string();
            if !key.contains("/manifests/") || !key.ends_with(".json") {
                continue;
            }
            if meta.size > MAX_JSON_BYTES {
                return Err(StoreError::TooLarge {
                    key,
                    size: meta.size,
                    limit: MAX_JSON_BYTES,
                });
            }
            let bytes = self
                .inner
                .get(&meta.location)
                .await
                .map_err(|error| Self::object(&key, error))?
                .bytes()
                .await
                .map_err(|error| Self::object(&key, error))?;
            let manifest: Manifest =
                serde_json::from_slice(&bytes).map_err(|source| StoreError::BadJson {
                    key: key.clone(),
                    source,
                })?;
            if let Some(cover) = manifest.cover {
                out.insert(cover.hash);
            }
            if let Some(article) = manifest.article {
                out.insert(article.hash);
            }
            out.extend(manifest.files.into_iter().map(|entry| entry.hash));
        }
        Ok(out)
    }

    /// 所有 blob 的哈希、修改时间和大小，用于带安全等待期的垃圾回收。
    pub async fn list_blobs(
        &self,
    ) -> Result<Vec<(String, std::time::SystemTime, u64)>, StoreError> {
        let prefix_key = "blobs";
        let prefix = Self::path(prefix_key)?;
        let mut objects = self.inner.list(Some(&prefix));
        let mut out = Vec::new();
        while let Some(meta) = objects
            .try_next()
            .await
            .map_err(|error| Self::object(prefix_key, error))?
        {
            let Some(name) = meta.location.filename() else {
                continue;
            };
            if !is_valid_hex(name) {
                continue;
            }
            out.push((name.to_string(), meta.last_modified.into(), meta.size));
        }
        Ok(out)
    }

    pub async fn remove_blob(&self, hash: &str) -> Result<(), StoreError> {
        Self::check_hash(hash)?;
        self.remove_key(&blob_key(hash)).await
    }

    async fn remove_key(&self, key: &str) -> Result<(), StoreError> {
        let path = Self::path(key)?;
        match self.inner.delete(&path).await {
            Ok(()) | Err(object_store::Error::NotFound { .. }) => Ok(()),
            Err(error) => Err(Self::object(key, error)),
        }
    }

    async fn checksum(&self, key: &str) -> Result<Option<(u64, String)>, StoreError> {
        let Some(result) = self.get_key(key).await? else {
            return Ok(None);
        };
        let mut stream = result.into_stream();
        let mut hasher = crate::hash::Hasher::new();
        let mut size = 0u64;
        while let Some(chunk) = stream
            .try_next()
            .await
            .map_err(|error| Self::object(key, error))?
        {
            size = size.saturating_add(chunk.len() as u64);
            hasher.update(&chunk);
        }
        Ok(Some((size, hasher.finish())))
    }

    async fn copy_key_to(&self, key: &str, destination: &Store) -> Result<(), StoreError> {
        let Some(result) = self.get_key(key).await? else {
            return Err(StoreError::Unsupported(format!(
                "迁移时源对象 {key} 消失了，请停止写入后重跑"
            )));
        };
        if result.meta.size == 0 {
            return destination.put_bytes(key, &[]).await;
        }

        let path = Self::path(key)?;
        let upload = destination
            .inner
            .put_multipart(&path)
            .await
            .map_err(|error| Self::object(key, error))?;
        let mut writer = WriteMultipart::new_with_chunk_size(upload, MULTIPART_CHUNK);
        let mut stream = result.into_stream();
        loop {
            match stream.try_next().await {
                Ok(Some(chunk)) => {
                    if let Err(error) = writer.wait_for_capacity(MULTIPART_CONCURRENCY).await {
                        let _ = writer.abort().await;
                        return Err(Self::object(key, error));
                    }
                    writer.put(chunk);
                }
                Ok(None) => break,
                Err(error) => {
                    let _ = writer.abort().await;
                    return Err(Self::object(key, error));
                }
            }
        }
        writer
            .finish()
            .await
            .map(|_| ())
            .map_err(|error| Self::object(key, error))
    }

    async fn head_key(&self, key: &str) -> Result<Option<u64>, StoreError> {
        let path = Self::path(key)?;
        match self.inner.head(&path).await {
            Ok(meta) => Ok(Some(meta.size)),
            Err(object_store::Error::NotFound { .. }) => Ok(None),
            Err(error) => Err(Self::object(key, error)),
        }
    }

    async fn get_key(&self, key: &str) -> Result<Option<GetResult>, StoreError> {
        let path = Self::path(key)?;
        match self.inner.get(&path).await {
            Ok(result) => Ok(Some(result)),
            Err(object_store::Error::NotFound { .. }) => Ok(None),
            Err(error) => Err(Self::object(key, error)),
        }
    }

    async fn read_json<T: for<'de> Deserialize<'de>>(
        &self,
        key: &str,
    ) -> Result<Option<T>, StoreError> {
        let Some(result) = self.get_key(key).await? else {
            return Ok(None);
        };
        if result.meta.size > MAX_JSON_BYTES {
            return Err(StoreError::TooLarge {
                key: key.to_string(),
                size: result.meta.size,
                limit: MAX_JSON_BYTES,
            });
        }
        let bytes = result
            .bytes()
            .await
            .map_err(|error| Self::object(key, error))?;
        serde_json::from_slice(&bytes)
            .map(Some)
            .map_err(|source| StoreError::BadJson {
                key: key.to_string(),
                source,
            })
    }

    async fn write_json<T: Serialize + ?Sized>(
        &self,
        key: &str,
        value: &T,
    ) -> Result<(), StoreError> {
        let data = serde_json::to_vec(value).map_err(|source| StoreError::BadJson {
            key: key.to_string(),
            source,
        })?;
        self.put_bytes(key, &data).await
    }
}

fn immutable_key(key: &str) -> bool {
    key.starts_with("blobs/") || key.contains("/manifests/")
}

fn blob_hash_from_key(key: &str) -> Option<&str> {
    if !key.starts_with("blobs/") {
        return None;
    }
    let hash = key.rsplit('/').next()?;
    is_valid_hex(hash).then_some(hash)
}

fn migration_rank(key: &str) -> u8 {
    if key.starts_with("blobs/") {
        0
    } else if key.contains("/manifests/") {
        1
    } else if key.ends_with("/current.json") {
        3
    } else {
        2
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hash::hash_bytes;
    use crate::manifest::{FileEntry, GateMode, SCHEMA};

    fn sample_manifest(slug: &str, version: u32) -> Manifest {
        Manifest {
            schema: SCHEMA,
            slug: slug.to_string(),
            version,
            title: "测试作品".into(),
            developer: "匿名开发者".into(),
            note: None,
            summary: None,
            cover: None,
            created_at: "2026-09-07T00:00:00Z".into(),
            expires_at: None,
            badge: true,
            gate: GateMode::Once,
            isolated: false,
            spa: false,
            engine: None,
            kind: crate::manifest::WorkKind::Web,
            entry: None,
            article: None,
            chapters: vec![],
            files: vec![FileEntry {
                path: "index.html".into(),
                hash: hash_bytes(b"<h1>hi</h1>"),
                size: 11,
            }],
        }
    }

    #[tokio::test]
    async fn cover_counts_as_referenced() {
        let dir = tempfile::tempdir().unwrap();
        let store = FsStore::new(dir.path());
        let mut manifest = sample_manifest("brisk-otter-41", 1);
        let cover_hash = hash_bytes(b"\x89PNG fake");
        manifest.cover = Some(crate::manifest::Cover {
            hash: cover_hash.clone(),
            size: 9,
            mime: "image/png".into(),
        });
        store.put_manifest(&manifest).await.unwrap();
        let referenced = store.referenced_hashes().await.unwrap();
        assert!(referenced.contains(&cover_hash), "封面的 blob 不能被回收");
        assert!(referenced.contains(&manifest.files[0].hash));
    }

    #[tokio::test]
    async fn plaza_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let store = FsStore::new(dir.path());
        assert!(store.get_plaza().await.unwrap().is_none());
        let plaza = crate::plaza::Plaza {
            schema: crate::plaza::SCHEMA,
            generated_at: "2026-09-08T00:00:00Z".into(),
            items: vec![],
            club_followers: 0,
            collections: Vec::new(),
        };
        store.put_plaza(&plaza).await.unwrap();
        assert_eq!(store.get_plaza().await.unwrap(), Some(plaza));
        assert!(dir.path().join("plaza.json").is_file());

        assert!(store.get_live("brisk-otter-41").await.unwrap().is_none());
        let mut live = crate::live::SiteLive::empty("brisk-otter-41");
        live.seats = Some(10);
        live.joined = 3;
        store.put_live(&live).await.unwrap();
        assert_eq!(store.get_live("brisk-otter-41").await.unwrap(), Some(live));
        assert!(dir.path().join("sites/brisk-otter-41/live.json").is_file());

        assert!(store.get_capabilities().await.unwrap().is_none());
        let caps = crate::capabilities::Capabilities {
            schema: crate::capabilities::SCHEMA,
            generated_at: "2026-09-09T00:00:00Z".into(),
            email: true,
            push_public_key: None,
        };
        store.put_capabilities(&caps).await.unwrap();
        assert_eq!(store.get_capabilities().await.unwrap(), Some(caps));
    }

    #[tokio::test]
    async fn blob_round_trip_layout_and_range() {
        let dir = tempfile::tempdir().unwrap();
        let store = FsStore::new(dir.path());
        let data = b"<h1>hi</h1>";
        let hash = hash_bytes(data);

        assert!(!store.has_blob(&hash).await.unwrap());
        store.put_blob(&hash, data).await.unwrap();
        assert!(store.has_blob(&hash).await.unwrap());
        assert_eq!(
            store.blob_size(&hash).await.unwrap(),
            Some(data.len() as u64)
        );

        let path = store.blob_path(&hash).unwrap();
        assert!(path.ends_with(format!("blobs/{}/{}", &hash[..2], hash)));
        assert_eq!(std::fs::read(path).unwrap(), data);

        let range = store
            .get_blob(&hash, Some(3..7))
            .await
            .unwrap()
            .unwrap()
            .bytes()
            .await
            .unwrap();
        assert_eq!(range.as_ref(), b">hi<");
    }

    #[tokio::test]
    async fn path_upload_is_streamed_and_probe_cleans_up() {
        let dir = tempfile::tempdir().unwrap();
        let store = FsStore::new(dir.path());
        let tmp = store.new_tmp_path().await.unwrap();
        let data = vec![b'x'; MULTIPART_CHUNK + 17];
        tokio::fs::write(&tmp, &data).await.unwrap();
        let hash = hash_bytes(&data);
        store.put_blob_from_path(&hash, &tmp).await.unwrap();
        assert_eq!(
            store.blob_size(&hash).await.unwrap(),
            Some(data.len() as u64)
        );
        store.probe_write().await.unwrap();
        assert!(!dir
            .path()
            .join("system/probes")
            .read_dir()
            .is_ok_and(|mut entries| entries.next().is_some()));
    }

    #[tokio::test]
    async fn multi_chunk_stream_and_range() {
        let dir = tempfile::tempdir().unwrap();
        let store = FsStore::new(dir.path());
        let tmp = store.new_tmp_path().await.unwrap();
        // 跨越两个 8 MiB 分段：8 MiB + 8 MiB + 1024 字节
        let total_size = 2 * MULTIPART_CHUNK + 1024;
        let mut data = vec![0u8; total_size];
        data[0] = b'S';
        data[MULTIPART_CHUNK - 1] = b'A';
        data[MULTIPART_CHUNK] = b'B';
        data[2 * MULTIPART_CHUNK - 1] = b'C';
        data[2 * MULTIPART_CHUNK] = b'D';
        data[total_size - 1] = b'E';

        tokio::fs::write(&tmp, &data).await.unwrap();
        let hash = hash_bytes(&data);
        store.put_blob_from_path(&hash, &tmp).await.unwrap();
        assert_eq!(
            store.blob_size(&hash).await.unwrap(),
            Some(total_size as u64)
        );

        // 跨第一个和第二个 chunk 边界读取 Range
        let boundary_range = (MULTIPART_CHUNK as u64 - 1)..(MULTIPART_CHUNK as u64 + 1);
        let bytes = store
            .get_blob(&hash, Some(boundary_range))
            .await
            .unwrap()
            .unwrap()
            .bytes()
            .await
            .unwrap();
        assert_eq!(bytes.as_ref(), b"AB");

        // 尾部 Range
        let tail_range = (total_size as u64 - 1)..(total_size as u64);
        let bytes = store
            .get_blob(&hash, Some(tail_range))
            .await
            .unwrap()
            .unwrap()
            .bytes()
            .await
            .unwrap();
        assert_eq!(bytes.as_ref(), b"E");
    }

    #[tokio::test]
    async fn migration_is_verified_resumable_and_never_overwrites_immutable_conflicts() {
        let source_dir = tempfile::tempdir().unwrap();
        let destination_dir = tempfile::tempdir().unwrap();
        let source = FsStore::new(source_dir.path());
        let destination = FsStore::new(destination_dir.path());
        let data = b"immutable work";
        let hash = hash_bytes(data);
        source.put_blob(&hash, data).await.unwrap();
        let mut manifest = sample_manifest("brisk-otter-41", 1);
        manifest.files[0].hash = hash.clone();
        manifest.files[0].size = data.len() as u64;
        source.put_manifest(&manifest).await.unwrap();
        source
            .set_current(
                &manifest.slug,
                &Current {
                    version: 1,
                    updated_at: "2026-09-13T00:00:00Z".into(),
                },
            )
            .await
            .unwrap();

        let first = source.migrate_to(&destination).await.unwrap();
        assert_eq!(first.copied, 3);
        assert_eq!(first.skipped, 0);
        let second = source.migrate_to(&destination).await.unwrap();
        assert_eq!(second.copied, 0);
        assert_eq!(second.skipped, 3);

        destination
            .put_bytes(&manifest_key(&manifest.slug, 1), b"different")
            .await
            .unwrap();
        let error = source
            .migrate_to(&destination)
            .await
            .unwrap_err()
            .to_string();
        assert!(error.contains("不可变对象"), "{error}");
    }

    #[tokio::test]
    async fn manifest_and_current_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let store = FsStore::new(dir.path());
        let manifest = sample_manifest("brisk-otter-41", 1);

        assert!(store.get_current("brisk-otter-41").await.unwrap().is_none());
        store.put_manifest(&manifest).await.unwrap();
        let current = Current {
            version: 1,
            updated_at: "2026-09-07T00:00:01Z".into(),
        };
        store.set_current("brisk-otter-41", &current).await.unwrap();

        assert_eq!(
            store.get_manifest("brisk-otter-41", 1).await.unwrap(),
            Some(manifest)
        );
        assert_eq!(
            store.get_current("brisk-otter-41").await.unwrap(),
            Some(current)
        );

        store.remove_site("brisk-otter-41").await.unwrap();
        assert!(store.get_current("brisk-otter-41").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn rejects_unsafe_keys() {
        let dir = tempfile::tempdir().unwrap();
        let store = FsStore::new(dir.path());
        assert!(matches!(
            store.blob_path("../etc"),
            Err(StoreError::BadHash(_))
        ));
        assert!(matches!(
            store.get_current("../x").await,
            Err(StoreError::BadSlug(_))
        ));
    }
}
