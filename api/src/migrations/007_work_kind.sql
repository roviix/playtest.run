-- 一件作品的主形态。旧作品都是网页；空作品可以在第一次提交时改成文章或视频。
-- 后续版本由控制面阻止跨形态覆盖，避免关注者点同一个链接却收到完全不同的东西。
ALTER TABLE sites ADD COLUMN work_kind TEXT NOT NULL DEFAULT 'web'
  CHECK(work_kind IN ('web', 'article', 'video'));
