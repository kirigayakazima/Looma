use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLog {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub actor: String,
    pub operation: String,
    pub target_type: String,
    pub target_id: String,
    pub result: String,
    pub details: serde_json::Value,
}
