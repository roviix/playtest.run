-- 俱乐部这一版（DESIGN 2026-09-09 按方向 B 重写）在控制面这一侧要的全部存储。
-- 每一样的出处写在它自己上面；新增的表对应 DESIGN §4.5「新增的表」那一段。

-- 开发者头像（DESIGN §3.9 卡片：「一张脸比一个 ID 更像真人，是压审核面的一层」）。
-- GitHub 登录时从用户 API 抄一份下来，匿名开发者没有。
ALTER TABLE users ADD COLUMN avatar_url TEXT;

-- 想找几位试玩者（DESIGN §3.3 第 4 条「某某在找 10 位试玩者 · 已有 6 位加入」）。NULL = 没设。
ALTER TABLE sites ADD COLUMN seats INTEGER;
-- 开发者的群（DESIGN §3.3 第 6 条）。去哪是他的事，我们对去向不承诺。
ALTER TABLE sites ADD COLUMN community_url TEXT;
-- 「让玩家看到彼此的反馈」（DESIGN §3.5），每个作品一个开关，默认关。
ALTER TABLE sites ADD COLUMN feedback_public INTEGER NOT NULL DEFAULT 0;

-- 点「开始」时留的名字（DESIGN §3.3 第 5 条）。它进的是这个作品的点名册，不进任何账号。
ALTER TABLE sessions ADD COLUMN name TEXT;

-- 开发者把某一条单独藏起来（DESIGN §3.5「开发者可以隐藏任何一条」）。
-- 一条反馈是否公开 = 作品的 feedback_public 为真、且这一列为 0。
ALTER TABLE feedback ADD COLUMN hidden INTEGER NOT NULL DEFAULT 0;

-- 玩家（DESIGN §3.6「身份从行为里长出来」）：没有密码、没有注册页、没有「创建账号」。
-- 一行就是一个愿意被叫回来的人：留了邮箱、或者允许了浏览器通知，两样都可以只有一样。
--   me_token_hash          根域那把钥匙（pt_me）的哈希，明文只在确认那一次交出去
--   unsubscribe_token 每封信底部的一键退订，一个人一个长期令牌
--   push_endpoint          从 push_subscription 里抄出来的那一条，只为了能建唯一索引找回同一个浏览器
CREATE TABLE IF NOT EXISTS players (
  id TEXT PRIMARY KEY,
  email TEXT,
  email_verified_at TEXT,
  push_subscription TEXT,
  push_endpoint TEXT,
  me_token_hash TEXT,
  unsubscribe_token TEXT,
  unsubscribed_at TEXT,
  created_at TEXT NOT NULL
);
CREATE UNIQUE INDEX players_by_email ON players(email) WHERE email IS NOT NULL;
CREATE UNIQUE INDEX players_by_push ON players(push_endpoint) WHERE push_endpoint IS NOT NULL;
CREATE UNIQUE INDEX players_by_me_token ON players(me_token_hash) WHERE me_token_hash IS NOT NULL;
CREATE UNIQUE INDEX players_by_unsub_token ON players(unsubscribe_token) WHERE unsubscribe_token IS NOT NULL;

-- 一次性令牌（DESIGN §3.6 的双重确认与「换一台设备就是再点一次确认信里的链接」）。
-- 关注对象挂在令牌上而不是先写进 follows：没点确认之前，这个关注一秒都不算数，
-- 否则任何人填别人的邮箱就能替他加一条关注。
CREATE TABLE IF NOT EXISTS follow_tokens (
  token_hash TEXT PRIMARY KEY,
  player_id TEXT NOT NULL,
  purpose TEXT NOT NULL,        -- confirm / send_link
  target_kind TEXT,             -- site / plaza；send_link 没有对象
  target_slug TEXT,
  source TEXT,
  created_at TEXT NOT NULL,
  expires_at TEXT NOT NULL,
  used_at TEXT
);
CREATE INDEX follow_tokens_by_player ON follow_tokens(player_id, created_at);

-- 关注：玩家 → 一个作品，或广场本身（DESIGN §3.6「先按作品和广场两级走」）。
CREATE TABLE IF NOT EXISTS follows (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  player_id TEXT NOT NULL,
  target_kind TEXT NOT NULL,    -- site / plaza
  target_slug TEXT,             -- 广场那一项是 NULL
  source TEXT,                  -- gate / sdk / plaza / me，只用来看哪个入口有效
  created_at TEXT NOT NULL
);
-- SQLite 里两个 NULL 互不相等，广场那一项会被判成永远不重复；用表达式索引把 NULL 折成空串。
CREATE UNIQUE INDEX follows_unique ON follows(player_id, target_kind, IFNULL(target_slug, ''));
CREATE INDEX follows_by_target ON follows(target_kind, target_slug);

-- 待发队列（DESIGN §4.10）。「每个作品每 24 小时最多一封」在入队时按这张表判；
-- 失败退避与死信也在这里，运维看板读的就是它。
CREATE TABLE IF NOT EXISTS notifications (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  player_id TEXT NOT NULL,
  kind TEXT NOT NULL,           -- confirm / site_version / digest / send_link
  target_slug TEXT,
  subject TEXT NOT NULL,
  body TEXT NOT NULL,
  url TEXT,
  channel TEXT NOT NULL,        -- email / push
  status TEXT NOT NULL,         -- pending / sent / failed / dead
  attempts INTEGER NOT NULL DEFAULT 0,
  not_before TEXT NOT NULL,
  created_at TEXT NOT NULL,
  sent_at TEXT,
  last_error TEXT
);
CREATE INDEX notifications_due ON notifications(status, not_before);
CREATE INDEX notifications_recent ON notifications(player_id, target_slug, kind, created_at);

-- 推广位（DESIGN §3.11）。买或被赠送一个时间窗，窗内出现在广场顶部并永远标「推广」。
-- 下单接口等境外主体与支付（DESIGN §9），所以现在只有管理接口往这里写。
CREATE TABLE IF NOT EXISTS boosts (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  slug TEXT NOT NULL,
  kind TEXT NOT NULL,           -- days3 / days7 / digest
  status TEXT NOT NULL,         -- pending / live / ended / rejected
  granted INTEGER NOT NULL DEFAULT 0,
  starts_at TEXT NOT NULL,
  ends_at TEXT,
  created_at TEXT NOT NULL,
  order_id TEXT,
  reason TEXT
);
CREATE INDEX boosts_by_slug ON boosts(slug, status);
CREATE INDEX boosts_by_window ON boosts(status, starts_at);
