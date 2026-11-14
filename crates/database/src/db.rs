use std::path::{Path, PathBuf};
use std::sync::Arc;
use chrono::{DateTime, Utc};
use parking_lot::Mutex;
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::Value;

use looma_core::error::{LoomaError, LoomaResult};
use looma_core::models::*;
use looma_core::services::*;

use crate::migration::run_migrations;

#[derive(Clone)]
/// The unified SQLite database implementation for Looma.
///
/// # Concurrency Guarantee Boundary (v0.6 Specification):
/// - **Guaranteed**: In-process multi-threading across threads sharing the same `LoomaDb` instance
///   is serialized via `Arc<Mutex<Connection>>` and transaction boundaries.
/// - **Not Guaranteed / Out of Current Scope**: Concurrent multi-process database file access
///   by independent OS processes, concurrent external SQLite write processes, or distributed/NFS
///   filesystem access without external distributed lock coordination.
pub struct LoomaDb {
    conn: Arc<Mutex<Connection>>,
    vault_path: PathBuf,
    db_path: PathBuf,
}

impl LoomaDb {
    pub fn open(vault_path: impl AsRef<Path>) -> LoomaResult<Self> {
        let vault_path = vault_path.as_ref().to_path_buf();
        let looma_dir = vault_path.join(".looma");
        if !looma_dir.exists() {
            std::fs::create_dir_all(&looma_dir)
                .map_err(|e| LoomaError::Vault(format!("Failed to create .looma dir: {e}")))?;
        }

        let db_path = looma_dir.join("vault.db");
        let mut conn = Connection::open(&db_path)
            .map_err(|e| LoomaError::Database(format!("Failed to open database at {:?}: {e}", db_path)))?;

        // PRAGMAs for performance and data safety
        let _mode: String = conn
            .query_row("PRAGMA journal_mode = WAL", [], |r| r.get(0))
            .map_err(|e| LoomaError::Database(format!("Failed to set journal_mode: {e}")))?;
        conn.pragma_update(None, "foreign_keys", "ON")
            .map_err(|e| LoomaError::Database(format!("Failed to set foreign_keys: {e}")))?;
        conn.pragma_update(None, "synchronous", "NORMAL")
            .map_err(|e| LoomaError::Database(format!("Failed to set synchronous: {e}")))?;
        conn.busy_timeout(std::time::Duration::from_millis(5000))
            .map_err(|e| LoomaError::Database(format!("Failed to set busy_timeout: {e}")))?;

        // Run migrations
        run_migrations(&mut conn)?;

        // Ensure vault_meta has name and version
        let vault_name = vault_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Personal Vault");

        conn.execute(
            "INSERT OR IGNORE INTO vault_meta (key, value, updated_at) VALUES ('name', ?1, ?2)",
            params![vault_name, Utc::now().to_rfc3339()],
        )
        .map_err(|e| LoomaError::Database(format!("Failed to initialize vault_meta: {e}")))?;

        conn.execute(
            "INSERT OR IGNORE INTO vault_meta (key, value, updated_at) VALUES ('version', '0.2.0', ?1)",
            params![Utc::now().to_rfc3339()],
        )
        .map_err(|e| LoomaError::Database(format!("Failed to initialize vault_meta: {e}")))?;

        conn.execute(
            "INSERT OR IGNORE INTO vault_meta (key, value, updated_at) VALUES ('created_at', ?1, ?1)",
            params![Utc::now().to_rfc3339()],
        )
        .map_err(|e| LoomaError::Database(format!("Failed to initialize vault_meta: {e}")))?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            vault_path,
            db_path,
        })
    }

    pub fn in_memory() -> LoomaResult<Self> {
        let mut conn = Connection::open_in_memory()
            .map_err(|e| LoomaError::Database(format!("Failed to open in-memory db: {e}")))?;

        conn.pragma_update(None, "foreign_keys", "ON")
            .map_err(|e| LoomaError::Database(format!("Failed to set foreign_keys: {e}")))?;
        conn.busy_timeout(std::time::Duration::from_millis(5000))
            .map_err(|e| LoomaError::Database(format!("Failed to set busy_timeout: {e}")))?;

        run_migrations(&mut conn)?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            vault_path: PathBuf::from(":memory:"),
            db_path: PathBuf::from(":memory:"),
        })
    }

    pub fn repair_work_integrity(&self, work_id: &str, plan: &IntegrityRepairPlan) -> LoomaResult<()> {
        <Self as EntityService>::repair_work_integrity(self, work_id, plan)
    }
}

impl VaultService for LoomaDb {
    fn get_vault_info(&self) -> LoomaResult<VaultInfo> {
        let conn = self.conn.lock();
        let name: String = conn
            .query_row(
                "SELECT value FROM vault_meta WHERE key = 'name'",
                [],
                |row| row.get(0),
            )
            .unwrap_or_else(|_| "Looma Vault".to_string());

        let version: String = conn
            .query_row(
                "SELECT value FROM vault_meta WHERE key = 'version'",
                [],
                |row| row.get(0),
            )
            .unwrap_or_else(|_| "0.2.0".to_string());

        let created_at_str: Option<String> = conn
            .query_row(
                "SELECT value FROM vault_meta WHERE key = 'created_at'",
                [],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| LoomaError::Database(format!("Failed to query created_at: {e}")))?;

        let created_at = created_at_str.and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|d| d.with_timezone(&Utc)));

        Ok(VaultInfo {
            name,
            version,
            vault_path: self.vault_path.clone(),
            database_path: self.db_path.clone(),
            is_initialized: true,
            created_at,
        })
    }

    fn get_vault_stats(&self) -> LoomaResult<VaultStats> {
        let conn = self.conn.lock();
        let total_assets: u64 = conn
            .query_row("SELECT COUNT(*) FROM assets", [], |r| r.get(0))
            .unwrap_or(0);
        let total_entities: u64 = conn
            .query_row("SELECT COUNT(*) FROM entities", [], |r| r.get(0))
            .unwrap_or(0);
        let total_relations: u64 = conn
            .query_row("SELECT COUNT(*) FROM relations", [], |r| r.get(0))
            .unwrap_or(0);
        let total_memories: u64 = conn
            .query_row("SELECT COUNT(*) FROM memories", [], |r| r.get(0))
            .unwrap_or(0);
        let total_collections: u64 = conn
            .query_row("SELECT COUNT(*) FROM collections", [], |r| r.get(0))
            .unwrap_or(0);

        let database_size_bytes = if self.db_path.exists() {
            std::fs::metadata(&self.db_path).map(|m| m.len()).unwrap_or(0)
        } else {
            0
        };

        Ok(VaultStats {
            total_assets,
            total_entities,
            total_relations,
            total_memories,
            total_collections,
            database_size_bytes,
        })
    }
}

impl AssetService for LoomaDb {
    fn list_assets(&self, limit: usize, offset: usize) -> LoomaResult<Vec<Asset>> {
        let conn = self.conn.lock();
        let mut stmt = conn
            .prepare(
                "SELECT id, kind, source, path, size, hash, mime_type, metadata_json, status,
                        created_at, modified_at, indexed_at
                 FROM assets ORDER BY indexed_at DESC LIMIT ?1 OFFSET ?2",
            )
            .map_err(|e| LoomaError::Database(e.to_string()))?;

        let asset_iter = stmt
            .query_map(params![limit as i64, offset as i64], |row| {
                let kind_str: String = row.get(1)?;
                let source_str: String = row.get(2)?;
                let meta_str: String = row.get(7)?;
                let status_str: String = row.get(8)?;
                let created_str: String = row.get(9)?;
                let mod_str: String = row.get(10)?;
                let idx_str: String = row.get(11)?;

                Ok((
                    row.get::<_, String>(0)?,
                    kind_str,
                    source_str,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, Option<i64>>(4)?.map(|s| s as u64),
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, Option<String>>(6)?,
                    meta_str,
                    status_str,
                    created_str,
                    mod_str,
                    idx_str,
                ))
            })
            .map_err(|e| LoomaError::Database(e.to_string()))?;

        let mut results = Vec::new();
        for item in asset_iter {
            let (id, kind_str, source_str, path, size, hash, mime, meta_str, status_str, c_str, m_str, i_str) =
                item.map_err(|e| LoomaError::Database(e.to_string()))?;

            let kind = serde_json::from_str(&format!("\"{}\"", kind_str)).unwrap_or_default();
            let source = serde_json::from_str(&format!("\"{}\"", source_str)).unwrap_or_default();
            let status = serde_json::from_str(&format!("\"{}\"", status_str)).unwrap_or_default();
            let metadata = serde_json::from_str(&meta_str).unwrap_or(Value::Object(Default::default()));

            let created_at = DateTime::parse_from_rfc3339(&c_str)
                .map(|d| d.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());
            let modified_at = DateTime::parse_from_rfc3339(&m_str)
                .map(|d| d.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());
            let indexed_at = DateTime::parse_from_rfc3339(&i_str)
                .map(|d| d.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());

            results.push(Asset {
                id,
                kind,
                source,
                path,
                size,
                hash,
                mime_type: mime,
                metadata,
                status,
                created_at,
                modified_at,
                indexed_at,
            });
        }

        Ok(results)
    }

    fn get_asset_by_id(&self, id: &str) -> LoomaResult<Option<Asset>> {
        let conn = self.conn.lock();
        let mut stmt = conn
            .prepare(
                "SELECT id, kind, source, path, size, hash, mime_type, metadata_json, status,
                        created_at, modified_at, indexed_at
                 FROM assets WHERE id = ?1",
            )
            .map_err(|e| LoomaError::Database(e.to_string()))?;

        let maybe_row = stmt
            .query_row(params![id], |row| {
                let kind_str: String = row.get(1)?;
                let source_str: String = row.get(2)?;
                let meta_str: String = row.get(7)?;
                let status_str: String = row.get(8)?;
                let created_str: String = row.get(9)?;
                let mod_str: String = row.get(10)?;
                let idx_str: String = row.get(11)?;

                Ok((
                    row.get::<_, String>(0)?,
                    kind_str,
                    source_str,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, Option<i64>>(4)?.map(|s| s as u64),
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, Option<String>>(6)?,
                    meta_str,
                    status_str,
                    created_str,
                    mod_str,
                    idx_str,
                ))
            })
            .optional()
            .map_err(|e| LoomaError::Database(e.to_string()))?;

        match maybe_row {
            Some((id, kind_str, source_str, path, size, hash, mime, meta_str, status_str, c_str, m_str, i_str)) => {
                let kind = serde_json::from_str(&format!("\"{}\"", kind_str)).unwrap_or_default();
                let source = serde_json::from_str(&format!("\"{}\"", source_str)).unwrap_or_default();
                let status = serde_json::from_str(&format!("\"{}\"", status_str)).unwrap_or_default();
                let metadata = serde_json::from_str(&meta_str).unwrap_or(Value::Object(Default::default()));

                let created_at = DateTime::parse_from_rfc3339(&c_str)
                    .map(|d| d.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());
                let modified_at = DateTime::parse_from_rfc3339(&m_str)
                    .map(|d| d.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());
                let indexed_at = DateTime::parse_from_rfc3339(&i_str)
                    .map(|d| d.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());

                Ok(Some(Asset {
                    id,
                    kind,
                    source,
                    path,
                    size,
                    hash,
                    mime_type: mime,
                    metadata,
                    status,
                    created_at,
                    modified_at,
                    indexed_at,
                }))
            }
            None => Ok(None),
        }
    }

    fn get_asset_by_path(&self, path: &str) -> LoomaResult<Option<Asset>> {
        let conn = self.conn.lock();
        let mut stmt = conn
            .prepare(
                "SELECT id, kind, source, path, size, hash, mime_type, metadata_json, status,
                        created_at, modified_at, indexed_at
                 FROM assets WHERE path = ?1",
            )
            .map_err(|e| LoomaError::Database(e.to_string()))?;

        let maybe_row = stmt
            .query_row(params![path], |row| {
                let kind_str: String = row.get(1)?;
                let source_str: String = row.get(2)?;
                let meta_str: String = row.get(7)?;
                let status_str: String = row.get(8)?;
                let created_str: String = row.get(9)?;
                let mod_str: String = row.get(10)?;
                let idx_str: String = row.get(11)?;

                Ok((
                    row.get::<_, String>(0)?,
                    kind_str,
                    source_str,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, Option<i64>>(4)?.map(|s| s as u64),
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, Option<String>>(6)?,
                    meta_str,
                    status_str,
                    created_str,
                    mod_str,
                    idx_str,
                ))
            })
            .optional()
            .map_err(|e| LoomaError::Database(e.to_string()))?;

        match maybe_row {
            Some((id, kind_str, source_str, p, size, hash, mime, meta_str, status_str, c_str, m_str, i_str)) => {
                let kind = serde_json::from_str(&format!("\"{}\"", kind_str)).unwrap_or_default();
                let source = serde_json::from_str(&format!("\"{}\"", source_str)).unwrap_or_default();
                let status = serde_json::from_str(&format!("\"{}\"", status_str)).unwrap_or_default();
                let metadata = serde_json::from_str(&meta_str).unwrap_or(Value::Object(Default::default()));

                let created_at = DateTime::parse_from_rfc3339(&c_str)
                    .map(|d| d.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());
                let modified_at = DateTime::parse_from_rfc3339(&m_str)
                    .map(|d| d.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());
                let indexed_at = DateTime::parse_from_rfc3339(&i_str)
                    .map(|d| d.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());

                Ok(Some(Asset {
                    id,
                    kind,
                    source,
                    path: p,
                    size,
                    hash,
                    mime_type: mime,
                    metadata,
                    status,
                    created_at,
                    modified_at,
                    indexed_at,
                }))
            }
            None => Ok(None),
        }
    }

    fn upsert_asset(&self, asset: &Asset) -> LoomaResult<()> {
        let conn = self.conn.lock();
        let kind_str = serde_json::to_string(&asset.kind).unwrap_or_default().trim_matches('"').to_string();
        let source_str = serde_json::to_string(&asset.source).unwrap_or_default().trim_matches('"').to_string();
        let status_str = serde_json::to_string(&asset.status).unwrap_or_default().trim_matches('"').to_string();
        let meta_str = serde_json::to_string(&asset.metadata).unwrap_or_else(|_| "{}".to_string());

        conn.execute(
            "INSERT INTO assets (id, kind, source, path, size, hash, mime_type, metadata_json, status,
                                created_at, modified_at, indexed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
             ON CONFLICT(id) DO UPDATE SET
                kind = excluded.kind,
                source = excluded.source,
                path = excluded.path,
                size = excluded.size,
                hash = excluded.hash,
                mime_type = excluded.mime_type,
                metadata_json = excluded.metadata_json,
                status = excluded.status,
                modified_at = excluded.modified_at,
                indexed_at = excluded.indexed_at",
            params![
                asset.id,
                kind_str,
                source_str,
                asset.path,
                asset.size.map(|s| s as i64),
                asset.hash,
                asset.mime_type,
                meta_str,
                status_str,
                asset.created_at.to_rfc3339(),
                asset.modified_at.to_rfc3339(),
                asset.indexed_at.to_rfc3339(),
            ],
        )
        .map_err(|e| LoomaError::Database(format!("Failed to upsert asset: {e}")))?;

        Ok(())
    }

    fn query_assets(&self, filter: &AssetFilter) -> LoomaResult<Vec<Asset>> {
        let conn = self.conn.lock();
        let mut query = "SELECT id, kind, source, path, size, hash, mime_type, metadata_json, status,
                                created_at, modified_at, indexed_at
                         FROM assets WHERE 1=1".to_string();
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(ref kind) = filter.kind {
            let kind_str = serde_json::to_string(kind).unwrap_or_default().trim_matches('"').to_string();
            query.push_str(" AND kind = ?");
            params_vec.push(Box::new(kind_str));
        }

        if let Some(ref status) = filter.status {
            let status_str = serde_json::to_string(status).unwrap_or_default().trim_matches('"').to_string();
            query.push_str(" AND status = ?");
            params_vec.push(Box::new(status_str));
        }

        if let Some(ref q) = filter.search_query {
            query.push_str(" AND (path LIKE ? OR metadata_json LIKE ?)");
            let pattern = format!("%{}%", q);
            params_vec.push(Box::new(pattern.clone()));
            params_vec.push(Box::new(pattern));
        }

        query.push_str(" ORDER BY indexed_at DESC");

        if let Some(limit) = filter.limit {
            query.push_str(&format!(" LIMIT {}", limit));
            if let Some(offset) = filter.offset {
                query.push_str(&format!(" OFFSET {}", offset));
            }
        }

        let mut stmt = conn.prepare(&query).map_err(|e| LoomaError::Database(e.to_string()))?;
        let rusqlite_params: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| &**p).collect();

        let asset_iter = stmt
            .query_map(&*rusqlite_params, |row| {
                let kind_str: String = row.get(1)?;
                let source_str: String = row.get(2)?;
                let meta_str: String = row.get(7)?;
                let status_str: String = row.get(8)?;
                let created_str: String = row.get(9)?;
                let mod_str: String = row.get(10)?;
                let idx_str: String = row.get(11)?;

                Ok((
                    row.get::<_, String>(0)?,
                    kind_str,
                    source_str,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, Option<i64>>(4)?.map(|s| s as u64),
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, Option<String>>(6)?,
                    meta_str,
                    status_str,
                    created_str,
                    mod_str,
                    idx_str,
                ))
            })
            .map_err(|e| LoomaError::Database(e.to_string()))?;

        let mut results = Vec::new();
        for item in asset_iter {
            let (id, kind_str, source_str, path, size, hash, mime, meta_str, status_str, c_str, m_str, i_str) =
                item.map_err(|e| LoomaError::Database(e.to_string()))?;

            let kind = serde_json::from_str(&format!("\"{}\"", kind_str)).unwrap_or_default();
            let source = serde_json::from_str(&format!("\"{}\"", source_str)).unwrap_or_default();
            let status = serde_json::from_str(&format!("\"{}\"", status_str)).unwrap_or_default();
            let metadata = serde_json::from_str(&meta_str).unwrap_or(Value::Object(Default::default()));

            let created_at = DateTime::parse_from_rfc3339(&c_str)
                .map(|d| d.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());
            let modified_at = DateTime::parse_from_rfc3339(&m_str)
                .map(|d| d.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());
            let indexed_at = DateTime::parse_from_rfc3339(&i_str)
                .map(|d| d.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());

            results.push(Asset {
                id,
                kind,
                source,
                path,
                size,
                hash,
                mime_type: mime,
                metadata,
                status,
                created_at,
                modified_at,
                indexed_at,
            });
        }

        Ok(results)
    }

    fn list_assets_by_prefix(&self, prefix: &str) -> LoomaResult<Vec<Asset>> {
        let conn = self.conn.lock();
        let norm_prefix = looma_core::normalize_directory_path(prefix);
        let escaped = norm_prefix
            .replace('\\', "\\\\")
            .replace('%', "\\%")
            .replace('_', "\\_");
        let slash_pattern = format!("{}/%", escaped.trim_end_matches('/'));
        let exact_pattern = escaped.trim_end_matches('/').to_string();

        let mut stmt = conn
            .prepare(
                "SELECT id, kind, source, path, size, hash, mime_type, metadata_json, status,
                        created_at, modified_at, indexed_at
                 FROM assets
                 WHERE (path = ?1 OR path LIKE ?2 ESCAPE '\\')",
            )
            .map_err(|e| LoomaError::Database(e.to_string()))?;

        let asset_iter = stmt
            .query_map(params![exact_pattern, slash_pattern], |row| {
                let kind_str: String = row.get(1)?;
                let source_str: String = row.get(2)?;
                let meta_str: String = row.get(7)?;
                let status_str: String = row.get(8)?;
                let created_str: String = row.get(9)?;
                let mod_str: String = row.get(10)?;
                let idx_str: String = row.get(11)?;

                Ok((
                    row.get::<_, String>(0)?,
                    kind_str,
                    source_str,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, Option<i64>>(4)?.map(|s| s as u64),
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, Option<String>>(6)?,
                    meta_str,
                    status_str,
                    created_str,
                    mod_str,
                    idx_str,
                ))
            })
            .map_err(|e| LoomaError::Database(e.to_string()))?;

        let mut results = Vec::new();
        for item in asset_iter {
            let (id, kind_str, source_str, path, size, hash, mime, meta_str, status_str, c_str, m_str, i_str) =
                item.map_err(|e| LoomaError::Database(e.to_string()))?;

            if let Some(ref p) = path {
                if !looma_core::is_path_under_root(p, &norm_prefix) {
                    continue;
                }
            } else {
                continue;
            }

            let kind = serde_json::from_str(&format!("\"{}\"", kind_str)).unwrap_or_default();
            let source = serde_json::from_str(&format!("\"{}\"", source_str)).unwrap_or_default();
            let status = serde_json::from_str(&format!("\"{}\"", status_str)).unwrap_or_default();
            let metadata = serde_json::from_str(&meta_str).unwrap_or(Value::Object(Default::default()));

            let created_at = DateTime::parse_from_rfc3339(&c_str)
                .map(|d| d.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());
            let modified_at = DateTime::parse_from_rfc3339(&m_str)
                .map(|d| d.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());
            let indexed_at = DateTime::parse_from_rfc3339(&i_str)
                .map(|d| d.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());

            results.push(Asset {
                id,
                kind,
                source,
                path,
                size,
                hash,
                mime_type: mime,
                metadata,
                status,
                created_at,
                modified_at,
                indexed_at,
            });
        }

        Ok(results)
    }

    fn delete_asset(&self, id: &str) -> LoomaResult<bool> {
        let conn = self.conn.lock();
        let rows = conn
            .execute("DELETE FROM assets WHERE id = ?1", params![id])
            .map_err(|e| LoomaError::Database(e.to_string()))?;
        Ok(rows > 0)
    }

    fn count_assets(&self) -> LoomaResult<u64> {
        let conn = self.conn.lock();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM assets", [], |r| r.get(0))
            .map_err(|e| LoomaError::Database(e.to_string()))?;
        Ok(count as u64)
    }
}

impl EntityService for LoomaDb {
    fn list_entities(&self, filter: &EntityFilter) -> LoomaResult<Vec<Entity>> {
        let conn = self.conn.lock();
        let mut query = "SELECT id, entity_type, title, description, properties_json, created_at, updated_at FROM entities WHERE 1=1".to_string();
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(ref et) = filter.entity_type {
            query.push_str(" AND entity_type = ?");
            params_vec.push(Box::new(et.clone()));
        }
        if let Some(ref q) = filter.search_query {
            query.push_str(" AND (title LIKE ? OR description LIKE ? OR properties_json LIKE ?)");
            let pattern = format!("%{}%", q);
            params_vec.push(Box::new(pattern.clone()));
            params_vec.push(Box::new(pattern.clone()));
            params_vec.push(Box::new(pattern));
        }

        query.push_str(" ORDER BY updated_at DESC");

        if let Some(limit) = filter.limit {
            query.push_str(&format!(" LIMIT {}", limit));
            if let Some(offset) = filter.offset {
                query.push_str(&format!(" OFFSET {}", offset));
            }
        }

        let mut stmt = conn.prepare(&query).map_err(|e| LoomaError::Database(e.to_string()))?;
        let rusqlite_params: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| &**p).collect();

        let entity_iter = stmt
            .query_map(&*rusqlite_params, |row| {
                let id: String = row.get(0)?;
                let entity_type: String = row.get(1)?;
                let title: String = row.get(2)?;
                let description: Option<String> = row.get(3)?;
                let props_str: String = row.get(4)?;
                let c_str: String = row.get(5)?;
                let u_str: String = row.get(6)?;

                Ok((id, entity_type, title, description, props_str, c_str, u_str))
            })
            .map_err(|e| LoomaError::Database(e.to_string()))?;

        let mut entities = Vec::new();
        for item in entity_iter {
            let (id, entity_type, title, description, props_str, c_str, u_str) =
                item.map_err(|e| LoomaError::Database(e.to_string()))?;

            let properties = serde_json::from_str(&props_str).unwrap_or(Value::Object(Default::default()));
            let created_at = DateTime::parse_from_rfc3339(&c_str)
                .map(|d| d.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());
            let updated_at = DateTime::parse_from_rfc3339(&u_str)
                .map(|d| d.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());

            entities.push(Entity {
                id,
                entity_type,
                title,
                description,
                properties,
                created_at,
                updated_at,
            });
        }

        Ok(entities)
    }

    fn get_entity_by_id(&self, id: &str) -> LoomaResult<Option<Entity>> {
        let conn = self.conn.lock();
        let mut stmt = conn
            .prepare("SELECT id, entity_type, title, description, properties_json, created_at, updated_at FROM entities WHERE id = ?1")
            .map_err(|e| LoomaError::Database(e.to_string()))?;

        let maybe_row = stmt
            .query_row(params![id], |row| {
                let id: String = row.get(0)?;
                let entity_type: String = row.get(1)?;
                let title: String = row.get(2)?;
                let description: Option<String> = row.get(3)?;
                let props_str: String = row.get(4)?;
                let c_str: String = row.get(5)?;
                let u_str: String = row.get(6)?;

                Ok((id, entity_type, title, description, props_str, c_str, u_str))
            })
            .optional()
            .map_err(|e| LoomaError::Database(e.to_string()))?;

        match maybe_row {
            Some((id, entity_type, title, description, props_str, c_str, u_str)) => {
                let properties = serde_json::from_str(&props_str).unwrap_or(Value::Object(Default::default()));
                let created_at = DateTime::parse_from_rfc3339(&c_str)
                    .map(|d| d.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());
                let updated_at = DateTime::parse_from_rfc3339(&u_str)
                    .map(|d| d.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());

                Ok(Some(Entity {
                    id,
                    entity_type,
                    title,
                    description,
                    properties,
                    created_at,
                    updated_at,
                }))
            }
            None => Ok(None),
        }
    }

    fn create_entity(&self, entity: &Entity) -> LoomaResult<()> {
        let conn = self.conn.lock();
        let props_str = serde_json::to_string(&entity.properties).unwrap_or_else(|_| "{}".to_string());
        conn.execute(
            "INSERT INTO entities (id, entity_type, title, description, properties_json, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                entity.id,
                entity.entity_type,
                entity.title,
                entity.description,
                props_str,
                entity.created_at.to_rfc3339(),
                entity.updated_at.to_rfc3339()
            ],
        )
        .map_err(|e| LoomaError::Database(format!("Failed to insert entity: {e}")))?;
        Ok(())
    }

    fn update_entity(&self, entity: &Entity) -> LoomaResult<()> {
        let conn = self.conn.lock();
        let props_str = serde_json::to_string(&entity.properties).unwrap_or_else(|_| "{}".to_string());
        conn.execute(
            "UPDATE entities SET entity_type = ?2, title = ?3, description = ?4, properties_json = ?5, updated_at = ?6
             WHERE id = ?1",
            params![
                entity.id,
                entity.entity_type,
                entity.title,
                entity.description,
                props_str,
                entity.updated_at.to_rfc3339()
            ],
        )
        .map_err(|e| LoomaError::Database(format!("Failed to update entity: {e}")))?;
        Ok(())
    }

    fn delete_entity(&self, id: &str) -> LoomaResult<bool> {
        let conn = self.conn.lock();
        let rows = conn
            .execute("DELETE FROM entities WHERE id = ?1", params![id])
            .map_err(|e| LoomaError::Database(e.to_string()))?;
        Ok(rows > 0)
    }

    fn count_entities(&self) -> LoomaResult<u64> {
        let conn = self.conn.lock();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM entities", [], |r| r.get(0))
            .map_err(|e| LoomaError::Database(e.to_string()))?;
        Ok(count as u64)
    }

    fn repair_work_integrity(&self, _work_id: &str, plan: &IntegrityRepairPlan) -> LoomaResult<()> {
        let mut conn = self.conn.lock();
        let tx = conn.transaction().map_err(|e| LoomaError::Database(e.to_string()))?;

        for op in &plan.operations {
            match op {
                IntegrityRepairOperation::RemoveDuplicateRelation { relation_id } => {
                    let exists: bool = tx.query_row(
                        "SELECT 1 FROM relations WHERE id = ?1",
                        params![relation_id],
                        |_| Ok(true),
                    ).optional().map_err(|e| LoomaError::Database(e.to_string()))?.unwrap_or(false);

                    if !exists {
                        return Err(LoomaError::Conflict(format!(
                            "Duplicate relation '{}' does not exist in database", relation_id
                        )));
                    }

                    tx.execute("DELETE FROM relations WHERE id = ?1", params![relation_id])
                        .map_err(|e| LoomaError::Database(format!("Failed to delete duplicate relation: {e}")))?;
                }
                IntegrityRepairOperation::RemoveDuplicatePrimary { reference_id } => {
                    let meta_str: Option<String> = tx.query_row(
                        "SELECT metadata_json FROM external_references WHERE id = ?1",
                        params![reference_id],
                        |row| row.get(0),
                    ).optional().map_err(|e| LoomaError::Database(e.to_string()))?;

                    let meta_str = meta_str.ok_or_else(|| {
                        LoomaError::Conflict(format!("External reference '{}' does not exist", reference_id))
                    })?;

                    let mut meta: serde_json::Value = serde_json::from_str(&meta_str).unwrap_or(serde_json::json!({}));
                    if meta.get("is_primary").and_then(|v| v.as_bool()) != Some(true) {
                        return Err(LoomaError::Conflict(format!(
                            "External reference '{}' is no longer primary", reference_id
                        )));
                    }

                    if let Some(obj) = meta.as_object_mut() {
                        obj.insert("is_primary".to_string(), serde_json::json!(false));
                    }
                    let now = Utc::now().to_rfc3339();
                    tx.execute(
                        "UPDATE external_references SET metadata_json = ?1, updated_at = ?2 WHERE id = ?3",
                        params![serde_json::to_string(&meta).unwrap_or_else(|_| "{}".to_string()), now, reference_id],
                    ).map_err(|e| LoomaError::Database(format!("Failed to demote primary reference: {e}")))?;
                }
                IntegrityRepairOperation::RemoveInvalidAttachment { relation_id } => {
                    let exists: bool = tx.query_row(
                        "SELECT 1 FROM relations WHERE id = ?1",
                        params![relation_id],
                        |_| Ok(true),
                    ).optional().map_err(|e| LoomaError::Database(e.to_string()))?.unwrap_or(false);

                    if !exists {
                        return Err(LoomaError::Conflict(format!(
                            "Invalid attachment relation '{}' does not exist", relation_id
                        )));
                    }

                    tx.execute("DELETE FROM relations WHERE id = ?1", params![relation_id])
                        .map_err(|e| LoomaError::Database(format!("Failed to delete invalid attachment relation: {e}")))?;
                }
                IntegrityRepairOperation::RemoveSelfRelation { relation_id } => {
                    let exists: bool = tx.query_row(
                        "SELECT 1 FROM relations WHERE id = ?1",
                        params![relation_id],
                        |_| Ok(true),
                    ).optional().map_err(|e| LoomaError::Database(e.to_string()))?.unwrap_or(false);

                    if !exists {
                        return Err(LoomaError::Conflict(format!(
                            "Self relation '{}' does not exist", relation_id
                        )));
                    }

                    tx.execute("DELETE FROM relations WHERE id = ?1", params![relation_id])
                        .map_err(|e| LoomaError::Database(format!("Failed to delete self relation: {e}")))?;
                }
            }
        }

        tx.commit().map_err(|e| LoomaError::Database(format!("Failed to commit repair transaction: {e}")))?;
        Ok(())
    }
}

impl RelationService for LoomaDb {
    fn list_relations_for_source(&self, source_id: &str) -> LoomaResult<Vec<Relation>> {
        let conn = self.conn.lock();
        let mut stmt = conn
            .prepare("SELECT id, source_id, source_type, relation_type, target_id, target_type, metadata_json, created_at FROM relations WHERE source_id = ?1")
            .map_err(|e| LoomaError::Database(e.to_string()))?;

        let rel_iter = stmt
            .query_map(params![source_id], |row| {
                let id: String = row.get(0)?;
                let s_id: String = row.get(1)?;
                let s_type: String = row.get(2)?;
                let r_type: String = row.get(3)?;
                let t_id: String = row.get(4)?;
                let t_type: String = row.get(5)?;
                let meta_str: String = row.get(6)?;
                let c_str: String = row.get(7)?;
                Ok((id, s_id, s_type, r_type, t_id, t_type, meta_str, c_str))
            })
            .map_err(|e| LoomaError::Database(e.to_string()))?;

        let mut results = Vec::new();
        for item in rel_iter {
            let (id, source_id, source_type, relation_type, target_id, target_type, meta_str, c_str) =
                item.map_err(|e| LoomaError::Database(e.to_string()))?;
            let metadata = serde_json::from_str(&meta_str).unwrap_or(Value::Object(Default::default()));
            let created_at = DateTime::parse_from_rfc3339(&c_str)
                .map(|d| d.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());

            results.push(Relation {
                id,
                source_id,
                source_type,
                relation_type,
                target_id,
                target_type,
                metadata,
                created_at,
            });
        }
        Ok(results)
    }

    fn list_relations_for_target(&self, target_id: &str) -> LoomaResult<Vec<Relation>> {
        let conn = self.conn.lock();
        let mut stmt = conn
            .prepare("SELECT id, source_id, source_type, relation_type, target_id, target_type, metadata_json, created_at FROM relations WHERE target_id = ?1")
            .map_err(|e| LoomaError::Database(e.to_string()))?;

        let rel_iter = stmt
            .query_map(params![target_id], |row| {
                let id: String = row.get(0)?;
                let s_id: String = row.get(1)?;
                let s_type: String = row.get(2)?;
                let r_type: String = row.get(3)?;
                let t_id: String = row.get(4)?;
                let t_type: String = row.get(5)?;
                let meta_str: String = row.get(6)?;
                let c_str: String = row.get(7)?;
                Ok((id, s_id, s_type, r_type, t_id, t_type, meta_str, c_str))
            })
            .map_err(|e| LoomaError::Database(e.to_string()))?;

        let mut results = Vec::new();
        for item in rel_iter {
            let (id, source_id, source_type, relation_type, target_id, target_type, meta_str, c_str) =
                item.map_err(|e| LoomaError::Database(e.to_string()))?;
            let metadata = serde_json::from_str(&meta_str).unwrap_or(Value::Object(Default::default()));
            let created_at = DateTime::parse_from_rfc3339(&c_str)
                .map(|d| d.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());

            results.push(Relation {
                id,
                source_id,
                source_type,
                relation_type,
                target_id,
                target_type,
                metadata,
                created_at,
            });
        }
        Ok(results)
    }

    fn list_relations_for_item(&self, item_id: &str) -> LoomaResult<Vec<Relation>> {
        let conn = self.conn.lock();
        let mut stmt = conn
            .prepare("SELECT id, source_id, source_type, relation_type, target_id, target_type, metadata_json, created_at FROM relations WHERE source_id = ?1 OR target_id = ?1")
            .map_err(|e| LoomaError::Database(e.to_string()))?;

        let rel_iter = stmt
            .query_map(params![item_id], |row| {
                let id: String = row.get(0)?;
                let s_id: String = row.get(1)?;
                let s_type: String = row.get(2)?;
                let r_type: String = row.get(3)?;
                let t_id: String = row.get(4)?;
                let t_type: String = row.get(5)?;
                let meta_str: String = row.get(6)?;
                let c_str: String = row.get(7)?;
                Ok((id, s_id, s_type, r_type, t_id, t_type, meta_str, c_str))
            })
            .map_err(|e| LoomaError::Database(e.to_string()))?;

        let mut results = Vec::new();
        for item in rel_iter {
            let (id, source_id, source_type, relation_type, target_id, target_type, meta_str, c_str) =
                item.map_err(|e| LoomaError::Database(e.to_string()))?;
            let metadata = serde_json::from_str(&meta_str).unwrap_or(Value::Object(Default::default()));
            let created_at = DateTime::parse_from_rfc3339(&c_str)
                .map(|d| d.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());

            results.push(Relation {
                id,
                source_id,
                source_type,
                relation_type,
                target_id,
                target_type,
                metadata,
                created_at,
            });
        }
        Ok(results)
    }

    fn create_relation(&self, relation: &Relation) -> LoomaResult<()> {
        let conn = self.conn.lock();
        // Logical relation identity: (source_id, relation_type, target_id)
        let exists: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM relations WHERE source_id = ?1 AND relation_type = ?2 AND target_id = ?3",
                params![relation.source_id, relation.relation_type, relation.target_id],
                |r| r.get(0),
            )
            .map_err(|e| LoomaError::Database(e.to_string()))?;
        if exists > 0 {
            return Ok(()); // Idempotent success
        }

        let meta_str = serde_json::to_string(&relation.metadata).unwrap_or_else(|_| "{}".to_string());
        conn.execute(
            "INSERT INTO relations (id, source_id, source_type, relation_type, target_id, target_type, metadata_json, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                relation.id,
                relation.source_id,
                relation.source_type,
                relation.relation_type,
                relation.target_id,
                relation.target_type,
                meta_str,
                relation.created_at.to_rfc3339()
            ],
        )
        .map_err(|e| LoomaError::Database(format!("Failed to create relation: {e}")))?;
        Ok(())
    }

    fn delete_relation(&self, id: &str) -> LoomaResult<bool> {
        let conn = self.conn.lock();
        let rows = conn
            .execute("DELETE FROM relations WHERE id = ?1", params![id])
            .map_err(|e| LoomaError::Database(e.to_string()))?;
        Ok(rows > 0)
    }

    fn delete_relation_between(&self, source_id: &str, target_id: &str) -> LoomaResult<bool> {
        let conn = self.conn.lock();
        let rows = conn
            .execute(
                "DELETE FROM relations WHERE (source_id = ?1 AND target_id = ?2) OR (source_id = ?2 AND target_id = ?1)",
                params![source_id, target_id],
            )
            .map_err(|e| LoomaError::Database(e.to_string()))?;
        Ok(rows > 0)
    }

    fn relocate_work_directory(
        &self,
        work_id: &str,
        old_asset_id: &str,
        new_asset: &Asset,
        new_relation: &Relation,
    ) -> LoomaResult<()> {
        let mut conn = self.conn.lock();
        let tx = conn.transaction().map_err(|e| LoomaError::Database(e.to_string()))?;

        // 1. Validate work exists
        let work_count: i64 = tx.query_row(
            "SELECT COUNT(*) FROM entities WHERE id = ?1",
            params![work_id],
            |r| r.get(0),
        ).map_err(|e| LoomaError::Database(e.to_string()))?;
        if work_count == 0 {
            return Err(LoomaError::NotFound(format!("Work entity not found: {work_id}")));
        }

        // 2. Validate old asset exists and kind is Directory
        let old_asset_row: Option<(String,)> = tx.query_row(
            "SELECT kind FROM assets WHERE id = ?1",
            params![old_asset_id],
            |r| Ok((r.get(0)?,)),
        ).optional().map_err(|e| LoomaError::Database(e.to_string()))?;

        let old_kind_str = match old_asset_row {
            Some((k,)) => k,
            None => return Err(LoomaError::NotFound(format!("Old asset not found: {old_asset_id}"))),
        };

        if old_kind_str != "directory" {
            return Err(LoomaError::Validation(format!(
                "Asset {old_asset_id} is not a directory asset"
            )));
        }

        // 3. Validate old asset is linked to work strictly as ATTACHES relation
        // Direction must be source = work_id, target = old_asset_id, relation_type = attaches
        let rel_count: i64 = tx.query_row(
            "SELECT COUNT(*) FROM relations WHERE source_id = ?1 AND target_id = ?2 AND relation_type = 'attaches'",
            params![work_id, old_asset_id],
            |r| r.get(0),
        ).map_err(|e| LoomaError::Database(e.to_string()))?;
        if rel_count == 0 {
            return Err(LoomaError::Validation(format!(
                "Old asset {old_asset_id} is not linked to work {work_id} via attaches relation"
            )));
        }

        // 4. Validate new asset is a directory and upsert
        if new_asset.kind != AssetKind::Directory {
            return Err(LoomaError::Validation(format!(
                "New asset {} is not a directory asset",
                new_asset.id
            )));
        }

        let meta_str = serde_json::to_string(&new_asset.metadata).unwrap_or_else(|_| "{}".to_string());
        tx.execute(
            "INSERT INTO assets (id, kind, source, path, size, hash, mime_type, metadata_json, status, created_at, modified_at, indexed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
             ON CONFLICT(id) DO UPDATE SET
                 kind = excluded.kind,
                 source = excluded.source,
                 path = excluded.path,
                 size = excluded.size,
                 hash = excluded.hash,
                 mime_type = excluded.mime_type,
                 metadata_json = excluded.metadata_json,
                 status = excluded.status,
                 modified_at = excluded.modified_at,
                 indexed_at = excluded.indexed_at",
            params![
                new_asset.id,
                serde_json::to_string(&new_asset.kind).unwrap_or_default().trim_matches('"'),
                serde_json::to_string(&new_asset.source).unwrap_or_default().trim_matches('"'),
                new_asset.path,
                new_asset.size.map(|s| s as i64),
                new_asset.hash,
                new_asset.mime_type,
                meta_str,
                serde_json::to_string(&new_asset.status).unwrap_or_default().trim_matches('"'),
                new_asset.created_at.to_rfc3339(),
                new_asset.modified_at.to_rfc3339(),
                new_asset.indexed_at.to_rfc3339(),
            ],
        ).map_err(|e| LoomaError::Database(format!("Failed to upsert new asset: {e}")))?;

        // 5. Delete strictly old attaches relation between work_id and old_asset_id
        // Preserves any other relations (e.g. references) between work_id and old_asset_id
        tx.execute(
            "DELETE FROM relations WHERE source_id = ?1 AND target_id = ?2 AND relation_type = 'attaches'",
            params![work_id, old_asset_id],
        ).map_err(|e| LoomaError::Database(format!("Failed to delete old attaches relation: {e}")))?;

        // 6. Check if target relation already exists (Section 7.1 Existing Target Idempotency)
        let existing_target_count: i64 = tx.query_row(
            "SELECT COUNT(*) FROM relations WHERE source_id = ?1 AND target_id = ?2 AND relation_type = 'attaches'",
            params![work_id, new_asset.id],
            |r| r.get(0),
        ).map_err(|e| LoomaError::Database(e.to_string()))?;

        if existing_target_count == 0 {
            let rel_meta_str = serde_json::to_string(&new_relation.metadata).unwrap_or_else(|_| "{}".to_string());
            tx.execute(
                "INSERT INTO relations (id, source_id, source_type, relation_type, target_id, target_type, metadata_json, created_at)
                 VALUES (?1, ?2, 'entity', 'attaches', ?3, 'asset', ?4, ?5)",
                params![
                    new_relation.id,
                    work_id,
                    new_asset.id,
                    rel_meta_str,
                    new_relation.created_at.to_rfc3339(),
                ],
            ).map_err(|e| LoomaError::Database(format!("Failed to insert new relation: {e}")))?;
        }

        tx.commit().map_err(|e| LoomaError::Database(e.to_string()))?;
        Ok(())
    }
}

impl MemoryService for LoomaDb {
    fn list_memories(&self, limit: usize, offset: usize) -> LoomaResult<Vec<Memory>> {
        let conn = self.conn.lock();
        let mut stmt = conn
            .prepare("SELECT id, title, content, category, metadata_json, recorded_at, created_at, updated_at FROM memories ORDER BY recorded_at DESC LIMIT ?1 OFFSET ?2")
            .map_err(|e| LoomaError::Database(e.to_string()))?;

        let mem_iter = stmt
            .query_map(params![limit as i64, offset as i64], |row| {
                let id: String = row.get(0)?;
                let title: String = row.get(1)?;
                let content: String = row.get(2)?;
                let category: Option<String> = row.get(3)?;
                let meta_str: String = row.get(4)?;
                let r_str: String = row.get(5)?;
                let c_str: String = row.get(6)?;
                let u_str: String = row.get(7)?;
                Ok((id, title, content, category, meta_str, r_str, c_str, u_str))
            })
            .map_err(|e| LoomaError::Database(e.to_string()))?;

        let mut results = Vec::new();
        for item in mem_iter {
            let (id, title, content, category, meta_str, r_str, c_str, u_str) =
                item.map_err(|e| LoomaError::Database(e.to_string()))?;

            let metadata = serde_json::from_str(&meta_str).unwrap_or(Value::Object(Default::default()));
            let recorded_at = DateTime::parse_from_rfc3339(&r_str)
                .map(|d| d.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());
            let created_at = DateTime::parse_from_rfc3339(&c_str)
                .map(|d| d.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());
            let updated_at = DateTime::parse_from_rfc3339(&u_str)
                .map(|d| d.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());

            results.push(Memory {
                id,
                title,
                content,
                category,
                metadata,
                recorded_at,
                created_at,
                updated_at,
            });
        }
        Ok(results)
    }

    fn get_memory_by_id(&self, id: &str) -> LoomaResult<Option<Memory>> {
        let conn = self.conn.lock();
        let mut stmt = conn
            .prepare("SELECT id, title, content, category, metadata_json, recorded_at, created_at, updated_at FROM memories WHERE id = ?1")
            .map_err(|e| LoomaError::Database(e.to_string()))?;

        let maybe_row = stmt
            .query_row(params![id], |row| {
                let id: String = row.get(0)?;
                let title: String = row.get(1)?;
                let content: String = row.get(2)?;
                let category: Option<String> = row.get(3)?;
                let meta_str: String = row.get(4)?;
                let r_str: String = row.get(5)?;
                let c_str: String = row.get(6)?;
                let u_str: String = row.get(7)?;
                Ok((id, title, content, category, meta_str, r_str, c_str, u_str))
            })
            .optional()
            .map_err(|e| LoomaError::Database(e.to_string()))?;

        match maybe_row {
            Some((id, title, content, category, meta_str, r_str, c_str, u_str)) => {
                let metadata = serde_json::from_str(&meta_str).unwrap_or(Value::Object(Default::default()));
                let recorded_at = DateTime::parse_from_rfc3339(&r_str)
                    .map(|d| d.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());
                let created_at = DateTime::parse_from_rfc3339(&c_str)
                    .map(|d| d.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());
                let updated_at = DateTime::parse_from_rfc3339(&u_str)
                    .map(|d| d.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());

                Ok(Some(Memory {
                    id,
                    title,
                    content,
                    category,
                    metadata,
                    recorded_at,
                    created_at,
                    updated_at,
                }))
            }
            None => Ok(None),
        }
    }

    fn create_memory(&self, memory: &Memory) -> LoomaResult<()> {
        let conn = self.conn.lock();
        let meta_str = serde_json::to_string(&memory.metadata).unwrap_or_else(|_| "{}".to_string());
        conn.execute(
            "INSERT INTO memories (id, title, content, category, metadata_json, recorded_at, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                memory.id,
                memory.title,
                memory.content,
                memory.category,
                meta_str,
                memory.recorded_at.to_rfc3339(),
                memory.created_at.to_rfc3339(),
                memory.updated_at.to_rfc3339()
            ],
        )
        .map_err(|e| LoomaError::Database(format!("Failed to create memory: {e}")))?;
        Ok(())
    }

    fn update_memory(&self, memory: &Memory) -> LoomaResult<()> {
        let conn = self.conn.lock();
        let meta_str = serde_json::to_string(&memory.metadata).unwrap_or_else(|_| "{}".to_string());
        conn.execute(
            "UPDATE memories SET title = ?2, content = ?3, category = ?4, metadata_json = ?5, recorded_at = ?6, updated_at = ?7
             WHERE id = ?1",
            params![
                memory.id,
                memory.title,
                memory.content,
                memory.category,
                meta_str,
                memory.recorded_at.to_rfc3339(),
                memory.updated_at.to_rfc3339()
            ],
        )
        .map_err(|e| LoomaError::Database(format!("Failed to update memory: {e}")))?;
        Ok(())
    }

    fn delete_memory(&self, id: &str) -> LoomaResult<bool> {
        let conn = self.conn.lock();
        let rows = conn
            .execute("DELETE FROM memories WHERE id = ?1", params![id])
            .map_err(|e| LoomaError::Database(e.to_string()))?;
        Ok(rows > 0)
    }
}

impl CollectionService for LoomaDb {
    fn list_collections(&self) -> LoomaResult<Vec<Collection>> {
        let conn = self.conn.lock();
        let mut stmt = conn
            .prepare("SELECT id, title, description, query, metadata_json, created_at, updated_at FROM collections ORDER BY updated_at DESC")
            .map_err(|e| LoomaError::Database(e.to_string()))?;

        let col_iter = stmt
            .query_map([], |row| {
                let id: String = row.get(0)?;
                let title: String = row.get(1)?;
                let description: Option<String> = row.get(2)?;
                let query: Option<String> = row.get(3)?;
                let meta_str: String = row.get(4)?;
                let c_str: String = row.get(5)?;
                let u_str: String = row.get(6)?;
                Ok((id, title, description, query, meta_str, c_str, u_str))
            })
            .map_err(|e| LoomaError::Database(e.to_string()))?;

        let mut results = Vec::new();
        for item in col_iter {
            let (id, title, description, query, meta_str, c_str, u_str) =
                item.map_err(|e| LoomaError::Database(e.to_string()))?;

            let metadata = serde_json::from_str(&meta_str).unwrap_or(Value::Object(Default::default()));
            let created_at = DateTime::parse_from_rfc3339(&c_str)
                .map(|d| d.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());
            let updated_at = DateTime::parse_from_rfc3339(&u_str)
                .map(|d| d.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());

            results.push(Collection {
                id,
                title,
                description,
                query,
                metadata,
                created_at,
                updated_at,
            });
        }
        Ok(results)
    }

    fn get_collection_by_id(&self, id: &str) -> LoomaResult<Option<Collection>> {
        let conn = self.conn.lock();
        let mut stmt = conn
            .prepare("SELECT id, title, description, query, metadata_json, created_at, updated_at FROM collections WHERE id = ?1")
            .map_err(|e| LoomaError::Database(e.to_string()))?;

        let maybe_row = stmt
            .query_row(params![id], |row| {
                let id: String = row.get(0)?;
                let title: String = row.get(1)?;
                let description: Option<String> = row.get(2)?;
                let query: Option<String> = row.get(3)?;
                let meta_str: String = row.get(4)?;
                let c_str: String = row.get(5)?;
                let u_str: String = row.get(6)?;
                Ok((id, title, description, query, meta_str, c_str, u_str))
            })
            .optional()
            .map_err(|e| LoomaError::Database(e.to_string()))?;

        match maybe_row {
            Some((id, title, description, query, meta_str, c_str, u_str)) => {
                let metadata = serde_json::from_str(&meta_str).unwrap_or(Value::Object(Default::default()));
                let created_at = DateTime::parse_from_rfc3339(&c_str)
                    .map(|d| d.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());
                let updated_at = DateTime::parse_from_rfc3339(&u_str)
                    .map(|d| d.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());

                Ok(Some(Collection {
                    id,
                    title,
                    description,
                    query,
                    metadata,
                    created_at,
                    updated_at,
                }))
            }
            None => Ok(None),
        }
    }

    fn create_collection(&self, collection: &Collection) -> LoomaResult<()> {
        let conn = self.conn.lock();
        let meta_str = serde_json::to_string(&collection.metadata).unwrap_or_else(|_| "{}".to_string());
        conn.execute(
            "INSERT INTO collections (id, title, description, query, metadata_json, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                collection.id,
                collection.title,
                collection.description,
                collection.query,
                meta_str,
                collection.created_at.to_rfc3339(),
                collection.updated_at.to_rfc3339()
            ],
        )
        .map_err(|e| LoomaError::Database(format!("Failed to create collection: {e}")))?;
        Ok(())
    }

    fn add_item_to_collection(&self, collection_id: &str, item_id: &str, item_type: &str) -> LoomaResult<()> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT OR REPLACE INTO collection_items (collection_id, item_id, item_type, added_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![collection_id, item_id, item_type, Utc::now().to_rfc3339()],
        )
        .map_err(|e| LoomaError::Database(format!("Failed to add item to collection: {e}")))?;
        Ok(())
    }

    fn delete_collection(&self, id: &str) -> LoomaResult<bool> {
        let conn = self.conn.lock();
        let rows = conn
            .execute("DELETE FROM collections WHERE id = ?1", params![id])
            .map_err(|e| LoomaError::Database(e.to_string()))?;
        Ok(rows > 0)
    }

    fn list_items_for_collection(&self, collection_id: &str) -> LoomaResult<Vec<CollectionItem>> {
        let conn = self.conn.lock();
        let mut stmt = conn
            .prepare("SELECT collection_id, item_id, item_type, position, added_at FROM collection_items WHERE collection_id = ?1 ORDER BY position ASC, added_at DESC")
            .map_err(|e| LoomaError::Database(e.to_string()))?;

        let item_iter = stmt
            .query_map(params![collection_id], |row| {
                let c_id: String = row.get(0)?;
                let i_id: String = row.get(1)?;
                let i_type: String = row.get(2)?;
                let pos: i32 = row.get(3)?;
                let a_str: String = row.get(4)?;
                Ok((c_id, i_id, i_type, pos, a_str))
            })
            .map_err(|e| LoomaError::Database(e.to_string()))?;

        let mut results = Vec::new();
        for item in item_iter {
            let (collection_id, item_id, item_type, position, a_str) =
                item.map_err(|e| LoomaError::Database(e.to_string()))?;
            let added_at = DateTime::parse_from_rfc3339(&a_str)
                .map(|d| d.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());

            results.push(CollectionItem {
                collection_id,
                item_id,
                item_type,
                position,
                added_at,
            });
        }
        Ok(results)
    }

    fn remove_item_from_collection(&self, collection_id: &str, item_id: &str) -> LoomaResult<bool> {
        let conn = self.conn.lock();
        let rows = conn
            .execute(
                "DELETE FROM collection_items WHERE collection_id = ?1 AND item_id = ?2",
                params![collection_id, item_id],
            )
            .map_err(|e| LoomaError::Database(e.to_string()))?;
        Ok(rows > 0)
    }
}

impl TimelineService for LoomaDb {
    fn query_timeline(&self, filter: &TimelineFilter) -> LoomaResult<Vec<TimelineItem>> {
        let conn = self.conn.lock();
        let types = filter.item_types.as_deref().unwrap_or(&[]);
        let include_all = types.is_empty() || types.iter().any(|t| t == "all");

        let mut items = Vec::new();

        if include_all || types.iter().any(|t| t == "asset") {
            let mut stmt = conn
                .prepare(
                    "SELECT id, 'asset', modified_at, COALESCE(path, id), mime_type, kind, metadata_json
                     FROM assets WHERE status = 'active'",
                )
                .map_err(|e| LoomaError::Database(e.to_string()))?;

            let asset_iter = stmt
                .query_map([], |row| {
                    let id: String = row.get(0)?;
                    let item_type: String = row.get(1)?;
                    let time_str: String = row.get(2)?;
                    let title: String = row.get(3)?;
                    let desc: Option<String> = row.get(4)?;
                    let badge: Option<String> = row.get(5)?;
                    let meta_str: String = row.get(6)?;
                    Ok((id, item_type, time_str, title, desc, badge, meta_str))
                })
                .map_err(|e| LoomaError::Database(e.to_string()))?;

            for row in asset_iter {
                let (id, item_type, time_str, title, desc, badge, meta_str) =
                    row.map_err(|e| LoomaError::Database(e.to_string()))?;
                let timestamp = DateTime::parse_from_rfc3339(&time_str)
                    .map(|d| d.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());
                let metadata = serde_json::from_str(&meta_str).unwrap_or(Value::Object(Default::default()));
                items.push(TimelineItem {
                    id,
                    item_type,
                    timestamp,
                    title,
                    description: desc,
                    badge,
                    metadata,
                });
            }
        }

        if include_all || types.iter().any(|t| t == "entity") {
            let mut stmt = conn
                .prepare(
                    "SELECT id, 'entity', created_at, title, description, entity_type, properties_json
                     FROM entities",
                )
                .map_err(|e| LoomaError::Database(e.to_string()))?;

            let entity_iter = stmt
                .query_map([], |row| {
                    let id: String = row.get(0)?;
                    let item_type: String = row.get(1)?;
                    let time_str: String = row.get(2)?;
                    let title: String = row.get(3)?;
                    let desc: Option<String> = row.get(4)?;
                    let badge: Option<String> = row.get(5)?;
                    let meta_str: String = row.get(6)?;
                    Ok((id, item_type, time_str, title, desc, badge, meta_str))
                })
                .map_err(|e| LoomaError::Database(e.to_string()))?;

            for row in entity_iter {
                let (id, item_type, time_str, title, desc, badge, meta_str) =
                    row.map_err(|e| LoomaError::Database(e.to_string()))?;
                let timestamp = DateTime::parse_from_rfc3339(&time_str)
                    .map(|d| d.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());
                let metadata = serde_json::from_str(&meta_str).unwrap_or(Value::Object(Default::default()));
                items.push(TimelineItem {
                    id,
                    item_type,
                    timestamp,
                    title,
                    description: desc,
                    badge,
                    metadata,
                });
            }
        }

        if include_all || types.iter().any(|t| t == "memory") {
            let mut stmt = conn
                .prepare(
                    "SELECT id, 'memory', recorded_at, title, content, category, metadata_json
                     FROM memories",
                )
                .map_err(|e| LoomaError::Database(e.to_string()))?;

            let mem_iter = stmt
                .query_map([], |row| {
                    let id: String = row.get(0)?;
                    let item_type: String = row.get(1)?;
                    let time_str: String = row.get(2)?;
                    let title: String = row.get(3)?;
                    let desc: Option<String> = row.get(4)?;
                    let badge: Option<String> = row.get(5)?;
                    let meta_str: String = row.get(6)?;
                    Ok((id, item_type, time_str, title, desc, badge, meta_str))
                })
                .map_err(|e| LoomaError::Database(e.to_string()))?;

            for row in mem_iter {
                let (id, item_type, time_str, title, desc, badge, meta_str) =
                    row.map_err(|e| LoomaError::Database(e.to_string()))?;
                let timestamp = DateTime::parse_from_rfc3339(&time_str)
                    .map(|d| d.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());
                let metadata = serde_json::from_str(&meta_str).unwrap_or(Value::Object(Default::default()));
                items.push(TimelineItem {
                    id,
                    item_type,
                    timestamp,
                    title,
                    description: desc,
                    badge,
                    metadata,
                });
            }
        }

        if include_all || types.iter().any(|t| t == "experience") {
            let mut stmt = conn
                .prepare(
                    "SELECT e.id, 'experience', e.created_at, e.occurred_value, e.occurred_precision, e.event_type, e.title, ent.title, e.note, e.context_json, e.subject_id
                     FROM experience_entries e
                     LEFT JOIN entities ent ON e.subject_id = ent.id
                     WHERE e.visibility = 'normal'",
                )
                .map_err(|e| LoomaError::Database(e.to_string()))?;

            let exp_iter = stmt
                .query_map([], |row| {
                    let id: String = row.get(0)?;
                    let item_type: String = row.get(1)?;
                    let created_str: String = row.get(2)?;
                    let occurred_value: Option<String> = row.get(3)?;
                    let occurred_precision: String = row.get(4)?;
                    let event_type: String = row.get(5)?;
                    let exp_title: Option<String> = row.get(6)?;
                    let ent_title: Option<String> = row.get(7)?;
                    let note: Option<String> = row.get(8)?;
                    let ctx_str: String = row.get(9)?;
                    let subject_id: String = row.get(10)?;
                    Ok((
                        id,
                        item_type,
                        created_str,
                        occurred_value,
                        occurred_precision,
                        event_type,
                        exp_title,
                        ent_title,
                        note,
                        ctx_str,
                        subject_id,
                    ))
                })
                .map_err(|e| LoomaError::Database(e.to_string()))?;

            for row in exp_iter {
                let (
                    id,
                    item_type,
                    created_str,
                    occurred_val,
                    precision,
                    event_type,
                    exp_title,
                    ent_title,
                    note,
                    ctx_str,
                    subject_id,
                ) = row.map_err(|e| LoomaError::Database(e.to_string()))?;

                let created_dt = DateTime::parse_from_rfc3339(&created_str)
                    .map(|d| d.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());

                let timestamp = match precision.as_str() {
                    "instant" => occurred_val
                        .as_deref()
                        .and_then(|v| DateTime::parse_from_rfc3339(v).ok())
                        .map(|d| d.with_timezone(&Utc))
                        .unwrap_or(created_dt),
                    "day" => occurred_val
                        .as_deref()
                        .and_then(|v| chrono::NaiveDate::parse_from_str(v, "%Y-%m-%d").ok())
                        .and_then(|d| d.and_hms_opt(0, 0, 0))
                        .map(|dt| DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc))
                        .unwrap_or(created_dt),
                    "month" => occurred_val
                        .as_deref()
                        .and_then(|v| {
                            let parts: Vec<&str> = v.split('-').collect();
                            if parts.len() == 2 {
                                let y: i32 = parts[0].parse().ok()?;
                                let m: u32 = parts[1].parse().ok()?;
                                chrono::NaiveDate::from_ymd_opt(y, m, 1)
                            } else {
                                None
                            }
                        })
                        .and_then(|d| d.and_hms_opt(0, 0, 0))
                        .map(|dt| DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc))
                        .unwrap_or(created_dt),
                    "year" => occurred_val
                        .as_deref()
                        .and_then(|v| {
                            let y: i32 = v.parse().ok()?;
                            chrono::NaiveDate::from_ymd_opt(y, 1, 1)
                        })
                        .and_then(|d| d.and_hms_opt(0, 0, 0))
                        .map(|dt| DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc))
                        .unwrap_or(created_dt),
                    _ => created_dt,
                };

                let target_name = ent_title.unwrap_or_else(|| "Unknown".to_string());
                let title = exp_title.unwrap_or_else(|| format!("{}: {}", target_name, event_type));
                let badge = Some(event_type.clone());
                let context: Value = serde_json::from_str(&ctx_str).unwrap_or(Value::Object(Default::default()));

                let metadata = serde_json::json!({
                    "subject_id": subject_id,
                    "subject_title": target_name,
                    "event_type": event_type,
                    "occurred_precision": precision,
                    "occurred_value": occurred_val,
                    "created_at": created_str,
                    "is_occurred_time_unknown": precision == "unknown",
                    "context": context,
                });

                items.push(TimelineItem {
                    id,
                    item_type,
                    timestamp,
                    title,
                    description: note,
                    badge,
                    metadata,
                });
            }
        }

        // Sort chronologically descending
        items.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        // Apply pagination
        let offset = filter.offset.unwrap_or(0);
        let items: Vec<TimelineItem> = if offset < items.len() {
            items.into_iter().skip(offset).collect()
        } else {
            Vec::new()
        };

        let items: Vec<TimelineItem> = if let Some(limit) = filter.limit {
            items.into_iter().take(limit).collect()
        } else {
            items
        };

        Ok(items)
    }
}

impl BackupService for LoomaDb {
    fn generate_manifest(&self) -> LoomaResult<VaultManifest> {
        let vault_info = self.get_vault_info()?;
        let stats = self.get_vault_stats()?;
        let assets_count = self.count_assets()? as usize;
        let entities_count = self.count_entities()? as usize;
        let memories = self.list_memories(100000, 0)?;
        let collections = self.list_collections()?;
        let ext_refs = self.list_external_references(None)?;
        let experiences_count = self.count_experiences()? as usize;
        let roots = self.list_source_roots()?;
        let suppressions = self.list_suppressions(None)?;

        let conn = self.conn.lock();
        // Checkpoint WAL to flush all transaction pages to disk before checksum computation
        conn.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |_| Ok(()))
            .map_err(|e| LoomaError::Database(format!("Failed to checkpoint WAL: {e}")))?;

        let relations_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM relations", [], |r| r.get(0))
            .map_err(|e| LoomaError::Database(e.to_string()))?;

        let schema_version: i64 = conn
            .query_row("SELECT COALESCE(MAX(version), 4) FROM _schema_migrations", [], |r| r.get(0))
            .map_err(|e| LoomaError::Database(e.to_string()))?;

        let mut checksums = std::collections::HashMap::new();
        if self.db_path.exists() {
            let bytes = std::fs::read(&self.db_path)
                .map_err(|e| LoomaError::Vault(format!("Failed to read database file for checksum: {e}")))?;
            use sha2::{Sha256, Digest};
            let mut hasher = Sha256::new();
            hasher.update(&bytes);
            checksums.insert("vault.db".to_string(), format!("{:x}", hasher.finalize()));
        }

        Ok(VaultManifest {
            manifest_version: "2.0.0".to_string(),
            schema_version: schema_version as u32,
            created_at: Utc::now(),
            vault_info,
            stats,
            assets_count,
            entities_count,
            memories_count: memories.len(),
            collections_count: collections.len(),
            relations_count: relations_count as usize,
            external_references_count: ext_refs.len(),
            experiences_count,
            source_roots_count: roots.len(),
            asset_suppressions_count: suppressions.len(),
            checksums,
        })
    }

    fn export_backup(&self, destination_dir: &Path) -> LoomaResult<BackupResult> {
        let start = std::time::Instant::now();
        let mut manifest = self.generate_manifest()?;

        let timestamp_str = Utc::now().format("%Y%m%d_%H%M%S").to_string();
        let backup_folder_name = format!("looma_vault_backup_{}", timestamp_str);
        let backup_path = destination_dir.join(backup_folder_name);

        std::fs::create_dir_all(&backup_path)
            .map_err(|e| LoomaError::Vault(format!("Failed to create backup directory: {e}")))?;

        // 1. Fetch all data
        let assets = self.query_assets(&AssetFilter::default())?;
        let entities = self.list_entities(&EntityFilter::default())?;
        let memories = self.list_memories(100000, 0)?;
        let collections = self.list_collections()?;
        let external_references = self.list_external_references(None)?;
        let experiences = self.list_experiences(&ExperienceFilter::default())?;
        let mut experience_links = Vec::new();
        for exp in &experiences {
            let links = self.list_experience_links(&exp.id)?;
            experience_links.extend(links);
        }

        let mut relations = Vec::new();
        {
            let conn = self.conn.lock();
            let mut stmt = conn
                .prepare("SELECT id, source_id, source_type, relation_type, target_id, target_type, metadata_json, created_at FROM relations")
                .map_err(|e| LoomaError::Database(e.to_string()))?;
            let rel_iter = stmt
                .query_map([], |row| {
                    let id: String = row.get(0)?;
                    let s_id: String = row.get(1)?;
                    let s_type: String = row.get(2)?;
                    let r_type: String = row.get(3)?;
                    let t_id: String = row.get(4)?;
                    let t_type: String = row.get(5)?;
                    let meta_str: String = row.get(6)?;
                    let c_str: String = row.get(7)?;
                    Ok((id, s_id, s_type, r_type, t_id, t_type, meta_str, c_str))
                })
                .map_err(|e| LoomaError::Database(e.to_string()))?;

            for r in rel_iter {
                let (id, source_id, source_type, relation_type, target_id, target_type, meta_str, c_str) =
                    r.map_err(|e| LoomaError::Database(e.to_string()))?;
                let metadata = serde_json::from_str(&meta_str).unwrap_or(Value::Object(Default::default()));
                let created_at = DateTime::parse_from_rfc3339(&c_str)
                    .map(|d| d.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());
                relations.push(Relation {
                    id,
                    source_id,
                    source_type,
                    relation_type,
                    target_id,
                    target_type,
                    metadata,
                    created_at,
                });
            }
        }

        // 2. Write vault_dump.json
        let source_roots = self.list_source_roots()?;
        let asset_suppressions = self.list_suppressions(None)?;
        let mut asset_lifecycles = Vec::new();
        {
            let conn = self.conn.lock();
            let mut stmt = conn.prepare(
                "SELECT asset_id, root_id, relative_path, availability, visibility, tracking, last_verified_at, untracked_at, untracked_reason
                 FROM asset_lifecycle"
            ).map_err(|e| LoomaError::Database(e.to_string()))?;
            let rows = stmt.query_map([], |row| {
                let last_verified_str: Option<String> = row.get(6)?;
                let untracked_str: Option<String> = row.get(7)?;
                Ok(AssetLifecycle {
                    asset_id: row.get(0)?,
                    root_id: row.get(1)?,
                    relative_path: row.get(2)?,
                    availability: row.get(3)?,
                    visibility: row.get(4)?,
                    tracking: row.get(5)?,
                    last_verified_at: last_verified_str.and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|d| d.with_timezone(&Utc))),
                    untracked_at: untracked_str.and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|d| d.with_timezone(&Utc))),
                    untracked_reason: row.get(8)?,
                })
            }).map_err(|e| LoomaError::Database(e.to_string()))?;
            for r in rows {
                asset_lifecycles.push(r.map_err(|e| LoomaError::Database(e.to_string()))?);
            }
        }

        let dump = VaultExportDump {
            manifest: manifest.clone(),
            assets,
            entities,
            memories: memories.clone(),
            collections,
            relations,
            external_references,
            experiences,
            experience_links,
            source_roots,
            asset_lifecycles,
            asset_suppressions,
        };
        let dump_bytes = serde_json::to_vec_pretty(&dump)
            .map_err(LoomaError::Serialization)?;
        
        // Compute checksum of vault_dump.json
        {
            use sha2::{Sha256, Digest};
            let mut hasher = Sha256::new();
            hasher.update(&dump_bytes);
            manifest.checksums.insert("vault_dump.json".to_string(), format!("{:x}", hasher.finalize()));
        }

        std::fs::write(backup_path.join("vault_dump.json"), &dump_bytes)
            .map_err(|e| LoomaError::Vault(format!("Failed to write vault_dump.json: {e}")))?;

        // 3. Write manifest.json with updated checksums
        let manifest_json = serde_json::to_string_pretty(&manifest)
            .map_err(LoomaError::Serialization)?;
        std::fs::write(backup_path.join("manifest.json"), manifest_json)
            .map_err(|e| LoomaError::Vault(format!("Failed to write manifest.json: {e}")))?;

        // 4. Export human-readable Markdown notes in memories/ subfolder
        let memories_dir = backup_path.join("memories");
        std::fs::create_dir_all(&memories_dir)
            .map_err(|e| LoomaError::Vault(format!("Failed to create memories backup dir: {e}")))?;

        for mem in memories {
            let sanitized_title: String = mem
                .title
                .chars()
                .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' || c == ' ' { c } else { '_' })
                .collect();
            let sanitized_title = sanitized_title.trim();
            let filename = if sanitized_title.is_empty() {
                format!("{}.md", mem.id)
            } else {
                format!("{}_{}.md", sanitized_title, &mem.id[..mem.id.len().min(8)])
            };

            let md_content = format!(
                "---\nid: {}\ntitle: {}\ncategory: {}\nrecorded_at: {}\n---\n\n{}",
                mem.id,
                mem.title,
                mem.category.unwrap_or_default(),
                mem.recorded_at.to_rfc3339(),
                mem.content
            );
            std::fs::write(memories_dir.join(filename), md_content)
                .map_err(|e| LoomaError::Vault(format!("Failed to write memory export file: {e}")))?;
        }

        let duration_ms = start.elapsed().as_millis() as u64;

        Ok(BackupResult {
            backup_path: backup_path.to_string_lossy().to_string(),
            manifest,
            duration_ms,
        })
    }

    fn restore_backup(&self, backup_dir_or_file: &Path) -> LoomaResult<RestoreResult> {
        let start = std::time::Instant::now();
        let target_dir = if backup_dir_or_file.is_dir() {
            backup_dir_or_file.to_path_buf()
        } else {
            backup_dir_or_file.parent().unwrap_or(backup_dir_or_file).to_path_buf()
        };

        // 1. Strict Manifest & Checksum Pre-validation
        let manifest_path = target_dir.join("manifest.json");
        if !manifest_path.exists() {
            return Err(LoomaError::Validation(format!(
                "Backup archive invalid: manifest.json is required for vault restoration but was not found in {:?}",
                target_dir
            )));
        }

        let manifest_str = std::fs::read_to_string(&manifest_path)
            .map_err(|e| LoomaError::Vault(format!("Failed to read manifest.json: {e}")))?;
        let manifest: VaultManifest = serde_json::from_str(&manifest_str)
            .map_err(LoomaError::Serialization)?;

        if manifest.schema_version > 4 {
            return Err(LoomaError::Validation(format!(
                "Unsupported backup schema version: {} (maximum supported schema version is 4)",
                manifest.schema_version
            )));
        }

        let dump_path = target_dir.join("vault_dump.json");
        if !dump_path.exists() {
            return Err(LoomaError::Vault(format!(
                "Backup archive invalid: vault_dump.json not found in {:?}",
                target_dir
            )));
        }

        let dump_bytes = std::fs::read(&dump_path)
            .map_err(|e| LoomaError::Vault(format!("Failed to read vault_dump.json: {e}")))?;

        let expected_checksum = manifest.checksums.get("vault_dump.json").ok_or_else(|| {
            LoomaError::Validation(
                "Backup archive invalid: manifest.json is missing required checksum for vault_dump.json".to_string(),
            )
        })?;

        {
            use sha2::{Sha256, Digest};
            let mut hasher = Sha256::new();
            hasher.update(&dump_bytes);
            let actual_checksum = format!("{:x}", hasher.finalize());

            if !actual_checksum.eq_ignore_ascii_case(expected_checksum) {
                return Err(LoomaError::Validation(format!(
                    "Backup archive corrupted: vault_dump.json checksum mismatch (expected {}, found {})",
                    expected_checksum, actual_checksum
                )));
            }
        }

        let dump: VaultExportDump = serde_json::from_slice(&dump_bytes)
            .map_err(LoomaError::Serialization)?;

        // 2. Execute Restoration Within a Single Unified Database Transaction
        let mut conn = self.conn.lock();
        let tx = conn.transaction().map_err(|e| LoomaError::Database(e.to_string()))?;

        let mut restored_assets = 0;
        let mut restored_entities = 0;
        let mut restored_memories = 0;
        let mut restored_collections = 0;
        let mut restored_relations = 0;
        let mut restored_external_references = 0;
        let mut restored_experiences = 0;
        let mut restored_source_roots = 0;
        let mut restored_asset_suppressions = 0;

        for asset in dump.assets {
            let kind_str = serde_json::to_string(&asset.kind).unwrap_or_default().trim_matches('"').to_string();
            let source_str = serde_json::to_string(&asset.source).unwrap_or_default().trim_matches('"').to_string();
            let status_str = serde_json::to_string(&asset.status).unwrap_or_default().trim_matches('"').to_string();
            let meta_str = serde_json::to_string(&asset.metadata).unwrap_or_else(|_| "{}".to_string());
            tx.execute(
                "INSERT INTO assets (id, kind, source, path, size, hash, mime_type, metadata_json, status,
                                    created_at, modified_at, indexed_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
                 ON CONFLICT(id) DO UPDATE SET
                    kind = excluded.kind,
                    source = excluded.source,
                    path = excluded.path,
                    size = excluded.size,
                    hash = excluded.hash,
                    mime_type = excluded.mime_type,
                    metadata_json = excluded.metadata_json,
                    status = excluded.status,
                    modified_at = excluded.modified_at,
                    indexed_at = excluded.indexed_at",
                params![
                    asset.id,
                    kind_str,
                    source_str,
                    asset.path,
                    asset.size.map(|s| s as i64),
                    asset.hash,
                    asset.mime_type,
                    meta_str,
                    status_str,
                    asset.created_at.to_rfc3339(),
                    asset.modified_at.to_rfc3339(),
                    asset.indexed_at.to_rfc3339(),
                ],
            ).map_err(|e| LoomaError::Database(e.to_string()))?;
            restored_assets += 1;
        }

        for entity in dump.entities {
            let props_str = serde_json::to_string(&entity.properties).unwrap_or_else(|_| "{}".to_string());
            tx.execute(
                "INSERT INTO entities (id, entity_type, title, description, properties_json, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                 ON CONFLICT(id) DO UPDATE SET
                    entity_type = excluded.entity_type,
                    title = excluded.title,
                    description = excluded.description,
                    properties_json = excluded.properties_json,
                    updated_at = excluded.updated_at",
                params![
                    entity.id,
                    entity.entity_type,
                    entity.title,
                    entity.description,
                    props_str,
                    entity.created_at.to_rfc3339(),
                    entity.updated_at.to_rfc3339(),
                ],
            ).map_err(|e| LoomaError::Database(e.to_string()))?;
            restored_entities += 1;
        }

        for memory in dump.memories {
            let meta_str = serde_json::to_string(&memory.metadata).unwrap_or_else(|_| "{}".to_string());
            tx.execute(
                "INSERT INTO memories (id, title, content, category, metadata_json, recorded_at, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                 ON CONFLICT(id) DO UPDATE SET
                    title = excluded.title,
                    content = excluded.content,
                    category = excluded.category,
                    metadata_json = excluded.metadata_json,
                    recorded_at = excluded.recorded_at,
                    updated_at = excluded.updated_at",
                params![
                    memory.id,
                    memory.title,
                    memory.content,
                    memory.category,
                    meta_str,
                    memory.recorded_at.to_rfc3339(),
                    memory.created_at.to_rfc3339(),
                    memory.updated_at.to_rfc3339(),
                ],
            ).map_err(|e| LoomaError::Database(e.to_string()))?;
            restored_memories += 1;
        }

        for collection in dump.collections {
            let meta_str = serde_json::to_string(&collection.metadata).unwrap_or_else(|_| "{}".to_string());
            tx.execute(
                "INSERT INTO collections (id, title, description, query, metadata_json, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                 ON CONFLICT(id) DO UPDATE SET
                    title = excluded.title,
                    description = excluded.description,
                    query = excluded.query,
                    metadata_json = excluded.metadata_json,
                    updated_at = excluded.updated_at",
                params![
                    collection.id,
                    collection.title,
                    collection.description,
                    collection.query,
                    meta_str,
                    collection.created_at.to_rfc3339(),
                    collection.updated_at.to_rfc3339(),
                ],
            ).map_err(|e| LoomaError::Database(e.to_string()))?;
            restored_collections += 1;
        }

        for relation in dump.relations {
            let meta_str = serde_json::to_string(&relation.metadata).unwrap_or_else(|_| "{}".to_string());
            tx.execute(
                "INSERT INTO relations (id, source_id, source_type, relation_type, target_id, target_type, metadata_json, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                 ON CONFLICT(id) DO UPDATE SET
                    source_id = excluded.source_id,
                    source_type = excluded.source_type,
                    relation_type = excluded.relation_type,
                    target_id = excluded.target_id,
                    target_type = excluded.target_type,
                    metadata_json = excluded.metadata_json",
                params![
                    relation.id,
                    relation.source_id,
                    relation.source_type,
                    relation.relation_type,
                    relation.target_id,
                    relation.target_type,
                    meta_str,
                    relation.created_at.to_rfc3339(),
                ],
            ).map_err(|e| LoomaError::Database(e.to_string()))?;
            restored_relations += 1;
        }

        for ext_ref in dump.external_references {
            let meta_str = serde_json::to_string(&ext_ref.metadata).unwrap_or_else(|_| "{}".to_string());
            tx.execute(
                "INSERT INTO external_references (id, entity_id, provider, title, url, description, metadata_json, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                 ON CONFLICT(id) DO UPDATE SET
                    entity_id = excluded.entity_id,
                    provider = excluded.provider,
                    title = excluded.title,
                    url = excluded.url,
                    description = excluded.description,
                    metadata_json = excluded.metadata_json,
                    updated_at = excluded.updated_at",
                params![
                    ext_ref.id,
                    ext_ref.entity_id,
                    ext_ref.provider,
                    ext_ref.title,
                    ext_ref.url,
                    ext_ref.description,
                    meta_str,
                    ext_ref.created_at.to_rfc3339(),
                    ext_ref.updated_at.to_rfc3339(),
                ],
            ).map_err(|e| LoomaError::Database(e.to_string()))?;
            restored_external_references += 1;
        }

        for exp in dump.experiences {
            let ctx_str = serde_json::to_string(&exp.context_json).unwrap_or_else(|_| "{}".to_string());
            tx.execute(
                "INSERT INTO experience_entries (id, subject_type, subject_id, event_type, title, note, occurred_value, occurred_precision, context_json, visibility, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
                 ON CONFLICT(id) DO UPDATE SET
                    subject_type = excluded.subject_type,
                    subject_id = excluded.subject_id,
                    event_type = excluded.event_type,
                    title = excluded.title,
                    note = excluded.note,
                    occurred_value = excluded.occurred_value,
                    occurred_precision = excluded.occurred_precision,
                    context_json = excluded.context_json,
                    visibility = excluded.visibility,
                    updated_at = excluded.updated_at",
                params![
                    exp.id,
                    exp.subject_type,
                    exp.subject_id,
                    exp.event_type,
                    exp.title,
                    exp.note,
                    exp.occurred_value,
                    exp.occurred_precision,
                    ctx_str,
                    exp.visibility,
                    exp.created_at.to_rfc3339(),
                    exp.updated_at.to_rfc3339(),
                ],
            ).map_err(|e| LoomaError::Database(e.to_string()))?;
            restored_experiences += 1;
        }

        for link in dump.experience_links {
            tx.execute(
                "INSERT OR REPLACE INTO experience_links (experience_id, linked_type, linked_id, relation_type, position)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    link.experience_id,
                    link.linked_type,
                    link.linked_id,
                    link.relation_type,
                    link.position,
                ],
            ).map_err(|e| LoomaError::Database(e.to_string()))?;
        }

        for root in dump.source_roots {
            let settings_str = serde_json::to_string(&root.settings_json).unwrap_or_else(|_| "{}".to_string());
            tx.execute(
                "INSERT INTO source_roots (id, label, path, source_kind, enabled, state, last_successful_scan_at, last_seen_at, settings_json)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                 ON CONFLICT(id) DO UPDATE SET
                    label = excluded.label,
                    path = excluded.path,
                    source_kind = excluded.source_kind,
                    enabled = excluded.enabled,
                    state = excluded.state,
                    last_successful_scan_at = excluded.last_successful_scan_at,
                    last_seen_at = excluded.last_seen_at,
                    settings_json = excluded.settings_json",
                params![
                    root.id,
                    root.label,
                    root.path,
                    root.source_kind,
                    if root.enabled { 1 } else { 0 },
                    root.state,
                    root.last_successful_scan_at.map(|d| d.to_rfc3339()),
                    root.last_seen_at.map(|d| d.to_rfc3339()),
                    settings_str,
                ],
            ).map_err(|e| LoomaError::Database(e.to_string()))?;
            restored_source_roots += 1;
        }

        for lc in dump.asset_lifecycles {
            tx.execute(
                "INSERT INTO asset_lifecycle (asset_id, root_id, relative_path, availability, visibility, tracking, last_verified_at, untracked_at, untracked_reason)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                 ON CONFLICT(asset_id) DO UPDATE SET
                   root_id = excluded.root_id,
                   relative_path = excluded.relative_path,
                   availability = excluded.availability,
                   visibility = excluded.visibility,
                   tracking = excluded.tracking,
                   last_verified_at = excluded.last_verified_at,
                   untracked_at = excluded.untracked_at,
                   untracked_reason = excluded.untracked_reason",
                params![
                    lc.asset_id,
                    lc.root_id,
                    lc.relative_path,
                    lc.availability,
                    lc.visibility,
                    lc.tracking,
                    lc.last_verified_at.map(|d| d.to_rfc3339()),
                    lc.untracked_at.map(|d| d.to_rfc3339()),
                    lc.untracked_reason,
                ],
            ).map_err(|e| LoomaError::Database(e.to_string()))?;
        }

        for sup in dump.asset_suppressions {
            let clean_rel = sup.relative_path.trim_start_matches('/').trim_start_matches('\\');
            let now = Utc::now().to_rfc3339();
            tx.execute(
                "INSERT INTO asset_scan_suppressions (id, root_id, relative_path, reason, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(root_id, relative_path) DO UPDATE SET reason = excluded.reason",
                params![sup.id, sup.root_id, clean_rel, sup.reason, now],
            ).map_err(|e| LoomaError::Database(e.to_string()))?;
            restored_asset_suppressions += 1;
        }

        // Commit the transaction atomically
        tx.commit().map_err(|e| LoomaError::Database(format!("Failed to commit restore transaction: {e}")))?;

        let duration_ms = start.elapsed().as_millis() as u64;

        Ok(RestoreResult {
            restored_assets,
            restored_entities,
            restored_memories,
            restored_collections,
            restored_relations,
            restored_external_references,
            restored_experiences,
            restored_source_roots,
            restored_asset_suppressions,
            duration_ms,
        })
    }

    fn doctor_inspect(&self) -> LoomaResult<VaultDoctorReport> {
        let roots = self.list_source_roots().unwrap_or_default();
        let mut offline_roots: Vec<SourceRoot> = Vec::new();
        let mut root_map: std::collections::HashMap<String, SourceRoot> = std::collections::HashMap::new();
        for root in roots {
            let p = Path::new(&root.path);
            let is_offline = !p.exists() || !p.is_dir() || root.state == "offline";
            if is_offline {
                offline_roots.push(root.clone());
            }
            root_map.insert(root.id.clone(), root);
        }

        let assets = self.query_assets(&AssetFilter::default())?;
        let total_assets = assets.len();
        let mut active_assets = 0;
        let mut missing_assets = Vec::new();

        for asset in assets {
            if let Some(ref p) = asset.path {
                if Path::new(p).exists() {
                    active_assets += 1;
                    continue;
                }

                // If path does not exist on disk, determine if it is protected under an offline SourceRoot
                let mut is_protected_offline = false;

                // 1. Check direct lifecycle root association
                if let Ok(Some(lc)) = self.get_asset_lifecycle(&asset.id) {
                    if let Some(ref rid) = lc.root_id {
                        if let Some(root) = root_map.get(rid) {
                            let root_p = Path::new(&root.path);
                            if !root_p.exists() || !root_p.is_dir() || root.state == "offline" {
                                is_protected_offline = true;
                            }
                        }
                    }
                }

                // 2. Fallback for unlinked assets: use boundary-safe & case-insensitive root check
                if !is_protected_offline {
                    for off_root in &offline_roots {
                        if looma_core::facade::is_path_under_root(p, &off_root.path) {
                            is_protected_offline = true;
                            break;
                        }
                    }
                }

                if is_protected_offline {
                    // Drive or root is offline; do NOT treat as missing per Section 9.4 and T09!
                    active_assets += 1;
                } else {
                    missing_assets.push(p.clone());
                }
            } else {
                active_assets += 1;
            }
        }

        let db_size = std::fs::metadata(&self.db_path).map(|m| m.len()).unwrap_or(0);

        let conn = self.conn.lock();
        let integrity_check: String = conn
            .query_row("PRAGMA integrity_check", [], |r| r.get(0))
            .unwrap_or_else(|e| e.to_string());
        let integrity_ok = integrity_check == "ok";

        Ok(VaultDoctorReport {
            total_assets,
            active_assets,
            missing_assets,
            database_size_bytes: db_size,
            integrity_ok,
            integrity_message: integrity_check,
        })
    }

    fn doctor_cleanup_missing(&self) -> LoomaResult<usize> {
        let report = self.doctor_inspect()?;
        let missing_count = report.missing_assets.len();

        if missing_count > 0 {
            let conn = self.conn.lock();
            for missing_path in &report.missing_assets {
                let _ = conn.execute(
                    "UPDATE assets SET status = 'missing' WHERE path = ?1",
                    params![missing_path],
                );
            }
        }

        // Run VACUUM to defragment and compact SQLite database
        {
            let conn = self.conn.lock();
            conn.execute_batch("VACUUM;").ok();
        }

        Ok(missing_count)
    }
}

impl AuditService for LoomaDb {
    fn record_audit(&self, log: &AuditLog) -> LoomaResult<()> {
        let conn = self.conn.lock();
        let details_json = serde_json::to_string(&log.details)
            .unwrap_or_else(|_| "{}".to_string());
        conn.execute(
            "INSERT INTO audit_logs (id, timestamp, actor, operation, target_type, target_id, result, details_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                log.id,
                log.timestamp.to_rfc3339(),
                log.actor,
                log.operation,
                log.target_type,
                log.target_id,
                log.result,
                details_json
            ],
        )
        .map_err(|e| LoomaError::Database(format!("Failed to record audit log: {e}")))?;
        Ok(())
    }

    fn list_recent_audits(&self, limit: usize) -> LoomaResult<Vec<AuditLog>> {
        let conn = self.conn.lock();
        let mut stmt = conn
            .prepare(
                "SELECT id, timestamp, actor, operation, target_type, target_id, result, details_json
                 FROM audit_logs
                 ORDER BY timestamp DESC
                 LIMIT ?1",
            )
            .map_err(|e| LoomaError::Database(format!("Failed to prepare audit query: {e}")))?;

        let rows = stmt
            .query_map(params![limit as i64], |row| {
                let id: String = row.get(0)?;
                let ts_str: String = row.get(1)?;
                let actor: String = row.get(2)?;
                let operation: String = row.get(3)?;
                let target_type: String = row.get(4)?;
                let target_id: String = row.get(5)?;
                let result: String = row.get(6)?;
                let details_json_str: String = row.get(7)?;

                let timestamp = DateTime::parse_from_rfc3339(&ts_str)
                    .map(|d| d.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());
                let details = serde_json::from_str(&details_json_str).unwrap_or(serde_json::json!({}));

                Ok(AuditLog {
                    id,
                    timestamp,
                    actor,
                    operation,
                    target_type,
                    target_id,
                    result,
                    details,
                })
            })
            .map_err(|e| LoomaError::Database(format!("Failed to query audit logs: {e}")))?;

        let mut logs = Vec::new();
        for r in rows {
            if let Ok(l) = r {
                logs.push(l);
            }
        }
        Ok(logs)
    }
}

impl ExternalReferenceService for LoomaDb {
    fn list_external_references(&self, entity_id: Option<&str>) -> LoomaResult<Vec<ExternalReference>> {
        let conn = self.conn.lock();
        let mut query = "SELECT id, entity_id, provider, title, url, description, metadata_json, created_at, updated_at FROM external_references".to_string();
        if entity_id.is_some() {
            query.push_str(" WHERE entity_id = ?1");
        }
        query.push_str(" ORDER BY created_at DESC");

        let mut stmt = conn.prepare(&query)
            .map_err(|e| LoomaError::Database(format!("Failed to prepare ext refs query: {e}")))?;

        let rows: Vec<ExternalReference> = match entity_id {
            Some(eid) => {
                stmt.query_map(params![eid], |row| {
                    let id: String = row.get(0)?;
                    let entity_id: Option<String> = row.get(1)?;
                    let provider: String = row.get(2)?;
                    let title: String = row.get(3)?;
                    let url: String = row.get(4)?;
                    let description: Option<String> = row.get(5)?;
                    let meta_str: String = row.get(6)?;
                    let c_str: String = row.get(7)?;
                    let u_str: String = row.get(8)?;

                    let metadata = serde_json::from_str(&meta_str).unwrap_or(serde_json::json!({}));
                    let created_at = DateTime::parse_from_rfc3339(&c_str).map(|d| d.with_timezone(&Utc)).unwrap_or_else(|_| Utc::now());
                    let updated_at = DateTime::parse_from_rfc3339(&u_str).map(|d| d.with_timezone(&Utc)).unwrap_or_else(|_| Utc::now());

                    Ok(ExternalReference { id, entity_id, provider, title, url, description, metadata, created_at, updated_at })
                }).map_err(|e| LoomaError::Database(e.to_string()))?.filter_map(|r| r.ok()).collect()
            }
            None => {
                stmt.query_map([], |row| {
                    let id: String = row.get(0)?;
                    let entity_id: Option<String> = row.get(1)?;
                    let provider: String = row.get(2)?;
                    let title: String = row.get(3)?;
                    let url: String = row.get(4)?;
                    let description: Option<String> = row.get(5)?;
                    let meta_str: String = row.get(6)?;
                    let c_str: String = row.get(7)?;
                    let u_str: String = row.get(8)?;

                    let metadata = serde_json::from_str(&meta_str).unwrap_or(serde_json::json!({}));
                    let created_at = DateTime::parse_from_rfc3339(&c_str).map(|d| d.with_timezone(&Utc)).unwrap_or_else(|_| Utc::now());
                    let updated_at = DateTime::parse_from_rfc3339(&u_str).map(|d| d.with_timezone(&Utc)).unwrap_or_else(|_| Utc::now());

                    Ok(ExternalReference { id, entity_id, provider, title, url, description, metadata, created_at, updated_at })
                }).map_err(|e| LoomaError::Database(e.to_string()))?.filter_map(|r| r.ok()).collect()
            }
        };

        Ok(rows)
    }

    fn get_external_reference_by_id(&self, id: &str) -> LoomaResult<Option<ExternalReference>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, entity_id, provider, title, url, description, metadata_json, created_at, updated_at
             FROM external_references WHERE id = ?1",
        ).map_err(|e| LoomaError::Database(e.to_string()))?;

        let res = stmt.query_row(params![id], |row| {
            let id: String = row.get(0)?;
            let entity_id: Option<String> = row.get(1)?;
            let provider: String = row.get(2)?;
            let title: String = row.get(3)?;
            let url: String = row.get(4)?;
            let description: Option<String> = row.get(5)?;
            let meta_str: String = row.get(6)?;
            let c_str: String = row.get(7)?;
            let u_str: String = row.get(8)?;

            let metadata = serde_json::from_str(&meta_str).unwrap_or(serde_json::json!({}));
            let created_at = DateTime::parse_from_rfc3339(&c_str).map(|d| d.with_timezone(&Utc)).unwrap_or_else(|_| Utc::now());
            let updated_at = DateTime::parse_from_rfc3339(&u_str).map(|d| d.with_timezone(&Utc)).unwrap_or_else(|_| Utc::now());

            Ok(ExternalReference { id, entity_id, provider, title, url, description, metadata, created_at, updated_at })
        }).optional().map_err(|e| LoomaError::Database(e.to_string()))?;

        Ok(res)
    }

    fn create_external_reference(&self, reference: &ExternalReference) -> LoomaResult<()> {
        let mut conn = self.conn.lock();
        let tx = conn.transaction().map_err(|e| LoomaError::Database(e.to_string()))?;

        // If new reference is primary and has an entity_id, demote any existing primary for that entity
        if reference.is_primary() {
            if let Some(ref eid) = reference.entity_id {
                let mut stmt = tx.prepare(
                    "SELECT id, metadata_json FROM external_references WHERE entity_id = ?1"
                ).map_err(|e| LoomaError::Database(e.to_string()))?;

                let existing: Vec<(String, String)> = stmt.query_map(params![eid], |row| {
                    Ok((row.get(0)?, row.get(1)?))
                }).map_err(|e| LoomaError::Database(e.to_string()))?
                .filter_map(|r| r.ok())
                .collect();

                drop(stmt);

                let now_str = Utc::now().to_rfc3339();
                for (id, meta_str) in existing {
                    let mut meta: serde_json::Value = serde_json::from_str(&meta_str).unwrap_or_else(|_| serde_json::json!({}));
                    if meta.get("is_primary").and_then(|v| v.as_bool()).unwrap_or(false) {
                        if let Some(obj) = meta.as_object_mut() {
                            obj.insert("is_primary".to_string(), serde_json::json!(false));
                        }
                        let new_meta_str = serde_json::to_string(&meta).unwrap_or_else(|_| "{}".to_string());
                        tx.execute(
                            "UPDATE external_references SET metadata_json = ?1, updated_at = ?2 WHERE id = ?3",
                            params![new_meta_str, now_str, id],
                        ).map_err(|e| LoomaError::Database(e.to_string()))?;
                    }
                }
            }
        }

        let meta_str = serde_json::to_string(&reference.metadata).unwrap_or_else(|_| "{}".to_string());
        tx.execute(
            "INSERT INTO external_references (id, entity_id, provider, title, url, description, metadata_json, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                reference.id,
                reference.entity_id,
                reference.provider,
                reference.title,
                reference.url,
                reference.description,
                meta_str,
                reference.created_at.to_rfc3339(),
                reference.updated_at.to_rfc3339(),
            ],
        ).map_err(|e| LoomaError::Database(format!("Failed to insert external reference: {e}")))?;

        tx.commit().map_err(|e| LoomaError::Database(e.to_string()))?;
        Ok(())
    }

    fn update_external_reference(&self, reference: &ExternalReference) -> LoomaResult<()> {
        let conn = self.conn.lock();
        let meta_str = serde_json::to_string(&reference.metadata).unwrap_or_else(|_| "{}".to_string());
        conn.execute(
            "UPDATE external_references
             SET entity_id = ?1, provider = ?2, title = ?3, url = ?4, description = ?5, metadata_json = ?6, updated_at = ?7
             WHERE id = ?8",
            params![
                reference.entity_id,
                reference.provider,
                reference.title,
                reference.url,
                reference.description,
                meta_str,
                reference.updated_at.to_rfc3339(),
                reference.id,
            ],
        ).map_err(|e| LoomaError::Database(e.to_string()))?;
        Ok(())
    }

    fn delete_external_reference(&self, id: &str) -> LoomaResult<bool> {
        let conn = self.conn.lock();
        let count = conn.execute("DELETE FROM external_references WHERE id = ?1", params![id])
            .map_err(|e| LoomaError::Database(e.to_string()))?;
        Ok(count > 0)
    }

    fn set_primary_external_reference(&self, entity_id: &str, reference_id: &str) -> LoomaResult<()> {
        let mut conn = self.conn.lock();
        let tx = conn.transaction().map_err(|e| LoomaError::Database(e.to_string()))?;

        // 1. Validate entity exists
        let entity_count: i64 = tx.query_row(
            "SELECT COUNT(*) FROM entities WHERE id = ?1",
            params![entity_id],
            |r| r.get(0),
        ).map_err(|e| LoomaError::Database(e.to_string()))?;
        if entity_count == 0 {
            return Err(LoomaError::NotFound(format!("Entity not found: {entity_id}")));
        }

        // 2. Validate reference exists and belongs to entity
        let mut ref_stmt = tx.prepare(
            "SELECT entity_id, metadata_json FROM external_references WHERE id = ?1"
        ).map_err(|e| LoomaError::Database(e.to_string()))?;

        let maybe_ref = ref_stmt.query_row(params![reference_id], |row| {
            let eid: Option<String> = row.get(0)?;
            let meta_str: String = row.get(1)?;
            Ok((eid, meta_str))
        }).optional().map_err(|e| LoomaError::Database(e.to_string()))?;

        drop(ref_stmt);

        let (owner_eid, _) = match maybe_ref {
            Some(res) => res,
            None => return Err(LoomaError::NotFound(format!("External reference not found: {reference_id}"))),
        };

        if owner_eid.as_deref() != Some(entity_id) {
            return Err(LoomaError::Validation(format!(
                "External reference {reference_id} does not belong to entity {entity_id}"
            )));
        }

        // 3. Atomically update all references for entity_id:
        // Set all to false, and set target to true
        let mut list_stmt = tx.prepare(
            "SELECT id, metadata_json FROM external_references WHERE entity_id = ?1"
        ).map_err(|e| LoomaError::Database(e.to_string()))?;

        let refs_to_update: Vec<(String, String)> = list_stmt.query_map(params![entity_id], |row| {
            Ok((row.get(0)?, row.get(1)?))
        }).map_err(|e| LoomaError::Database(e.to_string()))?
        .filter_map(|r| r.ok())
        .collect();

        drop(list_stmt);

        let now_str = Utc::now().to_rfc3339();
        for (id, meta_str) in refs_to_update {
            let mut meta: serde_json::Value = serde_json::from_str(&meta_str).unwrap_or_else(|_| serde_json::json!({}));
            let should_be_primary = id == reference_id;
            let current_is_primary = meta.get("is_primary").and_then(|v| v.as_bool()).unwrap_or(false);

            if current_is_primary != should_be_primary || id == reference_id {
                if let Some(obj) = meta.as_object_mut() {
                    obj.insert("is_primary".to_string(), serde_json::json!(should_be_primary));
                }
                let new_meta_str = serde_json::to_string(&meta).unwrap_or_else(|_| "{}".to_string());
                tx.execute(
                    "UPDATE external_references SET metadata_json = ?1, updated_at = ?2 WHERE id = ?3",
                    params![new_meta_str, now_str, id],
                ).map_err(|e| LoomaError::Database(e.to_string()))?;
            }
        }

        tx.commit().map_err(|e| LoomaError::Database(e.to_string()))?;
        Ok(())
    }
}

impl ExperienceService for LoomaDb {
    fn create_experience(
        &self,
        entry: &ExperienceEntry,
        links: &[ExperienceLink],
        request_id: Option<(&str, &str)>,
    ) -> LoomaResult<ExperienceEntry> {
        let mut conn = self.conn.lock();

        // 1. Check idempotency request_id
        if let Some((scope, req_id)) = request_id {
            let existing_id: Option<String> = conn
                .query_row(
                    "SELECT experience_id FROM experience_request_ids WHERE actor_scope = ?1 AND request_id = ?2",
                    params![scope, req_id],
                    |row| row.get(0),
                )
                .optional()
                .map_err(|e| LoomaError::Database(e.to_string()))?;

            if let Some(exp_id) = existing_id {
                let mut stmt = conn
                    .prepare(
                        "SELECT id, subject_type, subject_id, event_type, title, note, occurred_value, occurred_precision, context_json, visibility, created_at, updated_at
                         FROM experience_entries WHERE id = ?1",
                    )
                    .map_err(|e| LoomaError::Database(e.to_string()))?;

                let existing = stmt
                    .query_row(params![exp_id], |row| {
                        let id: String = row.get(0)?;
                        let subject_type: String = row.get(1)?;
                        let subject_id: String = row.get(2)?;
                        let event_type: String = row.get(3)?;
                        let title: Option<String> = row.get(4)?;
                        let note: Option<String> = row.get(5)?;
                        let occurred_value: Option<String> = row.get(6)?;
                        let occurred_precision: String = row.get(7)?;
                        let ctx_str: String = row.get(8)?;
                        let visibility: String = row.get(9)?;
                        let c_str: String = row.get(10)?;
                        let u_str: String = row.get(11)?;

                        let context_json = serde_json::from_str(&ctx_str).unwrap_or(Value::Object(Default::default()));
                        let created_at = DateTime::parse_from_rfc3339(&c_str).map(|d| d.with_timezone(&Utc)).unwrap_or_else(|_| Utc::now());
                        let updated_at = DateTime::parse_from_rfc3339(&u_str).map(|d| d.with_timezone(&Utc)).unwrap_or_else(|_| Utc::now());

                        Ok(ExperienceEntry {
                            id,
                            subject_type,
                            subject_id,
                            event_type,
                            title,
                            note,
                            occurred_value,
                            occurred_precision,
                            context_json,
                            visibility,
                            created_at,
                            updated_at,
                        })
                    })
                    .optional()
                    .map_err(|e| LoomaError::Database(e.to_string()))?;

                if let Some(exp) = existing {
                    return Ok(exp);
                }
            }
        }

        // 2. Insert into experience_entries and links in transaction
        let tx = conn
            .transaction()
            .map_err(|e| LoomaError::Database(e.to_string()))?;

        let ctx_str = serde_json::to_string(&entry.context_json).unwrap_or_else(|_| "{}".to_string());
        let c_str = entry.created_at.to_rfc3339();
        let u_str = entry.updated_at.to_rfc3339();

        tx.execute(
            "INSERT INTO experience_entries (id, subject_type, subject_id, event_type, title, note, occurred_value, occurred_precision, context_json, visibility, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                entry.id,
                entry.subject_type,
                entry.subject_id,
                entry.event_type,
                entry.title,
                entry.note,
                entry.occurred_value,
                entry.occurred_precision,
                ctx_str,
                entry.visibility,
                c_str,
                u_str,
            ],
        ).map_err(|e| LoomaError::Database(e.to_string()))?;

        for link in links {
            tx.execute(
                "INSERT OR REPLACE INTO experience_links (experience_id, linked_type, linked_id, relation_type, position)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    link.experience_id,
                    link.linked_type,
                    link.linked_id,
                    link.relation_type,
                    link.position,
                ],
            ).map_err(|e| LoomaError::Database(e.to_string()))?;
        }

        if let Some((scope, req_id)) = request_id {
            tx.execute(
                "INSERT INTO experience_request_ids (actor_scope, request_id, experience_id)
                 VALUES (?1, ?2, ?3)",
                params![scope, req_id, entry.id],
            ).map_err(|e| LoomaError::Database(e.to_string()))?;
        }

        tx.commit().map_err(|e| LoomaError::Database(e.to_string()))?;
        Ok(entry.clone())
    }

    fn get_experience_by_id(&self, id: &str) -> LoomaResult<Option<ExperienceEntry>> {
        let conn = self.conn.lock();
        let mut stmt = conn
            .prepare(
                "SELECT id, subject_type, subject_id, event_type, title, note, occurred_value, occurred_precision, context_json, visibility, created_at, updated_at
                 FROM experience_entries WHERE id = ?1",
            )
            .map_err(|e| LoomaError::Database(e.to_string()))?;

        let res = stmt
            .query_row(params![id], |row| {
                let id: String = row.get(0)?;
                let subject_type: String = row.get(1)?;
                let subject_id: String = row.get(2)?;
                let event_type: String = row.get(3)?;
                let title: Option<String> = row.get(4)?;
                let note: Option<String> = row.get(5)?;
                let occurred_value: Option<String> = row.get(6)?;
                let occurred_precision: String = row.get(7)?;
                let ctx_str: String = row.get(8)?;
                let visibility: String = row.get(9)?;
                let c_str: String = row.get(10)?;
                let u_str: String = row.get(11)?;

                let context_json = serde_json::from_str(&ctx_str).unwrap_or(Value::Object(Default::default()));
                let created_at = DateTime::parse_from_rfc3339(&c_str).map(|d| d.with_timezone(&Utc)).unwrap_or_else(|_| Utc::now());
                let updated_at = DateTime::parse_from_rfc3339(&u_str).map(|d| d.with_timezone(&Utc)).unwrap_or_else(|_| Utc::now());

                Ok(ExperienceEntry {
                    id,
                    subject_type,
                    subject_id,
                    event_type,
                    title,
                    note,
                    occurred_value,
                    occurred_precision,
                    context_json,
                    visibility,
                    created_at,
                    updated_at,
                })
            })
            .optional()
            .map_err(|e| LoomaError::Database(e.to_string()))?;

        Ok(res)
    }

    fn list_experiences(&self, filter: &ExperienceFilter) -> LoomaResult<Vec<ExperienceEntry>> {
        let conn = self.conn.lock();
        let mut query = "SELECT id, subject_type, subject_id, event_type, title, note, occurred_value, occurred_precision, context_json, visibility, created_at, updated_at FROM experience_entries WHERE 1=1".to_string();
        let mut params_vec: Vec<rusqlite::types::Value> = Vec::new();

        if let Some(ref sid) = filter.subject_id {
            params_vec.push(rusqlite::types::Value::Text(sid.clone()));
            query.push_str(&format!(" AND subject_id = ?{}", params_vec.len()));
        }
        if let Some(ref stype) = filter.subject_type {
            params_vec.push(rusqlite::types::Value::Text(stype.clone()));
            query.push_str(&format!(" AND subject_type = ?{}", params_vec.len()));
        }
        if let Some(ref etype) = filter.event_type {
            params_vec.push(rusqlite::types::Value::Text(etype.clone()));
            query.push_str(&format!(" AND event_type = ?{}", params_vec.len()));
        }
        if let Some(ref vis) = filter.visibility {
            params_vec.push(rusqlite::types::Value::Text(vis.clone()));
            query.push_str(&format!(" AND visibility = ?{}", params_vec.len()));
        }

        query.push_str(" ORDER BY created_at DESC");

        if let Some(limit) = filter.limit {
            query.push_str(&format!(" LIMIT {limit}"));
            if let Some(offset) = filter.offset {
                query.push_str(&format!(" OFFSET {offset}"));
            }
        } else if let Some(offset) = filter.offset {
            query.push_str(&format!(" LIMIT -1 OFFSET {offset}"));
        }

        let mut stmt = conn.prepare(&query).map_err(|e| LoomaError::Database(e.to_string()))?;
        let rusqlite_params = rusqlite::params_from_iter(params_vec.iter());

        let rows: Vec<ExperienceEntry> = stmt
            .query_map(rusqlite_params, |row| {
                let id: String = row.get(0)?;
                let subject_type: String = row.get(1)?;
                let subject_id: String = row.get(2)?;
                let event_type: String = row.get(3)?;
                let title: Option<String> = row.get(4)?;
                let note: Option<String> = row.get(5)?;
                let occurred_value: Option<String> = row.get(6)?;
                let occurred_precision: String = row.get(7)?;
                let ctx_str: String = row.get(8)?;
                let visibility: String = row.get(9)?;
                let c_str: String = row.get(10)?;
                let u_str: String = row.get(11)?;

                let context_json = serde_json::from_str(&ctx_str).unwrap_or(Value::Object(Default::default()));
                let created_at = DateTime::parse_from_rfc3339(&c_str).map(|d| d.with_timezone(&Utc)).unwrap_or_else(|_| Utc::now());
                let updated_at = DateTime::parse_from_rfc3339(&u_str).map(|d| d.with_timezone(&Utc)).unwrap_or_else(|_| Utc::now());

                Ok(ExperienceEntry {
                    id,
                    subject_type,
                    subject_id,
                    event_type,
                    title,
                    note,
                    occurred_value,
                    occurred_precision,
                    context_json,
                    visibility,
                    created_at,
                    updated_at,
                })
            })
            .map_err(|e| LoomaError::Database(e.to_string()))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(rows)
    }

    fn update_experience(&self, entry: &ExperienceEntry) -> LoomaResult<()> {
        let conn = self.conn.lock();
        let ctx_str = serde_json::to_string(&entry.context_json).unwrap_or_else(|_| "{}".to_string());
        let u_str = Utc::now().to_rfc3339();

        let affected = conn.execute(
            "UPDATE experience_entries SET title = ?1, note = ?2, occurred_value = ?3, occurred_precision = ?4, context_json = ?5, visibility = ?6, updated_at = ?7
             WHERE id = ?8",
            params![
                entry.title,
                entry.note,
                entry.occurred_value,
                entry.occurred_precision,
                ctx_str,
                entry.visibility,
                u_str,
                entry.id,
            ],
        ).map_err(|e| LoomaError::Database(e.to_string()))?;

        if affected == 0 {
            return Err(LoomaError::NotFound(format!("Experience entry '{}' not found", entry.id)));
        }
        Ok(())
    }

    fn archive_experience(&self, id: &str) -> LoomaResult<bool> {
        let conn = self.conn.lock();
        let u_str = Utc::now().to_rfc3339();
        let affected = conn.execute(
            "UPDATE experience_entries SET visibility = 'archived', updated_at = ?1 WHERE id = ?2",
            params![u_str, id],
        ).map_err(|e| LoomaError::Database(e.to_string()))?;

        Ok(affected > 0)
    }

    fn list_experience_links(&self, experience_id: &str) -> LoomaResult<Vec<ExperienceLink>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT experience_id, linked_type, linked_id, relation_type, position
             FROM experience_links WHERE experience_id = ?1 ORDER BY position ASC",
        ).map_err(|e| LoomaError::Database(e.to_string()))?;

        let links: Vec<ExperienceLink> = stmt
            .query_map(params![experience_id], |row| {
                Ok(ExperienceLink {
                    experience_id: row.get(0)?,
                    linked_type: row.get(1)?,
                    linked_id: row.get(2)?,
                    relation_type: row.get(3)?,
                    position: row.get(4)?,
                })
            })
            .map_err(|e| LoomaError::Database(e.to_string()))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(links)
    }

    fn add_experience_link(&self, link: &ExperienceLink) -> LoomaResult<()> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT OR REPLACE INTO experience_links (experience_id, linked_type, linked_id, relation_type, position)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                link.experience_id,
                link.linked_type,
                link.linked_id,
                link.relation_type,
                link.position,
            ],
        ).map_err(|e| LoomaError::Database(e.to_string()))?;
        Ok(())
    }

    fn remove_experience_link(
        &self,
        experience_id: &str,
        linked_type: &str,
        linked_id: &str,
    ) -> LoomaResult<bool> {
        let conn = self.conn.lock();
        let affected = conn.execute(
            "DELETE FROM experience_links WHERE experience_id = ?1 AND linked_type = ?2 AND linked_id = ?3",
            params![experience_id, linked_type, linked_id],
        ).map_err(|e| LoomaError::Database(e.to_string()))?;
        Ok(affected > 0)
    }

    fn count_experiences(&self) -> LoomaResult<u64> {
        let conn = self.conn.lock();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM experience_entries", [], |r| r.get(0))
            .unwrap_or(0);
        Ok(count as u64)
    }

    fn complete_work_and_record_experience(
        &self,
        work_id: &str,
        experience: &ExperienceEntry,
        links: &[ExperienceLink],
        request_id: Option<(&str, &str)>,
        audit_log: &AuditLog,
    ) -> LoomaResult<(Entity, ExperienceEntry)> {
        let mut conn = self.conn.lock();

        // 1. Check idempotency request_id before doing any writes
        if let Some((scope, req_id)) = request_id {
            let existing_exp_id: Option<String> = conn
                .query_row(
                    "SELECT experience_id FROM experience_request_ids WHERE actor_scope = ?1 AND request_id = ?2",
                    params![scope, req_id],
                    |row| row.get(0),
                )
                .optional()
                .map_err(|e| LoomaError::Database(e.to_string()))?;

            if let Some(exp_id) = existing_exp_id {
                let existing_exp = conn
                    .prepare(
                        "SELECT id, subject_type, subject_id, event_type, title, note, occurred_value, occurred_precision, context_json, visibility, created_at, updated_at
                         FROM experience_entries WHERE id = ?1",
                    )
                    .map_err(|e| LoomaError::Database(e.to_string()))?
                    .query_row(params![exp_id], |row| {
                        let id: String = row.get(0)?;
                        let subject_type: String = row.get(1)?;
                        let subject_id: String = row.get(2)?;
                        let event_type: String = row.get(3)?;
                        let title: Option<String> = row.get(4)?;
                        let note: Option<String> = row.get(5)?;
                        let occurred_value: Option<String> = row.get(6)?;
                        let occurred_precision: String = row.get(7)?;
                        let ctx_str: String = row.get(8)?;
                        let visibility: String = row.get(9)?;
                        let c_str: String = row.get(10)?;
                        let u_str: String = row.get(11)?;

                        let context_json = serde_json::from_str(&ctx_str).unwrap_or(Value::Object(Default::default()));
                        let created_at = DateTime::parse_from_rfc3339(&c_str).map(|d| d.with_timezone(&Utc)).unwrap_or_else(|_| Utc::now());
                        let updated_at = DateTime::parse_from_rfc3339(&u_str).map(|d| d.with_timezone(&Utc)).unwrap_or_else(|_| Utc::now());

                        Ok(ExperienceEntry {
                            id,
                            subject_type,
                            subject_id,
                            event_type,
                            title,
                            note,
                            occurred_value,
                            occurred_precision,
                            context_json,
                            visibility,
                            created_at,
                            updated_at,
                        })
                    })
                    .optional()
                    .map_err(|e| LoomaError::Database(e.to_string()))?;

                let work = conn
                    .prepare(
                        "SELECT id, entity_type, title, description, properties_json, created_at, updated_at
                         FROM entities WHERE id = ?1",
                    )
                    .map_err(|e| LoomaError::Database(e.to_string()))?
                    .query_row(params![work_id], |row| {
                        let id: String = row.get(0)?;
                        let entity_type: String = row.get(1)?;
                        let title: String = row.get(2)?;
                        let description: Option<String> = row.get(3)?;
                        let prop_str: String = row.get(4)?;
                        let c_str: String = row.get(5)?;
                        let u_str: String = row.get(6)?;

                        let properties = serde_json::from_str(&prop_str).unwrap_or(Value::Object(Default::default()));
                        let created_at = DateTime::parse_from_rfc3339(&c_str).map(|d| d.with_timezone(&Utc)).unwrap_or_else(|_| Utc::now());
                        let updated_at = DateTime::parse_from_rfc3339(&u_str).map(|d| d.with_timezone(&Utc)).unwrap_or_else(|_| Utc::now());

                        Ok(Entity {
                            id,
                            entity_type,
                            title,
                            description,
                            properties,
                            created_at,
                            updated_at,
                        })
                    })
                    .optional()
                    .map_err(|e| LoomaError::Database(e.to_string()))?;

                if let (Some(exp), Some(w)) = (existing_exp, work) {
                    return Ok((w, exp));
                }
            }
        }

        // 2. Start single atomic transaction
        let tx = conn
            .transaction()
            .map_err(|e| LoomaError::Database(e.to_string()))?;

        // 2a. Fetch entity and update its status to completed
        let (entity_type, title, description, prop_str, c_str) = tx
            .query_row(
                "SELECT entity_type, title, description, properties_json, created_at FROM entities WHERE id = ?1",
                params![work_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Option<String>>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                    ))
                },
            )
            .optional()
            .map_err(|e| LoomaError::Database(e.to_string()))?
            .ok_or_else(|| LoomaError::NotFound(format!("Work entity '{work_id}' not found")))?;

        let mut properties: Value = serde_json::from_str(&prop_str).unwrap_or(Value::Object(Default::default()));
        if let Some(obj) = properties.as_object_mut() {
            obj.insert("status".to_string(), serde_json::json!("completed"));
            if let Some(work_val) = obj.get_mut("work") {
                if let Some(work_obj) = work_val.as_object_mut() {
                    work_obj.insert("status".to_string(), serde_json::json!("completed"));
                    // Set end_date if not already set and precision is day/instant
                    let current_end_date = work_obj.get("end_date").and_then(|v| v.as_str());
                    if current_end_date.is_none() {
                        if experience.occurred_precision == "day" {
                            if let Some(ref date_str) = experience.occurred_value {
                                work_obj.insert("end_date".to_string(), serde_json::json!(date_str));
                            }
                        } else if experience.occurred_precision == "instant" {
                            if let Some(ref date_str) = experience.occurred_value {
                                if date_str.len() >= 10 {
                                    work_obj.insert("end_date".to_string(), serde_json::json!(&date_str[..10]));
                                }
                            }
                        }
                    }
                }
            }
        }

        let new_prop_str = serde_json::to_string(&properties).unwrap_or_else(|_| "{}".to_string());
        let now = Utc::now();
        let now_str = now.to_rfc3339();

        tx.execute(
            "UPDATE entities SET properties_json = ?1, updated_at = ?2 WHERE id = ?3",
            params![new_prop_str, now_str, work_id],
        ).map_err(|e| LoomaError::Database(e.to_string()))?;

        // 2b. Insert experience entry
        let ctx_str = serde_json::to_string(&experience.context_json).unwrap_or_else(|_| "{}".to_string());
        let exp_created_str = experience.created_at.to_rfc3339();
        let exp_updated_str = experience.updated_at.to_rfc3339();

        tx.execute(
            "INSERT INTO experience_entries (id, subject_type, subject_id, event_type, title, note, occurred_value, occurred_precision, context_json, visibility, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                experience.id,
                experience.subject_type,
                experience.subject_id,
                experience.event_type,
                experience.title,
                experience.note,
                experience.occurred_value,
                experience.occurred_precision,
                ctx_str,
                experience.visibility,
                exp_created_str,
                exp_updated_str,
            ],
        ).map_err(|e| LoomaError::Database(e.to_string()))?;

        // 2c. Insert experience links
        for link in links {
            tx.execute(
                "INSERT OR REPLACE INTO experience_links (experience_id, linked_type, linked_id, relation_type, position)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    link.experience_id,
                    link.linked_type,
                    link.linked_id,
                    link.relation_type,
                    link.position,
                ],
            ).map_err(|e| LoomaError::Database(e.to_string()))?;
        }

        // 2d. Insert request_id if present
        if let Some((scope, req_id)) = request_id {
            tx.execute(
                "INSERT INTO experience_request_ids (actor_scope, request_id, experience_id)
                 VALUES (?1, ?2, ?3)",
                params![scope, req_id, experience.id],
            ).map_err(|e| LoomaError::Database(e.to_string()))?;
        }

        // 2e. Insert audit log in same transaction
        let audit_details_str = serde_json::to_string(&audit_log.details).unwrap_or_else(|_| "{}".to_string());
        tx.execute(
            "INSERT INTO audit_logs (id, timestamp, actor, operation, target_type, target_id, result, details_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                audit_log.id,
                audit_log.timestamp.to_rfc3339(),
                audit_log.actor,
                audit_log.operation,
                audit_log.target_type,
                audit_log.target_id,
                audit_log.result,
                audit_details_str,
            ],
        ).map_err(|e| LoomaError::Database(e.to_string()))?;

        // 2f. Commit transaction atomically
        tx.commit().map_err(|e| LoomaError::Database(e.to_string()))?;

        let created_at_dt = DateTime::parse_from_rfc3339(&c_str)
            .map(|d| d.with_timezone(&Utc))
            .unwrap_or(now);

        let updated_entity = Entity {
            id: work_id.to_string(),
            entity_type,
            title,
            description,
            properties,
            created_at: created_at_dt,
            updated_at: now,
        };

        Ok((updated_entity, experience.clone()))
    }
}

impl SourceRootService for LoomaDb {
    fn create_source_root(&self, root: &SourceRoot) -> LoomaResult<SourceRoot> {
        let conn = self.conn.lock();
        let exists: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM source_roots WHERE id = ?1 OR path = ?2)",
                params![root.id, root.path],
                |r| r.get(0),
            )
            .map_err(|e| LoomaError::Database(e.to_string()))?;
        if exists {
            return Err(LoomaError::Database(format!(
                "SourceRoot with id '{}' or path '{}' already exists",
                root.id, root.path
            )));
        }

        let now = Utc::now();
        let is_dir = Path::new(&root.path).is_dir();
        let state = if is_dir { root_states::ONLINE } else { &root.state };
        let last_seen = if is_dir { Some(now) } else { root.last_seen_at };

        let settings_str = serde_json::to_string(&root.settings_json).unwrap_or_else(|_| "{}".to_string());
        conn.execute(
            "INSERT INTO source_roots (id, label, path, source_kind, enabled, state, last_successful_scan_at, last_seen_at, settings_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                root.id,
                root.label,
                root.path,
                root.source_kind,
                if root.enabled { 1 } else { 0 },
                state,
                root.last_successful_scan_at.map(|d| d.to_rfc3339()),
                last_seen.map(|d| d.to_rfc3339()),
                settings_str,
            ],
        )
        .map_err(|e| LoomaError::Database(e.to_string()))?;

        let mut res = root.clone();
        res.state = state.to_string();
        res.last_seen_at = last_seen;
        Ok(res)
    }

    fn get_source_root(&self, id: &str) -> LoomaResult<Option<SourceRoot>> {
        let conn = self.conn.lock();
        let mut stmt = conn
            .prepare("SELECT id, label, path, source_kind, enabled, state, last_successful_scan_at, last_seen_at, settings_json FROM source_roots WHERE id = ?1")
            .map_err(|e| LoomaError::Database(e.to_string()))?;

        let res = stmt
            .query_row(params![id], |row| {
                let enabled_int: i32 = row.get(4)?;
                let last_scan_str: Option<String> = row.get(6)?;
                let last_seen_str: Option<String> = row.get(7)?;
                let settings_str: String = row.get(8)?;
                let settings_json: Value = serde_json::from_str(&settings_str).unwrap_or_else(|_| serde_json::json!({}));

                Ok(SourceRoot {
                    id: row.get(0)?,
                    label: row.get(1)?,
                    path: row.get(2)?,
                    source_kind: row.get(3)?,
                    enabled: enabled_int == 1,
                    state: row.get(5)?,
                    last_successful_scan_at: last_scan_str.and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|d| d.with_timezone(&Utc))),
                    last_seen_at: last_seen_str.and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|d| d.with_timezone(&Utc))),
                    settings_json,
                })
            })
            .optional()
            .map_err(|e| LoomaError::Database(e.to_string()))?;

        Ok(res)
    }

    fn get_source_root_by_path(&self, path: &str) -> LoomaResult<Option<SourceRoot>> {
        let conn = self.conn.lock();
        let mut stmt = conn
            .prepare("SELECT id, label, path, source_kind, enabled, state, last_successful_scan_at, last_seen_at, settings_json FROM source_roots WHERE path = ?1")
            .map_err(|e| LoomaError::Database(e.to_string()))?;

        let res = stmt
            .query_row(params![path], |row| {
                let enabled_int: i32 = row.get(4)?;
                let last_scan_str: Option<String> = row.get(6)?;
                let last_seen_str: Option<String> = row.get(7)?;
                let settings_str: String = row.get(8)?;
                let settings_json: Value = serde_json::from_str(&settings_str).unwrap_or_else(|_| serde_json::json!({}));

                Ok(SourceRoot {
                    id: row.get(0)?,
                    label: row.get(1)?,
                    path: row.get(2)?,
                    source_kind: row.get(3)?,
                    enabled: enabled_int == 1,
                    state: row.get(5)?,
                    last_successful_scan_at: last_scan_str.and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|d| d.with_timezone(&Utc))),
                    last_seen_at: last_seen_str.and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|d| d.with_timezone(&Utc))),
                    settings_json,
                })
            })
            .optional()
            .map_err(|e| LoomaError::Database(e.to_string()))?;

        Ok(res)
    }

    fn list_source_roots(&self) -> LoomaResult<Vec<SourceRoot>> {
        let conn = self.conn.lock();
        let mut stmt = conn
            .prepare("SELECT id, label, path, source_kind, enabled, state, last_successful_scan_at, last_seen_at, settings_json FROM source_roots ORDER BY label ASC")
            .map_err(|e| LoomaError::Database(e.to_string()))?;

        let rows = stmt
            .query_map([], |row| {
                let enabled_int: i32 = row.get(4)?;
                let last_scan_str: Option<String> = row.get(6)?;
                let last_seen_str: Option<String> = row.get(7)?;
                let settings_str: String = row.get(8)?;
                let settings_json: Value = serde_json::from_str(&settings_str).unwrap_or_else(|_| serde_json::json!({}));

                Ok(SourceRoot {
                    id: row.get(0)?,
                    label: row.get(1)?,
                    path: row.get(2)?,
                    source_kind: row.get(3)?,
                    enabled: enabled_int == 1,
                    state: row.get(5)?,
                    last_successful_scan_at: last_scan_str.and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|d| d.with_timezone(&Utc))),
                    last_seen_at: last_seen_str.and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|d| d.with_timezone(&Utc))),
                    settings_json,
                })
            })
            .map_err(|e| LoomaError::Database(e.to_string()))?;

        let mut roots = Vec::new();
        for r in rows {
            roots.push(r.map_err(|e| LoomaError::Database(e.to_string()))?);
        }
        Ok(roots)
    }

    fn update_source_root(&self, root: &SourceRoot) -> LoomaResult<SourceRoot> {
        let conn = self.conn.lock();
        let settings_str = serde_json::to_string(&root.settings_json).unwrap_or_else(|_| "{}".to_string());
        conn.execute(
            "UPDATE source_roots SET label = ?1, path = ?2, source_kind = ?3, enabled = ?4, state = ?5,
             last_successful_scan_at = ?6, last_seen_at = ?7, settings_json = ?8 WHERE id = ?9",
            params![
                root.label,
                root.path,
                root.source_kind,
                if root.enabled { 1 } else { 0 },
                root.state,
                root.last_successful_scan_at.map(|d| d.to_rfc3339()),
                root.last_seen_at.map(|d| d.to_rfc3339()),
                settings_str,
                root.id,
            ],
        )
        .map_err(|e| LoomaError::Database(e.to_string()))?;

        Ok(root.clone())
    }

    fn delete_source_root(&self, id: &str) -> LoomaResult<()> {
        let conn = self.conn.lock();
        conn.execute("DELETE FROM source_roots WHERE id = ?1", params![id])
            .map_err(|e| LoomaError::Database(e.to_string()))?;
        Ok(())
    }

    fn rebind_source_root(&self, id: &str, new_path: &str) -> LoomaResult<SourceRoot> {
        let mut conn = self.conn.lock();
        let tx = conn.transaction().map_err(|e| LoomaError::Database(e.to_string()))?;

        // 1. Fetch current root
        let mut root: SourceRoot = tx.query_row(
            "SELECT id, label, path, source_kind, enabled, state, last_successful_scan_at, last_seen_at, settings_json
             FROM source_roots WHERE id = ?1",
            params![id],
            |row| {
                let enabled_int: i32 = row.get(4)?;
                let last_scan_str: Option<String> = row.get(6)?;
                let last_seen_str: Option<String> = row.get(7)?;
                let settings_str: String = row.get(8)?;
                let settings_json: Value = serde_json::from_str(&settings_str).unwrap_or_else(|_| serde_json::json!({}));
                Ok(SourceRoot {
                    id: row.get(0)?,
                    label: row.get(1)?,
                    path: row.get(2)?,
                    source_kind: row.get(3)?,
                    enabled: enabled_int == 1,
                    state: row.get(5)?,
                    last_successful_scan_at: last_scan_str.and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|d| d.with_timezone(&Utc))),
                    last_seen_at: last_seen_str.and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|d| d.with_timezone(&Utc))),
                    settings_json,
                })
            },
        ).optional().map_err(|e| LoomaError::Database(e.to_string()))?
        .ok_or_else(|| LoomaError::NotFound(format!("SourceRoot '{id}' not found")))?;

        let old_root_path = root.path.clone();

        // 2. Update source_roots table with new path
        let is_dir = Path::new(new_path).is_dir();
        let new_state = if is_dir { root_states::ONLINE } else { root_states::OFFLINE };
        let now = Utc::now();
        let now_str = now.to_rfc3339();

        tx.execute(
            "UPDATE source_roots SET path = ?1, state = ?2, last_seen_at = ?3 WHERE id = ?4",
            params![new_path, new_state, if is_dir { Some(&now_str) } else { None }, id],
        ).map_err(|e| LoomaError::Database(e.to_string()))?;

        // 3. Find all assets linked via asset_lifecycle with this root_id
        let mut lifecycles: Vec<(String, Option<String>)> = Vec::new();
        {
            let mut stmt = tx.prepare(
                "SELECT asset_id, relative_path FROM asset_lifecycle WHERE root_id = ?1"
            ).map_err(|e| LoomaError::Database(e.to_string()))?;
            let rows = stmt.query_map(params![id], |row| {
                Ok((row.get(0)?, row.get(1)?))
            }).map_err(|e| LoomaError::Database(e.to_string()))?;
            for r in rows {
                if let Ok(item) = r {
                    lifecycles.push(item);
                }
            }
        }

        // 4. Update each asset path and lifecycle availability
        for (asset_id, rel_path_opt) in lifecycles {
            if let Some(rel_path) = rel_path_opt {
                let clean_rel = rel_path.trim_start_matches('/').trim_start_matches('\\');
                let clean_root = new_path.trim_end_matches('/').trim_end_matches('\\');
                let full_new_path = format!("{clean_root}/{clean_rel}");

                let file_exists = Path::new(&full_new_path).exists();
                let avail = if file_exists { asset_availability::ONLINE } else { asset_availability::UNCHECKED };

                tx.execute(
                    "UPDATE assets SET path = ?1 WHERE id = ?2",
                    params![full_new_path, asset_id],
                ).map_err(|e| LoomaError::Database(e.to_string()))?;

                tx.execute(
                    "UPDATE asset_lifecycle SET availability = ?1, last_verified_at = ?2 WHERE asset_id = ?3",
                    params![avail, if file_exists { Some(&now_str) } else { None }, asset_id],
                ).map_err(|e| LoomaError::Database(e.to_string()))?;
            }
        }

        // Also update any assets whose path was starting with old_root_path
        let old_clean = old_root_path.trim_end_matches('/').trim_end_matches('\\');
        let new_clean = new_path.trim_end_matches('/').trim_end_matches('\\');
        let old_prefix = format!("{old_clean}/");
        let mut prefix_assets: Vec<(String, String)> = Vec::new();
        {
            let mut stmt = tx.prepare("SELECT id, path FROM assets WHERE path LIKE ?1").map_err(|e| LoomaError::Database(e.to_string()))?;
            let pattern = format!("{old_clean}%");
            let rows = stmt.query_map(params![pattern], |row| {
                let aid: String = row.get(0)?;
                let p: Option<String> = row.get(1)?;
                Ok((aid, p.unwrap_or_default()))
            }).map_err(|e| LoomaError::Database(e.to_string()))?;
            for r in rows {
                if let Ok((aid, p)) = r {
                    if p.starts_with(&old_prefix) || p == old_clean {
                        prefix_assets.push((aid, p));
                    }
                }
            }
        }

        for (aid, old_p) in prefix_assets {
            let rel = if old_p.len() >= old_prefix.len() {
                &old_p[old_prefix.len()..]
            } else {
                ""
            };
            let full_new = if rel.is_empty() {
                new_clean.to_string()
            } else {
                format!("{new_clean}/{rel}")
            };
            tx.execute("UPDATE assets SET path = ?1 WHERE id = ?2", params![full_new, aid])
                .map_err(|e| LoomaError::Database(e.to_string()))?;
            let file_exists = Path::new(&full_new).exists();
            let avail = if file_exists { asset_availability::ONLINE } else { asset_availability::UNCHECKED };
            tx.execute(
                "INSERT INTO asset_lifecycle (asset_id, root_id, relative_path, availability, visibility, tracking, last_verified_at)
                 VALUES (?1, ?2, ?3, ?4, 'active', 'tracked', ?5)
                 ON CONFLICT(asset_id) DO UPDATE SET
                   root_id = COALESCE(excluded.root_id, asset_lifecycle.root_id),
                   relative_path = COALESCE(excluded.relative_path, asset_lifecycle.relative_path),
                   availability = excluded.availability,
                   last_verified_at = excluded.last_verified_at",
                params![aid, id, rel, avail, if file_exists { Some(&now_str) } else { None }],
            ).map_err(|e| LoomaError::Database(e.to_string()))?;
        }

        tx.commit().map_err(|e| LoomaError::Database(e.to_string()))?;

        root.path = new_path.to_string();
        root.state = new_state.to_string();
        if is_dir {
            root.last_seen_at = Some(now);
        }
        Ok(root)
    }

    fn inspect_source_root(&self, id: &str) -> LoomaResult<SourceRootInspection> {
        let root = self.get_source_root(id)?
            .ok_or_else(|| LoomaError::NotFound(format!("SourceRoot '{id}' not found")))?;

        let p = Path::new(&root.path);
        let exists = p.exists();
        let is_dir = p.is_dir();
        let accessible = exists && is_dir && std::fs::read_dir(p).is_ok();

        let conn = self.conn.lock();
        let tracked_assets: i64 = conn.query_row(
            "SELECT COUNT(*) FROM asset_lifecycle WHERE root_id = ?1 AND tracking = 'tracked'",
            params![id],
            |r| r.get(0),
        ).unwrap_or(0);

        let online_assets: i64 = conn.query_row(
            "SELECT COUNT(*) FROM asset_lifecycle WHERE root_id = ?1 AND availability = 'online'",
            params![id],
            |r| r.get(0),
        ).unwrap_or(0);

        let offline_assets: i64 = conn.query_row(
            "SELECT COUNT(*) FROM asset_lifecycle WHERE root_id = ?1 AND availability = 'offline'",
            params![id],
            |r| r.get(0),
        ).unwrap_or(0);

        let missing_assets: i64 = conn.query_row(
            "SELECT COUNT(*) FROM asset_lifecycle WHERE root_id = ?1 AND availability = 'missing'",
            params![id],
            |r| r.get(0),
        ).unwrap_or(0);

        let unchecked_assets: i64 = conn.query_row(
            "SELECT COUNT(*) FROM asset_lifecycle WHERE root_id = ?1 AND availability = 'unchecked'",
            params![id],
            |r| r.get(0),
        ).unwrap_or(0);

        let suppressed_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM asset_scan_suppressions WHERE root_id = ?1",
            params![id],
            |r| r.get(0),
        ).unwrap_or(0);

        Ok(SourceRootInspection {
            root_id: root.id,
            label: root.label,
            path: root.path,
            state: root.state,
            exists,
            is_dir,
            accessible,
            tracked_assets: tracked_assets as usize,
            online_assets: online_assets as usize,
            offline_assets: offline_assets as usize,
            missing_assets: missing_assets as usize,
            unchecked_assets: unchecked_assets as usize,
            suppressed_count: suppressed_count as usize,
        })
    }

    fn get_asset_lifecycle(&self, asset_id: &str) -> LoomaResult<Option<AssetLifecycle>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT asset_id, root_id, relative_path, availability, visibility, tracking, last_verified_at, untracked_at, untracked_reason
             FROM asset_lifecycle WHERE asset_id = ?1"
        ).map_err(|e| LoomaError::Database(e.to_string()))?;

        let res = stmt.query_row(params![asset_id], |row| {
            let last_verified_str: Option<String> = row.get(6)?;
            let untracked_str: Option<String> = row.get(7)?;
            Ok(AssetLifecycle {
                asset_id: row.get(0)?,
                root_id: row.get(1)?,
                relative_path: row.get(2)?,
                availability: row.get(3)?,
                visibility: row.get(4)?,
                tracking: row.get(5)?,
                last_verified_at: last_verified_str.and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|d| d.with_timezone(&Utc))),
                untracked_at: untracked_str.and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|d| d.with_timezone(&Utc))),
                untracked_reason: row.get(8)?,
            })
        }).optional().map_err(|e| LoomaError::Database(e.to_string()))?;

        Ok(res)
    }

    fn set_asset_lifecycle(&self, lifecycle: &AssetLifecycle) -> LoomaResult<()> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO asset_lifecycle (asset_id, root_id, relative_path, availability, visibility, tracking, last_verified_at, untracked_at, untracked_reason)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
             ON CONFLICT(asset_id) DO UPDATE SET
               root_id = excluded.root_id,
               relative_path = excluded.relative_path,
               availability = excluded.availability,
               visibility = excluded.visibility,
               tracking = excluded.tracking,
               last_verified_at = excluded.last_verified_at,
               untracked_at = excluded.untracked_at,
               untracked_reason = excluded.untracked_reason",
            params![
                lifecycle.asset_id,
                lifecycle.root_id,
                lifecycle.relative_path,
                lifecycle.availability,
                lifecycle.visibility,
                lifecycle.tracking,
                lifecycle.last_verified_at.map(|d| d.to_rfc3339()),
                lifecycle.untracked_at.map(|d| d.to_rfc3339()),
                lifecycle.untracked_reason,
            ],
        ).map_err(|e| LoomaError::Database(e.to_string()))?;
        Ok(())
    }

    fn archive_asset(&self, asset_id: &str) -> LoomaResult<()> {
        let mut conn = self.conn.lock();
        let tx = conn.transaction().map_err(|e| LoomaError::Database(e.to_string()))?;

        let exists: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM assets WHERE id = ?1)",
            params![asset_id],
            |r| r.get(0),
        ).map_err(|e| LoomaError::Database(e.to_string()))?;

        if !exists {
            return Err(LoomaError::NotFound(format!("Asset '{asset_id}' not found")));
        }

        tx.execute(
            "INSERT INTO asset_lifecycle (asset_id, availability, visibility, tracking)
             VALUES (?1, 'unchecked', 'archived', 'tracked')
             ON CONFLICT(asset_id) DO UPDATE SET visibility = 'archived'",
            params![asset_id],
        ).map_err(|e| LoomaError::Database(e.to_string()))?;

        tx.execute("UPDATE assets SET status = 'archived' WHERE id = ?1", params![asset_id])
            .map_err(|e| LoomaError::Database(e.to_string()))?;

        tx.commit().map_err(|e| LoomaError::Database(e.to_string()))?;
        Ok(())
    }

    fn unarchive_asset(&self, asset_id: &str) -> LoomaResult<()> {
        let mut conn = self.conn.lock();
        let tx = conn.transaction().map_err(|e| LoomaError::Database(e.to_string()))?;

        tx.execute(
            "UPDATE asset_lifecycle SET visibility = 'active' WHERE asset_id = ?1",
            params![asset_id],
        ).map_err(|e| LoomaError::Database(e.to_string()))?;

        tx.execute("UPDATE assets SET status = 'active' WHERE id = ?1", params![asset_id])
            .map_err(|e| LoomaError::Database(e.to_string()))?;

        tx.commit().map_err(|e| LoomaError::Database(e.to_string()))?;
        Ok(())
    }

    fn untrack_asset(&self, asset_id: &str, reason: Option<&str>) -> LoomaResult<()> {
        let mut conn = self.conn.lock();
        let tx = conn.transaction().map_err(|e| LoomaError::Database(e.to_string()))?;

        let asset_path: Option<String> = tx.query_row(
            "SELECT path FROM assets WHERE id = ?1",
            params![asset_id],
            |r| r.get(0),
        ).optional().map_err(|e| LoomaError::Database(e.to_string()))?
        .ok_or_else(|| LoomaError::NotFound(format!("Asset '{asset_id}' not found")))?;

        let now = Utc::now();
        let now_str = now.to_rfc3339();

        // Check or create lifecycle
        let existing_lc: Option<(Option<String>, Option<String>)> = tx.query_row(
            "SELECT root_id, relative_path FROM asset_lifecycle WHERE asset_id = ?1",
            params![asset_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        ).optional().map_err(|e| LoomaError::Database(e.to_string()))?;

        let (mut root_id, mut rel_path) = match existing_lc {
            Some((r, p)) => (r, p),
            None => (None, None),
        };

        // If root_id is missing but we have path, match against source_roots
        if (root_id.is_none() || rel_path.is_none()) && asset_path.is_some() {
            let p_str = asset_path.as_ref().unwrap();
            let norm_asset = p_str.replace('\\', "/");
            let mut stmt = tx.prepare("SELECT id, path FROM source_roots").map_err(|e| LoomaError::Database(e.to_string()))?;
            let root_rows = stmt.query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            }).map_err(|e| LoomaError::Database(e.to_string()))?;

            for r in root_rows.flatten() {
                let norm_root = r.1.replace('\\', "/");
                let prefix = format!("{}/", norm_root.trim_end_matches('/'));
                if norm_asset.starts_with(&prefix) {
                    root_id = Some(r.0);
                    rel_path = Some(norm_asset[prefix.len()..].to_string());
                    break;
                }
            }
        }

        tx.execute(
            "INSERT INTO asset_lifecycle (asset_id, root_id, relative_path, availability, visibility, tracking, untracked_at, untracked_reason)
             VALUES (?1, ?2, ?3, 'unchecked', 'active', 'untracked', ?4, ?5)
             ON CONFLICT(asset_id) DO UPDATE SET
               root_id = COALESCE(excluded.root_id, asset_lifecycle.root_id),
               relative_path = COALESCE(excluded.relative_path, asset_lifecycle.relative_path),
               tracking = 'untracked',
               untracked_at = excluded.untracked_at,
               untracked_reason = excluded.untracked_reason",
            params![asset_id, root_id, rel_path, now_str, reason],
        ).map_err(|e| LoomaError::Database(e.to_string()))?;

        // If we have root_id and relative_path, record suppression tombstone
        if let (Some(rid), Some(rpath)) = (&root_id, &rel_path) {
            let sup_id = uuid::Uuid::new_v4().to_string();
            tx.execute(
                "INSERT INTO asset_scan_suppressions (id, root_id, relative_path, reason, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(root_id, relative_path) DO UPDATE SET reason = excluded.reason",
                params![sup_id, rid, rpath, reason, now_str],
            ).map_err(|e| LoomaError::Database(e.to_string()))?;
        }

        tx.commit().map_err(|e| LoomaError::Database(e.to_string()))?;
        Ok(())
    }

    fn resume_tracking(&self, asset_id: &str) -> LoomaResult<()> {
        let mut conn = self.conn.lock();
        let tx = conn.transaction().map_err(|e| LoomaError::Database(e.to_string()))?;

        let existing: Option<(Option<String>, Option<String>)> = tx.query_row(
            "SELECT root_id, relative_path FROM asset_lifecycle WHERE asset_id = ?1",
            params![asset_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        ).optional().map_err(|e| LoomaError::Database(e.to_string()))?;

        tx.execute(
            "UPDATE asset_lifecycle SET tracking = 'tracked', untracked_at = NULL, untracked_reason = NULL WHERE asset_id = ?1",
            params![asset_id],
        ).map_err(|e| LoomaError::Database(e.to_string()))?;

        if let Some((Some(rid), Some(rpath))) = existing {
            tx.execute(
                "DELETE FROM asset_scan_suppressions WHERE root_id = ?1 AND relative_path = ?2",
                params![rid, rpath],
            ).map_err(|e| LoomaError::Database(e.to_string()))?;
        }

        tx.commit().map_err(|e| LoomaError::Database(e.to_string()))?;
        Ok(())
    }

    fn is_asset_suppressed(&self, root_id: &str, relative_path: &str) -> LoomaResult<bool> {
        let clean_rel = relative_path.trim_start_matches('/').trim_start_matches('\\');
        let conn = self.conn.lock();
        let suppressed: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM asset_scan_suppressions WHERE root_id = ?1 AND relative_path = ?2)",
            params![root_id, clean_rel],
            |r| r.get(0),
        ).map_err(|e| LoomaError::Database(e.to_string()))?;
        Ok(suppressed)
    }

    fn list_suppressions(&self, root_id: Option<&str>) -> LoomaResult<Vec<AssetScanSuppression>> {
        let conn = self.conn.lock();
        let mut results = Vec::new();
        if let Some(rid) = root_id {
            let mut stmt = conn.prepare(
                "SELECT id, root_id, relative_path, reason, created_at FROM asset_scan_suppressions WHERE root_id = ?1 ORDER BY created_at DESC"
            ).map_err(|e| LoomaError::Database(e.to_string()))?;
            let rows = stmt.query_map(params![rid], |row| {
                let dt_str: String = row.get(4)?;
                let created_at = DateTime::parse_from_rfc3339(&dt_str)
                    .map(|d| d.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());
                Ok(AssetScanSuppression {
                    id: row.get(0)?,
                    root_id: row.get(1)?,
                    relative_path: row.get(2)?,
                    reason: row.get(3)?,
                    created_at,
                })
            }).map_err(|e| LoomaError::Database(e.to_string()))?;
            for r in rows {
                results.push(r.map_err(|e| LoomaError::Database(e.to_string()))?);
            }
        } else {
            let mut stmt = conn.prepare(
                "SELECT id, root_id, relative_path, reason, created_at FROM asset_scan_suppressions ORDER BY created_at DESC"
            ).map_err(|e| LoomaError::Database(e.to_string()))?;
            let rows = stmt.query_map([], |row| {
                let dt_str: String = row.get(4)?;
                let created_at = DateTime::parse_from_rfc3339(&dt_str)
                    .map(|d| d.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());
                Ok(AssetScanSuppression {
                    id: row.get(0)?,
                    root_id: row.get(1)?,
                    relative_path: row.get(2)?,
                    reason: row.get(3)?,
                    created_at,
                })
            }).map_err(|e| LoomaError::Database(e.to_string()))?;
            for r in rows {
                results.push(r.map_err(|e| LoomaError::Database(e.to_string()))?);
            }
        }
        Ok(results)
    }

    fn add_suppression(&self, root_id: &str, relative_path: &str, reason: Option<&str>) -> LoomaResult<()> {
        let clean_rel = relative_path.trim_start_matches('/').trim_start_matches('\\');
        let conn = self.conn.lock();
        let id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO asset_scan_suppressions (id, root_id, relative_path, reason, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(root_id, relative_path) DO UPDATE SET reason = excluded.reason",
            params![id, root_id, clean_rel, reason, now],
        ).map_err(|e| LoomaError::Database(e.to_string()))?;
        Ok(())
    }

    fn remove_suppression(&self, root_id: &str, relative_path: &str) -> LoomaResult<()> {
        let clean_rel = relative_path.trim_start_matches('/').trim_start_matches('\\');
        let conn = self.conn.lock();
        conn.execute(
            "DELETE FROM asset_scan_suppressions WHERE root_id = ?1 AND relative_path = ?2",
            params![root_id, clean_rel],
        ).map_err(|e| LoomaError::Database(e.to_string()))?;
        Ok(())
    }
}
