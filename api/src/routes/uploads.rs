//! 上传一个版本的第二步和第四步：问缺哪些哈希、提交清单（DESIGN §4.2）。
//!
//! 中间那两步在别处：`PUT /v1/blobs/{hash}` 收文件内容，CLI 自己并行重试。

use std::collections::HashSet;

use axum::extract::{Path, State};
use axum::Json;
use playtest_common::api::{CommitUploadResponse, PrepareUploadRequest, PrepareUploadResponse};
use playtest_common::manifest::{self, FileEntry, Manifest, ManifestError};
use playtest_common::limits;
use playtest_common::store::Current;
use uuid::Uuid;

use crate::auth::Caller;
use crate::clock;
use crate::db;
use crate::error::{ApiError, ApiResult};
use crate::routes::sites::NO_SUCH_SITE;
use crate::routes::JsonBody;
use crate::state::AppState;

const NO_SUCH_UPLOAD: &str = "没有这次上传记录，或者它不属于这个作品。重新走一遍准备上传。";
const ALREADY_COMMITTED: &str = "这次上传已经提交过了。要再发一版，重新走一遍准备上传。";

/// 提交失败时最多点名几个没传上来的文件。列全了终端会刷屏，列几个就够定位了。
const MISSING_SHOWN: usize = 5;

pub async fn prepare(
    State(state): State<AppState>,
    caller: Caller,
    Path(slug): Path<String>,
    JsonBody(request): JsonBody<PrepareUploadRequest>,
) -> ApiResult<Json<PrepareUploadResponse>> {
    let site = {
        let conn = state.db().lock().await;
        db::find_live_site(&conn, &slug, &caller.user_id)?
    }
    .ok_or_else(|| ApiError::not_found(NO_SUCH_SITE))?;

    clean_title(request.title.as_deref())?;
    clean_note(request.note.as_deref())?;

    let max_total = if caller.kind.is_anon() {
        limits::ANON_MAX_VERSION_BYTES
    } else {
        limits::MAX_VERSION_BYTES
    };
    manifest::validate_files(&request.files, max_total).map_err(as_api_error)?;

    let wanted = dedup_by_hash(&request.files);
    let present = present_hashes(&state, &wanted).await?;

    let mut missing = Vec::new();
    let mut missing_bytes: u64 = 0;
    for file in &wanted {
        if present.contains(&file.hash) {
            continue;
        }
        missing.push(file.hash.clone());
        missing_bytes = missing_bytes.saturating_add(file.size);
    }

    let upload_id = Uuid::new_v4().to_string();
    let request_json = serde_json::to_string(&request)?;
    {
        let conn = state.db().lock().await;
        db::insert_upload(
            &conn,
            &upload_id,
            &site.slug,
            &caller.user_id,
            &clock::now_string(),
            &request_json,
        )?;
    }

    tracing::info!(
        slug = %site.slug,
        files = request.files.len(),
        missing = missing.len(),
        missing_bytes,
        "准备上传"
    );
    Ok(Json(PrepareUploadResponse {
        upload_id,
        missing,
        missing_bytes,
    }))
}

pub async fn commit(
    State(state): State<AppState>,
    caller: Caller,
    Path((slug, upload_id)): Path<(String, String)>,
) -> ApiResult<Json<CommitUploadResponse>> {
    // 版本号是「当前版本 + 1」，读和写之间夹着两次对象存储写入，所以整个提交要串起来。
    let _serialized = state.lock_commit().await;

    let (site, upload) = {
        let conn = state.db().lock().await;
        let site = db::find_live_site(&conn, &slug, &caller.user_id)?
            .ok_or_else(|| ApiError::not_found(NO_SUCH_SITE))?;
        let upload = db::find_upload(&conn, &upload_id)?
            .ok_or_else(|| ApiError::not_found(NO_SUCH_UPLOAD))?;
        (site, upload)
    };

    if upload.slug != site.slug || upload.user_id != caller.user_id {
        return Err(ApiError::not_found(NO_SUCH_UPLOAD));
    }
    if upload.status != "pending" {
        return Err(ApiError::invalid(ALREADY_COMMITTED));
    }

    let request: PrepareUploadRequest = serde_json::from_str(&upload.request_json)?;
    let present = present_hashes(&state, &dedup_by_hash(&request.files)).await?;
    let absent: Vec<&str> = request
        .files
        .iter()
        .filter(|file| !present.contains(&file.hash))
        .map(|file| file.path.as_str())
        .collect();
    if !absent.is_empty() {
        return Err(ApiError::blobs_missing(missing_message(&absent)));
    }

    let created_at = clock::now_string();
    let version = site.current_version.unwrap_or(0) + 1;
    let title = clean_title(request.title.as_deref())?.unwrap_or_else(|| site.title.clone());
    let note = clean_note(request.note.as_deref())?;

    let mut files = request.files;
    files.sort_by(|a, b| a.path.cmp(&b.path));
    let manifest = Manifest {
        schema: manifest::SCHEMA,
        slug: site.slug.clone(),
        version,
        title: title.clone(),
        developer: caller.display_name.clone(),
        note: note.clone(),
        created_at: created_at.clone(),
        expires_at: site.expires_at.clone(),
        // v0.1 只有匿名和免费档，两档都带角标（DESIGN §3.3）。
        badge: true,
        gate: request.gate,
        isolated: request.isolated,
        spa: request.spa,
        // CLI 上传时认出来的引擎。形状不对就当没说，门禁页那时说「邀请你体验」。
        engine: manifest::clean_engine(request.engine.as_deref()),
        files,
    };
    let file_count = manifest.files.len();
    let total_bytes = manifest.total_bytes();

    // 先写对象存储再回写库：边缘只读对象存储，这个顺序保证「库里说有的版本，边缘一定服务得了」。
    state.store().put_manifest(&manifest).await?;
    state
        .store()
        .set_current(
            &site.slug,
            &Current {
                version,
                updated_at: created_at.clone(),
            },
        )
        .await?;

    {
        let mut conn = state.db().lock().await;
        db::commit_version(
            &mut conn,
            &site.slug,
            version,
            &upload_id,
            &title,
            &created_at,
            note.as_deref(),
            file_count,
            total_bytes,
        )?;
    }

    tracing::info!(slug = %site.slug, version, file_count, total_bytes, "提交了一个版本");
    Ok(Json(CommitUploadResponse {
        url: state.site_url(&site.slug),
        slug: site.slug,
        version,
        expires_at: site.expires_at,
    }))
}

/// 按清单顺序去重：同一份内容在目录里出现两次，只要传一次。
fn dedup_by_hash(files: &[FileEntry]) -> Vec<FileEntry> {
    let mut seen = HashSet::with_capacity(files.len());
    files
        .iter()
        .filter(|file| seen.insert(file.hash.as_str()))
        .cloned()
        .collect()
}

/// 哪些哈希是真的已经有了。blobs 表只是一张能被 SQL 查的索引，真身是对象存储里的文件，
/// 所以表里有、盘上没有的一律算没有——否则提交出去的清单会指向一个打不开的文件。
async fn present_hashes(state: &AppState, files: &[FileEntry]) -> ApiResult<HashSet<String>> {
    let hashes: Vec<String> = files.iter().map(|file| file.hash.clone()).collect();
    let indexed = {
        let conn = state.db().lock().await;
        db::known_blob_hashes(&conn, &hashes)?
    };

    let mut present = HashSet::with_capacity(indexed.len());
    for hash in indexed {
        if state.store().has_blob(&hash).await? {
            present.insert(hash);
        }
    }
    Ok(present)
}

fn missing_message(absent: &[&str]) -> String {
    let shown = absent
        .iter()
        .take(MISSING_SHOWN)
        .copied()
        .collect::<Vec<_>>()
        .join("、");
    let rest = absent.len().saturating_sub(MISSING_SHOWN);
    if rest > 0 {
        format!(
            "还有 {} 个文件没传上来：{shown}，以及另外 {rest} 个。重新运行一次上传，没传成功的会补上。",
            absent.len()
        )
    } else {
        format!("还有 {} 个文件没传上来：{shown}。重新运行一次上传，没传成功的会补上。", absent.len())
    }
}

/// 清单校验的中文原话直接给人看；数量和体积超限用 413，CLI 不看 message 也知道是「太大了」。
fn as_api_error(err: ManifestError) -> ApiError {
    let message = err.to_string();
    match err {
        ManifestError::TooManyFiles { .. }
        | ManifestError::FileTooLarge { .. }
        | ManifestError::VersionTooLarge { .. } => ApiError::too_large(message),
        _ => ApiError::invalid(message),
    }
}

/// 空白的作品名当作没给。
pub fn clean_title(title: Option<&str>) -> ApiResult<Option<String>> {
    clean_text(title, limits::MAX_TITLE_CHARS, "作品名")
}

pub fn clean_note(note: Option<&str>) -> ApiResult<Option<String>> {
    clean_text(note, limits::MAX_NOTE_CHARS, "这版改了什么")
}

fn clean_text(value: Option<&str>, max_chars: usize, what: &str) -> ApiResult<Option<String>> {
    let Some(text) = value.map(str::trim).filter(|t| !t.is_empty()) else {
        return Ok(None);
    };
    let count = text.chars().count();
    if count > max_chars {
        return Err(ApiError::invalid(format!(
            "{what}最多 {max_chars} 个字，这个有 {count} 个。"
        )));
    }
    Ok(Some(text.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(path: &str, hash: &str) -> FileEntry {
        FileEntry {
            path: path.into(),
            hash: hash.into(),
            size: 1,
        }
    }

    #[test]
    fn dedup_keeps_manifest_order() {
        let files = vec![entry("b", "22"), entry("a", "11"), entry("c", "22")];
        let unique = dedup_by_hash(&files);
        assert_eq!(
            unique.iter().map(|f| f.hash.as_str()).collect::<Vec<_>>(),
            ["22", "11"]
        );
    }

    #[test]
    fn missing_message_lists_at_most_five() {
        let all = ["a", "b", "c", "d", "e", "f", "g"];
        let message = missing_message(&all);
        assert!(message.contains("还有 7 个文件"), "{message}");
        assert!(message.contains("以及另外 2 个"), "{message}");
        assert!(!message.contains('f'), "{message}");
    }

    #[test]
    fn blank_title_counts_as_missing() {
        assert_eq!(clean_title(Some("   ")).unwrap(), None);
        assert_eq!(clean_title(Some(" 小球 ")).unwrap().as_deref(), Some("小球"));
        assert!(clean_title(Some(&"字".repeat(limits::MAX_TITLE_CHARS + 1))).is_err());
    }
}
