use std::collections::HashMap;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use super::{
    Asset, AssetLifecycle, AssetScanSuppression, Collection, Entity, ExperienceEntry,
    ExperienceLink, ExternalReference, Memory, Relation, SourceRoot, VaultInfo, VaultStats,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultManifest {
    pub manifest_version: String,
    pub schema_version: u32,
    pub created_at: DateTime<Utc>,
    pub vault_info: VaultInfo,
    pub stats: VaultStats,
    pub assets_count: usize,
    pub entities_count: usize,
    pub memories_count: usize,
    pub collections_count: usize,
    pub relations_count: usize,
    pub external_references_count: usize,
    #[serde(default)]
    pub experiences_count: usize,
    #[serde(default)]
    pub source_roots_count: usize,
    #[serde(default)]
    pub asset_suppressions_count: usize,
    pub checksums: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultExportDump {
    pub manifest: VaultManifest,
    pub assets: Vec<Asset>,
    pub entities: Vec<Entity>,
    pub memories: Vec<Memory>,
    pub collections: Vec<Collection>,
    pub relations: Vec<Relation>,
    pub external_references: Vec<ExternalReference>,
    #[serde(default)]
    pub experiences: Vec<ExperienceEntry>,
    #[serde(default)]
    pub experience_links: Vec<ExperienceLink>,
    #[serde(default)]
    pub source_roots: Vec<SourceRoot>,
    #[serde(default)]
    pub asset_lifecycles: Vec<AssetLifecycle>,
    #[serde(default)]
    pub asset_suppressions: Vec<AssetScanSuppression>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupResult {
    pub backup_path: String,
    pub manifest: VaultManifest,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestoreResult {
    pub restored_assets: usize,
    pub restored_entities: usize,
    pub restored_memories: usize,
    pub restored_collections: usize,
    pub restored_relations: usize,
    pub restored_external_references: usize,
    #[serde(default)]
    pub restored_experiences: usize,
    #[serde(default)]
    pub restored_source_roots: usize,
    #[serde(default)]
    pub restored_asset_suppressions: usize,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultDoctorReport {
    pub total_assets: usize,
    pub active_assets: usize,
    pub missing_assets: Vec<String>,
    pub database_size_bytes: u64,
    pub integrity_ok: bool,
    pub integrity_message: String,
}
