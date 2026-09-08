-- 会话：一次打开。id 由边缘的门禁页生成（SESSION_COOKIE），SDK 与边缘事件都挂在它下面。
CREATE TABLE IF NOT EXISTS sessions (
  id TEXT PRIMARY KEY,
  slug TEXT NOT NULL,
  version INTEGER NOT NULL,
  first_seen_at TEXT NOT NULL,
  last_seen_at TEXT NOT NULL,
  ua TEXT,
  device TEXT,            -- phone / tablet / desktop（从 UA 粗分）
  browser TEXT,           -- chrome / safari / firefox / wechat / other
  os TEXT,                -- ios / android / windows / macos / linux / other
  referrer_kind TEXT,     -- wechat / discord / direct / other（尽力而为）
  wechat INTEGER NOT NULL DEFAULT 0,
  gate_view_at TEXT,
  start_at TEXT,
  first_frame_at TEXT,    -- SDK 报的首帧 / 首次可交互
  load_ms INTEGER,
  last_input_at TEXT,
  is_return INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS sessions_slug_version ON sessions(slug, version, first_seen_at);

-- 事件：边缘的 gate_view / start / html_view / report / breaker_trip / resource_fail，SDK 的 error / load / event / input。
CREATE TABLE IF NOT EXISTS session_events (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  session_id TEXT NOT NULL,
  slug TEXT NOT NULL,
  version INTEGER NOT NULL,
  ts TEXT NOT NULL,
  source TEXT NOT NULL,   -- edge / sdk
  kind TEXT NOT NULL,     -- 见上
  name TEXT,              -- 自定义事件名 / 错误 fingerprint
  data TEXT               -- JSON
);
CREATE INDEX IF NOT EXISTS session_events_lookup ON session_events(slug, version, kind, ts);
CREATE INDEX IF NOT EXISTS session_events_session ON session_events(session_id, ts);

-- 反馈：一句话 + 上下文。截图 v0.2 再说，先留列。
CREATE TABLE IF NOT EXISTS feedback (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  session_id TEXT NOT NULL,
  slug TEXT NOT NULL,
  version INTEGER NOT NULL,
  ts TEXT NOT NULL,
  text TEXT NOT NULL,
  seconds_in INTEGER,
  device TEXT,
  browser TEXT,
  screenshot_hash TEXT,
  status TEXT NOT NULL DEFAULT 'new'   -- new / seen / done
);
CREATE INDEX IF NOT EXISTS feedback_lookup ON feedback(slug, version, ts);
