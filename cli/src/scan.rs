//! 把一个目录变成清单：相对路径、内容哈希、大小。

use std::io::Read;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result};
use playtest_common::hash::Hasher;
use playtest_common::manifest::{validate_path, FileEntry};
use tokio::sync::Semaphore;
use tokio::task::JoinSet;
use walkdir::{DirEntry, WalkDir};

/// 一次读多少。再大对 SHA-256 也没什么帮助，反而占内存。
const CHUNK_BYTES: usize = 64 * 1024;

/// 同时算几个文件的哈希。多数导出物是几十上百个小文件，并行主要省的是系统调用的往返。
const HASH_CONCURRENCY: usize = 8;

#[derive(Debug, Clone)]
pub struct ScannedFile {
    pub entry: FileEntry,
    /// 本机上的绝对路径，上传的时候按它读。
    pub source: PathBuf,
}

/// 遍历目录，算好哈希，按路径排序返回。
///
/// 不跟符号链接；以「.」开头的文件和目录（`.DS_Store`、`.git`）一律不进清单。
pub async fn scan_dir(root: &Path) -> Result<Vec<ScannedFile>> {
    let mut todo: Vec<(PathBuf, String)> = Vec::new();
    let walker = WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| e.depth() == 0 || !is_hidden(e));

    for entry in walker {
        let entry =
            entry.with_context(|| format!("could not read what is inside {}", root.display()))?;
        // 符号链接在 follow_links(false) 下 file_type().is_file() 是 false，连同管道、套接字一起跳过。
        if !entry.file_type().is_file() {
            continue;
        }
        let rel = entry
            .path()
            .strip_prefix(root)
            .unwrap_or_else(|_| entry.path());
        match manifest_path(rel) {
            Ok(path) => todo.push((entry.path().to_path_buf(), path)),
            Err(reason) => anyhow::bail!("{reason}"),
        }
    }

    let permits = Arc::new(Semaphore::new(HASH_CONCURRENCY));
    let mut tasks = JoinSet::new();
    for (source, path) in todo {
        // 名额在派活之前拿，拿不到就等——放进闭包里 spawn_blocking 就没法 await 了，
        // 那样几千个文件会一口气占满 tokio 的阻塞线程池和同样多的文件描述符。
        let permit = Arc::clone(&permits)
            .acquire_owned()
            .await
            .context("failed while queueing files for hashing")?;
        tasks.spawn_blocking(move || {
            let _permit = permit;
            let (hash, size) = hash_file(&source)
                .with_context(|| format!("could not read {}", source.display()))?;
            anyhow::Ok(ScannedFile {
                entry: FileEntry { path, hash, size },
                source,
            })
        });
    }

    let mut files = Vec::new();
    while let Some(joined) = tasks.join_next().await {
        files.push(joined.context("a hashing task did not finish")??);
    }
    files.sort_by(|a, b| a.entry.path.cmp(&b.entry.path));
    Ok(files)
}

/// 单文件作品只打包明确列出的路径。文章靠它收原稿与正文实际引用的图片，不会顺手遍历
/// 整个写作目录；视频则只有那一个 MP4。
pub async fn scan_selected(root: &Path, paths: &[String]) -> Result<Vec<ScannedFile>> {
    let root = std::fs::canonicalize(root)
        .with_context(|| format!("could not inspect {}", root.display()))?;
    let mut files = Vec::with_capacity(paths.len());
    for path in paths {
        validate_path(path).map_err(|error| anyhow::anyhow!(error.to_string()))?;
        let source = root.join(path);
        let meta = std::fs::symlink_metadata(&source)
            .with_context(|| format!("the project needs {path}, but that file is missing"))?;
        if meta.file_type().is_symlink() || !meta.is_file() {
            anyhow::bail!("{path} is not a regular file. A single-file project and its images can not be symlinks or directories.")
        }
        let canonical =
            std::fs::canonicalize(&source).with_context(|| format!("could not inspect {path}"))?;
        if !canonical.starts_with(&root) {
            anyhow::bail!("{path} points outside the project directory, so it can not be uploaded.")
        }
        let shown = path.clone();
        let scanned = tokio::task::spawn_blocking(move || {
            let (hash, size) = hash_file(&canonical)
                .with_context(|| format!("could not read {}", canonical.display()))?;
            anyhow::Ok(ScannedFile {
                entry: FileEntry {
                    path: shown,
                    hash,
                    size,
                },
                source: canonical,
            })
        })
        .await
        .context("a hashing task did not finish")??;
        files.push(scanned);
    }
    files.sort_by(|a, b| a.entry.path.cmp(&b.entry.path));
    Ok(files)
}

fn is_hidden(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .is_some_and(|name| name.starts_with('.'))
}

/// 相对路径 → 清单里的样子：正斜杠分隔、没有 `.` 和 `..`。
///
/// Windows 上分隔符是反斜杠，`components()` 会拆开，拼回去自然就是正斜杠；
/// Unix 上反斜杠是合法的文件名字符，拆不开，由 `validate_path` 挡下来并说清楚。
pub fn manifest_path(rel: &Path) -> Result<String, String> {
    let mut parts: Vec<&str> = Vec::new();
    for component in rel.components() {
        match component {
            Component::Normal(part) => match part.to_str() {
                Some(part) => parts.push(part),
                None => {
                    return Err(format!(
                        "This file name is not UTF-8 and the server can not store it. Rename it and upload again: {}",
                        rel.display()
                    ))
                }
            },
            _ => {
                return Err(format!(
                    "This path points outside the upload directory, so it can not be uploaded: {}",
                    rel.display()
                ))
            }
        }
    }
    let joined = parts.join("/");
    validate_path(&joined).map_err(|e| e.to_string())?;
    Ok(joined)
}

/// 流式算 SHA-256，顺便数出实际读到多少字节。
///
/// 大小用读出来的字节数而不是 metadata：清单里写的就该是我们真正会传上去的那些字节。
fn hash_file(path: &Path) -> std::io::Result<(String, u64)> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Hasher::new();
    let mut buffer = vec![0u8; CHUNK_BYTES];
    let mut size = 0u64;
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
        size += read as u64;
    }
    Ok((hasher.finish(), size))
}

/// 重新算一遍某个文件的哈希。上传被服务器判为「内容对不上」时用它确认文件是不是变了。
pub async fn rehash(path: &Path) -> Result<String> {
    let owned = path.to_path_buf();
    let (hash, _) = tokio::task::spawn_blocking(move || hash_file(&owned))
        .await
        .context("重新算哈希的任务没跑完")?
        .with_context(|| format!("读不了 {}", path.display()))?;
    Ok(hash)
}

#[cfg(test)]
mod tests {
    use super::*;
    use playtest_common::hash::hash_bytes;

    #[test]
    fn ordinary_paths_become_slash_separated() {
        assert_eq!(
            manifest_path(Path::new("index.html")).unwrap(),
            "index.html"
        );
        assert_eq!(
            manifest_path(Path::new("Build/game.wasm.br")).unwrap(),
            "Build/game.wasm.br"
        );
        assert_eq!(
            manifest_path(Path::new("深/中文 名.png")).unwrap(),
            "深/中文 名.png"
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_backslash_in_a_unix_filename_is_refused() {
        // Unix 上 `a\b.txt` 是一个文件名，不是两段路径；清单里不许有反斜杠。
        let err = manifest_path(Path::new("a\\b.txt")).unwrap_err();
        assert!(err.contains("backslashes"), "{err}");
    }

    #[cfg(windows)]
    #[test]
    fn windows_separators_are_rewritten() {
        assert_eq!(
            manifest_path(Path::new("Build\\game.wasm.br")).unwrap(),
            "Build/game.wasm.br"
        );
    }

    #[tokio::test]
    async fn walks_sorted_and_skips_dot_entries() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join("assets")).unwrap();
        std::fs::create_dir_all(root.join(".git")).unwrap();
        std::fs::write(root.join("index.html"), b"<html>hi</html>").unwrap();
        std::fs::write(root.join("assets/a.png"), b"png").unwrap();
        std::fs::write(root.join(".DS_Store"), b"junk").unwrap();
        std::fs::write(root.join(".git/HEAD"), b"ref").unwrap();

        let files = scan_dir(root).await.unwrap();
        let paths: Vec<_> = files.iter().map(|f| f.entry.path.as_str()).collect();
        assert_eq!(paths, ["assets/a.png", "index.html"]);

        let index = &files[1].entry;
        assert_eq!(index.hash, hash_bytes(b"<html>hi</html>"));
        assert_eq!(index.size, 15);
    }

    #[tokio::test]
    async fn empty_directory_yields_nothing() {
        let dir = tempfile::tempdir().unwrap();
        assert!(scan_dir(dir.path()).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn rehash_matches_the_scan() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("a.bin");
        // 跨过 64 KiB 一块的边界，确认流式和一次算出来一样。
        let bytes = vec![7u8; CHUNK_BYTES * 2 + 13];
        std::fs::write(&file, &bytes).unwrap();
        assert_eq!(rehash(&file).await.unwrap(), hash_bytes(&bytes));
    }
}
