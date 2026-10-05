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
            let existing_status = entity.as_work_metadata().map(|m| m.status).unwrap_or_else(|| {
                entity.properties.get("status")
                    .and_then(|v| v.as_str())
                    .map(RecordStatus::parse)
                    .unwrap_or(RecordStatus::Planned)
            });

            let mut work_meta: WorkMetadata = if let Some(w) = obj.get("work") {
                serde_json::from_value(w.clone()).unwrap_or_else(|_| WorkMetadata {
                    work_type: WorkType::parse(&entity.entity_type),
                    status: existing_status,
                    ..Default::default()
                })
            } else {
                WorkMetadata {
                    work_type: WorkType::parse(&entity.entity_type),
                    status: existing_status,
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

    pub fn update_work_aliases(
        &self,
        actor: &str,
        entity_id: &str,
        aliases: Vec<String>,
    ) -> LoomaResult<Entity> {
        let mut entity = self.get_entity(entity_id)?
            .ok_or_else(|| LoomaError::NotFound(format!("Work entity not found: {entity_id}")))?;

        entity.set_aliases(aliases.clone());
        self.update_entity(actor, &entity)?;

        self.record_audit(
            actor,
            "work.aliases_update",
            "entity",
            entity_id,
            "success",
            json!({ "aliases": aliases }),
        )?;

        self.emit_event(
            actor,
            DomainEvent::EntityUpdated { id: entity_id.to_string() },
        );

        Ok(entity)
    }

    pub fn set_primary_external_reference(
        &self,
        actor: &str,
        entity_id: &str,
        reference_id: &str,
    ) -> LoomaResult<Vec<ExternalReference>> {
        let res: LoomaResult<Vec<ExternalReference>> = (|| {
            // Pre-validation 1: Entity exists
            if self.get_entity(entity_id)?.is_none() {
                return Err(LoomaError::NotFound(format!("Entity not found: {entity_id}")));
            }

            // Pre-validation 2: Reference exists and belongs to entity
            let r = self.get_external_reference(reference_id)?
                .ok_or_else(|| LoomaError::NotFound(format!("External reference not found: {reference_id}")))?;
            if r.entity_id.as_deref() != Some(entity_id) {
                return Err(LoomaError::Validation(format!(
                    "External reference {reference_id} does not belong to entity {entity_id}"
                )));
            }

            // Execute atomic transaction in repository
            self.repo.set_primary_external_reference(entity_id, reference_id)?;

            self.list_external_references(Some(entity_id))
        })();

        match res {
            Ok(updated) => {
                self.record_audit(
                    actor,
                    "work.set_primary_external_reference",
                    "entity",
                    entity_id,
                    "success",
                    json!({ "primary_reference_id": reference_id }),
                )?;

                self.emit_event(
                    actor,
                    DomainEvent::EntityUpdated {
                        id: entity_id.to_string(),
                    },
                );

                Ok(updated)
            }
            Err(e) => {
                let _ = self.record_audit(
                    actor,
                    "work.set_primary_external_reference",
                    "entity",
                    entity_id,
                    "failure",
                    json!({
                        "target_reference_id": reference_id,
                        "error": e.to_string(),
                    }),
                );
                Err(e)
            }
        }
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

    /// Search works with broad SQLite filtering followed by strict Rust semantic verification (title, description, original_title, aliases)
    pub fn search_works(
        &self,
        work_type: Option<&str>,
        status: Option<&str>,
        query: Option<&str>,
    ) -> LoomaResult<Vec<Entity>> {
        let mut filter = EntityFilter::default();
        if let Some(q) = query {
            let trimmed = q.trim();
            if !trimmed.is_empty() {
                filter.search_query = Some(trimmed.to_string());
            }
        }
        let candidates = self.list_entities(&filter)?;
        let filtered = candidates
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

                if let Some(q) = query {
                    if !e.matches_query(q) {
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

    /// Lists all directory assets attached to a Work entity via strict Work -> Directory Asset 'attaches' relations.
    pub fn list_work_directory_attachments(
        &self,
        work_id: &str,
    ) -> LoomaResult<Vec<(Asset, Relation)>> {
        let entity = self.get_entity(work_id)?
            .ok_or_else(|| LoomaError::NotFound(format!("Work entity '{work_id}' not found")))?;

        if entity.as_work_metadata().is_none() && entity.entity_type != "work" {
            return Err(LoomaError::Validation(format!(
                "Entity '{work_id}' is of type '{}', expected a work entity",
                entity.entity_type
            )));
        }

        let relations = self.list_relations_for_source(work_id)?;
        let mut results = Vec::new();

        for rel in relations {
            if rel.relation_type == relation_types::ATTACHES && rel.target_type == "asset" {
                if let Some(asset) = self.get_asset(&rel.target_id)? {
                    if asset.kind == AssetKind::Directory {
                        results.push((asset, rel));
                    }
                }
            }
        }

        Ok(results)
    }

    /// Read-only validation of all invariants and integrity rules for a given Work entity.
    pub fn validate_work_integrity(&self, work_id: &str) -> LoomaResult<IntegrityReport> {
        let mut violations = Vec::new();

        // 1. Work Entity existence and type check
        let entity = match self.get_entity(work_id)? {
            Some(e) => e,
            None => {
                return Ok(IntegrityReport::with_violations(
                    vec![IntegrityViolation::with_context(
                        IntegrityViolationCode::WorkNotFound,
                        format!("Work entity '{work_id}' not found"),
                        Some(work_id.to_string()),
                        None,
                        None,
                    )],
                    0,
                    0,
                    0,
                ));
            }
        };

        if entity.as_work_metadata().is_none() && entity.entity_type != "work" {
            return Ok(IntegrityReport::with_violations(
                vec![IntegrityViolation::with_context(
                    IntegrityViolationCode::NotAWork,
                    format!("Entity '{work_id}' has entity_type '{}', expected a work entity", entity.entity_type),
                    Some(work_id.to_string()),
                    None,
                    None,
                )],
                0,
                0,
                0,
            ));
        }

        // 2. External References Primary Invariant (at most 1 primary)
        let ext_refs = self.list_external_references(Some(work_id))?;
        let primary_count = ext_refs.iter().filter(|r| r.is_primary()).count();
        if primary_count > 1 {
            violations.push(IntegrityViolation::with_context(
                IntegrityViolationCode::DuplicatePrimary,
                format!(
                    "Work '{work_id}' has {primary_count} primary external references, maximum allowed is 1"
                ),
                Some(work_id.to_string()),
                None,
                None,
            ));
        }

        // 3. Relations and Attachment Invariants
        let relations = self.list_relations_for_item(work_id)?;
        let mut seen_identities = std::collections::HashSet::new();
        let mut seen_attached_assets = std::collections::HashSet::new();
        let mut checked_assets = std::collections::HashSet::new();

        for r in &relations {
            // 3a. Self-relation check
            if r.source_id == r.target_id {
                violations.push(IntegrityViolation::with_context(
                    IntegrityViolationCode::SelfRelation,
                    format!(
                        "Relation '{}' has identical source and target endpoint: '{}'",
                        r.id, r.source_id
                    ),
                    Some(work_id.to_string()),
                    Some(r.id.clone()),
                    None,
                ));
            }

            // 3b. Missing endpoints check
            if !self.check_item_exists(&r.source_id, &r.source_type)? {
                violations.push(IntegrityViolation::with_context(
                    IntegrityViolationCode::MissingRelationEndpoint,
                    format!(
                        "Relation '{}' source {} '{}' does not exist",
                        r.id, r.source_type, r.source_id
                    ),
                    Some(work_id.to_string()),
                    Some(r.id.clone()),
                    None,
                ));
            }
            if !self.check_item_exists(&r.target_id, &r.target_type)? {
                violations.push(IntegrityViolation::with_context(
                    IntegrityViolationCode::MissingRelationEndpoint,
                    format!(
                        "Relation '{}' target {} '{}' does not exist",
                        r.id, r.target_type, r.target_id
                    ),
                    Some(work_id.to_string()),
                    Some(r.id.clone()),
                    None,
                ));
            }

            // 3c. Duplicate relation check (Logical Identity: source_id, relation_type, target_id)
            let identity = (r.source_id.clone(), r.relation_type.clone(), r.target_id.clone());
            if !seen_identities.insert(identity.clone()) {
                violations.push(IntegrityViolation::with_context(
                    IntegrityViolationCode::DuplicateRelation,
                    format!(
                        "Duplicate relation detected: source='{}', type='{}', target='{}'",
                        identity.0, identity.1, identity.2
                    ),
                    Some(work_id.to_string()),
                    Some(r.id.clone()),
                    None,
                ));
            }

            // 3d. Attachment checks
            if r.relation_type == relation_types::ATTACHES {
                if r.target_id == work_id {
                    // Reverse attachment detected!
                    violations.push(IntegrityViolation::with_context(
                        IntegrityViolationCode::InvalidAttachmentDirection,
                        format!(
                            "Reverse attachment detected: asset '{}' -> work '{}'",
                            r.source_id, r.target_id
                        ),
                        Some(work_id.to_string()),
                        Some(r.id.clone()),
                        None,
                    ));
                } else if r.source_id == work_id {
                    // Target must be directory asset
                    checked_assets.insert(r.target_id.clone());
                    if let Ok(Some(asset)) = self.get_asset(&r.target_id) {
                        if asset.kind != AssetKind::Directory {
                            violations.push(IntegrityViolation::with_context(
                                IntegrityViolationCode::InvalidAttachmentKind,
                                format!(
                                    "Attachment target asset '{}' is of kind '{:?}', expected Directory",
                                    asset.id, asset.kind
                                ),
                                Some(work_id.to_string()),
                                Some(r.id.clone()),
                                None,
                            ));
                        }
                    }
                    if !seen_attached_assets.insert(r.target_id.clone()) {
                        violations.push(IntegrityViolation::with_context(
                            IntegrityViolationCode::DuplicateRelation,
                            format!(
                                "Work '{work_id}' has duplicate attachment to directory asset '{}'",
                                r.target_id
                            ),
                            Some(work_id.to_string()),
                            Some(r.id.clone()),
                            None,
                        ));
                    }
                }
            }
        }

        Ok(IntegrityReport::with_violations(
            violations,
            relations.len(),
            ext_refs.len(),
            checked_assets.len(),
        ))
    }

    /// Deterministic, read-only preview of repair actions for an integrity violation on a Work entity.
    /// Does NOT perform any writes to the database.
    pub fn preview_work_integrity_repair(&self, work_id: &str) -> LoomaResult<IntegrityRepairPlan> {
        let report = self.validate_work_integrity(work_id)?;
        let mut actions = Vec::new();
        let mut safe = true;

        if report.valid {
            return Ok(IntegrityRepairPlan {
                work_id: work_id.to_string(),
                safe: true,
                actions: Vec::new(),
            });
        }

        for v in &report.violations {
            match v.code {
                IntegrityViolationCode::DuplicatePrimary => {
                    let refs = self.list_external_references(Some(work_id))?;
                    let mut primary_refs: Vec<_> = refs.into_iter().filter(|r| r.is_primary()).collect();
                    primary_refs.sort_by(|a, b| a.created_at.cmp(&b.created_at).then_with(|| a.id.cmp(&b.id)));
                    if primary_refs.len() > 1 {
                        for demotee in &primary_refs[1..] {
                            actions.push(IntegrityRepairAction {
                                kind: RepairActionKind::DemoteDuplicatePrimary,
                                description: format!("Demote redundant primary flag on reference '{}' (retaining '{}')", demotee.id, primary_refs[0].id),
                                target_id: demotee.id.clone(),
                                details: json!({
                                    "retained_primary_id": primary_refs[0].id,
                                    "demoted_reference_id": demotee.id,
                                }),
                            });
                        }
                    }
                }
                IntegrityViolationCode::DuplicateRelation => {
                    let rel_id = v.relation_id.clone().unwrap_or_default();
                    actions.push(IntegrityRepairAction {
                        kind: RepairActionKind::RemoveDuplicateRelation,
                        description: format!("Remove redundant duplicate relation '{}'", rel_id),
                        target_id: rel_id,
                        details: json!({ "violation_message": v.message }),
                    });
                }
                IntegrityViolationCode::InvalidAttachmentDirection | IntegrityViolationCode::InvalidAttachmentKind => {
                    let rel_id = v.relation_id.clone().unwrap_or_default();
                    actions.push(IntegrityRepairAction {
                        kind: RepairActionKind::DetachInvalidAttachment,
                        description: format!("Detach invalid attachment relation '{}'", rel_id),
                        target_id: rel_id,
                        details: json!({ "violation_message": v.message }),
                    });
                }
                IntegrityViolationCode::SelfRelation => {
                    let rel_id = v.relation_id.clone().unwrap_or_default();
                    actions.push(IntegrityRepairAction {
                        kind: RepairActionKind::RemoveSelfRelation,
                        description: format!("Remove self-referencing relation '{}'", rel_id),
                        target_id: rel_id,
                        details: json!({ "violation_message": v.message }),
                    });
                }
                IntegrityViolationCode::MissingRelationEndpoint => {
                    safe = false;
                    let rel_id = v.relation_id.clone().unwrap_or_default();
                    actions.push(IntegrityRepairAction {
                        kind: RepairActionKind::InspectMissingEndpoint,
                        description: format!("Inspect dangling relation endpoint for relation '{}'", rel_id),
                        target_id: rel_id,
                        details: json!({ "violation_message": v.message }),
                    });
                }
                IntegrityViolationCode::WorkNotFound | IntegrityViolationCode::NotAWork => {
                    safe = false;
                    actions.push(IntegrityRepairAction {
                        kind: RepairActionKind::InspectMissingEntity,
                        description: format!("Entity '{}' missing or not a work entity", work_id),
                        target_id: work_id.to_string(),
                        details: json!({ "violation_message": v.message }),
                    });
                }
                _ => {
                    safe = false;
                }
            }
        }

        Ok(IntegrityRepairPlan {
            work_id: work_id.to_string(),
            safe,
            actions,
        })
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
        let res: LoomaResult<(Asset, Relation)> = (|| {
            // Pre-validation 1: Work entity must exist
            if self.get_entity(work_id)?.is_none() {
                return Err(LoomaError::NotFound(format!("Work entity not found: {work_id}")));
            }

            // Pre-validation 2: Old asset must exist and be a directory
            let old_asset = self.get_asset(old_asset_id)?
                .ok_or_else(|| LoomaError::NotFound(format!("Old asset not found: {old_asset_id}")))?;
            if old_asset.kind != AssetKind::Directory {
                return Err(LoomaError::Validation(format!(
                    "Asset {old_asset_id} is not a directory asset"
                )));
            }

            // Pre-validation 3: Old asset must be linked to work strictly via ATTACHES relation
            // Direction must be source = work_id, target = old_asset_id
            let existing_source_relations = self.list_relations_for_source(work_id)?;
            let old_rel = existing_source_relations.iter().find(|r| {
                r.target_id == old_asset_id && r.relation_type == relation_types::ATTACHES
            }).ok_or_else(|| LoomaError::Validation(format!(
                "Old asset {old_asset_id} is not linked to work {work_id} via attaches relation"
            )))?;

            // Pre-validation 4: Validate new path exists on disk and is a directory
            let norm_path = normalize_directory_path(new_dir_path);
            if norm_path.is_empty() {
                return Err(LoomaError::Validation("Directory path cannot be empty".to_string()));
            }
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

            // Case E: New path and old path are identical -> Idempotent success!
            if let Some(ref old_path) = old_asset.path {
                if normalize_directory_path(old_path) == norm_path {
                    return Ok((old_asset, old_rel.clone()));
                }
            }

            // Prepare new asset (reuse if path already in DB, or create new)
            let new_asset = if let Some(existing) = self.get_asset_by_path(&norm_path)? {
                if existing.kind != AssetKind::Directory {
                    return Err(LoomaError::Validation(format!(
                        "Target asset {} is not a directory asset", existing.id
                    )));
                }
                existing
            } else {
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
                Asset {
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
                }
            };

            // Prepare new relation
            let new_rel = Relation {
                id: format!("rel_{}", &Uuid::new_v4().to_string()[..8]),
                source_id: work_id.to_string(),
                source_type: "entity".to_string(),
                relation_type: relation_types::ATTACHES.to_string(),
                target_id: new_asset.id.clone(),
                target_type: "asset".to_string(),
                metadata: json!({ "domain": "personal_records" }),
                created_at: Utc::now(),
            };

            // Check if target is already attached to work
            let existing_target_rel = existing_source_relations.iter().find(|r| {
                r.target_id == new_asset.id && r.relation_type == relation_types::ATTACHES
            });

            // Atomically relocate in DB (single transaction: upsert new asset, delete old relation, insert new relation if needed)
            self.repo.relocate_work_directory(work_id, old_asset_id, &new_asset, &new_rel)?;

            let final_rel = if let Some(existing_r) = existing_target_rel {
                existing_r.clone()
            } else {
                new_rel.clone()
            };

            Ok((new_asset, final_rel))
        })();

        match res {
            Ok((new_asset, final_rel)) => {
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

                self.emit_event(
                    actor,
                    DomainEvent::RelationCreated {
                        id: final_rel.id.clone(),
                        source_id: work_id.to_string(),
                        target_id: new_asset.id.clone(),
                    },
                );

                Ok((new_asset, final_rel))
            }
            Err(e) => {
                let _ = self.record_audit(
                    actor,
                    "work.relocate_directory",
                    "entity",
                    work_id,
                    "failure",
                    json!({
                        "old_asset_id": old_asset_id,
                        "new_path": new_dir_path,
                        "error": e.to_string(),
                    }),
                );
                Err(e)
            }
        }
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

    fn check_item_exists(&self, item_id: &str, item_type: &str) -> LoomaResult<bool> {
        match item_type {
            "entity" => Ok(self.repo.get_entity_by_id(item_id)?.is_some()),
            "asset" => Ok(self.repo.get_asset_by_id(item_id)?.is_some()),
            "memory" => Ok(self.repo.get_memory_by_id(item_id)?.is_some()),
            _ => {
                if self.repo.get_entity_by_id(item_id)?.is_some() {
                    return Ok(true);
                }
                if self.repo.get_asset_by_id(item_id)?.is_some() {
                    return Ok(true);
                }
                if self.repo.get_memory_by_id(item_id)?.is_some() {
                    return Ok(true);
                }
                Ok(false)
            }
        }
    }

    pub fn create_relation(&self, actor: &str, relation: &Relation) -> LoomaResult<()> {
        let res: LoomaResult<()> = (|| {
            // P1 Integrity Check 1: Self relation is rejected
            if relation.source_id == relation.target_id {
                return Err(LoomaError::Validation(format!(
                    "Self-relation is not allowed: source_id and target_id are both '{}'",
                    relation.source_id
                )));
            }

            // P1 Integrity Check 2: Relation source must exist
            if !self.check_item_exists(&relation.source_id, &relation.source_type)? {
                return Err(LoomaError::NotFound(format!(
                    "Relation source {} '{}' not found",
                    relation.source_type, relation.source_id
                )));
            }

            // P1 Integrity Check 3: Relation target must exist
            if !self.check_item_exists(&relation.target_id, &relation.target_type)? {
                return Err(LoomaError::NotFound(format!(
                    "Relation target {} '{}' not found",
                    relation.target_type, relation.target_id
                )));
            }

            // P1 Integrity Check 4: Duplicate relation protection (idempotent success)
            let existing = self.repo.list_relations_for_source(&relation.source_id)?;
            if existing.iter().any(|r| r.relation_type == relation.relation_type && r.target_id == relation.target_id) {
                return Ok(());
            }

            self.repo.create_relation(relation)?;
            Ok(())
        })();

        match res {
            Ok(()) => {
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
            Err(e) => {
                let _ = self.record_audit(
                    actor,
                    "relation.create",
                    "relation",
                    &relation.id,
                    "failure",
                    json!({
                        "source_id": relation.source_id,
                        "target_id": relation.target_id,
                        "relation_type": relation.relation_type,
                        "error": e.to_string(),
                    }),
                );
                Err(e)
            }
        }
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
