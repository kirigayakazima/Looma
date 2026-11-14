use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub mod root_states {
    pub const ONLINE: &str = "online";
    pub const OFFLINE: &str = "offline";
    pub const UNVERIFIED: &str = "unverified";
}

pub mod asset_availability {
    pub const ONLINE: &str = "online";
    pub const OFFLINE: &str = "offline";
    pub const MISSING: &str = "missing";
    pub const UNCHECKED: &str = "unchecked";
}

pub mod asset_visibility {
    pub const ACTIVE: &str = "active";
    pub const ARCHIVED: &str = "archived";
}

pub mod asset_tracking {
    pub const TRACKED: &str = "tracked";
    pub const UNTRACKED: &str = "untracked";
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceRoot {
    pub id: String,
    pub label: String,
    pub path: String,
    pub source_kind: String,
    pub enabled: bool,
    pub state: String,
    pub last_successful_scan_at: Option<DateTime<Utc>>,
    pub last_seen_at: Option<DateTime<Utc>>,
    #[serde(default = "default_settings")]
    pub settings_json: Value,
}

fn default_settings() -> Value {
    serde_json::json!({})
}

impl SourceRoot {
    pub fn new(id: impl Into<String>, label: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            path: path.into(),
            source_kind: "local".to_string(),
            enabled: true,
            state: root_states::UNVERIFIED.to_string(),
            last_successful_scan_at: None,
            last_seen_at: None,
            settings_json: default_settings(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetLifecycle {
    pub asset_id: String,
    pub root_id: Option<String>,
    pub relative_path: Option<String>,
    pub availability: String,
    pub visibility: String,
    pub tracking: String,
    pub last_verified_at: Option<DateTime<Utc>>,
    pub untracked_at: Option<DateTime<Utc>>,
    pub untracked_reason: Option<String>,
}

impl AssetLifecycle {
    pub fn new(asset_id: impl Into<String>) -> Self {
        Self {
            asset_id: asset_id.into(),
            root_id: None,
            relative_path: None,
            availability: asset_availability::UNCHECKED.to_string(),
            visibility: asset_visibility::ACTIVE.to_string(),
            tracking: asset_tracking::TRACKED.to_string(),
            last_verified_at: None,
            untracked_at: None,
            untracked_reason: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetScanSuppression {
    pub id: String,
    pub root_id: String,
    pub relative_path: String,
    pub reason: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceRootInspection {
    pub root_id: String,
    pub label: String,
    pub path: String,
    pub state: String,
    pub exists: bool,
    pub is_dir: bool,
    pub accessible: bool,
    pub tracked_assets: usize,
    pub online_assets: usize,
    pub offline_assets: usize,
    pub missing_assets: usize,
    pub unchecked_assets: usize,
    pub suppressed_count: usize,
}
