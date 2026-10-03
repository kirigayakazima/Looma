use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relation {
    pub id: String,
    pub source_id: String,
    pub source_type: String,
    pub relation_type: String,
    pub target_id: String,
    pub target_type: String,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
}
