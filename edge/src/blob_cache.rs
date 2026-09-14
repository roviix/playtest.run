//! 边缘本地不可变 Blob 缓存（DESIGN §4.2、§4.4）。
//!
//! 当生产环境配置为私有 S3 时，边缘按需从 S3 拉取对象并在本地磁盘做不可变缓存。
//! 优势：
//! 1. 内容哈希（SHA-256）保证对象内容不可变，一旦落盘绝不过期，直接做毫秒级本地文件流响应；
//! 2. 支持 HTTP 206 Range 局部断点续传与视频拖拽 Seek；
//! 3. 边向客户端流式输出、边写临时文件，传输完成原子重命名入库；
//! 4. 设定容量上限（默认 10 GiB，环境变量 `PLAYTEST_EDGE_CACHE_MAX_BYTES` 可调），超限按 LRU 自动清理。

use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::{Duration, SystemTime};

use bytes::Bytes;
use futures_util::{Stream, StreamExt};
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;

/// 临时未完成下载文件的最大保留时间（超过一小时未完成视作废弃并清理）。
const TEMP_FILE_MAX_AGE: Duration = Duration::from_secs(3600);

#[derive(Debug, Clone)]
pub struct BlobCache {
    inner: Arc<BlobCacheInner>,
}

#[derive(Debug)]
struct BlobCacheInner {
    cache_dir: PathBuf,
    temp_dir: PathBuf,
    max_bytes: u64,
    pruning: AtomicBool,
}

impl BlobCache {
    pub fn new(cache_dir: impl Into<PathBuf>, max_bytes: u64) -> Self {
        let cache_dir = cache_dir.into();
        let temp_dir = cache_dir.join("tmp");
        let _ = std::fs::create_dir_all(&cache_dir);
        let _ = std::fs::create_dir_all(&temp_dir);
        Self {
            inner: Arc::new(BlobCacheInner {
                cache_dir,
                temp_dir,
                max_bytes,
                pruning: AtomicBool::new(false),
            }),
        }
    }

    /// Blob 在本地磁盘上的不可变路径：`{cache_dir}/{hash[0..2]}/{hash}`。
    pub fn blob_path(&self, hash: &str) -> PathBuf {
        if hash.len() < 2 {
            return self.inner.cache_dir.join(hash);
        }
        self.inner.cache_dir.join(&hash[..2]).join(hash)
    }

    /// 查询本地是否已缓存该 Blob。若存在且是常规文件，返回其完整路径。
    pub fn get(&self, hash: &str) -> Option<PathBuf> {
        let path = self.blob_path(hash);
        if path.is_file() {
            Some(path)
        } else {
            None
        }
    }

    /// 分配一个位于 `tmp/` 的临时下载路径。
    pub fn temp_file_path(&self, hash: &str) -> PathBuf {
        let rand_suffix = rand::random::<u64>();
        self.inner
            .temp_dir
            .join(format!("{hash}.tmp.{rand_suffix:016x}"))
    }

    /// 将已校验完整的临时文件原子更名为正式不可变 Blob 路径。
    pub fn put_file(&self, hash: &str, temp_path: &Path) -> std::io::Result<PathBuf> {
        let final_path = self.blob_path(hash);
        if let Some(parent) = final_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::rename(temp_path, &final_path)?;
        self.trigger_prune();
        Ok(final_path)
    }

    /// 将远端流式响应同时写往客户端与本地临时文件。
    /// 完整传输完毕后原子重命名入库；若客户端中途断开或读取出错，自动清理临时文件。
    pub fn stream_and_cache<S, E>(
        &self,
        hash: &str,
        expected_size: u64,
        upstream: S,
    ) -> axum::body::Body
    where
        S: Stream<Item = Result<Bytes, E>> + Send + Unpin + 'static,
        E: std::fmt::Display + Send + 'static,
    {
        let temp_file_path = self.temp_file_path(hash);
        let final_path = self.blob_path(hash);
        let cache = self.clone();
        let hash_string = hash.to_string();

        let (tx, rx) = mpsc::channel::<Result<Bytes, std::io::Error>>(8);

        tokio::spawn(async move {
            let mut file = match tokio::fs::File::create(&temp_file_path).await {
                Ok(f) => f,
                Err(err) => {
                    tracing::warn!(%err, path = %temp_file_path.display(), "创建本地 blob 缓存临时文件失败");
                    let _ = tx.send(Err(err)).await;
                    return;
                }
            };

            let mut written: u64 = 0;
            let mut stream = upstream;

            while let Some(item) = stream.next().await {
                match item {
                    Ok(bytes) => {
                        if let Err(err) = file.write_all(&bytes).await {
                            tracing::warn!(%err, "写本地 blob 缓存失败");
                            let _ = tx.send(Err(err)).await;
                            let _ = tokio::fs::remove_file(&temp_file_path).await;
                            return;
                        }
                        written = written.saturating_add(bytes.len() as u64);
                        if tx.send(Ok(bytes)).await.is_err() {
                            // 客户端提前关闭了连接（如停止加载或跳出）
                            let _ = tokio::fs::remove_file(&temp_file_path).await;
                            return;
                        }
                    }
                    Err(err) => {
                        let io_err = std::io::Error::other(err.to_string());
                        let _ = tx.send(Err(io_err)).await;
                        let _ = tokio::fs::remove_file(&temp_file_path).await;
                        return;
                    }
                }
            }

            if let Err(err) = file.flush().await {
                tracing::warn!(%err, "刷新本地 blob 缓存失败");
                let _ = tokio::fs::remove_file(&temp_file_path).await;
                return;
            }
            drop(file);

            if written == expected_size {
                if let Some(parent) = final_path.parent() {
                    let _ = tokio::fs::create_dir_all(parent).await;
                }
                if let Err(err) = tokio::fs::rename(&temp_file_path, &final_path).await {
                    tracing::warn!(%err, "重命名本地 blob 缓存失败");
                    let _ = tokio::fs::remove_file(&temp_file_path).await;
                } else {
                    tracing::debug!(hash = %hash_string, size = written, "blob 缓存落盘成功");
                    cache.trigger_prune();
                }
            } else {
                tracing::warn!(
                    expected = expected_size,
                    actual = written,
                    "blob 流式传输字节数与清单不一致，丢弃缓存"
                );
                let _ = tokio::fs::remove_file(&temp_file_path).await;
            }
        });

        axum::body::Body::from_stream(ReceiverStream::new(rx))
    }

    /// 触发轻量后台磁盘清理任务（若已有清理在进行则跳过）。
    pub fn trigger_prune(&self) {
        if self
            .inner
            .pruning
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
        {
            let cache_dir = self.inner.cache_dir.clone();
            let temp_dir = self.inner.temp_dir.clone();
            let max_bytes = self.inner.max_bytes;
            let inner = Arc::clone(&self.inner);
            tokio::spawn(async move {
                let _ = tokio::task::spawn_blocking(move || {
                    prune_cache(&cache_dir, &temp_dir, max_bytes);
                })
                .await;
                inner.pruning.store(false, Ordering::SeqCst);
            });
        }
    }
}

/// 接收通道适配 Stream。
pub struct ReceiverStream<T> {
    inner: mpsc::Receiver<T>,
}

impl<T> ReceiverStream<T> {
    pub fn new(inner: mpsc::Receiver<T>) -> Self {
        Self { inner }
    }
}

impl<T> Stream for ReceiverStream<T> {
    type Item = T;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.inner.poll_recv(cx)
    }
}

struct CacheFileEntry {
    path: PathBuf,
    size: u64,
    modified: SystemTime,
}

fn prune_cache(cache_dir: &Path, temp_dir: &Path, max_bytes: u64) {
    // 1. 清理超时的临时文件
    if let Ok(entries) = std::fs::read_dir(temp_dir) {
        let now = SystemTime::now();
        for entry in entries.flatten() {
            let path = entry.path();
            if let Ok(meta) = path.metadata() {
                if let Ok(mtime) = meta.modified() {
                    if let Ok(age) = now.duration_since(mtime) {
                        if age > TEMP_FILE_MAX_AGE {
                            let _ = std::fs::remove_file(&path);
                        }
                    }
                }
            }
        }
    }

    // 2. 遍历收集所有持久缓存文件
    let mut files = Vec::new();
    let mut total_size: u64 = 0;
    collect_cache_files(cache_dir, temp_dir, &mut files, &mut total_size);

    // 3. 超额淘汰：淘汰至上限的 80%，避免频繁边缘触发
    if total_size > max_bytes {
        let target_size = (max_bytes as f64 * 0.8) as u64;
        files.sort_by_key(|f| f.modified);

        for file in files {
            if total_size <= target_size {
                break;
            }
            if std::fs::remove_file(&file.path).is_ok() {
                total_size = total_size.saturating_sub(file.size);
            }
        }
    }
}

fn collect_cache_files(
    dir: &Path,
    temp_dir: &Path,
    files: &mut Vec<CacheFileEntry>,
    total_size: &mut u64,
) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path == temp_dir {
            continue;
        }
        if path.is_dir() {
            collect_cache_files(&path, temp_dir, files, total_size);
        } else if path.is_file() {
            if let Ok(meta) = path.metadata() {
                let size = meta.len();
                let modified = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
                *total_size = total_size.saturating_add(size);
                files.push(CacheFileEntry {
                    path,
                    size,
                    modified,
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_cache_put_and_get() {
        let dir = tempdir().unwrap();
        let cache = BlobCache::new(dir.path(), 1024 * 1024);

        let hash = "a1b2c3d4e5f67890123456789012345678901234567890123456789012345678";
        assert_eq!(cache.get(hash), None);

        let temp_path = cache.temp_file_path(hash);
        std::fs::write(&temp_path, b"hello playtest cache").unwrap();

        let final_path = cache.put_file(hash, &temp_path).unwrap();
        assert!(final_path.is_file());
        assert_eq!(cache.get(hash), Some(final_path.clone()));

        let content = std::fs::read(&final_path).unwrap();
        assert_eq!(content, b"hello playtest cache");
    }

    #[tokio::test]
    async fn test_stream_and_cache() {
        use http_body_util::BodyExt;

        let dir = tempdir().unwrap();
        let cache = BlobCache::new(dir.path(), 1024 * 1024);
        let hash = "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";

        let chunks = vec![
            Ok::<Bytes, std::io::Error>(Bytes::from_static(b"part 1: ")),
            Ok::<Bytes, std::io::Error>(Bytes::from_static(b"part 2: ")),
            Ok::<Bytes, std::io::Error>(Bytes::from_static(b"part 3 done.")),
        ];
        let stream = futures_util::stream::iter(chunks);

        let expected_size = (b"part 1: ".len() + b"part 2: ".len() + b"part 3 done.".len()) as u64;
        let body = cache.stream_and_cache(hash, expected_size, stream);

        let collected = body.collect().await.unwrap().to_bytes();
        assert_eq!(collected, Bytes::from_static(b"part 1: part 2: part 3 done."));

        // 等待异步落盘完成
        tokio::time::sleep(Duration::from_millis(50)).await;
        let cached = cache.get(hash);
        assert!(cached.is_some(), "缓存文件应该已落盘");
        let content = std::fs::read(cached.unwrap()).unwrap();
        assert_eq!(content, b"part 1: part 2: part 3 done.");
    }

    #[test]
    fn test_prune_lru() {
        let dir = tempdir().unwrap();
        let cache_dir = dir.path().to_path_buf();
        let temp_dir = cache_dir.join("tmp");
        std::fs::create_dir_all(&temp_dir).unwrap();

        // 写入 3 个 100 字节的文件，上限 250 字节
        let f1 = cache_dir.join("01").join("file1");
        let f2 = cache_dir.join("02").join("file2");
        let f3 = cache_dir.join("03").join("file3");
        std::fs::create_dir_all(f1.parent().unwrap()).unwrap();
        std::fs::create_dir_all(f2.parent().unwrap()).unwrap();
        std::fs::create_dir_all(f3.parent().unwrap()).unwrap();

        std::fs::write(&f1, vec![1u8; 100]).unwrap();
        std::thread::sleep(Duration::from_millis(20));
        std::fs::write(&f2, vec![2u8; 100]).unwrap();
        std::thread::sleep(Duration::from_millis(20));
        std::fs::write(&f3, vec![3u8; 100]).unwrap();

        // 300 字节 > 250 字节，淘汰目标是 250 * 0.8 = 200 字节。
        // 最老的 f1 会被删掉，留 f2, f3 (各 100 = 200)
        prune_cache(&cache_dir, &temp_dir, 250);

        assert!(!f1.exists(), "最老的 f1 应该已被淘汰");
        assert!(f2.exists(), "较新的 f2 应该保留");
        assert!(f3.exists(), "最新的 f3 应该保留");
    }
}
