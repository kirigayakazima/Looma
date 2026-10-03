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
