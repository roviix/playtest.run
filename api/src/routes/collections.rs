use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use playtest_common::collection::{
    Collection, CollectionDraft, CollectionKind, EntryDraft, ModerateCollectionRequest,
    MAX_COLLECTIONS, MAX_ENTRIES,
};
use rusqlite::{params, Connection, OptionalExtension};

use crate::auth::Caller;
use crate::error::{ApiError, ApiResult};
use crate::routes::{admin::Admin, JsonBody};
use crate::state::AppState;
use crate::{clock, collections, db, plaza, words};

const MISSING: &str = "No such collection, or it is not yours to manage.";

fn require_account(caller: &Caller) -> ApiResult<()> {
    if caller.kind.is_anon() {
        return Err(ApiError::quota(
            "Sign in and keep a project before creating a collection or submitting to one.",
        ));
    }
    Ok(())
}

fn owned(conn: &Connection, slug: &str, caller: &Caller) -> ApiResult<Collection> {
    collections::list(conn, Some(&caller.user_id), &clock::now_string())?
        .into_iter()
        .find(|collection| collection.slug == slug)
        .ok_or_else(|| ApiError::not_found(MISSING))
}

fn text(value: &str, maximum: usize, label: &str) -> ApiResult<String> {
    let value = value.trim();
    if value.chars().count() > maximum
        || value
            .chars()
            .any(|character| character.is_control() && character != '\n' && character != '\t')
    {
        return Err(ApiError::invalid(format!(
            "{label} is capped at {maximum} characters and cannot contain invisible control characters."
        )));
    }
    Ok(value.to_string())
}

fn clean(mut draft: CollectionDraft) -> ApiResult<CollectionDraft> {
    draft.title = text(&draft.title, 80, "The collection title")?;
    if draft.title.is_empty() {
        return Err(ApiError::invalid("Give this collection a name."));
    }
    draft.summary = text(&draft.summary, 280, "The summary")?;
    draft.prompt = text(&draft.prompt, 6000, "The prompt")?;
    draft.rules = text(&draft.rules, 3000, "The rules")?;
    if let Some(close) = draft.closes_at.as_deref().filter(|value| !value.is_empty()) {
        let parsed = clock::parse(close).ok_or_else(|| {
            ApiError::invalid("The closing time needs a full date with a time zone.")
        })?;
        draft.closes_at = Some(clock::format(
            parsed
                .to_offset(time::UtcOffset::UTC)
                .replace_nanosecond(0)
                .unwrap(),
        ));
    } else {
        draft.closes_at = None;
    }
    if draft.kind == CollectionKind::Collection {
        draft.closes_at = None;
    }
    if draft.kind == CollectionKind::Challenge && draft.public && draft.prompt.is_empty() {
        return Err(ApiError::invalid(
            "Say what people are supposed to make before you make this challenge public.",
        ));
    }
    Ok(draft)
}

pub async fn list(
    State(state): State<AppState>,
    caller: Caller,
) -> ApiResult<Json<Vec<Collection>>> {
    let conn = state.db().read().await;
    Ok(Json(collections::list(
        &conn,
        Some(&caller.user_id),
        &clock::now_string(),
    )?))
}

pub async fn open(
    State(state): State<AppState>,
    _caller: Caller,
) -> ApiResult<Json<Vec<Collection>>> {
    let conn = state.db().read().await;
    Ok(Json(collections::list(&conn, None, &clock::now_string())?))
}

pub async fn show(
    State(state): State<AppState>,
    caller: Caller,
    Path(slug): Path<String>,
) -> ApiResult<Json<Collection>> {
    let conn = state.db().read().await;
    if collections::owner(&conn, &slug)?.as_deref() == Some(&caller.user_id) {
        return Ok(Json(owned(&conn, &slug, &caller)?));
    }
    Ok(Json(
        collections::public_one(&conn, &slug, &clock::now_string())?
            .ok_or_else(|| ApiError::not_found(MISSING))?,
    ))
}

pub async fn create(
    State(state): State<AppState>,
    caller: Caller,
    JsonBody(draft): JsonBody<CollectionDraft>,
) -> ApiResult<Json<Collection>> {
    require_account(&caller)?;
    let draft = clean(draft)?;
    let slug = draft
        .slug
        .clone()
        .filter(|slug| !slug.is_empty())
        .unwrap_or_else(|| {
            format!(
                "collection-{}",
                &uuid::Uuid::new_v4().simple().to_string()[..12]
            )
        });
    playtest_common::slug::validate(&slug)
        .map_err(|_| {
            ApiError::invalid(
                "A collection address uses lowercase letters, digits and hyphens, 3–63 characters long.",
            )
        })?;
    let value = {
        let conn = state.db().lock().await;
        if collections::list(&conn, Some(&caller.user_id), &clock::now_string())?.len()
            >= MAX_COLLECTIONS
        {
            return Err(ApiError::quota(format!(
                "You can run {} at a time. Tidy up the ones you have first.",
                words::count(MAX_COLLECTIONS as u64, "collection")
            )));
        }
        if conn
            .query_row("SELECT 1 FROM collections WHERE slug=?1", [&slug], |_| {
                Ok(())
            })
            .optional()?
            .is_some()
        {
            return Err(ApiError::invalid(
                "That collection address is taken. Try another one.",
            ));
        }
        let now = clock::now_string();
        conn.execute("INSERT INTO collections(slug,user_id,title,summary,kind,prompt,rules,closes_at,public,created_at,updated_at)
            VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?10)",
            params![slug,caller.user_id,draft.title,draft.summary,if draft.kind == CollectionKind::Challenge { "challenge" } else { "collection" },draft.prompt,draft.rules,draft.closes_at,draft.public,now])?;
        owned(&conn, &slug, &caller)?
    };
    plaza::publish(&state).await;
    Ok(Json(value))
}

pub async fn update(
    State(state): State<AppState>,
    caller: Caller,
    Path(slug): Path<String>,
    JsonBody(draft): JsonBody<CollectionDraft>,
) -> ApiResult<Json<Collection>> {
    let draft = clean(draft)?;
    let value = {
        let conn = state.db().lock().await;
        let old = owned(&conn, &slug, &caller)?;
        if old.kind != draft.kind {
            return Err(ApiError::invalid(
                "The kind of a collection is fixed once it exists, so the rules cannot change under the entries already in it.",
            ));
        }
        let has_entries: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM collection_entries WHERE collection_slug=?1)",
            [&slug],
            |row| row.get(0),
        )?;
        if has_entries
            && (old.prompt != draft.prompt
                || old.rules != draft.rules
                || old.closes_at != draft.closes_at)
        {
            return Err(ApiError::invalid(
                "There are entries already, so the prompt, the rules and the closing time are fixed. Create a new challenge for a new prompt.",
            ));
        }
        conn.execute("UPDATE collections SET title=?2,summary=?3,prompt=?4,rules=?5,closes_at=?6,public=?7,updated_at=?8 WHERE slug=?1",
            params![slug,draft.title,draft.summary,draft.prompt,draft.rules,draft.closes_at,draft.public,clock::now_string()])?;
        owned(&conn, &slug, &caller)?
    };
    plaza::publish(&state).await;
    Ok(Json(value))
}

pub async fn remove(
    State(state): State<AppState>,
    caller: Caller,
    Path(slug): Path<String>,
) -> ApiResult<StatusCode> {
    {
        let conn = state.db().lock().await;
        owned(&conn, &slug, &caller)?;
        conn.execute(
            "UPDATE collections SET deleted_at=?2, public=0 WHERE slug=?1",
            params![slug, clock::now_string()],
        )?;
    }
    plaza::publish(&state).await;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn submit(
    State(state): State<AppState>,
    caller: Caller,
    Path(slug): Path<String>,
    JsonBody(draft): JsonBody<EntryDraft>,
) -> ApiResult<Json<Collection>> {
    require_account(&caller)?;
    let note = text(&draft.note, 500, "The submission note")?;
    let now = clock::now_string();
    {
        let conn = state.db().lock().await;
        let is_owner = collections::owner(&conn, &slug)?.as_deref() == Some(&caller.user_id);
        let collection = if is_owner {
            owned(&conn, &slug, &caller)?
        } else {
            collections::public_one(&conn, &slug, &now)?
                .ok_or_else(|| ApiError::not_found(MISSING))?
        };
        if collection.hidden || (!is_owner && collection.kind != CollectionKind::Challenge) {
            return Err(ApiError::not_found(MISSING));
        }
        if collection.closed(&now) {
            return Err(ApiError::invalid(
                "This challenge has closed. Entries cannot be added or replaced, but they can still be withdrawn.",
            ));
        }
        let site = db::find_live_site(&conn, &draft.slug, &caller.user_id)?
            .ok_or_else(|| ApiError::not_found("You can only submit your own project."))?;
        if !site.listing.public
            || site.listing.hidden_at.is_some()
            || site.expires_at.is_some()
            || site.current_version.is_none()
        {
            return Err(ApiError::invalid(
                "Pick a lasting project that is published, public by your own choice, and not taken down. Submitting will not make a project public for you.",
            ));
        }
        let manifest = state
            .store()
            .get_manifest(&site.slug, site.current_version.unwrap())
            .await
            .map_err(|error| ApiError::Internal(error.into()))?
            .ok_or_else(|| {
                ApiError::invalid("We cannot read this project right now. Try again shortly.")
            })?;
        if manifest.files.is_empty() {
            return Err(ApiError::invalid(
                "A temporary tunnel cannot stand as an entry. Upload a directory version that lasts, then try again.",
            ));
        }
        let existing: Option<bool> = conn
            .query_row(
                "SELECT blocked FROM collection_entries WHERE collection_slug=?1 AND site_slug=?2",
                params![slug, draft.slug],
                |row| row.get(0),
            )
            .optional()?;
        if existing == Some(true) {
            return Err(ApiError::invalid(
                "The organiser removed this project. They have to allow it back before it can be submitted again.",
            ));
        }
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM collection_entries WHERE collection_slug=?1",
            [&slug],
            |row| row.get(0),
        )?;
        if existing.is_none() && count >= MAX_ENTRIES as i64 {
            return Err(ApiError::quota(format!(
                "This collection is full at {}. Ask the organiser to tidy it up.",
                words::count(MAX_ENTRIES as u64, "project")
            )));
        }
        conn.execute("INSERT INTO collection_entries(collection_slug,site_slug,user_id,submitted_version,submitted_at,note)
            VALUES(?1,?2,?3,?4,?5,?6) ON CONFLICT(collection_slug,site_slug) DO UPDATE SET
            submitted_version=excluded.submitted_version,note=excluded.note",
            params![slug,draft.slug,caller.user_id,site.current_version,now,note])?;
    }
    plaza::publish(&state).await;
    show(State(state), caller, Path(slug)).await
}

pub async fn withdraw(
    State(state): State<AppState>,
    caller: Caller,
    Path((slug, site)): Path<(String, String)>,
) -> ApiResult<StatusCode> {
    {
        let conn = state.db().lock().await;
        let entry: Option<(String,bool)> = conn.query_row("SELECT user_id,blocked FROM collection_entries WHERE collection_slug=?1 AND site_slug=?2", params![slug,site], |row| Ok((row.get(0)?,row.get(1)?))).optional()?;
        let (author, blocked) = entry.ok_or_else(|| ApiError::not_found(MISSING))?;
        if author == caller.user_id {
            if blocked {
                return Err(ApiError::invalid(
                    "The organiser already removed this project, so there is nothing to withdraw.",
                ));
            }
            conn.execute(
                "DELETE FROM collection_entries WHERE collection_slug=?1 AND site_slug=?2",
                params![slug, site],
            )?;
        } else {
            owned(&conn, &slug, &caller)?;
            conn.execute("UPDATE collection_entries SET blocked=1,note='' WHERE collection_slug=?1 AND site_slug=?2", params![slug,site])?;
        }
    }
    plaza::publish(&state).await;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn unblock(
    State(state): State<AppState>,
    caller: Caller,
    Path((slug, site)): Path<(String, String)>,
) -> ApiResult<StatusCode> {
    let conn = state.db().lock().await;
    owned(&conn, &slug, &caller)?;
    conn.execute(
        "DELETE FROM collection_entries WHERE collection_slug=?1 AND site_slug=?2 AND blocked=1",
        params![slug, site],
    )?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn moderate(
    State(state): State<AppState>,
    _admin: Admin,
    Path(slug): Path<String>,
    JsonBody(request): JsonBody<ModerateCollectionRequest>,
) -> ApiResult<StatusCode> {
    {
        let conn = state.db().lock().await;
        if conn.execute(
            "UPDATE collections SET hidden=?2 WHERE slug=?1 AND deleted_at IS NULL",
            params![slug, request.hidden],
        )? == 0
        {
            return Err(ApiError::not_found(MISSING));
        }
    }
    plaza::publish(&state).await;
    Ok(StatusCode::NO_CONTENT)
}
