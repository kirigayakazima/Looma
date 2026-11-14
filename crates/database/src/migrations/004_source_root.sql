-- 004_source_root.sql
-- SourceRoot and Reference Lifecycle management (v0.4)

CREATE TABLE IF NOT EXISTS source_roots (
  id TEXT PRIMARY KEY,
  label TEXT NOT NULL,
  path TEXT NOT NULL,
  source_kind TEXT NOT NULL DEFAULT 'local',
  enabled INTEGER NOT NULL DEFAULT 1,
  state TEXT NOT NULL DEFAULT 'unverified',
  last_successful_scan_at TEXT,
  last_seen_at TEXT,
  settings_json TEXT NOT NULL DEFAULT '{}'
);

CREATE TABLE IF NOT EXISTS asset_lifecycle (
  asset_id TEXT PRIMARY KEY REFERENCES assets(id) ON DELETE CASCADE,
  root_id TEXT REFERENCES source_roots(id) ON DELETE SET NULL,
  relative_path TEXT,
  availability TEXT NOT NULL DEFAULT 'unchecked',
  visibility TEXT NOT NULL DEFAULT 'active',
  tracking TEXT NOT NULL DEFAULT 'tracked',
  last_verified_at TEXT,
  untracked_at TEXT,
  untracked_reason TEXT
);

CREATE INDEX IF NOT EXISTS idx_asset_lifecycle_root_rel ON asset_lifecycle(root_id, relative_path);
CREATE INDEX IF NOT EXISTS idx_asset_lifecycle_state ON asset_lifecycle(availability, visibility, tracking);

CREATE TABLE IF NOT EXISTS asset_scan_suppressions (
  id TEXT PRIMARY KEY,
  root_id TEXT NOT NULL REFERENCES source_roots(id) ON DELETE CASCADE,
  relative_path TEXT NOT NULL,
  reason TEXT,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  UNIQUE(root_id, relative_path)
);
