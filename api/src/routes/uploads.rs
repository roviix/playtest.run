//! 上传一个版本的第二步和第四步：问缺哪些哈希、提交清单（DESIGN §4.2）。
//!
//! 中间那两步在别处：`PUT /v1/blobs/{hash}` 收文件内容，CLI 自己并行重试。

use std::collections::HashSet;

use axum::extract::{Path, State};
use axum::Json;
use futures_util::StreamExt;
use playtest_common::api::{CommitUploadResponse, PrepareUploadRequest, PrepareUploadResponse};
use playtest_common::limits;
use playtest_common::manifest::{
    self, ArticleArtifact, FileEntry, Manifest, ManifestError, WorkKind,
};
use playtest_common::store::Current;
use tokio::io::AsyncWriteExt;
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
const MAX_PENDING_UPLOADS_PER_USER: usize = 8;
pub(crate) const PENDING_UPLOAD_HOURS: i64 = 1;

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
    clean_summary(request.summary.as_deref())?;
    if let Some(cover) = &request.cover {
        manifest::validate_cover(cover).map_err(|e| ApiError::invalid(e.to_string()))?;
    }

    let max_total = if caller.kind.is_anon() {
        limits::ANON_MAX_VERSION_BYTES
    } else {
        limits::MAX_VERSION_BYTES
    };
    manifest::validate_files(&request.files, max_total).map_err(as_api_error)?;
    validate_upload_shape(&request)?;

    // 封面和目录里的文件走同一条上传路：也是一个 blob，缺了就在 `missing` 里。
    let mut wanted = dedup_by_hash(&request.files);
    if let Some(cover) = &request.cover {
        if !wanted.iter().any(|f| f.hash == cover.hash) {
            wanted.push(cover_entry(cover));
        }
    }
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
        let stale_before =
            clock::format(clock::now() - time::Duration::hours(PENDING_UPLOAD_HOURS));
        db::delete_pending_uploads_before(&conn, &stale_before)?;
        if db::count_pending_uploads(&conn, &caller.user_id)? >= MAX_PENDING_UPLOADS_PER_USER {
            return Err(ApiError::quota(format!(
                "还有 {MAX_PENDING_UPLOADS_PER_USER} 次上传没有提交。请先完成其中一次，或一小时后重新准备。"
            )));
        }
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
    validate_upload_shape(&request)?;
    let requested_kind = request.kind;
    let existing_kind = site.work_kind.parse::<WorkKind>().unwrap_or_default();
    if site.current_version.is_some() && existing_kind != requested_kind {
        return Err(ApiError::invalid(format!(
            "这个作品已经是{}，不能在同一个链接下改成{}。请加 --to new 新建一件作品，再用合集把它们关联起来。",
            kind_label(existing_kind),
            kind_label(requested_kind)
        )));
    }
    let mut wanted = dedup_by_hash(&request.files);
    if let Some(cover) = &request.cover {
        wanted.push(cover_entry(cover));
    }
    let present = present_hashes(&state, &wanted).await?;
    let absent: Vec<&str> = wanted
        .iter()
        .filter(|file| !present.contains(&file.hash))
        .map(|file| file.path.as_str())
        .collect();
    if !absent.is_empty() {
        return Err(ApiError::blobs_missing(missing_message(&absent)));
    }

    // 客户端的检查只负责早一点报错；真正写清单前，控制面必须重新检查它收到的字节。
    let article = match requested_kind {
        WorkKind::Article => Some(render_article(&state, &site.slug, &request).await?),
        WorkKind::Video => {
            let entry = presentation_entry(&request)?;
            validate_video(&state, entry).await?;
            None
        }
        WorkKind::Web => None,
    };

    let created_at = clock::now_string();
    let version = {
        let conn = state.db().lock().await;
        db::next_version(&conn, &site.slug)?
    };
    let title = clean_title(request.title.as_deref())?.unwrap_or_else(|| site.title.clone());
    let note = clean_note(request.note.as_deref())?;
    // 一句话介绍和封面是作品级的东西：这一版没给就沿用上一版的，别让人每次都重打一遍。
    let summary =
        clean_summary(request.summary.as_deref())?.or_else(|| site.listing.summary.clone());
    let cover = match request.cover {
        Some(cover) => Some(cover),
        None => match (&site.listing.cover_hash, &site.listing.cover_mime) {
            (Some(hash), Some(mime)) => {
                let conn = state.db().lock().await;
                // 上一版的封面 blob 可能已经被回收（比如作品很久没动）；那就当没有封面，不写一条指向空处的引用。
                db::blob_size(&conn, hash)?.map(|size| manifest::Cover {
                    hash: hash.clone(),
                    size,
                    mime: mime.clone(),
                })
            }
            _ => None,
        },
    };

    let mut files = request.files;
    files.sort_by(|a, b| a.path.cmp(&b.path));
    let manifest = Manifest {
        schema: manifest::SCHEMA,
        slug: site.slug.clone(),
        version,
        title: title.clone(),
        developer: caller.display_name.clone(),
        note: note.clone(),
        summary,
        cover,
        created_at: created_at.clone(),
        expires_at: site.expires_at.clone(),
        // v0.1 只有匿名和免费档，两档都带角标（DESIGN §3.3）。
        badge: true,
        gate: request.gate,
        isolated: request.isolated,
        spa: request.spa,
        // CLI 上传时认出来的引擎。形状不对就当没说，门禁页那时说「邀请你体验」。
        engine: (requested_kind == WorkKind::Web)
            .then(|| manifest::clean_engine(request.engine.as_deref()))
            .flatten(),
        kind: requested_kind,
        entry: request.entry,
        article,
        files,
    };
    let max_total = if caller.kind.is_anon() {
        limits::ANON_MAX_VERSION_BYTES
    } else {
        limits::MAX_VERSION_BYTES
    };
    manifest::validate_manifest(&manifest, max_total).map_err(as_api_error)?;
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
        // 广场卡片要用的几样抄到 sites 上（DESIGN §3.8）。
        db::record_version_meta(
            &conn,
            &site.slug,
            manifest.summary.as_deref(),
            manifest
                .cover
                .as_ref()
                .map(|c| (c.hash.as_str(), c.mime.as_str())),
            manifest.engine.as_deref(),
            manifest.kind,
            &created_at,
        )?;
    }
    // 关注者要收到「出新版本了」（DESIGN §3.7）。只入队，真发在 notify 的队列里，
    // 发信慢或者失败都不该让这次上传变成失败。
    {
        let conn = state.db().lock().await;
        let door_url = playtest_common::door_url(&state.site_url(&site.slug), &site.slug);
        match crate::notify::enqueue_site_version(
            &conn,
            &site.slug,
            &title,
            version,
            note.as_deref(),
            &door_url,
            clock::now(),
        ) {
            Ok(queued) if queued > 0 => {
                tracing::info!(slug = %site.slug, version, queued, "给关注者排了通知")
            }
            Ok(_) => {}
            Err(e) => tracing::warn!(error = %e, "排版本通知失败，这一版的关注者收不到信"),
        }
    }
    // 公开着的作品，新版本要立刻出现在广场上；没公开的这一步只是重写一份一样的文件。
    if site.listing.public {
        crate::plaza::publish(&state).await;
    }
    crate::live::publish(&state, &site.slug).await;

    tracing::info!(slug = %site.slug, version, file_count, total_bytes, "提交了一个版本");
    Ok(Json(CommitUploadResponse {
        url: state.site_url(&site.slug),
        slug: site.slug,
        version,
        expires_at: site.expires_at,
    }))
}

fn kind_label(kind: WorkKind) -> &'static str {
    match kind {
        WorkKind::Web => "网页作品",
        WorkKind::Article => "文章作品",
        WorkKind::Video => "视频作品",
    }
}

/// 不读字节也能确定的输入形态。prepare 先挡一次，commit 从数据库里的原请求再挡一次。
fn validate_upload_shape(request: &PrepareUploadRequest) -> ApiResult<()> {
    match request.kind {
        WorkKind::Web => {
            if request.entry.is_some() {
                return Err(ApiError::invalid("网页目录不需要指定单文件入口。"));
            }
        }
        WorkKind::Article => {
            let entry = presentation_entry(request)?;
            if !entry.path.to_ascii_lowercase().ends_with(".md") {
                return Err(ApiError::invalid("文章入口必须是 .md 文件。"));
            }
            if request.isolated || request.spa || request.engine.is_some() {
                return Err(ApiError::invalid(
                    "文章不使用 --isolated、--spa 或网页引擎设置。",
                ));
            }
        }
        WorkKind::Video => {
            let entry = presentation_entry(request)?;
            if !entry.path.to_ascii_lowercase().ends_with(".mp4") {
                return Err(ApiError::invalid("视频入口必须是 .mp4 文件。"));
            }
            if request.files.len() != 1 {
                return Err(ApiError::invalid(
                    "首批视频作品只接受一个 MP4 文件；封面请用 --cover 单独传。",
                ));
            }
            if request.isolated || request.spa || request.engine.is_some() {
                return Err(ApiError::invalid(
                    "视频不使用 --isolated、--spa 或网页引擎设置。",
                ));
            }
        }
    }
    Ok(())
}

fn presentation_entry(request: &PrepareUploadRequest) -> ApiResult<&FileEntry> {
    let path = request.entry.as_deref().ok_or_else(|| {
        ApiError::invalid(match request.kind {
            WorkKind::Article => "文章缺少 Markdown 原稿入口。",
            WorkKind::Video => "视频缺少 MP4 文件入口。",
            WorkKind::Web => "网页目录没有单文件入口。",
        })
    })?;
    request
        .files
        .iter()
        .find(|file| file.path == path)
        .ok_or_else(|| ApiError::invalid("作品入口不在这次上传的文件清单里。"))
}

async fn render_article(
    state: &AppState,
    slug: &str,
    request: &PrepareUploadRequest,
) -> ApiResult<ArticleArtifact> {
    let source = presentation_entry(request)?;
    let bytes = state
        .store()
        .get_blob_bytes(&source.hash, limits::MAX_ARTICLE_SOURCE_BYTES)
        .await?
        .ok_or_else(|| ApiError::blobs_missing("文章原稿没有传完整，请重传一次。"))?;
    let markdown = std::str::from_utf8(&bytes)
        .map_err(|_| ApiError::invalid("Markdown 原稿不是 UTF-8 文本，请另存为 UTF-8 后重试。"))?;
    let inspected = playtest_common::article::inspect(markdown, &source.path)
        .map_err(|error| ApiError::invalid(error.to_string()))?;
    let expected: HashSet<&str> = std::iter::once(source.path.as_str())
        .chain(inspected.images.iter().map(String::as_str))
        .collect();
    let supplied: HashSet<&str> = request
        .files
        .iter()
        .map(|file| file.path.as_str())
        .collect();
    if expected != supplied {
        let extra = supplied.difference(&expected).next().copied();
        let missing = expected.difference(&supplied).next().copied();
        let message = match (missing, extra) {
            (Some(path), _) => format!("文章引用了 {path}，但这张图片没有随原稿上传。"),
            (_, Some(path)) => format!(
                "{path} 没有被文章引用。文章发布只收原稿和明确引用的本地图片，不会顺手上传整个目录。"
            ),
            _ => "文章文件清单与正文引用对不上。".to_string(),
        };
        return Err(ApiError::invalid(message));
    }
    for path in &inspected.images {
        let image = request
            .files
            .iter()
            .find(|file| &file.path == path)
            .expect("集合已经确认图片存在");
        validate_article_image(state, image).await?;
    }

    let rendered = playtest_common::article::render(markdown, &source.path, &state.site_url(slug))
        .map_err(|error| ApiError::invalid(error.to_string()))?;
    if rendered.is_empty() || rendered.len() as u64 > limits::MAX_ARTICLE_HTML_BYTES {
        return Err(ApiError::invalid(format!(
            "文章渲染后太大了，上限是 {} MiB。",
            limits::MAX_ARTICLE_HTML_BYTES / limits::MIB
        )));
    }
    let hash = playtest_common::hash::hash_bytes(rendered.as_bytes());
    state.store().put_blob(&hash, rendered.as_bytes()).await?;
    {
        let conn = state.db().lock().await;
        db::record_blob(&conn, &hash, rendered.len() as u64, &clock::now_string())?;
    }
    Ok(ArticleArtifact {
        hash,
        size: rendered.len() as u64,
    })
}

async fn validate_article_image(state: &AppState, entry: &FileEntry) -> ApiResult<()> {
    let end = entry.size.min(16);
    let bytes = state
        .store()
        .get_blob(&entry.hash, Some(0..end))
        .await?
        .ok_or_else(|| ApiError::blobs_missing(format!("配图 {} 没有传完整。", entry.path)))?
        .bytes()
        .await
        .map_err(|error| ApiError::invalid(format!("配图 {} 读不到：{error}", entry.path)))?;
    let actual = manifest::sniff_image_mime(&bytes);
    let expected = playtest_common::article::image_mime(&entry.path);
    if actual.is_none() || actual != expected {
        return Err(ApiError::invalid(format!(
            "{} 的扩展名和实际图片格式对不上（实际是 {}），不能只改扩展名。",
            entry.path,
            actual.unwrap_or("无法识别的格式")
        )));
    }
    Ok(())
}

async fn validate_video(state: &AppState, entry: &FileEntry) -> ApiResult<()> {
    let tmp = state.store().new_tmp_path().await?;
    let result = async {
        let object = state
            .store()
            .get_blob(&entry.hash, None)
            .await?
            .ok_or_else(|| ApiError::blobs_missing("视频没有传完整，请重传一次。"))?;
        let mut file = tokio::fs::File::create(&tmp).await?;
        let mut stream = object.into_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|error| {
                ApiError::invalid(format!("重新读取视频做编码检查时中断了：{error}"))
            })?;
            file.write_all(&chunk).await?;
        }
        file.sync_all().await?;
        let path = tmp.clone();
        tokio::task::spawn_blocking(move || {
            let mut file = std::fs::File::open(path)
                .map_err(|error| ApiError::invalid(format!("视频检查失败：{error}")))?;
            playtest_common::video::inspect(&mut file)
                .map_err(|error| ApiError::invalid(error.to_string()))?;
            Ok::<(), ApiError>(())
        })
        .await
        .map_err(|error| ApiError::invalid(format!("视频检查任务没有完成：{error}")))??;
        Ok::<(), ApiError>(())
    }
    .await;
    if let Err(error) = tokio::fs::remove_file(&tmp).await {
        if error.kind() != std::io::ErrorKind::NotFound {
            tracing::warn!(path = %tmp.display(), %error, "视频检查临时文件没删掉");
        }
    }
    result
}

/// 封面在「缺哪些 blob」这一步里的样子：它不在目录里，给它一个只在报错信息里出现的名字。
fn cover_entry(cover: &manifest::Cover) -> FileEntry {
    FileEntry {
        path: "（封面）".to_string(),
        hash: cover.hash.clone(),
        size: cover.size,
    }
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
        let conn = state.db().read().await;
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
        format!(
            "还有 {} 个文件没传上来：{shown}。重新运行一次上传，没传成功的会补上。",
            absent.len()
        )
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

pub fn clean_summary(summary: Option<&str>) -> ApiResult<Option<String>> {
    clean_text(summary, limits::MAX_SUMMARY_CHARS, "一句话介绍")
}

pub fn clean_text(value: Option<&str>, max_chars: usize, what: &str) -> ApiResult<Option<String>> {
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
        assert_eq!(
            clean_title(Some(" 小球 ")).unwrap().as_deref(),
            Some("小球")
        );
        assert!(clean_title(Some(&"字".repeat(limits::MAX_TITLE_CHARS + 1))).is_err());
    }
}
