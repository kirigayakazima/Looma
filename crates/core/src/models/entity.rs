use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub id: String,
    pub entity_type: String,
    pub title: String,
    pub description: Option<String>,
    pub properties: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EntityFilter {
    pub entity_type: Option<String>,
    pub search_query: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}
