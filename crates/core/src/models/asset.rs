use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AssetKind {
    Image,
    Video,
    Audio,
    Document,
    Archive,
    Model3d,
    Code,
    Other,
}

impl Default for AssetKind {
    fn default() -> Self {
        Self::Other
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AssetSource {
    Local,
    Managed,
    External,
}

impl Default for AssetSource {
    fn default() -> Self {
        Self::Local
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AssetStatus {
    Active,
    Missing,
    Archived,
}

impl Default for AssetStatus {
    fn default() -> Self {
        Self::Active
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Asset {
    pub id: String,
    pub kind: AssetKind,
    pub source: AssetSource,
    pub path: Option<String>,
    pub size: Option<u64>,
    pub hash: Option<String>,
    pub mime_type: Option<String>,
    pub metadata: serde_json::Value,
    pub status: AssetStatus,
    pub created_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
    pub indexed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AssetFilter {
    pub kind: Option<AssetKind>,
    pub status: Option<AssetStatus>,
    pub search_query: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

