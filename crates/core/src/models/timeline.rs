use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineItem {
    pub id: String,
    pub item_type: String, // "asset" | "entity" | "memory"
    pub timestamp: DateTime<Utc>,
    pub title: String,
    pub description: Option<String>,
    pub badge: Option<String>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TimelineFilter {
    pub item_types: Option<Vec<String>>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}
