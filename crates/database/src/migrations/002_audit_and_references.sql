-- Migration 002: Audit logs and External references

CREATE TABLE IF NOT EXISTS audit_logs (
    id TEXT PRIMARY KEY,
    timestamp TEXT NOT NULL,
    actor TEXT NOT NULL,                -- 'desktop', 'cli', 'mcp', 'system'
    operation TEXT NOT NULL,            -- 'asset.delete', 'entity.create', etc.
    target_type TEXT NOT NULL,          -- 'asset', 'entity', 'memory', 'relation', 'collection'
    target_id TEXT NOT NULL,
    result TEXT NOT NULL,               -- 'success', 'failure'
    details_json TEXT NOT NULL DEFAULT '{}'
);

CREATE INDEX IF NOT EXISTS idx_audit_timestamp ON audit_logs(timestamp);
CREATE INDEX IF NOT EXISTS idx_audit_actor ON audit_logs(actor);
CREATE INDEX IF NOT EXISTS idx_audit_operation ON audit_logs(operation);

CREATE TABLE IF NOT EXISTS external_references (
    id TEXT PRIMARY KEY,
    entity_id TEXT,
    provider TEXT NOT NULL,             -- 'github', 'bilibili', 'website', 'steam', 'docs', 'other'
    title TEXT NOT NULL,
    url TEXT NOT NULL,
    description TEXT,
    metadata_json TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (entity_id) REFERENCES entities(id) ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS idx_ext_refs_entity ON external_references(entity_id);
CREATE INDEX IF NOT EXISTS idx_ext_refs_provider ON external_references(provider);
