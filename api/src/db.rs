//! SQLite：账号、令牌、作品、版本、上传、blob 索引。
//!
//! **一把写锁、一池读连接。** SQLite 在 WAL 下是「一个写者、任意多读者、读不挡写」：
//! 写走 [`Db::lock`]，一个连接配一把 [`tokio::sync::Mutex`]——所有「先查再写」的序列
//! （找不到就建、余额够就扣）都在这把锁里做，不会被另一个请求插进来；读走 [`Db::read`]，
//! 从几个 `query_only` 的连接里拿一个，互不排队，也不排在写者后面。
//! 原来只有一把锁，一次 `live.json` 重算或一页结果查询会让同一时刻的上传排队；
//! 现在读者之间、读者与写者之间都不等。规模仍是单机（REWRITE §9.5：暂不换 Postgres），
//! 换库时换掉的是这一层，不是这些 SQL。

use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

use anyhow::Context;
use rusqlite::{params, Connection, OpenFlags, OptionalExtension, Row};
use tokio::sync::{Mutex, MutexGuard};

/// 迁移按顺序编号，下标 + 1 就是版本号。只增不改：已经跑过的那条永远不动。
const MIGRATIONS: &[&str] = &[
    include_str!("migrations/001_init.sql"),
    include_str!("migrations/002_sessions_events_feedback.sql"),
    include_str!("migrations/003_plaza.sql"),
    include_str!("migrations/004_github_login.sql"),
    include_str!("migrations/005_club.sql"),
    include_str!("migrations/006_collections.sql"),
];

/// 读连接的个数。控制面同一时刻在读的东西：几个请求、一次 `live.json` 重算、一次广场重算；
/// 四个够，多了只是多几个文件句柄。
const READERS: usize = 4;

pub struct Db {
    writer: Mutex<Connection>,
    /// 空的话（内存库）读也走写锁：`:memory:` 的每个连接是各自一个库。
    readers: Vec<Mutex<Connection>>,
    next: AtomicUsize,
}

/// [`Db::read`] 拿到的连接。只能读：连接开着 `query_only`，写会在 SQLite 那一层被拒。
pub struct ReadGuard<'a>(MutexGuard<'a, Connection>);

impl std::ops::Deref for ReadGuard<'_> {
    type Target = Connection;
    fn deref(&self) -> &Connection {
        &self.0
    }
}

impl Db {
    /// 打开（不存在就新建）数据库并把迁移跑完。
    pub fn open(path: &Path) -> anyhow::Result<Self> {
        let mut conn = Connection::open(path)
            .with_context(|| format!("打不开数据库文件 {}", path.display()))?;
        // WAL：读不挡写。busy_timeout 给同一台机器上另一个进程（比如手工用 sqlite3 看一眼）留出让路的时间。
        conn.pragma_update(None, "journal_mode", "WAL")
            .context("开 WAL 模式失败")?;
        tune(&conn)?;
        migrate(&mut conn)?;

        let mut readers = Vec::with_capacity(READERS);
        for _ in 0..READERS {
            let reader = Connection::open_with_flags(
                path,
                OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
            )
            .with_context(|| format!("开不了第二个连接 {}", path.display()))?;
            tune(&reader)?;
            // 读连接只读：谁不小心拿它写，SQLite 当场报错，而不是悄悄绕过写锁。
            reader.pragma_update(None, "query_only", "ON")?;
            readers.push(Mutex::new(reader));
        }
        Ok(Self {
            writer: Mutex::new(conn),
            readers,
            next: AtomicUsize::new(0),
        })
    }

    #[cfg(test)]
    pub fn open_in_memory() -> anyhow::Result<Self> {
        let mut conn = Connection::open_in_memory()?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        migrate(&mut conn)?;
        Ok(Self {
            writer: Mutex::new(conn),
            readers: Vec::new(),
            next: AtomicUsize::new(0),
        })
    }

    /// 写连接。「先查再写」的一整段都握着它做。
    pub async fn lock(&self) -> MutexGuard<'_, Connection> {
        self.writer.lock().await
    }

    /// 一个只读连接。先看有没有空着的；都忙就轮着排一个，不排在写者后面。
    pub async fn read(&self) -> ReadGuard<'_> {
        if self.readers.is_empty() {
            return ReadGuard(self.writer.lock().await);
        }
        for reader in &self.readers {
            if let Ok(guard) = reader.try_lock() {
                return ReadGuard(guard);
            }
        }
        let i = self.next.fetch_add(1, Ordering::Relaxed) % self.readers.len();
        ReadGuard(self.readers[i].lock().await)
    }
}

fn tune(conn: &Connection) -> anyhow::Result<()> {
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.busy_timeout(std::time::Duration::from_secs(5))?;
    Ok(())
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
    pub login: Option<String>,
    pub avatar_url: Option<String>,
    pub token_expires_at: Option<String>,
    pub user_expires_at: Option<String>,
}

pub fn find_token_owner(
    conn: &Connection,
    token_hash: &str,
) -> rusqlite::Result<Option<TokenOwner>> {
    conn.query_row(
        "SELECT u.id, u.kind, u.display_name, t.expires_at, u.expires_at, u.login, u.avatar_url
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
                login: row.get(5)?,
                avatar_url: row.get(6)?,
            })
        },
    )
    .optional()
}

// ---- GitHub 登录 ----

/// 认人靠 GitHub 的数字 id；用户名、显示名和头像每次登录都刷新（人会改名、会换头像）。
/// 返回我们这边的 user_id，以及头像有没有变——变了要重写这个人所有作品的 `live.json`。
pub fn upsert_github_user(
    conn: &Connection,
    github_id: i64,
    login: &str,
    display_name: &str,
    avatar_url: Option<&str>,
    now: &str,
) -> rusqlite::Result<(String, bool)> {
    let existing: Option<(String, Option<String>)> = conn
        .query_row(
            "SELECT id, avatar_url FROM users WHERE github_id = ?1",
            params![github_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    if let Some((id, was)) = existing {
        conn.execute(
            "UPDATE users SET login = ?2, display_name = ?3, avatar_url = ?4 WHERE id = ?1",
            params![id, login, display_name, avatar_url],
        )?;
        return Ok((id, was.as_deref() != avatar_url));
    }
    let id = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO users (id, kind, display_name, created_at, expires_at, github_id, login, avatar_url)
         VALUES (?1, 'github', ?2, ?3, NULL, ?4, ?5, ?6)",
        params![id, display_name, now, github_id, login, avatar_url],
    )?;
    Ok((id, avatar_url.is_some()))
}

/// 这个人名下还活着的作品，登录后头像变了要挨个重写 `live.json`。
pub fn live_slugs_of(conn: &Connection, user_id: &str) -> rusqlite::Result<Vec<String>> {
    let mut stmt =
        conn.prepare("SELECT slug FROM sites WHERE user_id = ?1 AND deleted_at IS NULL")?;
    let rows = stmt.query_map(params![user_id], |row| row.get(0))?;
    rows.collect()
}

/// 把一个匿名身份下还活着的作品全部归到账号里：换主人、去掉到期时间。
/// 返回挪过去的 slug——清单里的到期时间和开发者名字要由调用方另外改（那在对象存储里）。
pub fn adopt_sites(
    conn: &Connection,
    from_user: &str,
    to_user: &str,
) -> rusqlite::Result<Vec<String>> {
    let mut stmt =
        conn.prepare("SELECT slug FROM sites WHERE user_id = ?1 AND deleted_at IS NULL")?;
    let slugs: Vec<String> = stmt
        .query_map(params![from_user], |row| row.get(0))?
        .collect::<Result<_, _>>()?;
    if slugs.is_empty() {
        return Ok(slugs);
    }
    conn.execute(
        "UPDATE sites SET user_id = ?2, expires_at = NULL WHERE user_id = ?1 AND deleted_at IS NULL",
        params![from_user, to_user],
    )?;
    conn.execute(
        "UPDATE uploads SET user_id = ?2 WHERE user_id = ?1",
        params![from_user, to_user],
    )?;
    Ok(slugs)
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
    /// 想找几位试玩者（DESIGN §3.3）。
    pub seats: Option<u32>,
    /// 开发者的群。
    pub community_url: Option<String>,
    /// 「让玩家看到彼此的反馈」。
    pub feedback_public: bool,
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
    "seats",
    "community_url",
    "feedback_public",
];

const SITE_COLUMNS: &str = "slug, title, created_at, expires_at, current_version, \
    public, seeking, seek_note, summary, cover_hash, cover_mime, engine, updated_at, \
    hidden_at, hidden_reason, reviewed_at, seats, community_url, feedback_public";

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
            seats: row.get(16)?,
            community_url: row.get(17)?,
            feedback_public: row.get::<_, i64>(18)? != 0,
        },
    })
}

/// 一个作品是谁的，以及他的头像——`live.json` 与通知都要用（DESIGN §3.9、§4.10）。
/// 不看 `deleted_at`：删掉的作品也要能查出主人来清尾巴。
pub struct SiteOwner {
    pub user_id: String,
    /// `anon` 或 `github`。匿名作品不能上推广位（DESIGN §3.11）。
    pub kind: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
}

pub fn site_owner(conn: &Connection, slug: &str) -> rusqlite::Result<Option<SiteOwner>> {
    conn.query_row(
        "SELECT u.id, u.kind, u.display_name, u.avatar_url
           FROM sites s JOIN users u ON u.id = s.user_id
          WHERE s.slug = ?1",
        params![slug],
        |row| {
            Ok(SiteOwner {
                user_id: row.get(0)?,
                kind: row.get(1)?,
                display_name: row.get(2)?,
                avatar_url: row.get(3)?,
            })
        },
    )
    .optional()
}

/// 不问主人是谁的一行。`live.json` 和通知要读任何一个作品，不只是「我的」。
pub fn find_site(conn: &Connection, slug: &str) -> rusqlite::Result<Option<SiteRow>> {
    let sql = format!("SELECT {SITE_COLUMNS} FROM sites WHERE slug = ?1 AND deleted_at IS NULL");
    conn.query_row(&sql, params![slug], site_from_row)
        .optional()
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

/// 俱乐部那三项设置（DESIGN §3.3、§3.5）。外层 `Some` 是「这次带了」，
/// 里层 `None` 是「清掉」——`--seats 0` 和 `--community ""` 都落在这里。
pub fn update_club_settings(
    conn: &Connection,
    slug: &str,
    seats: Option<Option<u32>>,
    community_url: Option<Option<&str>>,
    feedback_public: Option<bool>,
) -> rusqlite::Result<()> {
    if let Some(seats) = seats {
        conn.execute(
            "UPDATE sites SET seats = ?2 WHERE slug = ?1",
            params![slug, seats],
        )?;
    }
    if let Some(url) = community_url {
        conn.execute(
            "UPDATE sites SET community_url = ?2 WHERE slug = ?1",
            params![slug, url],
        )?;
    }
    if let Some(public) = feedback_public {
        conn.execute(
            "UPDATE sites SET feedback_public = ?2 WHERE slug = ?1",
            params![slug, public as i64],
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
    pub avatar_url: Option<String>,
}

pub fn plaza_candidates(conn: &Connection, now: &str) -> rusqlite::Result<Vec<PlazaCandidate>> {
    let sql = format!(
        "SELECT {}, u.display_name, u.avatar_url
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
            // 开发者的两列跟在作品的那组列后面。
            developer: row.get(SITE_COLUMN_NAMES.len())?,
            avatar_url: row.get(SITE_COLUMN_NAMES.len() + 1)?,
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

// ---- 留名与名额（DESIGN §3.3） ----

/// 一个会话只留一次名：后来的 `start` 不覆盖前一次——同一个人刷新一遍不该改掉点名册上的名字。
/// 返回真的写进去了没有；写进去了才值得重写 `live.json`。
pub fn set_session_name(conn: &Connection, session_id: &str, name: &str) -> rusqlite::Result<bool> {
    let changed = conn.execute(
        "UPDATE sessions SET name = ?2 WHERE id = ?1 AND (name IS NULL OR name = '')",
        params![session_id, name],
    )?;
    Ok(changed > 0)
}

/// 「已加入」= 留了名字的人（DESIGN §3.3 第 4 条），不是所有打开的人。所有版本一起数。
pub fn joined_count(conn: &Connection, slug: &str) -> rusqlite::Result<u32> {
    conn.query_row(
        "SELECT COUNT(DISTINCT id) FROM sessions WHERE slug = ?1 AND name IS NOT NULL AND name <> ''",
        params![slug],
        |row| row.get::<_, i64>(0),
    )
    .map(|n| n as u32)
}

pub fn joined_counts(
    conn: &Connection,
) -> rusqlite::Result<std::collections::HashMap<String, u32>> {
    let mut stmt = conn.prepare(
        "SELECT slug, COUNT(DISTINCT id) FROM sessions
          WHERE name IS NOT NULL AND name <> '' GROUP BY slug",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as u32))
    })?;
    rows.collect()
}

/// 这一版每种来源各有几个人（DESIGN §3.5「来自：邀请卡 4 · 广场 2 · 微信 2」）。
pub fn sources_by_version(
    conn: &Connection,
    slug: &str,
) -> rusqlite::Result<std::collections::HashMap<u32, Vec<(String, u32)>>> {
    let mut stmt = conn.prepare(
        "SELECT version, referrer_kind, COUNT(*) FROM sessions
          WHERE slug = ?1 AND referrer_kind IS NOT NULL AND referrer_kind <> ''
          GROUP BY version, referrer_kind
          ORDER BY version, COUNT(*) DESC, referrer_kind",
    )?;
    let rows = stmt.query_map(params![slug], |row| {
        Ok((
            row.get::<_, u32>(0)?,
            (row.get::<_, String>(1)?, row.get::<_, i64>(2)? as u32),
        ))
    })?;
    let mut out: std::collections::HashMap<u32, Vec<(String, u32)>> =
        std::collections::HashMap::new();
    for row in rows {
        let (version, tally) = row?;
        out.entry(version).or_default().push(tally);
    }
    Ok(out)
}

/// 门禁页上那最多三条（DESIGN §3.3 第 7 条）。作品级的开关由调用方判，这里只管
/// 「没被单独藏起来的、最近的几条」，署名从会话里的留名来。
pub struct PublicFeedbackRow {
    pub name: Option<String>,
    pub text: String,
    pub version: u32,
    pub at: String,
}

pub fn recent_public_feedback(
    conn: &Connection,
    slug: &str,
    limit: usize,
) -> rusqlite::Result<Vec<PublicFeedbackRow>> {
    let mut stmt = conn.prepare(
        "SELECT NULLIF(s.name, ''), f.text, f.version, f.ts
           FROM feedback f LEFT JOIN sessions s ON s.id = f.session_id
          WHERE f.slug = ?1 AND f.hidden = 0
          ORDER BY f.ts DESC, f.id DESC
          LIMIT ?2",
    )?;
    let rows = stmt.query_map(params![slug, limit as i64], |row| {
        Ok(PublicFeedbackRow {
            name: row.get(0)?,
            text: row.get(1)?,
            version: row.get(2)?,
            at: row.get(3)?,
        })
    })?;
    rows.collect()
}

/// 把一条反馈藏起来 / 放出来（DESIGN §3.5）。返回改到了没有。
pub fn set_feedback_hidden(
    conn: &Connection,
    slug: &str,
    id: i64,
    hidden: bool,
) -> rusqlite::Result<bool> {
    let changed = conn.execute(
        "UPDATE feedback SET hidden = ?3 WHERE id = ?1 AND slug = ?2",
        params![id, slug, hidden as i64],
    )?;
    Ok(changed > 0)
}

pub fn set_feedback_status(
    conn: &Connection,
    slug: &str,
    id: i64,
    status: &str,
) -> rusqlite::Result<bool> {
    let changed = conn.execute(
        "UPDATE feedback SET status = ?3 WHERE id = ?1 AND slug = ?2",
        params![id, slug, status],
    )?;
    Ok(changed > 0)
}

/// 最近有动静的作品：`live.json` 那一档定时刷新的范围。全量重写在几千个作品上
/// 就是几千次写盘，而没人玩的作品那份文件本来也不会变。
pub fn active_slugs_since(conn: &Connection, since: &str) -> rusqlite::Result<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT DISTINCT slug FROM (
             SELECT slug FROM sessions WHERE last_seen_at >= ?1
             UNION SELECT slug FROM versions WHERE created_at >= ?1
             UNION SELECT target_slug AS slug FROM follows
                    WHERE target_kind = 'site' AND target_slug IS NOT NULL AND created_at >= ?1
         ) WHERE slug IN (SELECT slug FROM sites WHERE deleted_at IS NULL)",
    )?;
    let rows = stmt.query_map(params![since], |row| row.get(0))?;
    rows.collect()
}

// ---- 玩家与关注（DESIGN §3.6） ----

/// 只有「确认过、还没退订」的人才算关注者，也才收得到信。
/// 邮箱那条路的确认是点了确认信；浏览器通知那条路浏览器已经问过用户了，有订阅就算数。
const CONFIRMED: &str =
    "p.unsubscribed_at IS NULL AND (p.email_verified_at IS NOT NULL OR p.push_endpoint IS NOT NULL)";

#[derive(Debug, Clone)]
pub struct PlayerRow {
    pub id: String,
    pub email: Option<String>,
    pub email_verified_at: Option<String>,
    pub push_subscription: Option<String>,
    pub push_endpoint: Option<String>,
    pub me_token_hash: Option<String>,
    pub unsubscribe_token: Option<String>,
    pub unsubscribed_at: Option<String>,
}

const PLAYER_COLUMNS: &str = "id, email, email_verified_at, push_subscription, push_endpoint, \
    me_token_hash, unsubscribe_token, unsubscribed_at";

fn player_from_row(row: &Row<'_>) -> rusqlite::Result<PlayerRow> {
    Ok(PlayerRow {
        id: row.get(0)?,
        email: row.get(1)?,
        email_verified_at: row.get(2)?,
        push_subscription: row.get(3)?,
        push_endpoint: row.get(4)?,
        me_token_hash: row.get(5)?,
        unsubscribe_token: row.get(6)?,
        unsubscribed_at: row.get(7)?,
    })
}

fn find_player_by(
    conn: &Connection,
    column: &str,
    value: &str,
) -> rusqlite::Result<Option<PlayerRow>> {
    // 列名是本模块里的字面量，不来自请求。
    let sql = format!("SELECT {PLAYER_COLUMNS} FROM players WHERE {column} = ?1");
    conn.query_row(&sql, params![value], player_from_row)
        .optional()
}

pub fn find_player(conn: &Connection, id: &str) -> rusqlite::Result<Option<PlayerRow>> {
    find_player_by(conn, "id", id)
}

pub fn find_player_by_email(conn: &Connection, email: &str) -> rusqlite::Result<Option<PlayerRow>> {
    find_player_by(conn, "email", email)
}

pub fn find_player_by_push(
    conn: &Connection,
    endpoint: &str,
) -> rusqlite::Result<Option<PlayerRow>> {
    find_player_by(conn, "push_endpoint", endpoint)
}

pub fn find_player_by_me_token(
    conn: &Connection,
    token_hash: &str,
) -> rusqlite::Result<Option<PlayerRow>> {
    find_player_by(conn, "me_token_hash", token_hash)
}

pub fn find_player_by_unsubscribe_token(
    conn: &Connection,
    token_hash: &str,
) -> rusqlite::Result<Option<PlayerRow>> {
    find_player_by(conn, "unsubscribe_token", token_hash)
}

pub struct NewPlayer<'a> {
    pub id: &'a str,
    pub email: Option<&'a str>,
    pub push_subscription: Option<&'a str>,
    pub push_endpoint: Option<&'a str>,
    /// 一个人一个长期的退订令牌，每封信底部那个链接用它（DESIGN §3.6）。
    pub unsubscribe_token: &'a str,
    pub created_at: &'a str,
}

pub fn insert_player(conn: &Connection, player: &NewPlayer<'_>) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO players
             (id, email, push_subscription, push_endpoint, unsubscribe_token, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            player.id,
            player.email,
            player.push_subscription,
            player.push_endpoint,
            player.unsubscribe_token,
            player.created_at,
        ],
    )?;
    Ok(())
}

/// 点了确认信：邮箱从此算数。退订过的人再确认一次就是回来了，把退订标记清掉。
pub fn confirm_player_email(conn: &Connection, id: &str, at: &str) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE players SET email_verified_at = COALESCE(email_verified_at, ?2),
                            unsubscribed_at = NULL
         WHERE id = ?1",
        params![id, at],
    )?;
    Ok(())
}

pub fn set_player_me_token(conn: &Connection, id: &str, token_hash: &str) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE players SET me_token_hash = ?2 WHERE id = ?1",
        params![id, token_hash],
    )?;
    Ok(())
}

/// 同一个人换了浏览器又允许了一次通知：覆盖那条订阅。
pub fn set_player_push(
    conn: &Connection,
    id: &str,
    subscription: &str,
    endpoint: &str,
) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE players SET push_subscription = ?2, push_endpoint = ?3, unsubscribed_at = NULL
         WHERE id = ?1",
        params![id, subscription, endpoint],
    )?;
    Ok(())
}

/// 推送服务说这个订阅没了（410 / 404），或者人退订了。
pub fn clear_player_push(conn: &Connection, id: &str) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE players SET push_subscription = NULL, push_endpoint = NULL WHERE id = ?1",
        params![id],
    )?;
    Ok(())
}

/// 一键退订（DESIGN §3.6「点了就退，不问为什么」）：全部关注消失，推送订阅删掉，
/// 待发的信也不再发——退订之后还收到一封是最伤发信信誉的事（DESIGN §4.8）。
/// 返回受影响的作品 slug，调用方据此重写 `live.json`。
pub fn unsubscribe_player(conn: &Connection, id: &str, at: &str) -> rusqlite::Result<Vec<String>> {
    let slugs = followed_slugs(conn, id)?;
    conn.execute("DELETE FROM follows WHERE player_id = ?1", params![id])?;
    conn.execute(
        "DELETE FROM notifications WHERE player_id = ?1 AND status = 'pending'",
        params![id],
    )?;
    conn.execute(
        "UPDATE players SET unsubscribed_at = ?2, push_subscription = NULL, push_endpoint = NULL
         WHERE id = ?1",
        params![id, at],
    )?;
    Ok(slugs)
}

fn followed_slugs(conn: &Connection, player_id: &str) -> rusqlite::Result<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT target_slug FROM follows WHERE player_id = ?1 AND target_kind = 'site'
                                           AND target_slug IS NOT NULL",
    )?;
    let rows = stmt.query_map(params![player_id], |row| row.get(0))?;
    rows.collect()
}

pub struct NewFollowToken<'a> {
    pub token_hash: &'a str,
    pub player_id: &'a str,
    /// `confirm`（确认关注）或 `send_link`（把「我的」的链接寄给自己）。
    pub purpose: &'a str,
    pub target_kind: Option<&'a str>,
    pub target_slug: Option<&'a str>,
    pub source: Option<&'a str>,
    pub created_at: &'a str,
    pub expires_at: &'a str,
}

pub fn insert_follow_token(conn: &Connection, token: &NewFollowToken<'_>) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO follow_tokens
             (token_hash, player_id, purpose, target_kind, target_slug, source, created_at, expires_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            token.token_hash,
            token.player_id,
            token.purpose,
            token.target_kind,
            token.target_slug,
            token.source,
            token.created_at,
            token.expires_at,
        ],
    )?;
    Ok(())
}

pub struct FollowTokenRow {
    pub player_id: String,
    pub purpose: String,
    pub target_kind: Option<String>,
    pub target_slug: Option<String>,
    pub source: Option<String>,
}

/// 一次性：没用过、没过期才算数，取到就当场标记用掉。
pub fn take_follow_token(
    conn: &Connection,
    token_hash: &str,
    now: &str,
) -> rusqlite::Result<Option<FollowTokenRow>> {
    let row = conn
        .query_row(
            "SELECT player_id, purpose, target_kind, target_slug, source
               FROM follow_tokens
              WHERE token_hash = ?1 AND used_at IS NULL AND expires_at > ?2",
            params![token_hash, now],
            |row| {
                Ok(FollowTokenRow {
                    player_id: row.get(0)?,
                    purpose: row.get(1)?,
                    target_kind: row.get(2)?,
                    target_slug: row.get(3)?,
                    source: row.get(4)?,
                })
            },
        )
        .optional()?;
    if row.is_some() {
        conn.execute(
            "UPDATE follow_tokens SET used_at = ?2 WHERE token_hash = ?1",
            params![token_hash, now],
        )?;
    }
    Ok(row)
}

/// 加一条关注。已经关注着就返回 false，不报错——玩家点两下不该看到错误。
pub fn insert_follow(
    conn: &Connection,
    player_id: &str,
    target_kind: &str,
    target_slug: Option<&str>,
    source: Option<&str>,
    created_at: &str,
) -> rusqlite::Result<bool> {
    let changed = conn.execute(
        "INSERT OR IGNORE INTO follows (player_id, target_kind, target_slug, source, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![player_id, target_kind, target_slug, source, created_at],
    )?;
    Ok(changed > 0)
}

pub fn delete_follow(
    conn: &Connection,
    player_id: &str,
    target_kind: &str,
    target_slug: Option<&str>,
) -> rusqlite::Result<bool> {
    let changed = conn.execute(
        "DELETE FROM follows
          WHERE player_id = ?1 AND target_kind = ?2 AND IFNULL(target_slug, '') = IFNULL(?3, '')",
        params![player_id, target_kind, target_slug],
    )?;
    Ok(changed > 0)
}

pub struct FollowRow {
    pub target_kind: String,
    pub target_slug: Option<String>,
    pub title: Option<String>,
    pub since: String,
}

/// 「我的」里那一列（DESIGN §3.10）。作品已经删掉的那几条不显示，但也不悄悄删——
/// 作品可能只是被主人下架，等他回来。
pub fn follows_of(conn: &Connection, player_id: &str) -> rusqlite::Result<Vec<FollowRow>> {
    let mut stmt = conn.prepare(
        "SELECT f.target_kind, f.target_slug, s.title, f.created_at
           FROM follows f LEFT JOIN sites s
             ON s.slug = f.target_slug AND s.deleted_at IS NULL
          WHERE f.player_id = ?1
          ORDER BY f.created_at DESC, f.id DESC",
    )?;
    let rows = stmt.query_map(params![player_id], |row| {
        Ok(FollowRow {
            target_kind: row.get(0)?,
            target_slug: row.get(1)?,
            title: row.get(2)?,
            since: row.get(3)?,
        })
    })?;
    rows.collect()
}

/// 开发者看到的那个数字（DESIGN §3.6「控制台只给『12 人关注』」）。
pub fn followers_count(conn: &Connection, slug: &str) -> rusqlite::Result<u32> {
    conn.query_row(
        &format!(
            "SELECT COUNT(*) FROM follows f JOIN players p ON p.id = f.player_id
              WHERE f.target_kind = 'site' AND f.target_slug = ?1 AND {CONFIRMED}"
        ),
        params![slug],
        |row| row.get::<_, i64>(0),
    )
    .map(|n| n as u32)
}

pub fn followers_counts(
    conn: &Connection,
) -> rusqlite::Result<std::collections::HashMap<String, u32>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT f.target_slug, COUNT(*) FROM follows f JOIN players p ON p.id = f.player_id
          WHERE f.target_kind = 'site' AND f.target_slug IS NOT NULL AND {CONFIRMED}
          GROUP BY f.target_slug"
    ))?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as u32))
    })?;
    rows.collect()
}

/// 关注广场本身的人数（DESIGN §3.9 的 `club_followers`，也是周报的收件人数）。
pub fn plaza_followers_count(conn: &Connection) -> rusqlite::Result<u32> {
    conn.query_row(
        &format!(
            "SELECT COUNT(*) FROM follows f JOIN players p ON p.id = f.player_id
              WHERE f.target_kind = 'plaza' AND {CONFIRMED}"
        ),
        [],
        |row| row.get::<_, i64>(0),
    )
    .map(|n| n as u32)
}

/// 该给谁发信。一个人一行，能走哪条渠道由这两列决定（都有就发邮件，邮件更可靠）。
pub struct Recipient {
    pub player_id: String,
    pub email: Option<String>,
    pub push_subscription: Option<String>,
}

pub fn confirmed_followers(
    conn: &Connection,
    target_kind: &str,
    target_slug: Option<&str>,
) -> rusqlite::Result<Vec<Recipient>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT p.id, p.email, p.push_subscription
           FROM follows f JOIN players p ON p.id = f.player_id
          WHERE f.target_kind = ?1 AND IFNULL(f.target_slug, '') = IFNULL(?2, '') AND {CONFIRMED}
          ORDER BY f.created_at"
    ))?;
    let rows = stmt.query_map(params![target_kind, target_slug], |row| {
        Ok(Recipient {
            player_id: row.get(0)?,
            email: row.get(1)?,
            push_subscription: row.get(2)?,
        })
    })?;
    rows.collect()
}

// ---- 通知队列（DESIGN §4.10） ----

pub struct NewNotification<'a> {
    pub player_id: &'a str,
    pub kind: &'a str,
    pub target_slug: Option<&'a str>,
    pub subject: &'a str,
    pub body: &'a str,
    pub url: Option<&'a str>,
    pub channel: &'a str,
    pub not_before: &'a str,
    pub created_at: &'a str,
}

pub fn enqueue_notification(
    conn: &Connection,
    notification: &NewNotification<'_>,
) -> rusqlite::Result<i64> {
    conn.execute(
        "INSERT INTO notifications
             (player_id, kind, target_slug, subject, body, url, channel, status, attempts,
              not_before, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'pending', 0, ?8, ?9)",
        params![
            notification.player_id,
            notification.kind,
            notification.target_slug,
            notification.subject,
            notification.body,
            notification.url,
            notification.channel,
            notification.not_before,
            notification.created_at,
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

/// 同一个人、同一个作品，`since` 之后已经有过的那一封（DESIGN §3.6「每 24 小时最多一封」）。
/// 待发的就把正文换成最新一版，已发的就跳过。
pub fn recent_notice(
    conn: &Connection,
    player_id: &str,
    slug: &str,
    kind: &str,
    since: &str,
) -> rusqlite::Result<Option<(i64, String)>> {
    conn.query_row(
        "SELECT id, status FROM notifications
          WHERE player_id = ?1 AND target_slug = ?2 AND kind = ?3
            AND created_at >= ?4 AND status IN ('pending', 'sent')
          ORDER BY created_at DESC, id DESC LIMIT 1",
        params![player_id, slug, kind, since],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )
    .optional()
}

pub fn rewrite_pending_notification(
    conn: &Connection,
    id: i64,
    subject: &str,
    body: &str,
    url: Option<&str>,
) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE notifications SET subject = ?2, body = ?3, url = ?4
          WHERE id = ?1 AND status = 'pending'",
        params![id, subject, body, url],
    )?;
    Ok(())
}

pub struct NotificationRow {
    pub id: i64,
    pub player_id: String,
    pub kind: String,
    pub subject: String,
    pub body: String,
    pub url: Option<String>,
    pub channel: String,
    pub attempts: u32,
    /// 收件人当下的地址：入队时不抄一份，人换了邮箱队列里的旧信不该寄到旧地址。
    pub email: Option<String>,
    pub push_subscription: Option<String>,
    pub unsubscribe_token: Option<String>,
}

pub fn due_notifications(
    conn: &Connection,
    now: &str,
    limit: usize,
) -> rusqlite::Result<Vec<NotificationRow>> {
    let mut stmt = conn.prepare(
        "SELECT n.id, n.player_id, n.kind, n.subject, n.body, n.url, n.channel, n.attempts,
                p.email, p.push_subscription, p.unsubscribe_token
           FROM notifications n JOIN players p ON p.id = n.player_id
          WHERE n.status = 'pending' AND n.not_before <= ?1
          ORDER BY n.not_before, n.id
          LIMIT ?2",
    )?;
    let rows = stmt.query_map(params![now, limit as i64], |row| {
        Ok(NotificationRow {
            id: row.get(0)?,
            player_id: row.get(1)?,
            kind: row.get(2)?,
            subject: row.get(3)?,
            body: row.get(4)?,
            url: row.get(5)?,
            channel: row.get(6)?,
            attempts: row.get::<_, i64>(7)? as u32,
            email: row.get(8)?,
            push_subscription: row.get(9)?,
            unsubscribe_token: row.get(10)?,
        })
    })?;
    rows.collect()
}

pub fn mark_notification_sent(conn: &Connection, id: i64, at: &str) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE notifications SET status = 'sent', sent_at = ?2, attempts = attempts + 1
          WHERE id = ?1",
        params![id, at],
    )?;
    Ok(())
}

/// 失败一次：还有机会就退到 `not_before` 之后再试，机会用完进死信（DESIGN §4.10）。
pub fn mark_notification_failed(
    conn: &Connection,
    id: i64,
    status: &str,
    not_before: &str,
    error: &str,
) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE notifications SET status = ?2, not_before = ?3, last_error = ?4,
                                  attempts = attempts + 1
          WHERE id = ?1",
        params![id, status, not_before, error],
    )?;
    Ok(())
}

/// 运维看板那四个数（DESIGN §4.8「退订与投诉率进运维看板」的第一步）。
pub struct QueueCounts {
    pub pending: u32,
    pub sent_24h: u32,
    pub failed_24h: u32,
    pub dead: u32,
}

pub fn queue_counts(conn: &Connection, since: &str) -> rusqlite::Result<QueueCounts> {
    let one = |sql: &str, args: &[&dyn rusqlite::ToSql]| -> rusqlite::Result<u32> {
        conn.query_row(sql, args, |row| row.get::<_, i64>(0))
            .map(|n| n as u32)
    };
    Ok(QueueCounts {
        pending: one(
            "SELECT COUNT(*) FROM notifications WHERE status = 'pending'",
            &[],
        )?,
        sent_24h: one(
            "SELECT COUNT(*) FROM notifications WHERE status = 'sent' AND sent_at >= ?1",
            &[&since],
        )?,
        failed_24h: one(
            "SELECT COUNT(*) FROM notifications WHERE status = 'failed' AND created_at >= ?1",
            &[&since],
        )?,
        dead: one(
            "SELECT COUNT(*) FROM notifications WHERE status = 'dead'",
            &[],
        )?,
    })
}

// ---- 推广（DESIGN §3.11） ----

#[derive(Debug, Clone)]
pub struct BoostRow {
    pub id: i64,
    pub slug: String,
    pub kind: String,
    pub status: String,
    pub granted: bool,
    pub starts_at: String,
    pub ends_at: Option<String>,
    pub created_at: String,
    pub order_id: Option<String>,
    pub reason: Option<String>,
}

const BOOST_COLUMNS: &str =
    "id, slug, kind, status, granted, starts_at, ends_at, created_at, order_id, reason";

fn boost_from_row(row: &Row<'_>) -> rusqlite::Result<BoostRow> {
    Ok(BoostRow {
        id: row.get(0)?,
        slug: row.get(1)?,
        kind: row.get(2)?,
        status: row.get(3)?,
        granted: row.get::<_, i64>(4)? != 0,
        starts_at: row.get(5)?,
        ends_at: row.get(6)?,
        created_at: row.get(7)?,
        order_id: row.get(8)?,
        reason: row.get(9)?,
    })
}

#[allow(clippy::too_many_arguments)]
pub fn insert_boost(
    conn: &Connection,
    slug: &str,
    kind: &str,
    status: &str,
    granted: bool,
    starts_at: &str,
    ends_at: Option<&str>,
    created_at: &str,
) -> rusqlite::Result<i64> {
    conn.execute(
        "INSERT INTO boosts (slug, kind, status, granted, starts_at, ends_at, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            slug,
            kind,
            status,
            granted as i64,
            starts_at,
            ends_at,
            created_at
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn find_boost(conn: &Connection, id: i64) -> rusqlite::Result<Option<BoostRow>> {
    conn.query_row(
        &format!("SELECT {BOOST_COLUMNS} FROM boosts WHERE id = ?1"),
        params![id],
        boost_from_row,
    )
    .optional()
}

/// 全部推广，新的在前。运营者一屏看完，不分页。
pub fn list_boosts(conn: &Connection) -> rusqlite::Result<Vec<BoostRow>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {BOOST_COLUMNS} FROM boosts ORDER BY created_at DESC, id DESC"
    ))?;
    let rows = stmt.query_map([], boost_from_row)?;
    rows.collect()
}

pub fn update_boost(
    conn: &Connection,
    id: i64,
    status: &str,
    starts_at: Option<&str>,
    ends_at: Option<&str>,
    reason: Option<&str>,
) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE boosts SET status = ?2,
                           starts_at = COALESCE(?3, starts_at),
                           ends_at = COALESCE(?4, ends_at),
                           reason = COALESCE(?5, reason)
          WHERE id = ?1",
        params![id, status, starts_at, ends_at, reason],
    )?;
    Ok(())
}

/// 现在在广场推广位上的那几个，按上位时间排（DESIGN §3.11「按购买顺序、按日历顺延」）。
pub fn live_plaza_boosts(conn: &Connection) -> rusqlite::Result<Vec<BoostRow>> {
    live_boosts(conn, "<>")
}

/// 这一期周报里要带的那几个。周报位不占广场的位子，两边分开数。
pub fn live_digest_boosts(conn: &Connection) -> rusqlite::Result<Vec<BoostRow>> {
    live_boosts(conn, "=")
}

fn live_boosts(conn: &Connection, op: &str) -> rusqlite::Result<Vec<BoostRow>> {
    // `op` 是本模块里的字面量，不来自请求。
    let mut stmt = conn.prepare(&format!(
        "SELECT {BOOST_COLUMNS} FROM boosts
          WHERE status = 'live' AND kind {op} 'digest'
          ORDER BY starts_at, id"
    ))?;
    let rows = stmt.query_map([], boost_from_row)?;
    rows.collect()
}

/// 排队用：某段时间里有多少个占位的推广（`live` 的加还没到点的 `pending`）。
/// 周报那一项不占广场的位，不算在内。
pub fn boosts_overlapping(
    conn: &Connection,
    from: &str,
    to: &str,
) -> rusqlite::Result<Vec<BoostRow>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {BOOST_COLUMNS} FROM boosts
          WHERE status IN ('live', 'pending') AND kind <> 'digest'
            AND starts_at < ?2 AND IFNULL(ends_at, ?2) > ?1
          ORDER BY starts_at, id"
    ))?;
    let rows = stmt.query_map(params![from, to], boost_from_row)?;
    rows.collect()
}

/// 一个作品当下的推广：在位的优先，其次是排队中最早的那一条（`Listing.boost` 的语义）。
pub fn site_boost(conn: &Connection, slug: &str) -> rusqlite::Result<Option<BoostRow>> {
    conn.query_row(
        &format!(
            "SELECT {BOOST_COLUMNS} FROM boosts
              WHERE slug = ?1 AND status IN ('live', 'pending')
              ORDER BY CASE status WHEN 'live' THEN 0 ELSE 1 END, starts_at, id
              LIMIT 1"
        ),
        params![slug],
        boost_from_row,
    )
    .optional()
}

/// 作品被撤下或删掉时，它在位的推广一起结束（我们收了钱就要看过，撤下了就不该还在卖）。
pub fn end_boosts_of(conn: &Connection, slug: &str, at: &str) -> rusqlite::Result<usize> {
    conn.execute(
        "UPDATE boosts SET status = 'ended', ends_at = COALESCE(ends_at, ?2)
          WHERE slug = ?1 AND status IN ('live', 'pending')",
        params![slug, at],
    )
}

/// 到点上位、到点下位。返回有没有东西变过——变了要重写 `plaza.json`。
pub fn advance_boosts(conn: &Connection, now: &str) -> rusqlite::Result<bool> {
    let up = conn.execute(
        "UPDATE boosts SET status = 'live'
          WHERE status = 'pending' AND starts_at <= ?1 AND (ends_at IS NULL OR ends_at > ?1)",
        params![now],
    )?;
    let down = conn.execute(
        "UPDATE boosts SET status = 'ended'
          WHERE status IN ('live', 'pending') AND ends_at IS NOT NULL AND ends_at <= ?1",
        params![now],
    )?;
    Ok(up + down > 0)
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

    #[tokio::test]
    async fn readers_see_what_the_writer_wrote_and_refuse_to_write() {
        let dir = tempfile::tempdir().unwrap();
        let db = Db::open(&dir.path().join("api.sqlite")).unwrap();
        {
            let conn = db.lock().await;
            insert_user(
                &conn,
                &NewUser {
                    id: "u1",
                    kind: "anon",
                    display_name: "匿名开发者",
                    created_at: "2026-09-07T00:00:00Z",
                    expires_at: None,
                },
            )
            .unwrap();
        }
        // 写锁握着的时候读照样进行：这就是分开的意义。
        let _writing = db.lock().await;
        let reader = db.read().await;
        assert_eq!(count_live_sites(&reader, "u1").unwrap(), 0);
        let n: i64 = reader
            .query_row("SELECT COUNT(*) FROM users", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 1);
        // 拿读连接写：SQLite 那一层拒绝，而不是绕过写锁。
        let err = reader
            .execute("DELETE FROM users", [])
            .expect_err("query_only 的连接不能写");
        assert!(err.to_string().contains("readonly"), "{err}");
        // 四个读者都被占着，第五个排队而不是死锁。
        let a = db.read().await;
        let b = db.read().await;
        let c = db.read().await;
        drop(reader);
        let d = db.read().await;
        drop((a, b, c, d));
    }

    #[test]
    fn deleted_slug_stays_taken() {
        let db = Db::open_in_memory().unwrap();
        let conn = db.writer.blocking_lock();
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
