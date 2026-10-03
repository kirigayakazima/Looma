use crate::error::LoomaResult;
use crate::models::{
    Asset, Collection, Entity, EntityFilter, Memory, Relation,
    VaultInfo, VaultStats,
};

pub trait VaultService: Send + Sync {
    fn get_vault_info(&self) -> LoomaResult<VaultInfo>;
    fn get_vault_stats(&self) -> LoomaResult<VaultStats>;
}

pub trait AssetService: Send + Sync {
    fn list_assets(&self, limit: usize, offset: usize) -> LoomaResult<Vec<Asset>>;
    fn get_asset_by_id(&self, id: &str) -> LoomaResult<Option<Asset>>;
    fn get_asset_by_path(&self, path: &str) -> LoomaResult<Option<Asset>>;
    fn upsert_asset(&self, asset: &Asset) -> LoomaResult<()>;
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
    fn create_relation(&self, relation: &Relation) -> LoomaResult<()>;
    fn delete_relation(&self, id: &str) -> LoomaResult<bool>;
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
    fn add_item_to_collection(&self, collection_id: &str, item_id: &str, item_type: &str) -> LoomaResult<()>;
    fn remove_item_from_collection(&self, collection_id: &str, item_id: &str) -> LoomaResult<bool>;
}
