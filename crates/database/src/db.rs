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
