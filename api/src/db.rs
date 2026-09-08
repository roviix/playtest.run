//! SQLite：账号、令牌、作品、版本、上传、blob 索引。
//!
//! 一个连接配一把 [`tokio::sync::Mutex`]。v0.1 是单机、单进程、私测 20 个开发者的规模
//! （DESIGN §4.5），查询都是主键或单列索引上的一两行，串行化的代价看不见；
//! 换 Postgres 时换掉的是这一层，不是这些 SQL。

use std::path::Path;

use anyhow::Context;
use rusqlite::{params, Connection, OptionalExtension, Row};
use tokio::sync::{Mutex, MutexGuard};

/// 迁移按顺序编号，下标 + 1 就是版本号。只增不改：已经跑过的那条永远不动。
const MIGRATIONS: &[&str] = &[
    include_str!("migrations/001_init.sql"),
    include_str!("migrations/002_sessions_events_feedback.sql"),
    include_str!("migrations/003_plaza.sql"),
];

pub struct Db {
    conn: Mutex<Connection>,
}

impl Db {
    /// 打开（不存在就新建）数据库并把迁移跑完。
    pub fn open(path: &Path) -> anyhow::Result<Self> {
        let mut conn = Connection::open(path)
            .with_context(|| format!("打不开数据库文件 {}", path.display()))?;
        // WAL：读不挡写。busy_timeout 给同一台机器上另一个进程（比如手工用 sqlite3 看一眼）留出让路的时间。
        conn.pragma_update(None, "journal_mode", "WAL")
            .context("开 WAL 模式失败")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
        migrate(&mut conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    #[cfg(test)]
    pub fn open_in_memory() -> anyhow::Result<Self> {
        let mut conn = Connection::open_in_memory()?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        migrate(&mut conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub async fn lock(&self) -> MutexGuard<'_, Connection> {
        self.conn.lock().await
    }
}

fn migrate(conn: &mut Connection) -> anyhow::Result<()> {
    conn.execute_batch("CREATE TABLE IF NOT EXISTS schema_version (version INTEGER PRIMARY KEY)")
        .context("建 schema_version 表失败")?;
    let applied: i64 = conn.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM schema_version",
        [],
        |row| row.get(0),
    )?;

    for (index, sql) in MIGRATIONS.iter().enumerate() {
        let version = index as i64 + 1;
        if version <= applied {
            continue;
        }
        let tx = conn.transaction()?;
        tx.execute_batch(sql)
            .with_context(|| format!("第 {version} 号迁移执行失败"))?;
        tx.execute(
            "INSERT INTO schema_version (version) VALUES (?1)",
            params![version],
        )?;
        tx.commit()?;
        tracing::info!(version, "数据库迁移已应用");
    }
    Ok(())
}

// ---- 用户与令牌 ----

pub struct NewUser<'a> {
    pub id: &'a str,
    pub kind: &'a str,
    pub display_name: &'a str,
    pub created_at: &'a str,
    pub expires_at: Option<&'a str>,
}

pub fn insert_user(conn: &Connection, user: &NewUser<'_>) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO users (id, kind, display_name, created_at, expires_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![user.id, user.kind, user.display_name, user.created_at, user.expires_at],
    )?;
    Ok(())
}

pub fn insert_token(
    conn: &Connection,
    token_hash: &str,
    user_id: &str,
    created_at: &str,
    expires_at: Option<&str>,
) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO tokens (token_hash, user_id, created_at, expires_at) VALUES (?1, ?2, ?3, ?4)",
        params![token_hash, user_id, created_at, expires_at],
    )?;
    Ok(())
}

/// 令牌对应的人，以及两个到期时间——令牌自己的，和这个人的。
pub struct TokenOwner {
    pub user_id: String,
    pub kind: String,
    pub display_name: String,
    pub token_expires_at: Option<String>,
    pub user_expires_at: Option<String>,
}

pub fn find_token_owner(
    conn: &Connection,
    token_hash: &str,
) -> rusqlite::Result<Option<TokenOwner>> {
    conn.query_row(
        "SELECT u.id, u.kind, u.display_name, t.expires_at, u.expires_at
           FROM tokens t JOIN users u ON u.id = t.user_id
          WHERE t.token_hash = ?1",
        params![token_hash],
        |row| {
            Ok(TokenOwner {
                user_id: row.get(0)?,
                kind: row.get(1)?,
                display_name: row.get(2)?,
                token_expires_at: row.get(3)?,
                user_expires_at: row.get(4)?,
            })
        },
    )
    .optional()
}

// ---- 作品 ----

#[derive(Debug, Clone)]
pub struct SiteRow {
    pub slug: String,
    pub title: String,
    pub created_at: String,
    pub expires_at: Option<String>,
    pub current_version: Option<u32>,
    pub listing: ListingRow,
}

/// 广场相关的那几列（DESIGN §3.8、迁移 003）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ListingRow {
    pub public: bool,
    pub seeking: bool,
    pub seek_note: Option<String>,
    pub summary: Option<String>,
    pub cover_hash: Option<String>,
    pub cover_mime: Option<String>,
    pub engine: Option<String>,
    pub updated_at: Option<String>,
    pub hidden_at: Option<String>,
    pub hidden_reason: Option<String>,
    /// 人工复核恢复的时间；自动撤下只数它之后的举报。
    pub reviewed_at: Option<String>,
}

const SITE_COLUMN_NAMES: &[&str] = &[
    "slug",
    "title",
    "created_at",
    "expires_at",
    "current_version",
    "public",
    "seeking",
    "seek_note",
    "summary",
    "cover_hash",
    "cover_mime",
    "engine",
    "updated_at",
    "hidden_at",
    "hidden_reason",
    "reviewed_at",
];

const SITE_COLUMNS: &str = "slug, title, created_at, expires_at, current_version, \
    public, seeking, seek_note, summary, cover_hash, cover_mime, engine, updated_at, hidden_at, hidden_reason, reviewed_at";

/// 同一组列，带表别名——和别的表 JOIN 时 `created_at` / `expires_at` 会重名。
fn site_columns(prefix: &str) -> String {
    SITE_COLUMN_NAMES
        .iter()
        .map(|c| format!("{prefix}{c}"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn site_from_row(row: &Row<'_>) -> rusqlite::Result<SiteRow> {
    Ok(SiteRow {
        slug: row.get(0)?,
        title: row.get(1)?,
        created_at: row.get(2)?,
        expires_at: row.get(3)?,
        current_version: row.get(4)?,
        listing: ListingRow {
            public: row.get::<_, i64>(5)? != 0,
            seeking: row.get::<_, i64>(6)? != 0,
            seek_note: row.get(7)?,
            summary: row.get(8)?,
            cover_hash: row.get(9)?,
            cover_mime: row.get(10)?,
            engine: row.get(11)?,
            updated_at: row.get(12)?,
            hidden_at: row.get(13)?,
            hidden_reason: row.get(14)?,
            reviewed_at: row.get(15)?,
        },
    })
}

/// 这个 slug 有没有被占。删掉的也算占着：清单和对象已经没了，再发给别人会拿到一个空作品。
pub fn slug_taken(conn: &Connection, slug: &str) -> rusqlite::Result<bool> {
    conn.query_row("SELECT 1 FROM sites WHERE slug = ?1", params![slug], |_| {
        Ok(())
    })
    .optional()
    .map(|found| found.is_some())
}

pub fn count_live_sites(conn: &Connection, user_id: &str) -> rusqlite::Result<u32> {
    conn.query_row(
        "SELECT COUNT(*) FROM sites WHERE user_id = ?1 AND deleted_at IS NULL",
        params![user_id],
        |row| row.get(0),
    )
}

pub fn insert_site(
    conn: &Connection,
    slug: &str,
    user_id: &str,
    title: &str,
    created_at: &str,
    expires_at: Option<&str>,
) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO sites (slug, user_id, title, created_at, expires_at, current_version, deleted_at)
         VALUES (?1, ?2, ?3, ?4, ?5, NULL, NULL)",
        params![slug, user_id, title, created_at, expires_at],
    )?;
    Ok(())
}

pub fn list_live_sites(conn: &Connection, user_id: &str) -> rusqlite::Result<Vec<SiteRow>> {
    let sql = format!(
        "SELECT {SITE_COLUMNS} FROM sites
          WHERE user_id = ?1 AND deleted_at IS NULL
          ORDER BY created_at DESC, slug"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params![user_id], site_from_row)?;
    rows.collect()
}

/// 只认「我的、没删的」。别人的作品和已删的作品一样返回 None，调用方一律回 404——
/// 不告诉外面这个 slug 存不存在。
pub fn find_live_site(
    conn: &Connection,
    slug: &str,
    user_id: &str,
) -> rusqlite::Result<Option<SiteRow>> {
    let sql = format!(
        "SELECT {SITE_COLUMNS} FROM sites
          WHERE slug = ?1 AND user_id = ?2 AND deleted_at IS NULL"
    );
    conn.query_row(&sql, params![slug, user_id], site_from_row)
        .optional()
}

/// 返回是不是真的删掉了一行（本来就删过的返回 false）。
pub fn mark_site_deleted(
    conn: &Connection,
    slug: &str,
    deleted_at: &str,
) -> rusqlite::Result<bool> {
    let changed = conn.execute(
        "UPDATE sites SET deleted_at = ?2 WHERE slug = ?1 AND deleted_at IS NULL",
        params![slug, deleted_at],
    )?;
    Ok(changed > 0)
}

/// 过期匿名用户名下还没删的作品。后台清理任务用。
pub fn expired_site_slugs(conn: &Connection, now: &str) -> rusqlite::Result<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT s.slug FROM sites s JOIN users u ON u.id = s.user_id
          WHERE s.deleted_at IS NULL AND u.expires_at IS NOT NULL AND u.expires_at <= ?1",
    )?;
    let rows = stmt.query_map(params![now], |row| row.get(0))?;
    rows.collect()
}

pub fn delete_expired_tokens(conn: &Connection, now: &str) -> rusqlite::Result<usize> {
    conn.execute(
        "DELETE FROM tokens WHERE expires_at IS NOT NULL AND expires_at <= ?1",
        params![now],
    )
}

// ---- 上传与版本 ----

pub struct UploadRow {
    pub slug: String,
    pub user_id: String,
    pub request_json: String,
    pub status: String,
}

pub fn insert_upload(
    conn: &Connection,
    id: &str,
    slug: &str,
    user_id: &str,
    created_at: &str,
    request_json: &str,
) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO uploads (id, slug, user_id, created_at, request_json, status)
         VALUES (?1, ?2, ?3, ?4, ?5, 'pending')",
        params![id, slug, user_id, created_at, request_json],
    )?;
    Ok(())
}

pub fn find_upload(conn: &Connection, id: &str) -> rusqlite::Result<Option<UploadRow>> {
    conn.query_row(
        "SELECT slug, user_id, request_json, status FROM uploads WHERE id = ?1",
        params![id],
        |row| {
            Ok(UploadRow {
                slug: row.get(0)?,
                user_id: row.get(1)?,
                request_json: row.get(2)?,
                status: row.get(3)?,
            })
        },
    )
    .optional()
}

/// 这些哈希里，索引表认识哪些。
///
/// 一条一条查而不是拼一个大 IN：一个 Unity 导出物有上千个文件，
/// 拼出来的占位符会撞上 SQLite 对单条语句变量个数的上限，而主键查一千次是微秒级的事。
pub fn known_blob_hashes(
    conn: &Connection,
    hashes: &[String],
) -> rusqlite::Result<std::collections::HashSet<String>> {
    let mut stmt = conn.prepare("SELECT 1 FROM blobs WHERE hash = ?1")?;
    let mut known = std::collections::HashSet::with_capacity(hashes.len());
    for hash in hashes {
        if stmt.exists(params![hash])? {
            known.insert(hash.clone());
        }
    }
    Ok(known)
}

/// 同一个哈希可能被并发上传两次，内容一样，后到的覆盖前面那行即可。
pub fn record_blob(
    conn: &Connection,
    hash: &str,
    size: u64,
    created_at: &str,
) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO blobs (hash, size, created_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(hash) DO UPDATE SET size = excluded.size",
        params![hash, size as i64, created_at],
    )?;
    Ok(())
}

/// 索引里记着的大小；不在索引里就是 `None`。
pub fn blob_size(conn: &Connection, hash: &str) -> rusqlite::Result<Option<u64>> {
    conn.query_row(
        "SELECT size FROM blobs WHERE hash = ?1",
        params![hash],
        |row| row.get::<_, i64>(0),
    )
    .optional()
    .map(|size| size.map(|s| s as u64))
}

/// blob 被回收之后把记录也删掉，下次同样的文件再来时 `missing` 里要有它。
pub fn delete_blob(conn: &Connection, hash: &str) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM blobs WHERE hash = ?1", params![hash])?;
    Ok(())
}

/// 「数据默认保留 90 天」（DESIGN §3.4）的兑现：最后一次见到早于 `before` 的会话，连同它的事件与反馈一起删。
/// 返回删掉的会话数。一个事务里做完，不留半个会话。
pub fn delete_sessions_before(conn: &mut Connection, before: &str) -> rusqlite::Result<usize> {
    let tx = conn.transaction()?;
    tx.execute(
        "DELETE FROM session_events WHERE session_id IN (SELECT id FROM sessions WHERE last_seen_at < ?1)",
        params![before],
    )?;
    tx.execute(
        "DELETE FROM feedback WHERE session_id IN (SELECT id FROM sessions WHERE last_seen_at < ?1)",
        params![before],
    )?;
    let sessions = tx.execute(
        "DELETE FROM sessions WHERE last_seen_at < ?1",
        params![before],
    )?;
    tx.commit()?;
    Ok(sessions)
}

// 参数多是因为提交一个版本要写三张表；拆成结构体只是把同样九个字段换个地方写。
#[allow(clippy::too_many_arguments)]
/// 提交一个版本：库这一侧的三处改动一起生效，中途失败就都不生效。
pub fn commit_version(
    conn: &mut Connection,
    slug: &str,
    version: u32,
    upload_id: &str,
    title: &str,
    created_at: &str,
    note: Option<&str>,
    file_count: usize,
    total_bytes: u64,
) -> rusqlite::Result<()> {
    let tx = conn.transaction()?;
    tx.execute(
        "INSERT INTO versions (slug, version, created_at, note, file_count, total_bytes)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            slug,
            version,
            created_at,
            note,
            file_count as i64,
            total_bytes as i64
        ],
    )?;
    tx.execute(
        "UPDATE sites SET current_version = ?2, title = ?3 WHERE slug = ?1",
        params![slug, version, title],
    )?;
    tx.execute(
        "UPDATE uploads SET status = 'committed' WHERE id = ?1",
        params![upload_id],
    )?;
    tx.commit()
}

/// 一个作品发过的每一版，新的在前。
pub fn list_versions(conn: &Connection, slug: &str) -> rusqlite::Result<Vec<VersionRow>> {
    let mut stmt = conn.prepare(
        "SELECT version, created_at, note, file_count, total_bytes
           FROM versions WHERE slug = ?1 ORDER BY version DESC",
    )?;
    let rows = stmt.query_map(params![slug], |row| {
        Ok(VersionRow {
            version: row.get::<_, i64>(0)? as u32,
            created_at: row.get(1)?,
            note: row.get(2)?,
            file_count: row.get::<_, i64>(3)? as u32,
            total_bytes: row.get::<_, i64>(4)? as u64,
        })
    })?;
    rows.collect()
}

pub struct VersionRow {
    pub version: u32,
    pub created_at: String,
    pub note: Option<String>,
    pub file_count: u32,
    pub total_bytes: u64,
}

/// 下一版是几：按发过的最大版本号加一，不看「当前版本」——回滚会把指针指回去，版本号不能跟着倒退。
pub fn next_version(conn: &Connection, slug: &str) -> rusqlite::Result<u32> {
    let max: Option<i64> = conn.query_row(
        "SELECT MAX(version) FROM versions WHERE slug = ?1",
        params![slug],
        |row| row.get(0),
    )?;
    Ok(max.unwrap_or(0) as u32 + 1)
}

pub fn version_exists(conn: &Connection, slug: &str, version: u32) -> rusqlite::Result<bool> {
    conn.query_row(
        "SELECT 1 FROM versions WHERE slug = ?1 AND version = ?2",
        params![slug, version],
        |_| Ok(()),
    )
    .optional()
    .map(|found| found.is_some())
}

/// 回滚只动指针这一列；版本行不动，所以「下一版是第几版」仍按最大版本号加一，不会撞。
pub fn set_current_version(conn: &Connection, slug: &str, version: u32) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE sites SET current_version = ?2 WHERE slug = ?1",
        params![slug, version],
    )?;
    Ok(())
}

// ---- 广场（DESIGN §3.8） ----

/// 提交一个版本时，把清单里广场要用的几样抄到 sites 上：一句话介绍、封面、引擎、更新时间。
/// `summary` / `cover` 传 `None` 表示这一版没给，沿用上一版的（COALESCE）。
pub fn record_version_meta(
    conn: &Connection,
    slug: &str,
    summary: Option<&str>,
    cover: Option<(&str, &str)>,
    engine: Option<&str>,
    updated_at: &str,
) -> rusqlite::Result<()> {
    let (cover_hash, cover_mime) = match cover {
        Some((hash, mime)) => (Some(hash), Some(mime)),
        None => (None, None),
    };
    conn.execute(
        "UPDATE sites SET
             summary    = COALESCE(?2, summary),
             cover_hash = COALESCE(?3, cover_hash),
             cover_mime = COALESCE(?4, cover_mime),
             engine     = ?5,
             updated_at = ?6
         WHERE slug = ?1",
        params![slug, summary, cover_hash, cover_mime, engine, updated_at],
    )?;
    Ok(())
}

/// 开发者改广场状态：只改传了的。`seek_note` 是 `Some(None)` 时清掉。
pub fn update_listing(
    conn: &Connection,
    slug: &str,
    public: Option<bool>,
    seeking: Option<bool>,
    seek_note: Option<Option<&str>>,
) -> rusqlite::Result<()> {
    if let Some(public) = public {
        conn.execute(
            "UPDATE sites SET public = ?2 WHERE slug = ?1",
            params![slug, public as i64],
        )?;
    }
    if let Some(seeking) = seeking {
        conn.execute(
            "UPDATE sites SET seeking = ?2 WHERE slug = ?1",
            params![slug, seeking as i64],
        )?;
    }
    if let Some(note) = seek_note {
        conn.execute(
            "UPDATE sites SET seek_note = ?2 WHERE slug = ?1",
            params![slug, note],
        )?;
    }
    Ok(())
}

/// 从广场上撤下（举报到阈值或我们手工）。作品链接不受影响。已经撤下的不改时间。
pub fn hide_from_plaza(
    conn: &Connection,
    slug: &str,
    at: &str,
    reason: &str,
) -> rusqlite::Result<bool> {
    let changed = conn.execute(
        "UPDATE sites SET hidden_at = ?2, hidden_reason = ?3 WHERE slug = ?1 AND hidden_at IS NULL",
        params![slug, at, reason],
    )?;
    Ok(changed > 0)
}

/// 人工复核后恢复。记下复核时间：之前的举报不再算数，否则同一批举报会把它立刻再撤一次。
pub fn unhide_from_plaza(conn: &Connection, slug: &str, reviewed_at: &str) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE sites SET hidden_at = NULL, hidden_reason = NULL, reviewed_at = ?2 WHERE slug = ?1",
        params![slug, reviewed_at],
    )?;
    Ok(())
}

/// 广场上该出现的作品：开发者公开了、没被撤下、没删、有版本、主人没到期。
/// 带上开发者的显示名；排序交给调用方（它还要合上人数）。
pub struct PlazaCandidate {
    pub site: SiteRow,
    pub developer: String,
}

pub fn plaza_candidates(conn: &Connection, now: &str) -> rusqlite::Result<Vec<PlazaCandidate>> {
    let sql = format!(
        "SELECT {}, u.display_name
           FROM sites s JOIN users u ON u.id = s.user_id
          WHERE s.public = 1 AND s.hidden_at IS NULL AND s.deleted_at IS NULL
            AND s.current_version IS NOT NULL
            AND (u.expires_at IS NULL OR u.expires_at > ?1)",
        site_columns("s.")
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params![now], |row| {
        Ok(PlazaCandidate {
            site: site_from_row(row)?,
            // 开发者名字跟在作品的那组列后面。
            developer: row.get(SITE_COLUMN_NAMES.len())?,
        })
    })?;
    rows.collect()
}

/// 每个作品在 `since` 之后点了「开始」的去重人数（DESIGN §3.8「N 人玩过」）。
pub fn players_since(
    conn: &Connection,
    since: &str,
) -> rusqlite::Result<std::collections::HashMap<String, u32>> {
    let mut stmt = conn.prepare(
        "SELECT slug, COUNT(DISTINCT id) FROM sessions
          WHERE start_at IS NOT NULL AND start_at >= ?1 GROUP BY slug",
    )?;
    let rows = stmt.query_map(params![since], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as u32))
    })?;
    rows.collect()
}

/// 一个作品在 `since` 之后被几个不同会话举报过（自动撤下的依据，DESIGN §3.8）。
pub fn report_count(conn: &Connection, slug: &str, since: &str) -> rusqlite::Result<u32> {
    conn.query_row(
        "SELECT COUNT(DISTINCT session_id) FROM session_events
          WHERE slug = ?1 AND source = 'edge' AND kind = 'report' AND ts >= ?2",
        params![slug, since],
        |row| row.get::<_, i64>(0),
    )
    .map(|n| n as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_are_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("api.sqlite");
        Db::open(&path).unwrap();
        // 第二次打开不该再跑一遍建表语句。
        Db::open(&path).unwrap();
    }

    #[test]
    fn deleted_slug_stays_taken() {
        let db = Db::open_in_memory().unwrap();
        let conn = db.conn.blocking_lock();
        insert_user(
            &conn,
            &NewUser {
                id: "u1",
                kind: "anon",
                display_name: "匿名开发者",
                created_at: "2026-09-07T00:00:00Z",
                expires_at: Some("2026-09-08T00:00:00Z"),
            },
        )
        .unwrap();
        insert_site(
            &conn,
            "brisk-otter-41",
            "u1",
            "小游戏",
            "2026-09-07T00:00:00Z",
            None,
        )
        .unwrap();
        assert_eq!(count_live_sites(&conn, "u1").unwrap(), 1);

        assert!(mark_site_deleted(&conn, "brisk-otter-41", "2026-09-07T01:00:00Z").unwrap());
        assert!(!mark_site_deleted(&conn, "brisk-otter-41", "2026-09-07T02:00:00Z").unwrap());
        assert_eq!(count_live_sites(&conn, "u1").unwrap(), 0);
        assert!(slug_taken(&conn, "brisk-otter-41").unwrap());
    }
}
