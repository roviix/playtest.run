CREATE TABLE collections (
  slug TEXT PRIMARY KEY,
  user_id TEXT NOT NULL REFERENCES users(id),
  title TEXT NOT NULL,
  summary TEXT NOT NULL DEFAULT '',
  kind TEXT NOT NULL CHECK(kind IN ('collection', 'challenge')),
  prompt TEXT NOT NULL DEFAULT '',
  rules TEXT NOT NULL DEFAULT '',
  closes_at TEXT,
  public INTEGER NOT NULL DEFAULT 0,
  hidden INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  deleted_at TEXT
);
CREATE INDEX collections_by_owner ON collections(user_id, deleted_at);
CREATE TABLE collection_entries (
  collection_slug TEXT NOT NULL REFERENCES collections(slug),
  site_slug TEXT NOT NULL REFERENCES sites(slug),
  user_id TEXT NOT NULL REFERENCES users(id),
  submitted_version INTEGER NOT NULL,
  submitted_at TEXT NOT NULL,
  note TEXT NOT NULL DEFAULT '',
  blocked INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY(collection_slug, site_slug)
);
CREATE INDEX collection_entries_by_site ON collection_entries(site_slug);
CREATE TABLE collection_digests (
  collection_slug TEXT NOT NULL,
  player_id TEXT NOT NULL,
  period TEXT NOT NULL,
  PRIMARY KEY(collection_slug, player_id, period)
);
