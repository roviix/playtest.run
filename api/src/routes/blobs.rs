//! 收文件内容。上传路径里唯一一个搬字节的端点。
//!
//! 请求体从头到尾不进内存：边收边写临时文件、边算 SHA-256，算完对得上才以分段上传
//! 原子提交进对象存储。一个 200 MB 的 Unity `.data` 文件内存有明确上限。

use std::path::Path as FsPath;

use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use futures_util::StreamExt;
use playtest_common::api::PrepareUploadRequest;
use playtest_common::hash::{self, Hasher};
use playtest_common::limits;
use tokio::io::AsyncWriteExt;

use crate::auth::Caller;
use crate::clock;
use crate::db;
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

pub async fn upload(
    State(state): State<AppState>,
    caller: Caller,
    Path(hash): Path<String>,
    body: Body,
) -> ApiResult<StatusCode> {
    // 先挡住形态不对的哈希：它后面要被拼进对象存储的路径。
    if !hash::is_valid_hex(&hash) {
        return Err(ApiError::invalid(
            "That path segment is not a content hash: it needs 64 lowercase hex characters.",
        ));
    }

    let expected_sizes = pending_sizes_for(&state, &caller.user_id, &hash).await?;
    if expected_sizes.is_empty() {
        return Err(ApiError::invalid(
            "This file is not in any of your uncommitted uploads. Run the upload again.",
        ));
    }

    let tmp = state.store().new_tmp_path().await?;
    let (actual, size) = match receive(body, &tmp).await {
        Ok(received) => received,
        Err(err) => {
            discard(&tmp).await;
            return Err(err);
        }
    };

    if actual != hash {
        discard(&tmp).await;
        return Err(ApiError::hash_mismatch(format!(
            "These bytes hash to {actual}, not the {hash} in the URL. The file changed on the way or went to the wrong place. Upload it again."
        )));
    }

    if !expected_sizes.contains(&size) {
        discard(&tmp).await;
        return Err(ApiError::invalid(format!(
            "This file is {size} bytes, which is not the size declared when the upload was prepared. Run the upload again."
        )));
    }

    // 同一份内容可能被两个作品同时传上来，谁先到都一样，后到的把自己那份临时文件丢掉。
    let existed = state.store().has_blob(&hash).await?;
    if existed {
        discard(&tmp).await;
    } else if let Err(err) = state.store().put_blob_from_path(&hash, &tmp).await {
        discard(&tmp).await;
        return Err(err.into());
    } else {
        // S3 的最终提交不会像旧文件系统实现那样把这份本机临时文件 rename 掉。
        discard(&tmp).await;
    }

    {
        let conn = state.db().lock().await;
        db::record_blob(&conn, &hash, size, &clock::now_string())?;
    }

    tracing::info!(%hash, size, existed, user_id = %caller.user_id, "收到一个文件");
    Ok(if existed {
        StatusCode::OK
    } else {
        StatusCode::CREATED
    })
}

/// 返回调用者所有仍待提交的清单里，这个哈希被声明过的大小。
async fn pending_sizes_for(state: &AppState, user_id: &str, hash: &str) -> ApiResult<Vec<u64>> {
    let requests = {
        let conn = state.db().lock().await;
        let stale_before = clock::format(
            clock::now() - time::Duration::hours(super::uploads::PENDING_UPLOAD_HOURS),
        );
        db::delete_pending_uploads_before(&conn, &stale_before)?;
        db::pending_upload_requests(&conn, user_id)?
    };
    let mut sizes = Vec::new();
    for request_json in requests {
        let request: PrepareUploadRequest = serde_json::from_str(&request_json)?;
        sizes.extend(
            request
                .files
                .iter()
                .filter(|file| file.hash == hash)
                .map(|file| file.size),
        );
        if let Some(cover) = request.cover.filter(|cover| cover.hash == hash) {
            sizes.push(cover.size);
        }
    }
    sizes.sort_unstable();
    sizes.dedup();
    Ok(sizes)
}

/// 收完整个请求体，返回实际的内容哈希与字节数。
async fn receive(body: Body, tmp: &FsPath) -> ApiResult<(String, u64)> {
    let mut file = tokio::fs::File::create(tmp).await?;
    let mut stream = body.into_data_stream();
    let mut hasher = Hasher::new();
    let mut received: u64 = 0;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|err| interrupted(received, err))?;
        received = received.saturating_add(chunk.len() as u64);
        hasher.update(&chunk);
        file.write_all(&chunk).await?;
    }
    file.sync_all().await?;
    Ok((hasher.finish(), received))
}

/// 收到一半断了。断在哪一层看不出来，用已收字节数判断：收满上限就是被体积上限掐的，
/// 其余当成网络断了——两种说法对应的下一步不一样，不能混成一句。
fn interrupted(received: u64, err: axum::Error) -> ApiError {
    if received >= limits::MAX_FILE_BYTES {
        ApiError::too_large(super::too_large_message())
    } else {
        tracing::debug!(error = %err, received, "上传中途断了");
        ApiError::invalid("The connection dropped before this file finished. Upload it again.")
    }
}

/// 临时文件删不掉不该让整个请求失败：内容已经安全了，剩下的是磁盘上一个孤儿文件。
async fn discard(tmp: &FsPath) {
    if let Err(err) = tokio::fs::remove_file(tmp).await {
        if err.kind() != std::io::ErrorKind::NotFound {
            tracing::warn!(path = %tmp.display(), error = %err, "临时文件没删掉");
        }
    }
}
