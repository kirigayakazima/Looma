use std::path::PathBuf;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultInfo {
    pub name: String,
    pub version: String,
    pub vault_path: PathBuf,
    pub database_path: PathBuf,
    pub is_initialized: bool,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VaultStats {
    pub total_assets: u64,
    pub total_entities: u64,
    pub total_relations: u64,
    pub total_memories: u64,
    pub total_collections: u64,
    pub database_size_bytes: u64,
}
