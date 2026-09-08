-- 广场（DESIGN §3.8）：一个作品要不要出现在根域那一页上，以及卡片上要显示的东西。
-- 都挂在 sites 上：广场展示的永远是「这个作品的最新版本」，不是某一版。

-- 开发者勾了「放到广场上」。默认不公开。
ALTER TABLE sites ADD COLUMN public INTEGER NOT NULL DEFAULT 0;
-- 「正在找人测」，只在 public 时有意义。
ALTER TABLE sites ADD COLUMN seeking INTEGER NOT NULL DEFAULT 0;
-- 想让来的人重点看什么。
ALTER TABLE sites ADD COLUMN seek_note TEXT;
-- 下面四列是最新版本清单里的东西的副本，省得整理广场时读一遍对象存储。
ALTER TABLE sites ADD COLUMN summary TEXT;
ALTER TABLE sites ADD COLUMN cover_hash TEXT;
ALTER TABLE sites ADD COLUMN cover_mime TEXT;
ALTER TABLE sites ADD COLUMN engine TEXT;
-- 最近一次提交版本的时间。广场默认按它排。
ALTER TABLE sites ADD COLUMN updated_at TEXT;
-- 被举报到阈值、或我们手工撤下：public 仍是开发者的意愿，但广场上不出现。
ALTER TABLE sites ADD COLUMN hidden_at TEXT;
ALTER TABLE sites ADD COLUMN hidden_reason TEXT;
-- 人工复核并恢复的时间。之后自动撤下只数这个时间之后的举报，否则一恢复就被同一批举报再撤一次。
ALTER TABLE sites ADD COLUMN reviewed_at TEXT;

CREATE INDEX sites_public ON sites(public, deleted_at);
