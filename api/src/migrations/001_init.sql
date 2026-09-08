-- v0.1 的全部表。时间一律是 UTC、秒精度的 RFC 3339 文本（见 clock.rs）。
-- SQL 只用 SQLite 和 Postgres 都有的子集：换库时改的是连接层，不是这些语句（DESIGN §4.5）。

CREATE TABLE users (
    id            TEXT PRIMARY KEY,
    -- anon：匿名 24 小时链接；github：登录用户。v0.1 只签发 anon。
    kind          TEXT NOT NULL CHECK (kind IN ('anon', 'github')),
    display_name  TEXT NOT NULL,
    created_at    TEXT NOT NULL,
    -- 只有匿名用户有到期时间，到点连人带作品一起失效。
    expires_at    TEXT
);

-- 只存 sha256(token)：库被人拿走也不能拿去冒充谁。
CREATE TABLE tokens (
    token_hash  TEXT PRIMARY KEY,
    user_id     TEXT NOT NULL REFERENCES users(id),
    created_at  TEXT NOT NULL,
    expires_at  TEXT
);

CREATE INDEX tokens_by_user ON tokens(user_id);

CREATE TABLE sites (
    slug             TEXT PRIMARY KEY,
    user_id          TEXT NOT NULL REFERENCES users(id),
    title            TEXT NOT NULL,
    created_at       TEXT NOT NULL,
    expires_at       TEXT,
    -- 还没上传过版本时是 NULL。真正决定玩家看到哪一版的是对象存储里的 current.json，
    -- 这一列是控制台和「下一版是第几版」用的。
    current_version  INTEGER,
    -- 软删除：链接立刻失效，但记录留着，方便查滥用举报。
    deleted_at       TEXT
);

CREATE INDEX sites_by_user ON sites(user_id);

CREATE TABLE versions (
    slug         TEXT NOT NULL,
    version      INTEGER NOT NULL,
    created_at   TEXT NOT NULL,
    note         TEXT,
    file_count   INTEGER NOT NULL,
    total_bytes  INTEGER NOT NULL,
    PRIMARY KEY (slug, version)
);

-- 一次「问缺哪些哈希」的记录。request_json 是整个 PrepareUploadRequest，
-- 提交时按它组装清单，CLI 不用把文件列表再发一遍。
CREATE TABLE uploads (
    id            TEXT PRIMARY KEY,
    slug          TEXT NOT NULL,
    user_id       TEXT NOT NULL,
    created_at    TEXT NOT NULL,
    request_json  TEXT NOT NULL,
    status        TEXT NOT NULL CHECK (status IN ('pending', 'committed'))
);

CREATE INDEX uploads_by_site ON uploads(slug);

-- 对象存储里已经有哪些内容。真身在 store 的 blobs/ 下，这里只是一张能被 SQL 查的索引，
-- 所以凡是要判断「这个哈希在不在」的地方都会再问一次文件系统。
CREATE TABLE blobs (
    hash        TEXT PRIMARY KEY,
    size        INTEGER NOT NULL,
    created_at  TEXT NOT NULL
);
