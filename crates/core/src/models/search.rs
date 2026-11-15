use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultSearchResultItem {
    pub id: String,
    pub category: String, // "asset" | "entity" | "collection" | "memory"
    pub title: String,
    pub subtitle: String,
    pub badge: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VaultSearchResults {
    pub items: Vec<VaultSearchResultItem>,
    pub total_matches: usize,
}
