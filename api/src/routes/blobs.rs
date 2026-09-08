//! 收文件内容。上传路径里唯一一个搬字节的端点。
//!
//! 请求体从头到尾不进内存：边收边写临时文件、边算 SHA-256，算完对得上才原子改名进
//! 对象存储。一个 200 MB 的 Unity `.data` 文件在这里只占几十 KB 的缓冲区。

use std::path::Path as FsPath;

use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use futures_util::StreamExt;
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
            "地址里的这一段不是内容哈希：要 64 个小写十六进制字符。",
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
            "这些字节算出来的内容哈希是 {actual}，和地址里的 {hash} 对不上。文件在路上变了或者传错了地方，重传一次。"
        )));
    }

    // 同一份内容可能被两个作品同时传上来，谁先到都一样，后到的把自己那份临时文件丢掉。
    let existed = state.store().has_blob(&hash).await?;
    if existed {
        discard(&tmp).await;
    } else if let Err(err) = state.store().put_blob_from_path(&hash, &tmp).await {
        discard(&tmp).await;
        return Err(err.into());
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
        ApiError::invalid("这个文件没传完，连接就断了。重传一次。")
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
