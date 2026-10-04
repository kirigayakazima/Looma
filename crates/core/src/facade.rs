use std::path::Path;
use std::sync::Arc;
use chrono::Utc;
use serde_json::json;
use uuid::Uuid;

use crate::error::{LoomaError, LoomaResult};
use crate::events::{DomainEvent, EventBus, EventEnvelope};
use crate::models::*;
use crate::services::*;

pub trait VaultRepository:
    VaultService
    + AssetService
    + EntityService
    + RelationService
    + MemoryService
    + CollectionService
    + TimelineService
    + BackupService
    + AuditService
    + ExternalReferenceService
    + Send
    + Sync
{
}

impl<T> VaultRepository for T where
    T: VaultService
        + AssetService
        + EntityService
        + RelationService
        + MemoryService
        + CollectionService
        + TimelineService
        + BackupService
        + AuditService
        + ExternalReferenceService
        + Send
        + Sync
{
}

/// Normalizes a directory path string for stable cross-platform deduplication and storage.
pub fn normalize_directory_path(raw: &str) -> String {
    let s = raw.trim();
    // Replace all backslashes with forward slashes
    let mut normalized = s.replace('\\', "/");
    // Strip Windows extended-length prefix \\?\ or //?/
    if let Some(stripped) = normalized.strip_prefix("//?/") {
        normalized = stripped.to_string();
    }
    // Remove duplicate slashes
    while normalized.contains("//") {
        normalized = normalized.replace("//", "/");
    }
    // Uppercase drive letter on Windows (e.g. "e:/..." -> "E:/...")
    if normalized.len() >= 2 && normalized.as_bytes()[1] == b':' {
        let drive = normalized[0..1].to_ascii_uppercase();
        normalized = format!("{}{}", drive, &normalized[1..]);
    }
    // Trim trailing slash unless it's root like "E:/" or "/"
    if normalized.len() > 3 && normalized.ends_with('/') {
        normalized.pop();
    }
    normalized
}

/// The unified Core Application Facade for Looma.
/// Desktop, CLI, and MCP MUST interact with this facade instead of directly manipulating database or internal state.
#[derive(Clone)]
pub struct LoomaCore {
    repo: Arc<dyn VaultRepository>,
    event_bus: EventBus,
}

impl LoomaCore {
    pub fn new(repo: Arc<dyn VaultRepository>) -> Self {
        Self {
            repo,
            event_bus: EventBus::new(),
        }
    }

    pub fn with_event_bus(repo: Arc<dyn VaultRepository>, event_bus: EventBus) -> Self {
        Self { repo, event_bus }
    }

    pub fn repository(&self) -> &Arc<dyn VaultRepository> {
        &self.repo
    }

    pub fn event_bus(&self) -> &EventBus {
        &self.event_bus
    }

    pub fn subscribe_events<F>(&self, handler: F)
    where
        F: Fn(&EventEnvelope) + Send + Sync + 'static,
    {
        self.event_bus.subscribe(handler);
    }

    fn emit_event(&self, actor: &str, event: DomainEvent) {
        self.event_bus.publish(EventEnvelope::new(actor, event));
    }

    // --- Audit Log ---
    pub fn record_audit(
        &self,
        actor: &str,
        operation: &str,
        target_type: &str,
        target_id: &str,
        result: &str,
        details: serde_json::Value,
    ) -> LoomaResult<()> {
        let log = AuditLog {
            id: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            actor: actor.to_string(),
            operation: operation.to_string(),
            target_type: target_type.to_string(),
            target_id: target_id.to_string(),
            result: result.to_string(),
            details,
        };
        self.repo.record_audit(&log)
    }

    pub fn list_recent_audits(&self, limit: usize) -> LoomaResult<Vec<AuditLog>> {
        self.repo.list_recent_audits(limit)
    }

    // --- Vault Meta & Stats ---
    pub fn get_vault_info(&self) -> LoomaResult<VaultInfo> {
        self.repo.get_vault_info()
    }

    pub fn get_vault_stats(&self) -> LoomaResult<VaultStats> {
        self.repo.get_vault_stats()
    }

    // --- Assets ---
    pub fn list_assets(&self, limit: usize, offset: usize) -> LoomaResult<Vec<Asset>> {
        self.repo.list_assets(limit, offset)
    }

    pub fn get_asset(&self, id: &str) -> LoomaResult<Option<Asset>> {
        self.repo.get_asset_by_id(id)
    }

    pub fn get_asset_by_path(&self, path: &str) -> LoomaResult<Option<Asset>> {
        self.repo.get_asset_by_path(path)
    }

    pub fn list_assets_by_prefix(&self, prefix: &str) -> LoomaResult<Vec<Asset>> {
        self.repo.list_assets_by_prefix(prefix)
    }

    pub fn count_assets(&self) -> LoomaResult<u64> {
        self.repo.count_assets()
    }

    pub fn count_entities(&self) -> LoomaResult<u64> {
        self.repo.count_entities()
    }

    pub fn upsert_asset(&self, actor: &str, asset: &Asset) -> LoomaResult<()> {
        self.repo.upsert_asset(asset)?;
        self.record_audit(
            actor,
            "asset.upsert",
            "asset",
            &asset.id,
            "success",
            json!({ "path": asset.path, "kind": format!("{:?}", asset.kind) }),
        )?;
        self.emit_event(
            actor,
            DomainEvent::AssetCreated {
                id: asset.id.clone(),
                path: asset.path.clone(),
            },
        );
        Ok(())
    }

    pub fn query_assets(&self, filter: &AssetFilter) -> LoomaResult<Vec<Asset>> {
        self.repo.query_assets(filter)
    }

    pub fn delete_asset(&self, actor: &str, id: &str) -> LoomaResult<bool> {
        let deleted = self.repo.delete_asset(id)?;
        if deleted {
            self.record_audit(actor, "asset.delete", "asset", id, "success", json!({}))?;
            self.emit_event(actor, DomainEvent::AssetDeleted { id: id.to_string() });
        }
        Ok(deleted)
    }

    // --- Entities ---
    pub fn list_entities(&self, filter: &EntityFilter) -> LoomaResult<Vec<Entity>> {
        self.repo.list_entities(filter)
    }

    pub fn get_entity(&self, id: &str) -> LoomaResult<Option<Entity>> {
        self.repo.get_entity_by_id(id)
    }

    pub fn create_entity(&self, actor: &str, entity: &Entity) -> LoomaResult<()> {
        if entity.title.trim().is_empty() {
            return Err(LoomaError::Validation("Entity title cannot be empty".to_string()));
        }
        self.repo.create_entity(entity)?;
        self.record_audit(
            actor,
            "entity.create",
            "entity",
            &entity.id,
            "success",
            json!({ "title": entity.title, "type": entity.entity_type }),
        )?;
        self.emit_event(
            actor,
            DomainEvent::EntityCreated {
                id: entity.id.clone(),
                title: entity.title.clone(),
            },
        );
        Ok(())
    }

    pub fn update_entity(&self, actor: &str, entity: &Entity) -> LoomaResult<()> {
        if entity.title.trim().is_empty() {
            return Err(LoomaError::Validation("Entity title cannot be empty".to_string()));
        }
        self.repo.update_entity(entity)?;
        self.record_audit(
            actor,
            "entity.update",
            "entity",
            &entity.id,
            "success",
            json!({ "title": entity.title, "type": entity.entity_type }),
        )?;
        self.emit_event(
            actor,
            DomainEvent::EntityUpdated {
                id: entity.id.clone(),
            },
        );
        Ok(())
    }

    pub fn delete_entity(&self, actor: &str, id: &str) -> LoomaResult<bool> {
        let deleted = self.repo.delete_entity(id)?;
        if deleted {
            self.record_audit(actor, "entity.delete", "entity", id, "success", json!({}))?;
            self.emit_event(actor, DomainEvent::EntityDeleted { id: id.to_string() });
        }
        Ok(deleted)
    }

    // --- Personal Records / Work Domain (Expansion Pack) ---
    pub fn create_work(
        &self,
        actor: &str,
        title: &str,
        work_type: WorkType,
        status: RecordStatus,
        original_title: Option<String>,
        description: Option<String>,
    ) -> LoomaResult<Entity> {
        let entity = Entity::new_work(title, work_type, status, original_title, description);
        self.create_entity(actor, &entity)?;
        Ok(entity)
    }

    pub fn update_work_status(
        &self,
        actor: &str,
        entity_id: &str,
        status: RecordStatus,
    ) -> LoomaResult<Entity> {
        let mut entity = self.get_entity(entity_id)?
            .ok_or_else(|| LoomaError::NotFound(format!("Work entity not found: {entity_id}")))?;
        
        let mut props = entity.properties.clone();
        if let Some(obj) = props.as_object_mut() {
            obj.insert("status".to_string(), json!(status.as_str()));
            if let Some(work_val) = obj.get_mut("work") {
                if let Some(work_obj) = work_val.as_object_mut() {
                    work_obj.insert("status".to_string(), json!(status));
                }
            }
        }
        entity.properties = props;
        entity.updated_at = Utc::now();
        self.update_entity(actor, &entity)?;
        Ok(entity)
    }

    pub fn update_work_progress(
        &self,
        actor: &str,
        entity_id: &str,
        position: f64,
        position_type: Option<ProgressPositionType>,
        total_positions: Option<f64>,
        unit: Option<String>,
    ) -> LoomaResult<Entity> {
        let mut entity = self.get_entity(entity_id)?
            .ok_or_else(|| LoomaError::NotFound(format!("Work entity not found: {entity_id}")))?;

        let mut props = entity.properties.clone();
        if let Some(obj) = props.as_object_mut() {
            let mut work_meta: WorkMetadata = if let Some(w) = obj.get("work") {
                serde_json::from_value(w.clone()).unwrap_or_else(|_| WorkMetadata {
                    work_type: WorkType::parse(&entity.entity_type),
                    status: RecordStatus::InProgress,
                    ..Default::default()
                })
            } else {
                WorkMetadata {
                    work_type: WorkType::parse(&entity.entity_type),
                    status: RecordStatus::InProgress,
                    ..Default::default()
                }
            };

            let p_type = position_type.unwrap_or_else(|| {
                work_meta.progress.as_ref().map(|p| p.position_type.clone()).unwrap_or_else(|| {
                    match work_meta.work_type {
                        WorkType::Anime | WorkType::TvSeries => ProgressPositionType::Episode,
                        WorkType::Manga => ProgressPositionType::Chapter,
                        WorkType::Book | WorkType::Novel => ProgressPositionType::Chapter,
                        WorkType::Movie | WorkType::Documentary => ProgressPositionType::Minute,
                        _ => ProgressPositionType::Custom("step".to_string()),
                    }
                })
            });

            let total = total_positions.or_else(|| work_meta.progress.as_ref().and_then(|p| p.total_positions));
            let unit_str = unit.or_else(|| work_meta.progress.as_ref().and_then(|p| p.unit.clone()));

            let progress = WorkProgress {
                position,
                position_type: p_type,
                total_positions: total,
                unit: unit_str,
                updated_at: Utc::now(),
            };

            work_meta.progress = Some(progress.clone());
            obj.insert("work".to_string(), json!(work_meta));
            obj.insert("progress".to_string(), json!(progress));
        }

        entity.properties = props;
        entity.updated_at = Utc::now();
        self.update_entity(actor, &entity)?;

        self.record_audit(
            actor,
            "work.progress_update",
            "entity",
            entity_id,
            "success",
            json!({ "position": position }),
        )?;

        self.emit_event(
            actor,
            DomainEvent::EntityUpdated { id: entity_id.to_string() },
        );

        Ok(entity)
    }

    pub fn update_work_cover_asset(
        &self,
        actor: &str,
        entity_id: &str,
        cover_asset_id: Option<String>,
    ) -> LoomaResult<Entity> {
        let mut entity = self.get_entity(entity_id)?
            .ok_or_else(|| LoomaError::NotFound(format!("Work entity not found: {entity_id}")))?;

        entity.set_cover_asset_id(cover_asset_id.as_deref());
        self.update_entity(actor, &entity)?;

        self.record_audit(
            actor,
            "work.cover_update",
            "entity",
            entity_id,
            "success",
            json!({ "cover_asset_id": cover_asset_id }),
        )?;

        self.emit_event(
            actor,
            DomainEvent::EntityUpdated { id: entity_id.to_string() },
        );

        Ok(entity)
    }

    pub fn list_works(
        &self,
        work_type: Option<&str>,
        status: Option<&str>,
    ) -> LoomaResult<Vec<Entity>> {
        let all_entities = self.list_entities(&EntityFilter::default())?;
        let filtered = all_entities
            .into_iter()
            .filter(|e| {
                let meta = e.as_work_metadata();
                if meta.is_none() && e.entity_type != "work" && e.properties.get("domain").and_then(|v| v.as_str()) != Some("work") {
                    if WorkType::parse(&e.entity_type) == WorkType::Other && e.entity_type != "other" {
                        return false;
                    }
                }

                if let Some(wt) = work_type {
                    let parsed = WorkType::parse(wt);
                    let e_type = meta.as_ref().map(|m| m.work_type).unwrap_or_else(|| WorkType::parse(&e.entity_type));
                    if e_type != parsed && e.entity_type != wt {
                        return false;
                    }
                }

                if let Some(st) = status {
                    let parsed_st = RecordStatus::parse(st);
                    let e_st = meta.as_ref().map(|m| m.status).unwrap_or_else(|| {
                        e.properties.get("status")
                            .and_then(|v| v.as_str())
                            .map(RecordStatus::parse)
                            .unwrap_or(RecordStatus::Unknown)
                    });
                    if e_st != parsed_st {
                        return false;
                    }
                }

                true
            })
            .collect();
        Ok(filtered)
    }

    pub fn get_work_summary(&self, entity_id: &str) -> LoomaResult<Option<WorkSummary>> {
        let entity = match self.get_entity(entity_id)? {
            Some(e) => e,
            None => return Ok(None),
        };

        let work_metadata = entity.as_work_metadata();
        let relations = self.list_relations_for_item(entity_id)?;
        let external_references = self.list_external_references(Some(entity_id))?;

        let mut linked_assets = Vec::new();
        let mut linked_memories = Vec::new();

        for r in &relations {
            let other_id = if r.source_id == entity_id {
                &r.target_id
            } else {
                &r.source_id
            };
            let other_type = if r.source_id == entity_id {
                &r.target_type
            } else {
                &r.source_type
            };

            if other_type == "asset" {
                if let Ok(Some(mut asset)) = self.get_asset(other_id) {
                    if let Some(ref p) = asset.path {
                        let exists = Path::new(p).exists();
                        if !exists && asset.status == AssetStatus::Active {
                            asset.status = AssetStatus::Missing;
                            let _ = self.repo.upsert_asset(&asset);
                        } else if exists && asset.status == AssetStatus::Missing {
                            asset.status = AssetStatus::Active;
                            let _ = self.repo.upsert_asset(&asset);
                        }
                    }
                    linked_assets.push(asset);
                }
            } else if other_type == "memory" {
                if let Ok(Some(memory)) = self.get_memory(other_id) {
                    linked_memories.push(memory);
                }
            }
        }

        let mut linked_collections = Vec::new();
        if let Ok(collections) = self.list_collections() {
            for col in collections {
                if let Ok(items) = self.list_collection_items(&col.id) {
                    if items.iter().any(|item| item.item_id == entity_id) {
                        linked_collections.push(col);
                    }
                }
            }
        }

        Ok(Some(WorkSummary {
            entity,
            work_metadata,
            relations,
            linked_assets,
            linked_memories,
            linked_collections,
            external_references,
        }))
    }

    pub fn link_work_asset(
        &self,
        actor: &str,
        work_id: &str,
        asset_id: &str,
        relation_type: Option<&str>,
    ) -> LoomaResult<Relation> {
        // Enforce Data Integrity Invariant: target asset and source work must exist
        if self.get_asset(asset_id)?.is_none() {
            return Err(LoomaError::NotFound(format!(
                "Cannot link nonexistent asset: {asset_id}"
            )));
        }
        if self.get_entity(work_id)?.is_none() {
            return Err(LoomaError::NotFound(format!(
                "Cannot link nonexistent work entity: {work_id}"
            )));
        }

        let r_type = relation_type.unwrap_or(relation_types::ATTACHES);
        let existing = self.list_relations_for_item(work_id)?;
        if let Some(r) = existing.into_iter().find(|r| {
            r.relation_type == r_type && (
                (r.source_id == work_id && r.target_id == asset_id) ||
                (r.source_id == asset_id && r.target_id == work_id)
            )
        }) {
            return Ok(r);
        }

        let rel = Relation {
            id: format!("rel_{}", &Uuid::new_v4().to_string()[..8]),
            source_id: work_id.to_string(),
            source_type: "entity".to_string(),
            relation_type: r_type.to_string(),
            target_id: asset_id.to_string(),
            target_type: "asset".to_string(),
            metadata: json!({ "domain": "personal_records" }),
            created_at: Utc::now(),
        };
        self.create_relation(actor, &rel)?;
        Ok(rel)
    }

    pub fn ensure_directory_asset(
        &self,
        actor: &str,
        raw_path: &str,
    ) -> LoomaResult<Asset> {
        let norm_path = normalize_directory_path(raw_path);
        if norm_path.is_empty() {
            return Err(LoomaError::Validation("Directory path cannot be empty".to_string()));
        }

        // Case A: Asset already exists in database
        if let Some(existing) = self.get_asset_by_path(&norm_path)? {
            return Ok(existing);
        }

        // Case C: Path does not exist on disk or is not a directory
        let p = Path::new(&norm_path);
        if !p.exists() {
            return Err(LoomaError::Validation(format!(
                "Directory path does not exist: {norm_path}"
            )));
        }
        if !p.is_dir() {
            return Err(LoomaError::Validation(format!(
                "Path is not a directory: {norm_path}"
            )));
        }

        // Case B: Create new Directory Asset
        let dir_name = p
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| norm_path.clone());

        let drive = if norm_path.len() >= 2 && &norm_path[1..2] == ":" {
            norm_path[0..2].to_uppercase()
        } else {
            "Local".to_string()
        };

        let now = Utc::now();
        let new_asset = Asset {
            id: Uuid::new_v4().to_string(),
            kind: AssetKind::Directory,
            source: AssetSource::Local,
            path: Some(norm_path.clone()),
            size: None,
            hash: None,
            mime_type: Some("inode/directory".to_string()),
            metadata: json!({
                "directory_name": dir_name,
                "drive": drive,
            }),
            status: AssetStatus::Active,
            created_at: now,
            modified_at: now,
            indexed_at: now,
        };

        self.upsert_asset(actor, &new_asset)?;
        Ok(new_asset)
    }

    pub fn link_work_directory(
        &self,
        actor: &str,
        work_id: &str,
        dir_path: &str,
    ) -> LoomaResult<(Asset, Relation)> {
        let asset = self.ensure_directory_asset(actor, dir_path)?;
        let rel = self.link_work_asset(actor, work_id, &asset.id, Some(relation_types::ATTACHES))?;
        Ok((asset, rel))
    }

    /// Atomically relocates a work's directory asset to a new filesystem path without modifying the Work identity.
    pub fn relocate_work_directory(
        &self,
        actor: &str,
        work_id: &str,
        old_asset_id: &str,
        new_dir_path: &str,
    ) -> LoomaResult<(Asset, Relation)> {
        let (new_asset, new_rel) = self.link_work_directory(actor, work_id, new_dir_path)?;
        let _ = self.delete_relation_between(actor, work_id, old_asset_id);
        let _ = self.delete_relation_between(actor, old_asset_id, work_id);
        self.record_audit(
            actor,
            "work.relocate_directory",
            "entity",
            work_id,
            "success",
            json!({
                "old_asset_id": old_asset_id,
                "new_asset_id": new_asset.id,
                "new_path": new_asset.path,
            }),
        )?;
        Ok((new_asset, new_rel))
    }

    /// Update type-specific metadata overlay on a Work entity
    pub fn update_work_type_metadata(
        &self,
        actor: &str,
        entity_id: &str,
        meta: Option<TypeSpecificMetadata>,
    ) -> LoomaResult<Entity> {
        let mut entity = self.get_entity(entity_id)?
            .ok_or_else(|| LoomaError::NotFound(format!("Work entity not found: {entity_id}")))?;
        entity.set_type_metadata(meta.clone());
        self.update_entity(actor, &entity)?;
        self.record_audit(
            actor,
            "work.type_metadata_update",
            "entity",
            entity_id,
            "success",
            json!({ "type_metadata": meta }),
        )?;
        self.emit_event(
            actor,
            DomainEvent::EntityUpdated { id: entity_id.to_string() },
        );
        Ok(entity)
    }

    pub fn link_work_memory(
        &self,
        actor: &str,
        work_id: &str,
        memory_id: &str,
    ) -> LoomaResult<Relation> {
        let rel = Relation {
            id: format!("rel_{}", &Uuid::new_v4().to_string()[..8]),
            source_id: memory_id.to_string(),
            source_type: "memory".to_string(),
            relation_type: relation_types::REFERENCED_BY.to_string(),
            target_id: work_id.to_string(),
            target_type: "entity".to_string(),
            metadata: json!({ "domain": "personal_records" }),
            created_at: Utc::now(),
        };
        self.create_relation(actor, &rel)?;
        Ok(rel)
    }

    // --- Relations ---
    pub fn list_relations_for_item(&self, item_id: &str) -> LoomaResult<Vec<Relation>> {
        self.repo.list_relations_for_item(item_id)
    }

    pub fn list_relations_for_source(&self, source_id: &str) -> LoomaResult<Vec<Relation>> {
        self.repo.list_relations_for_source(source_id)
    }

    pub fn list_relations_for_target(&self, target_id: &str) -> LoomaResult<Vec<Relation>> {
        self.repo.list_relations_for_target(target_id)
    }

    pub fn create_relation(&self, actor: &str, relation: &Relation) -> LoomaResult<()> {
        self.repo.create_relation(relation)?;
        self.record_audit(
            actor,
            "relation.create",
            "relation",
            &relation.id,
            "success",
            json!({
                "source_id": relation.source_id,
                "target_id": relation.target_id,
                "relation_type": relation.relation_type,
            }),
        )?;
        self.emit_event(
            actor,
            DomainEvent::RelationCreated {
                id: relation.id.clone(),
                source_id: relation.source_id.clone(),
                target_id: relation.target_id.clone(),
            },
        );
        Ok(())
    }

    pub fn delete_relation(&self, actor: &str, id: &str) -> LoomaResult<bool> {
        let deleted = self.repo.delete_relation(id)?;
        if deleted {
            self.record_audit(actor, "relation.delete", "relation", id, "success", json!({}))?;
            self.emit_event(actor, DomainEvent::RelationDeleted { id: id.to_string() });
        }
        Ok(deleted)
    }

    pub fn delete_relation_between(&self, actor: &str, source_id: &str, target_id: &str) -> LoomaResult<bool> {
        let deleted = self.repo.delete_relation_between(source_id, target_id)?;
        if deleted {
            self.record_audit(
                actor,
                "relation.delete_between",
                "relation",
                &format!("{source_id}<->{target_id}"),
                "success",
                json!({ "source_id": source_id, "target_id": target_id }),
            )?;
            self.emit_event(actor, DomainEvent::RelationDeleted { id: format!("{source_id}<->{target_id}") });
        }
        Ok(deleted)
    }

    // --- Memories ---
    pub fn list_memories(&self, limit: usize, offset: usize) -> LoomaResult<Vec<Memory>> {
        self.repo.list_memories(limit, offset)
    }

    pub fn get_memory(&self, id: &str) -> LoomaResult<Option<Memory>> {
        self.repo.get_memory_by_id(id)
    }

    pub fn create_memory(&self, actor: &str, memory: &Memory) -> LoomaResult<()> {
        if memory.title.trim().is_empty() {
            return Err(LoomaError::Validation("Memory title cannot be empty".to_string()));
        }
        self.repo.create_memory(memory)?;
        self.record_audit(
            actor,
            "memory.create",
            "memory",
            &memory.id,
            "success",
            json!({ "title": memory.title }),
        )?;
        self.emit_event(
            actor,
            DomainEvent::MemoryCreated {
                id: memory.id.clone(),
                title: memory.title.clone(),
            },
        );
        Ok(())
    }

    pub fn update_memory(&self, actor: &str, memory: &Memory) -> LoomaResult<()> {
        if memory.title.trim().is_empty() {
            return Err(LoomaError::Validation("Memory title cannot be empty".to_string()));
        }
        self.repo.update_memory(memory)?;
        self.record_audit(
            actor,
            "memory.update",
            "memory",
            &memory.id,
            "success",
            json!({ "title": memory.title }),
        )?;
        self.emit_event(
            actor,
            DomainEvent::MemoryUpdated {
                id: memory.id.clone(),
            },
        );
        Ok(())
    }

    pub fn delete_memory(&self, actor: &str, id: &str) -> LoomaResult<bool> {
        let deleted = self.repo.delete_memory(id)?;
        if deleted {
            self.record_audit(actor, "memory.delete", "memory", id, "success", json!({}))?;
            self.emit_event(actor, DomainEvent::MemoryDeleted { id: id.to_string() });
        }
        Ok(deleted)
    }

    // --- Collections ---
    pub fn list_collections(&self) -> LoomaResult<Vec<Collection>> {
        self.repo.list_collections()
    }

    pub fn get_collection(&self, id: &str) -> LoomaResult<Option<Collection>> {
        self.repo.get_collection_by_id(id)
    }

    pub fn create_collection(&self, actor: &str, collection: &Collection) -> LoomaResult<()> {
        if collection.title.trim().is_empty() {
            return Err(LoomaError::Validation("Collection title cannot be empty".to_string()));
        }
        self.repo.create_collection(collection)?;
        self.record_audit(
            actor,
            "collection.create",
            "collection",
            &collection.id,
            "success",
            json!({ "title": collection.title }),
        )?;
        Ok(())
    }

    pub fn delete_collection(&self, actor: &str, id: &str) -> LoomaResult<bool> {
        let deleted = self.repo.delete_collection(id)?;
        if deleted {
            self.record_audit(actor, "collection.delete", "collection", id, "success", json!({}))?;
        }
        Ok(deleted)
    }

    pub fn list_collection_items(&self, collection_id: &str) -> LoomaResult<Vec<CollectionItem>> {
        self.repo.list_items_for_collection(collection_id)
    }

    pub fn add_item_to_collection(&self, actor: &str, collection_id: &str, item_id: &str, item_type: &str) -> LoomaResult<()> {
        self.repo.add_item_to_collection(collection_id, item_id, item_type)?;
        self.record_audit(
            actor,
            "collection.add_item",
            "collection_item",
            item_id,
            "success",
            json!({ "collection_id": collection_id, "item_type": item_type }),
        )?;
        Ok(())
    }

    pub fn remove_item_from_collection(&self, actor: &str, collection_id: &str, item_id: &str) -> LoomaResult<bool> {
        let removed = self.repo.remove_item_from_collection(collection_id, item_id)?;
        if removed {
            self.record_audit(
                actor,
                "collection.remove_item",
                "collection_item",
                item_id,
                "success",
                json!({ "collection_id": collection_id }),
            )?;
        }
        Ok(removed)
    }

    // --- External References (P1) ---
    pub fn list_external_references(&self, entity_id: Option<&str>) -> LoomaResult<Vec<ExternalReference>> {
        self.repo.list_external_references(entity_id)
    }

    pub fn get_external_reference(&self, id: &str) -> LoomaResult<Option<ExternalReference>> {
        self.repo.get_external_reference_by_id(id)
    }

    pub fn create_external_reference(&self, actor: &str, reference: &ExternalReference) -> LoomaResult<()> {
        if reference.url.trim().is_empty() {
            return Err(LoomaError::Validation("External reference URL cannot be empty".to_string()));
        }
        self.repo.create_external_reference(reference)?;
        self.record_audit(
            actor,
            "external_reference.create",
            "external_reference",
            &reference.id,
            "success",
            json!({ "url": reference.url, "provider": reference.provider }),
        )?;
        self.emit_event(
            actor,
            DomainEvent::ExternalReferenceCreated {
                id: reference.id.clone(),
                url: reference.url.clone(),
            },
        );
        Ok(())
    }

    pub fn update_external_reference(&self, actor: &str, reference: &ExternalReference) -> LoomaResult<()> {
        if reference.url.trim().is_empty() {
            return Err(LoomaError::Validation("External reference URL cannot be empty".to_string()));
        }
        self.repo.update_external_reference(reference)?;
        self.record_audit(
            actor,
            "external_reference.update",
            "external_reference",
            &reference.id,
            "success",
            json!({ "url": reference.url, "provider": reference.provider }),
        )?;
        Ok(())
    }

    pub fn delete_external_reference(&self, actor: &str, id: &str) -> LoomaResult<bool> {
        let deleted = self.repo.delete_external_reference(id)?;
        if deleted {
            self.record_audit(actor, "external_reference.delete", "external_reference", id, "success", json!({}))?;
            self.emit_event(actor, DomainEvent::ExternalReferenceDeleted { id: id.to_string() });
        }
        Ok(deleted)
    }

    // --- Timeline ---
    pub fn query_timeline(&self, filter: &TimelineFilter) -> LoomaResult<Vec<TimelineItem>> {
        self.repo.query_timeline(filter)
    }

    // --- Backup & Doctor ---
    pub fn generate_manifest(&self) -> LoomaResult<VaultManifest> {
        self.repo.generate_manifest()
    }

    pub fn export_backup(&self, actor: &str, destination_dir: &Path) -> LoomaResult<BackupResult> {
        let res = self.repo.export_backup(destination_dir)?;
        self.record_audit(
            actor,
            "backup.export",
            "backup",
            &res.backup_path,
            "success",
            json!({ "duration_ms": res.duration_ms }),
        )?;
        self.emit_event(
            actor,
            DomainEvent::BackupCompleted {
                path: res.backup_path.clone(),
                duration_ms: res.duration_ms,
            },
        );
        Ok(res)
    }

    pub fn restore_backup(&self, actor: &str, backup_dir_or_file: &Path) -> LoomaResult<RestoreResult> {
        let res = self.repo.restore_backup(backup_dir_or_file)?;
        self.record_audit(
            actor,
            "backup.restore",
            "backup",
            &backup_dir_or_file.to_string_lossy(),
            "success",
            json!({ "restored_assets": res.restored_assets, "duration_ms": res.duration_ms }),
        )?;
        Ok(res)
    }

    pub fn doctor_inspect(&self) -> LoomaResult<VaultDoctorReport> {
        self.repo.doctor_inspect()
    }

    pub fn doctor_cleanup_missing(&self, actor: &str) -> LoomaResult<usize> {
        let count = self.repo.doctor_cleanup_missing()?;
        self.record_audit(
            actor,
            "doctor.cleanup_missing",
            "system",
            "vault",
            "success",
            json!({ "cleaned_count": count }),
        )?;
        Ok(count)
    }
}

impl AssetService for LoomaCore {
    fn list_assets(&self, limit: usize, offset: usize) -> LoomaResult<Vec<Asset>> {
        self.repo.list_assets(limit, offset)
    }

    fn query_assets(&self, filter: &AssetFilter) -> LoomaResult<Vec<Asset>> {
        self.repo.query_assets(filter)
    }

    fn get_asset_by_id(&self, id: &str) -> LoomaResult<Option<Asset>> {
        self.repo.get_asset_by_id(id)
    }

    fn get_asset_by_path(&self, path: &str) -> LoomaResult<Option<Asset>> {
        self.repo.get_asset_by_path(path)
    }

    fn list_assets_by_prefix(&self, prefix: &str) -> LoomaResult<Vec<Asset>> {
        self.repo.list_assets_by_prefix(prefix)
    }

    fn upsert_asset(&self, asset: &Asset) -> LoomaResult<()> {
        self.repo.upsert_asset(asset)
    }

    fn delete_asset(&self, id: &str) -> LoomaResult<bool> {
        self.repo.delete_asset(id)
    }

    fn count_assets(&self) -> LoomaResult<u64> {
        self.repo.count_assets()
    }
}
