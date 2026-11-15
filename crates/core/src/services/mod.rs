pub mod cloud;

use crate::error::LoomaResult;
pub use cloud::{CloudStorage, LocalDirStorage, RemoteObjectMeta};
use crate::models::{
    Asset, AssetFilter, AssetLifecycle, AssetScanSuppression, BackupResult, Collection,
    CollectionItem, Entity, EntityFilter, ExperienceEntry, ExperienceFilter, ExperienceLink,
    GameActivityRecord, IntegrityRepairPlan, Memory, Relation, RestoreResult, SourceRoot, SourceRootInspection,
    TimelineFilter, TimelineItem, VaultDoctorReport, VaultInfo, VaultManifest, VaultSearchResults, VaultStats,
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
    fn count_assets_with_filter(&self, filter: &AssetFilter) -> LoomaResult<u64>;
}

pub trait EntityService: Send + Sync {
    fn list_entities(&self, filter: &EntityFilter) -> LoomaResult<Vec<Entity>>;
    fn get_entity_by_id(&self, id: &str) -> LoomaResult<Option<Entity>>;
    fn create_entity(&self, entity: &Entity) -> LoomaResult<()>;
    fn update_entity(&self, entity: &Entity) -> LoomaResult<()>;
    fn update_entity_cas(&self, entity: &Entity, expected_updated_at: &chrono::DateTime<chrono::Utc>) -> LoomaResult<bool>;
    fn delete_entity(&self, id: &str) -> LoomaResult<bool>;
    fn count_entities(&self) -> LoomaResult<u64>;
    fn repair_work_integrity(&self, work_id: &str, plan: &IntegrityRepairPlan) -> LoomaResult<()>;
}

pub trait RelationService: Send + Sync {
    fn list_relations_for_source(&self, source_id: &str) -> LoomaResult<Vec<Relation>>;
    fn list_relations_for_target(&self, target_id: &str) -> LoomaResult<Vec<Relation>>;
    fn list_relations_for_item(&self, item_id: &str) -> LoomaResult<Vec<Relation>>;
    fn create_relation(&self, relation: &Relation) -> LoomaResult<()>;
    fn delete_relation(&self, id: &str) -> LoomaResult<bool>;
    fn delete_relation_between(&self, source_id: &str, target_id: &str) -> LoomaResult<bool>;
    fn relocate_work_directory(&self, work_id: &str, old_asset_id: &str, new_asset: &Asset, new_relation: &Relation) -> LoomaResult<()>;
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
    fn set_primary_external_reference(&self, entity_id: &str, reference_id: &str) -> LoomaResult<()>;
}

pub trait ExperienceService: Send + Sync {
    fn create_experience(
        &self,
        entry: &ExperienceEntry,
        links: &[ExperienceLink],
        request_id: Option<(&str, &str)>,
    ) -> LoomaResult<ExperienceEntry>;
    fn get_experience_by_id(&self, id: &str) -> LoomaResult<Option<ExperienceEntry>>;
    fn list_experiences(&self, filter: &ExperienceFilter) -> LoomaResult<Vec<ExperienceEntry>>;
    fn update_experience(&self, entry: &ExperienceEntry) -> LoomaResult<()>;
    fn archive_experience(&self, id: &str) -> LoomaResult<bool>;
    fn list_experience_links(&self, experience_id: &str) -> LoomaResult<Vec<ExperienceLink>>;
    fn add_experience_link(&self, link: &ExperienceLink) -> LoomaResult<()>;
    fn remove_experience_link(
        &self,
        experience_id: &str,
        linked_type: &str,
        linked_id: &str,
    ) -> LoomaResult<bool>;
    fn count_experiences(&self) -> LoomaResult<u64>;

    /// Atomic composite transaction:
    /// Updates work status, creates experience entry, records idempotency request_id,
    /// establishes links, and records audit log in a SINGLE SQLite transaction.
    fn complete_work_and_record_experience(
        &self,
        work_id: &str,
        experience: &ExperienceEntry,
        links: &[ExperienceLink],
        request_id: Option<(&str, &str)>,
        audit_log: &crate::models::AuditLog,
    ) -> LoomaResult<(Entity, ExperienceEntry)>;
}

pub trait SourceRootService: Send + Sync {
    fn create_source_root(&self, root: &SourceRoot) -> LoomaResult<SourceRoot>;
    fn get_source_root(&self, id: &str) -> LoomaResult<Option<SourceRoot>>;
    fn get_source_root_by_path(&self, path: &str) -> LoomaResult<Option<SourceRoot>>;
    fn list_source_roots(&self) -> LoomaResult<Vec<SourceRoot>>;
    fn update_source_root(&self, root: &SourceRoot) -> LoomaResult<SourceRoot>;
    fn delete_source_root(&self, id: &str) -> LoomaResult<()>;
    fn rebind_source_root(&self, id: &str, new_path: &str) -> LoomaResult<SourceRoot>;
    fn inspect_source_root(&self, id: &str) -> LoomaResult<SourceRootInspection>;

    fn get_asset_lifecycle(&self, asset_id: &str) -> LoomaResult<Option<AssetLifecycle>>;
    fn set_asset_lifecycle(&self, lifecycle: &AssetLifecycle) -> LoomaResult<()>;
    fn archive_asset(&self, asset_id: &str) -> LoomaResult<()>;
    fn unarchive_asset(&self, asset_id: &str) -> LoomaResult<()>;
    fn untrack_asset(&self, asset_id: &str, reason: Option<&str>) -> LoomaResult<()>;
    fn resume_tracking(&self, asset_id: &str) -> LoomaResult<()>;

    fn is_asset_suppressed(&self, root_id: &str, relative_path: &str) -> LoomaResult<bool>;
    fn list_suppressions(&self, root_id: Option<&str>) -> LoomaResult<Vec<AssetScanSuppression>>;
    fn add_suppression(&self, root_id: &str, relative_path: &str, reason: Option<&str>) -> LoomaResult<()>;
    fn remove_suppression(&self, root_id: &str, relative_path: &str) -> LoomaResult<()>;
}

pub trait SearchService: Send + Sync {
    fn search_vault(&self, query: &str, category: Option<&str>, limit: usize) -> LoomaResult<VaultSearchResults>;
}

pub trait GameActivityService: Send + Sync {
    fn list_game_activities_for_work(&self, work_id: &str) -> LoomaResult<Vec<GameActivityRecord>>;
    fn get_game_activity(&self, provider: &str, source_key: &str, external_id: &str) -> LoomaResult<Option<GameActivityRecord>>;
    fn upsert_game_activity(&self, record: &GameActivityRecord) -> LoomaResult<()>;
    fn delete_game_activity(&self, id: &str) -> LoomaResult<bool>;
    fn list_all_game_activities(&self) -> LoomaResult<Vec<GameActivityRecord>>;
}

pub type ScanProgressCallback = Box<dyn FnMut(crate::models::ScanProgress) + Send>;

pub trait ScanEngine: Send + Sync {
    fn scan_directory(
        &self,
        service: &dyn AssetService,
        root: &std::path::Path,
        options: &crate::models::ScanOptions,
        progress_cb: Option<ScanProgressCallback>,
    ) -> LoomaResult<crate::models::ScanSummary>;
}


