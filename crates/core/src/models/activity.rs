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

// ==========================================
// NX Activity Log Raw Models
// ==========================================

fn deserialize_version_opt<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let val: Option<serde_json::Value> = Option::deserialize(deserializer)?;
    match val {
        Some(serde_json::Value::String(s)) => Ok(Some(s)),
        Some(serde_json::Value::Number(n)) => Ok(Some(n.to_string())),
        _ => Ok(None),
    }
}

fn deserialize_u64_lenient<'de, D>(deserializer: D) -> Result<Option<u64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let val: Option<serde_json::Value> = Option::deserialize(deserializer)?;
    match val {
        Some(serde_json::Value::Number(n)) => {
            if let Some(u) = n.as_u64() {
                Ok(Some(u))
            } else if let Some(f) = n.as_f64() {
                Ok(Some(f.max(0.0) as u64))
            } else {
                Ok(None)
            }
        }
        Some(serde_json::Value::String(s)) => Ok(s.trim().parse::<u64>().ok()),
        _ => Ok(None),
    }
}

fn deserialize_i64_lenient<'de, D>(deserializer: D) -> Result<Option<i64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let val: Option<serde_json::Value> = Option::deserialize(deserializer)?;
    match val {
        Some(serde_json::Value::Number(n)) => {
            if let Some(i) = n.as_i64() {
                Ok(Some(i))
            } else if let Some(f) = n.as_f64() {
                Ok(Some(f as i64))
            } else {
                Ok(None)
            }
        }
        Some(serde_json::Value::String(s)) => Ok(s.trim().parse::<i64>().ok()),
        _ => Ok(None),
    }
}

fn deserialize_u32_lenient<'de, D>(deserializer: D) -> Result<Option<u32>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let val: Option<serde_json::Value> = Option::deserialize(deserializer)?;
    match val {
        Some(serde_json::Value::Number(n)) => {
            if let Some(u) = n.as_u64() {
                Ok(Some(u as u32))
            } else {
                Ok(None)
            }
        }
        Some(serde_json::Value::String(s)) => Ok(s.trim().parse::<u32>().ok()),
        _ => Ok(None),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NxActivityLogExport {
    #[serde(rename = "exportTimestamp", default, deserialize_with = "deserialize_i64_lenient")]
    pub export_timestamp: Option<i64>,
    #[serde(rename = "exportVersion", default, deserialize_with = "deserialize_version_opt")]
    pub export_version: Option<String>,
    #[serde(default, alias = "user")]
    pub users: Vec<NxUser>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NxUser {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default, alias = "title")]
    pub titles: Vec<NxTitle>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NxTitle {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub summary: Option<NxSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NxSummary {
    #[serde(rename = "firstPlayed", default, deserialize_with = "deserialize_i64_lenient")]
    pub first_played: Option<i64>,
    #[serde(rename = "lastPlayed", default, deserialize_with = "deserialize_i64_lenient")]
    pub last_played: Option<i64>,
    #[serde(default, deserialize_with = "deserialize_u32_lenient")]
    pub launches: Option<u32>,
    #[serde(default, deserialize_with = "deserialize_u64_lenient")]
    pub playtime: Option<u64>,
}

// ==========================================
// Switch Preview & Candidate Models
// ==========================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwitchProfilePreview {
    pub user_id: String,
    pub user_name: String,
    pub title_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SwitchGameMatchStatus {
    ExactMapped,    // Already bound to a Work in DB
    SuggestedMatch, // Found existing Work matching name/alias
    NewGame,        // No existing Work found, suggest creating new Work
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwitchGameCandidatePreview {
    pub external_id: String,
    pub title_name: String,
    pub playtime_seconds: u64,
    pub playtime_hours: f64,
    pub launches: Option<u32>,
    pub first_played_at: Option<DateTime<Utc>>,
    pub last_played_at: Option<DateTime<Utc>>,

    pub match_status: SwitchGameMatchStatus,
    pub matched_work_id: Option<String>,
    pub matched_work_title: Option<String>,

    pub existing_playtime_seconds: Option<u64>,
    pub existing_peak_seconds: Option<u64>,
    pub is_time_decreased: bool,
    pub is_duplicate_snapshot: bool,
    pub warning: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwitchImportPreview {
    pub file_sha256: String,
    pub export_timestamp: Option<i64>,
    pub export_version: Option<String>,
    pub profiles: Vec<SwitchProfilePreview>,
    pub selected_user_id: String,
    pub selected_user_name: String,
    pub games: Vec<SwitchGameCandidatePreview>,
    pub total_games: usize,
    pub new_games_count: usize,
    pub matched_games_count: usize,
    pub warning_count: usize,
}

// ==========================================
// Switch Import Decision & Outcome Models
// ==========================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwitchImportDecision {
    pub file_sha256: String,
    pub selected_user_id: String,
    pub user_label: Option<String>,
    pub source_exported_at: Option<i64>,
    pub items: Vec<SwitchImportItemDecision>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwitchImportItemDecision {
    pub external_id: String,
    pub title_hint: String,
    pub playtime_seconds: u64,
    pub launch_count: Option<u32>,
    pub first_played_at: Option<DateTime<Utc>>,
    pub last_played_at: Option<DateTime<Utc>>,
    pub action: SwitchImportAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SwitchImportAction {
    LinkExistingWork { work_id: String },
    CreateNewWork { title: String },
    Skip,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwitchImportOutcome {
    pub imported_records: usize,
    pub created_works: usize,
    pub updated_records: usize,
    pub skipped_records: usize,
}
