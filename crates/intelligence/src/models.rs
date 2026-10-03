use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SuggestionType {
    Relation,
    Tag,
    Cluster,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Suggestion {
    pub id: String,
    pub suggestion_type: SuggestionType,
    pub title: String,
    pub description: String,
    pub confidence: f32,
    pub source_id: String,
    pub source_name: String,
    pub target_id: Option<String>,
    pub target_name: Option<String>,
    pub relation_type: Option<String>,
    pub tags: Vec<String>,
    pub action_payload: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SmartInsightsReport {
    pub total_suggestions: usize,
    pub relation_suggestions: Vec<Suggestion>,
    pub tag_suggestions: Vec<Suggestion>,
    pub cluster_suggestions: Vec<Suggestion>,
}
