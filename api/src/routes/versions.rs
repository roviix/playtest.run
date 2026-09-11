//! 版本列表与回滚（DESIGN §3.5）。
//!
//! 每次上传都是一个不可变的 `vN`，玩家看到的永远是「当前版本」指针指着的那一版。
//! 回滚就是把指针指回去：清单都在对象存储里，不用重传一个字节，边缘 1 秒内看到新指针。
//! 版本号只增不减——回滚到 v1 之后再上传是 v4 不是 v2，否则会话与反馈按版本归档就乱了。

use axum::extract::{Path, State};
use axum::Json;
use playtest_common::api::{Site, VersionFile, VersionFiles, VersionInfo, VersionList};
use playtest_common::store::Current;

use crate::auth::Caller;
use crate::clock;
use crate::db;
use crate::error::{ApiError, ApiResult};
use crate::routes::sites::{load_site, NO_SUCH_SITE};
use crate::state::AppState;

pub async fn list(
    State(state): State<AppState>,
    caller: Caller,
    Path(slug): Path<String>,
) -> ApiResult<Json<VersionList>> {
    let conn = state.db().lock().await;
    let site = db::find_live_site(&conn, &slug, &caller.user_id)?
        .ok_or_else(|| ApiError::not_found(NO_SUCH_SITE))?;
    let versions = db::list_versions(&conn, &site.slug)?
        .into_iter()
        .map(|v| VersionInfo {
            current: site.current_version == Some(v.version),
            version: v.version,
            created_at: v.created_at,
            note: v.note,
            file_count: v.file_count,
            total_bytes: v.total_bytes,
        })
        .collect();
    Ok(Json(VersionList {
        slug: site.slug,
        current_version: site.current_version,
        versions,
    }))
}

/// 一个版本里到底有哪些文件（REWRITE §1.3「读回」）。
///
/// 为什么值得有：发出去之后，「线上现在到底是哪一份」只有我们知道。开发者换了台电脑、
/// 或者一个 agent 接手了别人发的作品时，唯一的办法是重新构建再对比——而清单本来就在
/// 我们这儿，按内容哈希存着，说出来是不花钱的。
///
/// 只给清单和哈希，不给字节：文件本身在作品自己的域上按路径就能取，走的是玩家那条路。
pub async fn files(
    State(state): State<AppState>,
    caller: Caller,
    Path((slug, version)): Path<(String, u32)>,
) -> ApiResult<Json<VersionFiles>> {
    let site = {
        let conn = state.db().lock().await;
        db::find_live_site(&conn, &slug, &caller.user_id)?
            .ok_or_else(|| ApiError::not_found(NO_SUCH_SITE))?
    };
    let manifest = state
        .store()
        .get_manifest(&site.slug, version)
        .await?
        .ok_or_else(|| {
            ApiError::not_found(format!(
                "这个作品没有 v{version}。用 playtest versions {} 看看发过哪几版。",
                site.slug
            ))
        })?;
    let url = state.site_url(&site.slug);
    Ok(Json(VersionFiles {
        slug: site.slug,
        version,
        current: site.current_version == Some(version),
        total_bytes: manifest.files.iter().map(|f| f.size).sum(),
        files: manifest
            .files
            .iter()
            .map(|f| VersionFile {
                url: format!("{}/{}", url.trim_end_matches('/'), f.path),
                path: f.path.clone(),
                size: f.size,
                hash: f.hash.clone(),
            })
            .collect(),
    }))
}

pub async fn activate(
    State(state): State<AppState>,
    caller: Caller,
    Path((slug, version)): Path<(String, u32)>,
) -> ApiResult<Json<Site>> {
    let site = {
        let conn = state.db().lock().await;
        let site = db::find_live_site(&conn, &slug, &caller.user_id)?
            .ok_or_else(|| ApiError::not_found(NO_SUCH_SITE))?;
        if !db::version_exists(&conn, &site.slug, version)? {
            return Err(ApiError::not_found(format!(
                "这个作品没有 v{version}。用 playtest versions {} 看看发过哪几版。",
                site.slug
            )));
        }
        site
    };

    // 和提交一样先写对象存储再回写库：边缘只看对象存储，库落后一步无伤，反过来会出现库里指着、边缘没有的版本。
    state
        .store()
        .set_current(
            &site.slug,
            &Current {
                version,
                updated_at: clock::now_string(),
            },
        )
        .await?;
    let row = {
        let conn = state.db().lock().await;
        db::set_current_version(&conn, &site.slug, version)?;
        db::find_live_site(&conn, &site.slug, &caller.user_id)?
            .ok_or_else(|| ApiError::not_found(NO_SUCH_SITE))?
    };
    tracing::info!(slug = %site.slug, version, "把当前版本指回了 v{version}");
    Ok(Json(load_site(&state, row).await?))
}
