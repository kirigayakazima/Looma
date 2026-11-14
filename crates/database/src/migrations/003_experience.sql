-- Migration 003: Personal Experience and Footprints

CREATE TABLE IF NOT EXISTS experience_entries (
    id TEXT PRIMARY KEY,
    subject_type TEXT NOT NULL CHECK(subject_type IN ('entity')),
    subject_id TEXT NOT NULL REFERENCES entities(id) ON DELETE RESTRICT,
    event_type TEXT NOT NULL,
    title TEXT,
    note TEXT,
    occurred_value TEXT,
    occurred_precision TEXT NOT NULL CHECK(occurred_precision IN ('unknown','year','month','day','instant')),
    context_json TEXT NOT NULL DEFAULT '{}',
    visibility TEXT NOT NULL DEFAULT 'normal' CHECK(visibility IN ('normal','archived')),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK ((occurred_precision='unknown' AND occurred_value IS NULL) OR
           (occurred_precision<>'unknown' AND occurred_value IS NOT NULL))
);

CREATE INDEX IF NOT EXISTS idx_experience_subject ON experience_entries(subject_type, subject_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_experience_event ON experience_entries(event_type, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_experience_occurred ON experience_entries(occurred_precision, occurred_value);

CREATE TABLE IF NOT EXISTS experience_links (
    experience_id TEXT NOT NULL REFERENCES experience_entries(id) ON DELETE CASCADE,
    linked_type TEXT NOT NULL CHECK(linked_type IN ('asset','memory','external_reference')),
    linked_id TEXT NOT NULL,
    relation_type TEXT NOT NULL DEFAULT 'evidence',
    position INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY(experience_id, linked_type, linked_id)
);

CREATE INDEX IF NOT EXISTS idx_experience_link_target ON experience_links(linked_type, linked_id);

CREATE TABLE IF NOT EXISTS experience_request_ids (
    actor_scope TEXT NOT NULL,
    request_id TEXT NOT NULL,
    experience_id TEXT NOT NULL REFERENCES experience_entries(id) ON DELETE CASCADE,
    PRIMARY KEY(actor_scope, request_id)
);
