-- GitHub 登录（DESIGN §3.2）。users.kind = 'github' 的行多两列：GitHub 的数字 id（认人靠它，
-- 用户名可以改）和用户名（门禁页、广场上显示 @login 用）。匿名用户这两列是 NULL。

ALTER TABLE users ADD COLUMN github_id INTEGER;
ALTER TABLE users ADD COLUMN login TEXT;

CREATE UNIQUE INDEX users_by_github_id ON users(github_id) WHERE github_id IS NOT NULL;
