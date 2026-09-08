//! 对象存储的布局，以及本机文件系统实现。
//!
//! 布局（键相对于存储根）：
//!
//! ```text
//! blobs/<hash 前两位>/<hash>              文件内容，不可变，按内容哈希去重
//! sites/<slug>/manifests/<version>.json   一个版本的清单，不可变
//! sites/<slug>/current.json               指向当前版本的指针，回滚就是改它
//! tmp/                                    正在上传的半个文件，只有 api 用，edge 不要读
//! ```
//!
//! api 写、edge 读。本机开发两边共用一个目录；上线后换成 S3 兼容存储时
//! 键不变，只换实现。edge 对 `current.json` 不做长缓存（回滚要立即生效），
//! 对 blob 与清单可以无限期缓存。

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;

use crate::hash::is_valid_hex;
use crate::manifest::Manifest;
use crate::slug;

/// `sites/<slug>/current.json` 的内容。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
    #[error("读写对象存储失败：{key}：{source}")]
    Io {
        key: String,
        #[source]
        source: std::io::Error,
    },
}

/// 本机文件系统上的对象存储。
#[derive(Debug, Clone)]
pub struct FsStore {
    root: PathBuf,
}

impl FsStore {
    /// `root` 不存在会被创建。
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    fn path_of(&self, key: &str) -> PathBuf {
        self.root.join(key)
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

    fn check_slug(s: &str) -> Result<(), StoreError> {
        slug::validate(s).map_err(|_| StoreError::BadSlug(s.to_string()))
    }

    /// blob 在磁盘上的位置。edge 直接用它打开文件、按 Range 定位，不经过内存。
    pub fn blob_path(&self, hash: &str) -> Result<PathBuf, StoreError> {
        Self::check_hash(hash)?;
        Ok(self.path_of(&blob_key(hash)))
    }

    pub async fn has_blob(&self, hash: &str) -> Result<bool, StoreError> {
        let path = self.blob_path(hash)?;
        match tokio::fs::metadata(&path).await {
            Ok(m) => Ok(m.is_file()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(e) => Err(Self::io(&blob_key(hash), e)),
        }
    }

    /// 写入一个 blob。先写临时文件再原子改名，读的一方永远看不到半个文件。
    /// 调用方负责在写之前校验内容哈希等于 `hash`。
    pub async fn put_blob(&self, hash: &str, data: &[u8]) -> Result<(), StoreError> {
        let key = blob_key(hash);
        let path = self.blob_path(hash)?;
        self.write_atomic(&key, &path, data).await
    }

    /// 上传中的文件先落在这里。和 blob 在同一个存储根下，最后那一次改名才是同一个
    /// 文件系统内的原子操作；跨文件系统的 rename 会退化成复制，大文件上传会慢一倍。
    pub fn tmp_dir(&self) -> PathBuf {
        self.path_of("tmp")
    }

    /// 建好临时目录，给一个还没被用过的临时文件路径。
    ///
    /// 几百兆的引擎导出物不能先读进内存再写盘，所以这里只给路径，由调用方自己边收边写、
    /// 边算哈希，确认哈希对得上之后再调 [`Self::put_blob_from_path`]。
    pub async fn new_tmp_path(&self) -> Result<PathBuf, StoreError> {
        static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

        let dir = self.tmp_dir();
        tokio::fs::create_dir_all(&dir)
            .await
            .map_err(|e| Self::io("tmp", e))?;
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let seq = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Ok(dir.join(format!("{}-{nanos}-{seq}.part", std::process::id())))
    }

    /// 把已经落盘的临时文件原子改名到 blob 的位置。
    ///
    /// 调用方负责先确认这个文件的内容哈希等于 `hash`。失败时临时文件留在原处不动，
    /// 由调用方决定是删掉还是留着排查。
    pub async fn put_blob_from_path(&self, hash: &str, tmp: &Path) -> Result<(), StoreError> {
        let key = blob_key(hash);
        let path = self.blob_path(hash)?;
        let parent = path.parent().expect("对象键至少有一级目录");
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| Self::io(&key, e))?;
        tokio::fs::rename(tmp, &path)
            .await
            .map_err(|e| Self::io(&key, e))
    }

    pub async fn put_manifest(&self, m: &Manifest) -> Result<(), StoreError> {
        Self::check_slug(&m.slug)?;
        let key = manifest_key(&m.slug, m.version);
        let data = serde_json::to_vec_pretty(m).map_err(|source| StoreError::BadJson {
            key: key.clone(),
            source,
        })?;
        let path = self.path_of(&key);
        self.write_atomic(&key, &path, &data).await
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
        let key = current_key(slug);
        let data = serde_json::to_vec(current).map_err(|source| StoreError::BadJson {
            key: key.clone(),
            source,
        })?;
        let path = self.path_of(&key);
        self.write_atomic(&key, &path, &data).await
    }

    pub async fn get_current(&self, slug: &str) -> Result<Option<Current>, StoreError> {
        Self::check_slug(slug)?;
        self.read_json(&current_key(slug)).await
    }

    /// 删除一个作品的所有清单与指针。blob 是跨作品去重的，不在这里删（见 [`Self::referenced_hashes`] 与垃圾回收）。
    pub async fn remove_site(&self, slug: &str) -> Result<(), StoreError> {
        Self::check_slug(slug)?;
        let key = format!("sites/{slug}");
        match tokio::fs::remove_dir_all(self.path_of(&key)).await {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(Self::io(&key, e)),
        }
    }

    /// 还活着的所有清单引用的全部哈希——垃圾回收的「根」。
    /// 读的是 `sites/*/manifests/*.json`，包括同一作品的历史版本（回滚要用）；已删作品的目录不在了，自然不算。
    pub async fn referenced_hashes(&self) -> Result<std::collections::HashSet<String>, StoreError> {
        let mut out = std::collections::HashSet::new();
        let sites = self.path_of("sites");
        let mut site_dirs = match tokio::fs::read_dir(&sites).await {
            Ok(d) => d,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(out),
            Err(e) => return Err(Self::io("sites", e)),
        };
        while let Some(site) = site_dirs.next_entry().await.map_err(|e| Self::io("sites", e))? {
            let manifests = site.path().join("manifests");
            let mut files = match tokio::fs::read_dir(&manifests).await {
                Ok(d) => d,
                Err(_) => continue,
            };
            while let Some(f) = files.next_entry().await.map_err(|e| Self::io("manifests", e))? {
                let Ok(bytes) = tokio::fs::read(f.path()).await else { continue };
                // 解析不了的清单当作「引用了所有东西」不现实；跳过它，但绝不因此删 blob——
                // 调用方在有解析失败时应放弃这一轮回收（见返回的 error）。
                let m: Manifest = serde_json::from_slice(&bytes).map_err(|source| StoreError::BadJson {
                    key: f.path().display().to_string(),
                    source,
                })?;
                out.extend(m.files.into_iter().map(|e| e.hash));
            }
        }
        Ok(out)
    }

    /// 所有 blob 的哈希与修改时间（用来只删「够老」的，避免删掉正在上传、还没提交清单的那些）。
    pub async fn list_blobs(&self) -> Result<Vec<(String, std::time::SystemTime)>, StoreError> {
        let mut out = Vec::new();
        let blobs = self.path_of("blobs");
        let mut shards = match tokio::fs::read_dir(&blobs).await {
            Ok(d) => d,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(out),
            Err(e) => return Err(Self::io("blobs", e)),
        };
        while let Some(shard) = shards.next_entry().await.map_err(|e| Self::io("blobs", e))? {
            let mut files = match tokio::fs::read_dir(shard.path()).await {
                Ok(d) => d,
                Err(_) => continue,
            };
            while let Some(f) = files.next_entry().await.map_err(|e| Self::io("blobs", e))? {
                let name = f.file_name().to_string_lossy().into_owned();
                if !is_valid_hex(&name) {
                    continue;
                }
                let modified = f
                    .metadata()
                    .await
                    .and_then(|m| m.modified())
                    .map_err(|e| Self::io(&blob_key(&name), e))?;
                out.push((name, modified));
            }
        }
        Ok(out)
    }

    pub async fn remove_blob(&self, hash: &str) -> Result<(), StoreError> {
        let path = self.blob_path(hash)?;
        match tokio::fs::remove_file(&path).await {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(Self::io(&blob_key(hash), e)),
        }
    }

    async fn read_json<T: for<'de> Deserialize<'de>>(
        &self,
        key: &str,
    ) -> Result<Option<T>, StoreError> {
        match tokio::fs::read(self.path_of(key)).await {
            Ok(bytes) => {
                serde_json::from_slice(&bytes)
                    .map(Some)
                    .map_err(|source| StoreError::BadJson {
                        key: key.to_string(),
                        source,
                    })
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(Self::io(key, e)),
        }
    }

    async fn write_atomic(&self, key: &str, path: &Path, data: &[u8]) -> Result<(), StoreError> {
        let parent = path.parent().expect("对象键至少有一级目录");
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| Self::io(key, e))?;
        let tmp = parent.join(format!(
            ".{}.{}.tmp",
            path.file_name().and_then(|n| n.to_str()).unwrap_or("obj"),
            std::process::id()
        ));
        let mut f = tokio::fs::File::create(&tmp)
            .await
            .map_err(|e| Self::io(key, e))?;
        f.write_all(data).await.map_err(|e| Self::io(key, e))?;
        f.sync_all().await.map_err(|e| Self::io(key, e))?;
        drop(f);
        tokio::fs::rename(&tmp, path)
            .await
            .map_err(|e| Self::io(key, e))
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
            created_at: "2026-09-07T00:00:00Z".into(),
            expires_at: None,
            badge: true,
            gate: GateMode::Once,
            isolated: false,
            spa: false,
            engine: None,
            files: vec![FileEntry {
                path: "index.html".into(),
                hash: hash_bytes(b"<h1>hi</h1>"),
                size: 11,
            }],
        }
    }

    #[tokio::test]
    async fn blob_round_trip_and_layout() {
        let dir = tempfile::tempdir().unwrap();
        let store = FsStore::new(dir.path());
        let data = b"<h1>hi</h1>";
        let hash = hash_bytes(data);

        assert!(!store.has_blob(&hash).await.unwrap());
        store.put_blob(&hash, data).await.unwrap();
        assert!(store.has_blob(&hash).await.unwrap());

        let path = store.blob_path(&hash).unwrap();
        assert!(path.ends_with(format!("blobs/{}/{}", &hash[..2], hash)));
        assert_eq!(std::fs::read(path).unwrap(), data);
    }

    #[tokio::test]
    async fn manifest_and_current_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let store = FsStore::new(dir.path());
        let m = sample_manifest("brisk-otter-41", 1);

        assert!(store.get_current("brisk-otter-41").await.unwrap().is_none());
        store.put_manifest(&m).await.unwrap();
        let cur = Current {
            version: 1,
            updated_at: "2026-09-07T00:00:01Z".into(),
        };
        store.set_current("brisk-otter-41", &cur).await.unwrap();

        assert_eq!(
            store.get_manifest("brisk-otter-41", 1).await.unwrap(),
            Some(m)
        );
        assert_eq!(
            store.get_current("brisk-otter-41").await.unwrap(),
            Some(cur)
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
