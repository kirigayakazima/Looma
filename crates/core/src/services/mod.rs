pub mod cloud;

use crate::error::LoomaResult;
pub use cloud::{CloudStorage, LocalDirStorage, RemoteObjectMeta};
use crate::models::{
    Asset, AssetFilter, BackupResult, Collection, CollectionItem, Entity, EntityFilter, Memory,
    Relation, RestoreResult, TimelineFilter, TimelineItem, VaultDoctorReport, VaultInfo,
    VaultManifest, VaultStats,
};

pub trait VaultService: Send + Sync {
    fn get_vault_info(&self) -> LoomaResult<VaultInfo>;
    fn get_vault_stats(&self) -> LoomaResult<VaultStats>;
}

pub trait AssetService: Send + Sync {
    fn list_assets(&self, limit: usize, offset: usize) -> LoomaResult<Vec<Asset>>;
    fn query_assets(&self, filter: &AssetFilter) -> LoomaResult<Vec<Asset>>;
    fn get_asset_by_id(&self, id: &str) -> LoomaResult<Option<Asset>>;
    fn get_asset_by_path(&self, path: &str) -> LoomaResult<Option<Asset>>;
    fn list_assets_by_prefix(&self, prefix: &str) -> LoomaResult<Vec<Asset>>;
    fn upsert_asset(&self, asset: &Asset) -> LoomaResult<()>;
    fn delete_asset(&self, id: &str) -> LoomaResult<bool>;
    fn count_assets(&self) -> LoomaResult<u64>;
}

pub trait EntityService: Send + Sync {
    fn list_entities(&self, filter: &EntityFilter) -> LoomaResult<Vec<Entity>>;
    fn get_entity_by_id(&self, id: &str) -> LoomaResult<Option<Entity>>;
    fn create_entity(&self, entity: &Entity) -> LoomaResult<()>;
    fn update_entity(&self, entity: &Entity) -> LoomaResult<()>;
    fn delete_entity(&self, id: &str) -> LoomaResult<bool>;
    fn count_entities(&self) -> LoomaResult<u64>;
}

pub trait RelationService: Send + Sync {
    fn list_relations_for_source(&self, source_id: &str) -> LoomaResult<Vec<Relation>>;
    fn list_relations_for_target(&self, target_id: &str) -> LoomaResult<Vec<Relation>>;
    fn list_relations_for_item(&self, item_id: &str) -> LoomaResult<Vec<Relation>>;
    fn create_relation(&self, relation: &Relation) -> LoomaResult<()>;
    fn delete_relation(&self, id: &str) -> LoomaResult<bool>;
    fn delete_relation_between(&self, source_id: &str, target_id: &str) -> LoomaResult<bool>;
}

pub trait MemoryService: Send + Sync {
    fn list_memories(&self, limit: usize, offset: usize) -> LoomaResult<Vec<Memory>>;
    fn get_memory_by_id(&self, id: &str) -> LoomaResult<Option<Memory>>;
    fn create_memory(&self, memory: &Memory) -> LoomaResult<()>;
    fn update_memory(&self, memory: &Memory) -> LoomaResult<()>;
    fn delete_memory(&self, id: &str) -> LoomaResult<bool>;
}

pub trait CollectionService: Send + Sync {
    fn list_collections(&self) -> LoomaResult<Vec<Collection>>;
    fn get_collection_by_id(&self, id: &str) -> LoomaResult<Option<Collection>>;
    fn create_collection(&self, collection: &Collection) -> LoomaResult<()>;
    fn delete_collection(&self, id: &str) -> LoomaResult<bool>;
    fn list_items_for_collection(&self, collection_id: &str) -> LoomaResult<Vec<CollectionItem>>;
    fn add_item_to_collection(&self, collection_id: &str, item_id: &str, item_type: &str) -> LoomaResult<()>;
    fn remove_item_from_collection(&self, collection_id: &str, item_id: &str) -> LoomaResult<bool>;
}

pub trait TimelineService: Send + Sync {
    fn query_timeline(&self, filter: &TimelineFilter) -> LoomaResult<Vec<TimelineItem>>;
}

pub trait BackupService: Send + Sync {
    fn generate_manifest(&self) -> LoomaResult<VaultManifest>;
    fn export_backup(&self, destination_dir: &std::path::Path) -> LoomaResult<BackupResult>;
    fn restore_backup(&self, backup_dir_or_file: &std::path::Path) -> LoomaResult<RestoreResult>;
    fn doctor_inspect(&self) -> LoomaResult<VaultDoctorReport>;
    fn doctor_cleanup_missing(&self) -> LoomaResult<usize>;
}

pub trait AuditService: Send + Sync {
    fn record_audit(&self, log: &crate::models::AuditLog) -> LoomaResult<()>;
    fn list_recent_audits(&self, limit: usize) -> LoomaResult<Vec<crate::models::AuditLog>>;
}

pub trait ExternalReferenceService: Send + Sync {
    fn list_external_references(&self, entity_id: Option<&str>) -> LoomaResult<Vec<crate::models::ExternalReference>>;
    fn get_external_reference_by_id(&self, id: &str) -> LoomaResult<Option<crate::models::ExternalReference>>;
    fn create_external_reference(&self, reference: &crate::models::ExternalReference) -> LoomaResult<()>;
    fn update_external_reference(&self, reference: &crate::models::ExternalReference) -> LoomaResult<()>;
    fn delete_external_reference(&self, id: &str) -> LoomaResult<bool>;
}

