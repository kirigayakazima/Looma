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
        let pattern = format!("{}%", prefix);
        let mut stmt = conn
            .prepare(
                "SELECT id, kind, source, path, size, hash, mime_type, metadata_json, status,
                        created_at, modified_at, indexed_at
                 FROM assets WHERE path LIKE ?1",
            )
            .map_err(|e| LoomaError::Database(e.to_string()))?;

        let asset_iter = stmt
            .query_map(params![pattern], |row| {
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
            query.push_str(" AND (title LIKE ? OR description LIKE ?)");
            let pattern = format!("%{}%", q);
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

        let conn = self.conn.lock();
        let relations_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM relations", [], |r| r.get(0))
            .unwrap_or(0);

        Ok(VaultManifest {
            manifest_version: "1.0.0".to_string(),
            created_at: Utc::now(),
            vault_info,
            stats,
            assets_count,
            entities_count,
            memories_count: memories.len(),
            collections_count: collections.len(),
            relations_count: relations_count as usize,
        })
    }

    fn export_backup(&self, destination_dir: &Path) -> LoomaResult<BackupResult> {
        let start = std::time::Instant::now();
        let manifest = self.generate_manifest()?;

        let timestamp_str = Utc::now().format("%Y%m%d_%H%M%S").to_string();
        let backup_folder_name = format!("looma_vault_backup_{}", timestamp_str);
        let backup_path = destination_dir.join(backup_folder_name);

        std::fs::create_dir_all(&backup_path)
            .map_err(|e| LoomaError::Vault(format!("Failed to create backup directory: {e}")))?;

        // 1. Write manifest.json
        let manifest_json = serde_json::to_string_pretty(&manifest)
            .map_err(LoomaError::Serialization)?;
        std::fs::write(backup_path.join("manifest.json"), manifest_json)
            .map_err(|e| LoomaError::Vault(format!("Failed to write manifest.json: {e}")))?;

        // 2. Fetch all data
        let assets = self.query_assets(&AssetFilter::default())?;
        let entities = self.list_entities(&EntityFilter::default())?;
        let memories = self.list_memories(100000, 0)?;
        let collections = self.list_collections()?;

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

        // 3. Write vault_dump.json
        let dump = VaultExportDump {
            manifest: manifest.clone(),
            assets,
            entities,
            memories: memories.clone(),
            collections,
            relations,
        };
        let dump_json = serde_json::to_string_pretty(&dump)
            .map_err(LoomaError::Serialization)?;
        std::fs::write(backup_path.join("vault_dump.json"), dump_json)
            .map_err(|e| LoomaError::Vault(format!("Failed to write vault_dump.json: {e}")))?;

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
            std::fs::write(memories_dir.join(filename), md_content).ok();
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

        let dump_path = target_dir.join("vault_dump.json");
        if !dump_path.exists() {
            return Err(LoomaError::Vault(format!(
                "Backup archive invalid: vault_dump.json not found in {:?}",
                target_dir
            )));
        }

        let dump_str = std::fs::read_to_string(&dump_path)
            .map_err(|e| LoomaError::Vault(format!("Failed to read vault_dump.json: {e}")))?;
        let dump: VaultExportDump = serde_json::from_str(&dump_str)
            .map_err(LoomaError::Serialization)?;

        let mut restored_assets = 0;
        let mut restored_entities = 0;
        let mut restored_memories = 0;
        let mut restored_collections = 0;
        let mut restored_relations = 0;

        for asset in dump.assets {
            self.upsert_asset(&asset)?;
            restored_assets += 1;
        }

        for entity in dump.entities {
            if self.get_entity_by_id(&entity.id)?.is_some() {
                self.update_entity(&entity)?;
            } else {
                self.create_entity(&entity)?;
            }
            restored_entities += 1;
        }

        for memory in dump.memories {
            if self.get_memory_by_id(&memory.id)?.is_some() {
                self.update_memory(&memory)?;
            } else {
                self.create_memory(&memory)?;
            }
            restored_memories += 1;
        }

        for collection in dump.collections {
            if self.get_collection_by_id(&collection.id)?.is_none() {
                self.create_collection(&collection)?;
            }
            restored_collections += 1;
        }

        for relation in dump.relations {
            let _ = self.create_relation(&relation);
            restored_relations += 1;
        }

        let duration_ms = start.elapsed().as_millis() as u64;

        Ok(RestoreResult {
            restored_assets,
            restored_entities,
            restored_memories,
            restored_collections,
            restored_relations,
            duration_ms,
        })
    }

    fn doctor_inspect(&self) -> LoomaResult<VaultDoctorReport> {
        let assets = self.query_assets(&AssetFilter::default())?;
        let total_assets = assets.len();
        let mut active_assets = 0;
        let mut missing_assets = Vec::new();

        for asset in assets {
            if let Some(ref p) = asset.path {
                if Path::new(p).exists() {
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
