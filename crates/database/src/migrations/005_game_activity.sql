-- Migration 005: Game Activity Records (Cross-Platform Playtime Tracking)
CREATE TABLE IF NOT EXISTS game_activity_records (
    id TEXT PRIMARY KEY,
    work_id TEXT NOT NULL REFERENCES entities(id) ON DELETE CASCADE,

    -- Source provider and account/profile identifier
    provider TEXT NOT NULL,           -- 'switch_nx' | 'steam' | 'pc_observer'
    source_key TEXT NOT NULL,         -- Switch users[].id / steam:steamid64 / pc:local
    external_id TEXT NOT NULL,        -- Switch Title ID (16-char hex) / Steam AppID
    source_label TEXT,                -- 'Switch (xuaner)' / 'Steam'
    title_hint TEXT,                  -- Original game title reported by source
    
    playtime_seconds INTEGER NOT NULL CHECK (playtime_seconds >= 0),
    peak_seen_seconds INTEGER NOT NULL CHECK (peak_seen_seconds >= 0),
    launch_count INTEGER CHECK (launch_count IS NULL OR launch_count >= 0),
    first_played_at TEXT,             -- RFC3339 string
    last_played_at TEXT,              -- RFC3339 string
    source_exported_at INTEGER,       -- Unix timestamp (seconds)
    last_import_sha256 TEXT,          -- SHA-256 of imported source file
    updated_at TEXT NOT NULL,

    CHECK (length(provider) > 0 AND length(source_key) > 0 AND length(external_id) > 0),
    UNIQUE(provider, source_key, external_id)
);

CREATE INDEX IF NOT EXISTS idx_game_activity_work ON game_activity_records(work_id);
CREATE INDEX IF NOT EXISTS idx_game_activity_provider ON game_activity_records(provider);
