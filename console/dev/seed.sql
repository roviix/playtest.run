-- 本机看控制台用的一组假数据。真机记录见 docs/spikes/2026-09-07-console-results.md。
--
--   cargo run -p playtest-api            # 先让它把迁移跑完，再 Ctrl-C
--   sqlite3 .data/console/api.sqlite < console/dev/seed.sql
--   PLAYTEST_API_LISTEN=127.0.0.1:48787 PLAYTEST_DATA_DIR=.data/console cargo run -p playtest-api
--   PLAYTEST_API=http://127.0.0.1:48787 pnpm --dir console dev
--
-- 控制台里粘这个令牌：console-dev-token
-- （库里存的是它的 sha256。这是一个只在本机用的固定值，别拿到任何真机上。）
--
-- 数字是照 DESIGN §3.4 的示例摆的：v7 那段话应该正好是
-- 「8 个人打开，6 个人进到游戏（2 个在加载时走了）… 1 个错误撞了 3 次… 1 条反馈。」
-- v5 故意没有 SDK，用来看「不知道」那条路长什么样。

DELETE FROM feedback WHERE slug IN ('brisk-otter-41', 'sage-lynx-30');
DELETE FROM session_events WHERE slug IN ('brisk-otter-41', 'sage-lynx-30');
DELETE FROM sessions WHERE slug IN ('brisk-otter-41', 'sage-lynx-30');
DELETE FROM versions WHERE slug IN ('brisk-otter-41', 'sage-lynx-30');
DELETE FROM sites WHERE slug IN ('brisk-otter-41', 'sage-lynx-30');
DELETE FROM tokens WHERE user_id = 'dev-user';
DELETE FROM users WHERE id = 'dev-user';

INSERT INTO users (id, kind, display_name, created_at, expires_at)
VALUES ('dev-user', 'anon', '匿名开发者', '2026-09-01T09:00:00Z', '2027-01-01T00:00:00Z');

INSERT INTO tokens (token_hash, user_id, created_at, expires_at)
VALUES ('14052dec726b93e56404b6fb06e67f38be021ed0f111fa90068a0eecc73825ef',
        'dev-user', '2026-09-01T09:00:00Z', '2027-01-01T00:00:00Z');

INSERT INTO sites (slug, user_id, title, created_at, expires_at, current_version, deleted_at) VALUES
  ('brisk-otter-41', 'dev-user', '小球大冒险', '2026-09-01T09:10:00Z', '2027-01-01T00:00:00Z', 7, NULL),
  ('sage-lynx-30',   'dev-user', '跳跳方块',   '2026-09-06T20:00:00Z', '2027-01-01T00:00:00Z', 1, NULL);

INSERT INTO versions (slug, version, created_at, note, file_count, total_bytes) VALUES
  ('brisk-otter-41', 5, '2026-09-01T09:12:00Z', '第一版，能跑了',   12, 4200000),
  ('brisk-otter-41', 6, '2026-09-03T16:40:00Z', '加了音效',         13, 5100000),
  ('brisk-otter-41', 7, '2026-09-05T14:20:00Z', '改了新手引导',     13, 5120000),
  ('sage-lynx-30',   1, '2026-09-06T20:05:00Z', '先发给两个人看看', 6,  380000);

-- v5：没接 playtest.js。四个人点了开始，之后发生了什么我们不知道。
INSERT INTO sessions (id, slug, version, first_seen_at, last_seen_at, ua, device, browser, os,
                      referrer_kind, wechat, gate_view_at, start_at, first_frame_at, load_ms,
                      last_input_at, is_return) VALUES
  ('v5a', 'brisk-otter-41', 5, '2026-09-01T10:00:00Z', '2026-09-01T10:03:20Z', NULL, 'desktop', 'chrome',  'macos',   'direct',  0, '2026-09-01T10:00:00Z', '2026-09-01T10:00:06Z', NULL, NULL, NULL, 0),
  ('v5b', 'brisk-otter-41', 5, '2026-09-01T10:31:00Z', '2026-09-01T10:31:40Z', NULL, 'phone',   'wechat',  'android', 'wechat',  1, '2026-09-01T10:31:00Z', '2026-09-01T10:31:05Z', NULL, NULL, NULL, 0),
  ('v5c', 'brisk-otter-41', 5, '2026-09-01T11:02:00Z', '2026-09-01T11:02:12Z', NULL, 'phone',   'safari',  'ios',     'wechat',  1, '2026-09-01T11:02:00Z', '2026-09-01T11:02:04Z', NULL, NULL, NULL, 0),
  ('v5d', 'brisk-otter-41', 5, '2026-09-01T14:15:00Z', '2026-09-01T14:22:30Z', NULL, 'desktop', 'firefox', 'windows', 'discord', 0, '2026-09-01T14:15:00Z', '2026-09-01T14:15:09Z', NULL, NULL, NULL, 0);

-- v6：接上了 SDK，但音效那一版有两次资源没下下来。
INSERT INTO sessions (id, slug, version, first_seen_at, last_seen_at, ua, device, browser, os,
                      referrer_kind, wechat, gate_view_at, start_at, first_frame_at, load_ms,
                      last_input_at, is_return) VALUES
  ('v6a', 'brisk-otter-41', 6, '2026-09-03T17:00:00Z', '2026-09-03T17:01:00Z', NULL, 'desktop', 'chrome', 'macos',   'direct', 0, '2026-09-03T17:00:00Z', '2026-09-03T17:00:05Z', '2026-09-03T17:00:11Z', 6200, '2026-09-03T17:00:55Z', 0),
  ('v6b', 'brisk-otter-41', 6, '2026-09-03T17:20:00Z', '2026-09-03T17:26:40Z', NULL, 'phone',   'safari', 'ios',     'wechat', 1, '2026-09-03T17:20:00Z', '2026-09-03T17:20:04Z', '2026-09-03T17:20:19Z', 15000, '2026-09-03T17:26:10Z', 0),
  ('v6c', 'brisk-otter-41', 6, '2026-09-03T18:05:00Z', '2026-09-03T18:05:25Z', NULL, 'tablet',  'safari', 'ios',     'direct', 0, '2026-09-03T18:05:00Z', '2026-09-03T18:05:03Z', '2026-09-03T18:05:10Z', 7100, NULL, 0),
  ('v6d', 'brisk-otter-41', 6, '2026-09-03T19:40:00Z', '2026-09-03T19:40:09Z', NULL, 'phone',   'wechat', 'android', 'wechat', 1, '2026-09-03T19:40:00Z', '2026-09-03T19:40:04Z', NULL, NULL, NULL, 0),
  ('v6e', 'brisk-otter-41', 6, '2026-09-03T21:12:00Z', '2026-09-03T21:12:06Z', NULL, 'phone',   'chrome', 'android', 'direct', 0, '2026-09-03T21:12:00Z', '2026-09-03T21:12:03Z', NULL, NULL, NULL, 0);

-- v7：DESIGN §3.4 里的那一版。
INSERT INTO sessions (id, slug, version, first_seen_at, last_seen_at, ua, device, browser, os,
                      referrer_kind, wechat, gate_view_at, start_at, first_frame_at, load_ms,
                      last_input_at, is_return) VALUES
  ('v7a', 'brisk-otter-41', 7, '2026-09-05T14:00:00Z', '2026-09-05T14:00:45Z', NULL, 'desktop', 'chrome',  'macos',   'direct',  0, '2026-09-05T14:00:00Z', '2026-09-05T14:00:05Z', '2026-09-05T14:00:09Z', 4300,  '2026-09-05T14:00:40Z', 0),
  ('v7b', 'brisk-otter-41', 7, '2026-09-05T14:02:10Z', '2026-09-05T14:14:10Z', NULL, 'phone',   'safari',  'ios',     'wechat',  1, '2026-09-05T14:02:10Z', '2026-09-05T14:02:15Z', '2026-09-05T14:02:24Z', 9100,  '2026-09-05T14:13:40Z', 0),
  ('v7c', 'brisk-otter-41', 7, '2026-09-05T14:05:00Z', '2026-09-05T14:16:20Z', NULL, 'phone',   'chrome',  'android', 'discord', 0, '2026-09-05T14:05:00Z', '2026-09-05T14:05:04Z', '2026-09-05T14:05:12Z', 8000,  '2026-09-05T14:16:00Z', 1),
  ('v7d', 'brisk-otter-41', 7, '2026-09-05T14:20:00Z', '2026-09-05T14:31:00Z', NULL, 'desktop', 'firefox', 'windows', 'direct',  0, '2026-09-05T14:20:00Z', '2026-09-05T14:20:03Z', '2026-09-05T14:20:07Z', 3900,  '2026-09-05T14:30:20Z', 1),
  ('v7e', 'brisk-otter-41', 7, '2026-09-05T14:22:30Z', '2026-09-05T14:24:10Z', NULL, 'desktop', 'chrome',  'windows', 'direct',  0, '2026-09-05T14:22:30Z', '2026-09-05T14:22:34Z', '2026-09-05T14:22:39Z', 4600,  '2026-09-05T14:23:50Z', 0),
  ('v7f', 'brisk-otter-41', 7, '2026-09-05T14:25:00Z', '2026-09-05T14:25:30Z', NULL, 'tablet',  'safari',  'ios',     'direct',  0, '2026-09-05T14:25:00Z', '2026-09-05T14:25:04Z', '2026-09-05T14:25:13Z', 8800,  '2026-09-05T14:25:26Z', 0),
  ('v7g', 'brisk-otter-41', 7, '2026-09-05T14:28:00Z', '2026-09-05T14:28:11Z', NULL, 'phone',   'wechat',  'android', 'wechat',  1, '2026-09-05T14:28:00Z', '2026-09-05T14:28:04Z', NULL, NULL, NULL, 0),
  ('v7h', 'brisk-otter-41', 7, '2026-09-05T14:35:00Z', '2026-09-05T14:35:07Z', NULL, 'phone',   'chrome',  'android', 'direct',  0, '2026-09-05T14:35:00Z', '2026-09-05T14:35:03Z', NULL, NULL, NULL, 0);

INSERT INTO sessions (id, slug, version, first_seen_at, last_seen_at, ua, device, browser, os,
                      referrer_kind, wechat, gate_view_at, start_at, first_frame_at, load_ms,
                      last_input_at, is_return) VALUES
  ('lynx1', 'sage-lynx-30', 1, '2026-09-06T20:30:00Z', '2026-09-06T20:33:10Z', NULL, 'desktop', 'chrome', 'macos',   'direct', 0, '2026-09-06T20:30:00Z', '2026-09-06T20:30:04Z', '2026-09-06T20:30:08Z', 3800, '2026-09-06T20:32:50Z', 0),
  ('lynx2', 'sage-lynx-30', 1, '2026-09-06T21:02:00Z', '2026-09-06T21:02:14Z', NULL, 'phone',   'wechat', 'android', 'wechat', 1, '2026-09-06T21:02:00Z', '2026-09-06T21:02:05Z', NULL, NULL, NULL, 0);

-- 边缘看得见的：看到门禁页、点了开始、资源没加载出来。
INSERT INTO session_events (session_id, slug, version, ts, source, kind, name, data) VALUES
  ('v5a', 'brisk-otter-41', 5, '2026-09-01T10:00:00Z', 'edge', 'gate_view', NULL, NULL),
  ('v5a', 'brisk-otter-41', 5, '2026-09-01T10:00:06Z', 'edge', 'start', NULL, NULL),
  ('v5b', 'brisk-otter-41', 5, '2026-09-01T10:31:00Z', 'edge', 'gate_view', NULL, NULL),
  ('v5b', 'brisk-otter-41', 5, '2026-09-01T10:31:05Z', 'edge', 'start', NULL, NULL),
  ('v5c', 'brisk-otter-41', 5, '2026-09-01T11:02:00Z', 'edge', 'gate_view', NULL, NULL),
  ('v5c', 'brisk-otter-41', 5, '2026-09-01T11:02:04Z', 'edge', 'start', NULL, NULL),
  ('v5d', 'brisk-otter-41', 5, '2026-09-01T14:15:00Z', 'edge', 'gate_view', NULL, NULL),
  ('v5d', 'brisk-otter-41', 5, '2026-09-01T14:15:09Z', 'edge', 'start', NULL, NULL),

  ('v6a', 'brisk-otter-41', 6, '2026-09-03T17:00:05Z', 'edge', 'start', NULL, NULL),
  ('v6a', 'brisk-otter-41', 6, '2026-09-03T17:00:11Z', 'sdk',  'load',  NULL, '{"ms":6200}'),
  ('v6b', 'brisk-otter-41', 6, '2026-09-03T17:20:04Z', 'edge', 'start', NULL, NULL),
  ('v6b', 'brisk-otter-41', 6, '2026-09-03T17:20:19Z', 'sdk',  'load',  NULL, '{"ms":15000}'),
  ('v6b', 'brisk-otter-41', 6, '2026-09-03T17:22:40Z', 'sdk',  'event', '第一关过了', '{"level":1}'),
  ('v6c', 'brisk-otter-41', 6, '2026-09-03T18:05:03Z', 'edge', 'start', NULL, NULL),
  ('v6c', 'brisk-otter-41', 6, '2026-09-03T18:05:10Z', 'sdk',  'load',  NULL, '{"ms":7100}'),
  ('v6c', 'brisk-otter-41', 6, '2026-09-03T18:05:21Z', 'sdk',  'error', 'TypeError: Cannot read ''x'' of undefined @ main.js:412', '{"stack":"main.js:412"}'),
  ('v6d', 'brisk-otter-41', 6, '2026-09-03T19:40:04Z', 'edge', 'start', NULL, NULL),
  ('v6d', 'brisk-otter-41', 6, '2026-09-03T19:40:08Z', 'edge', 'resource_fail', 'audio/bgm.ogg', '{"reason":"下到 43% 断了"}'),
  ('v6e', 'brisk-otter-41', 6, '2026-09-03T21:12:03Z', 'edge', 'start', NULL, NULL),
  ('v6e', 'brisk-otter-41', 6, '2026-09-03T21:12:05Z', 'edge', 'resource_fail', 'audio/bgm.ogg', '{"reason":"404"}'),
  ('v6a', 'brisk-otter-41', 6, '2026-09-03T17:00:41Z', 'sdk',  'error', 'TypeError: Cannot read ''x'' of undefined @ main.js:412', '{"stack":"main.js:412"}'),
  ('v6b', 'brisk-otter-41', 6, '2026-09-03T17:24:02Z', 'sdk',  'error', 'TypeError: Cannot read ''x'' of undefined @ main.js:412', '{"stack":"main.js:412"}'),

  ('v7a', 'brisk-otter-41', 7, '2026-09-05T14:00:05Z', 'edge', 'start', NULL, NULL),
  ('v7a', 'brisk-otter-41', 7, '2026-09-05T14:00:09Z', 'sdk',  'load',  NULL, '{"ms":4300}'),
  ('v7a', 'brisk-otter-41', 7, '2026-09-05T14:00:22Z', 'sdk',  'event', '过了引导', '{"step":3}'),
  ('v7a', 'brisk-otter-41', 7, '2026-09-05T14:00:40Z', 'sdk',  'input', NULL, NULL),
  ('v7b', 'brisk-otter-41', 7, '2026-09-05T14:02:15Z', 'edge', 'start', NULL, NULL),
  ('v7b', 'brisk-otter-41', 7, '2026-09-05T14:02:24Z', 'sdk',  'load',  NULL, '{"ms":9100}'),
  ('v7b', 'brisk-otter-41', 7, '2026-09-05T14:02:51Z', 'sdk',  'event', '过了引导', '{"step":3}'),
  ('v7b', 'brisk-otter-41', 7, '2026-09-05T14:08:30Z', 'sdk',  'event', '第一关过了', '{"level":1}'),
  ('v7b', 'brisk-otter-41', 7, '2026-09-05T14:13:40Z', 'sdk',  'input', NULL, NULL),
  ('v7c', 'brisk-otter-41', 7, '2026-09-05T14:05:04Z', 'edge', 'start', NULL, NULL),
  ('v7c', 'brisk-otter-41', 7, '2026-09-05T14:05:12Z', 'sdk',  'load',  NULL, '{"ms":8000}'),
  ('v7c', 'brisk-otter-41', 7, '2026-09-05T14:09:10Z', 'sdk',  'event', '第一关过了', '{"level":1}'),
  ('v7c', 'brisk-otter-41', 7, '2026-09-05T14:15:02Z', 'sdk',  'event', '第二关过了', '{"level":2}'),
  ('v7d', 'brisk-otter-41', 7, '2026-09-05T14:20:03Z', 'edge', 'start', NULL, NULL),
  ('v7d', 'brisk-otter-41', 7, '2026-09-05T14:20:07Z', 'sdk',  'load',  NULL, '{"ms":3900}'),
  ('v7d', 'brisk-otter-41', 7, '2026-09-05T14:24:44Z', 'sdk',  'event', '第一关过了', '{"level":1}'),
  ('v7d', 'brisk-otter-41', 7, '2026-09-05T14:30:20Z', 'sdk',  'input', NULL, NULL),
  ('v7e', 'brisk-otter-41', 7, '2026-09-05T14:22:34Z', 'edge', 'start', NULL, NULL),
  ('v7e', 'brisk-otter-41', 7, '2026-09-05T14:22:39Z', 'sdk',  'load',  NULL, '{"ms":4600}'),
  ('v7e', 'brisk-otter-41', 7, '2026-09-05T14:23:12Z', 'sdk',  'error', 'TypeError: Cannot read ''x'' of undefined @ main.js:412', '{"stack":"main.js:412 → update() → tick()"}'),
  ('v7e', 'brisk-otter-41', 7, '2026-09-05T14:23:50Z', 'sdk',  'error', 'TypeError: Cannot read ''x'' of undefined @ main.js:412', '{"stack":"main.js:412 → update() → tick()"}'),
  ('v7f', 'brisk-otter-41', 7, '2026-09-05T14:25:04Z', 'edge', 'start', NULL, NULL),
  ('v7f', 'brisk-otter-41', 7, '2026-09-05T14:25:13Z', 'sdk',  'load',  NULL, '{"ms":8800}'),
  ('v7f', 'brisk-otter-41', 7, '2026-09-05T14:25:26Z', 'sdk',  'error', 'TypeError: Cannot read ''x'' of undefined @ main.js:412', '{"stack":"main.js:412 → update() → tick()"}'),
  ('v7g', 'brisk-otter-41', 7, '2026-09-05T14:28:00Z', 'edge', 'gate_view', NULL, NULL),
  ('v7g', 'brisk-otter-41', 7, '2026-09-05T14:28:04Z', 'edge', 'start', NULL, NULL),
  ('v7h', 'brisk-otter-41', 7, '2026-09-05T14:35:00Z', 'edge', 'gate_view', NULL, NULL),
  ('v7h', 'brisk-otter-41', 7, '2026-09-05T14:35:03Z', 'edge', 'start', NULL, NULL),

  ('lynx1', 'sage-lynx-30', 1, '2026-09-06T20:30:04Z', 'edge', 'start', NULL, NULL),
  ('lynx1', 'sage-lynx-30', 1, '2026-09-06T20:30:08Z', 'sdk',  'load',  NULL, '{"ms":3800}'),
  ('lynx2', 'sage-lynx-30', 1, '2026-09-06T21:02:05Z', 'edge', 'start', NULL, NULL);

INSERT INTO feedback (session_id, slug, version, ts, text, seconds_in, device, browser, screenshot_hash, status) VALUES
  ('v7b', 'brisk-otter-41', 7, '2026-09-05T14:02:57Z', '不知道要按哪个键', 47, 'phone', 'safari', NULL, 'new'),
  ('v6b', 'brisk-otter-41', 6, '2026-09-03T17:22:00Z', '音效太大了，第一下吓一跳', 120, 'phone', 'safari', NULL, 'seen'),
  ('lynx2', 'sage-lynx-30', 1, '2026-09-06T21:02:12Z', '在微信里点开是白的', 7, 'phone', 'wechat', NULL, 'new');
