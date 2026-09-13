ALTER TABLE players ADD COLUMN user_id TEXT REFERENCES users(id);
CREATE UNIQUE INDEX players_by_user ON players(user_id) WHERE user_id IS NOT NULL;

CREATE TABLE browser_sessions (
    token_hash TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id),
    created_at TEXT NOT NULL,
    expires_at TEXT NOT NULL
);
CREATE INDEX browser_sessions_by_user ON browser_sessions(user_id);

CREATE TABLE account_links (
    token_hash TEXT PRIMARY KEY,
    player_id TEXT NOT NULL REFERENCES players(id),
    link_user_id TEXT REFERENCES users(id),
    return_to TEXT NOT NULL,
    expires_at TEXT NOT NULL
);

CREATE TABLE browser_oauth (
    state_hash TEXT PRIMARY KEY,
    browser_hash TEXT NOT NULL,
    link_user_id TEXT REFERENCES users(id),
    return_to TEXT NOT NULL,
    expires_at TEXT NOT NULL
);

CREATE TABLE device_authorizations (
    device_hash TEXT PRIMARY KEY,
    user_code_hash TEXT NOT NULL UNIQUE,
    user_id TEXT REFERENCES users(id),
    anon_user_id TEXT REFERENCES users(id),
    created_at TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    last_poll_at TEXT
);
