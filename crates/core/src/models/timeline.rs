use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineItem {
    pub id: String,
    pub item_type: String, // "asset" | "entity" | "memory" | "experience"
    pub timestamp: DateTime<Utc>,
    pub title: String,
    pub description: Option<String>,
    pub badge: Option<String>,
    pub metadata: serde_json::Value,
    #[serde(default)]
    pub track: Option<String>, // "personal" | "system"
    #[serde(default)]
    pub occurred_precision: Option<String>, // "unknown" | "year" | "month" | "day" | "instant"
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TimelineFilter {
    pub item_types: Option<Vec<String>>,
    pub track: Option<String>, // "personal" | "system" | "all"
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}
