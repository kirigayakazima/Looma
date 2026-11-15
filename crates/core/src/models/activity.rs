use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameActivityRecord {
    pub id: String,
    pub work_id: String,
    pub provider: String,
    pub source_key: String,
    pub external_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title_hint: Option<String>,
    pub playtime_seconds: u64,
    pub peak_seen_seconds: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub launch_count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_played_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_played_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_exported_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_import_sha256: Option<String>,
    pub updated_at: DateTime<Utc>,
}

impl GameActivityRecord {
    /// Return the playtime formatted as floating point hours (e.g. 85.5)
    pub fn playtime_hours(&self) -> f64 {
        (self.playtime_seconds as f64) / 3600.0
    }
}
