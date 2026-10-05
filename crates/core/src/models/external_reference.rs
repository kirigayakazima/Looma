use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalReference {
    pub id: String,
    pub entity_id: Option<String>,
    pub provider: String,
    pub title: String,
    pub url: String,
    pub description: Option<String>,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ExternalReference {
    pub fn is_primary(&self) -> bool {
        self.metadata.get("is_primary").and_then(|v| v.as_bool()).unwrap_or(false)
    }

    pub fn with_primary(mut self, primary: bool) -> Self {
        self.set_primary(primary);
        self
    }

    pub fn set_primary(&mut self, is_primary: bool) {
        if let Some(obj) = self.metadata.as_object_mut() {
            obj.insert("is_primary".to_string(), serde_json::json!(is_primary));
        } else {
            self.metadata = serde_json::json!({ "is_primary": is_primary });
        }
        self.updated_at = Utc::now();
    }
}
