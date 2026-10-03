use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use super::{Asset, Collection, Entity, ExternalReference, Memory, Relation};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkType {
    Anime,
    Manga,
    Game,
    Movie,
    TvSeries,
    Music,
    Album,
    Book,
    Novel,
    Documentary,
    Other,
}

impl WorkType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Anime => "anime",
            Self::Manga => "manga",
            Self::Game => "game",
            Self::Movie => "movie",
            Self::TvSeries => "tv_series",
            Self::Music => "music",
            Self::Album => "album",
            Self::Book => "book",
            Self::Novel => "novel",
            Self::Documentary => "documentary",
            Self::Other => "other",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "anime" => Self::Anime,
            "manga" => Self::Manga,
            "game" => Self::Game,
            "movie" => Self::Movie,
            "tv_series" | "tv" | "series" => Self::TvSeries,
            "music" | "song" => Self::Music,
            "album" => Self::Album,
            "book" => Self::Book,
            "novel" => Self::Novel,
            "documentary" => Self::Documentary,
            _ => Self::Other,
        }
    }
}

impl Default for WorkType {
    fn default() -> Self {
        Self::Other
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecordStatus {
    Planned,
    InProgress,
    Completed,
    Paused,
    Dropped,
    Revisit,
    Unknown,
}

impl RecordStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Planned => "planned",
            Self::InProgress => "in_progress",
            Self::Completed => "completed",
            Self::Paused => "paused",
            Self::Dropped => "dropped",
            Self::Revisit => "revisit",
            Self::Unknown => "unknown",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "planned" | "plan" | "wishlist" => Self::Planned,
            "in_progress" | "watching" | "playing" | "reading" | "ongoing" => Self::InProgress,
            "completed" | "done" | "finished" => Self::Completed,
            "paused" | "on_hold" => Self::Paused,
            "dropped" | "abandoned" => Self::Dropped,
            "revisit" | "rewatch" | "replay" => Self::Revisit,
            _ => Self::Unknown,
        }
    }
}

impl Default for RecordStatus {
    fn default() -> Self {
        Self::Unknown
    }
}

/// Standardized relation vocabularies across Personal Records
pub mod relation_types {
    pub const CREATED_BY: &str = "created_by";
    pub const CREATED: &str = "created";
    pub const PERFORMED_BY: &str = "performed_by";
    pub const DEVELOPED_BY: &str = "developed_by";
    pub const PUBLISHED_BY: &str = "published_by";
    pub const ADAPTATION_OF: &str = "adaptation_of";
    pub const SEQUEL_OF: &str = "sequel_of";
    pub const PREQUEL_OF: &str = "prequel_of";
    pub const PART_OF: &str = "part_of";
    pub const RELATED_TO: &str = "related_to";
    pub const INSPIRED_BY: &str = "inspired_by";
    pub const FEATURES: &str = "features";
    pub const REFERENCED_BY: &str = "referenced_by";
    pub const ATTACHES: &str = "attaches";
}

/// Strongly-typed view of a Work entity (stored transparently in Entity.properties)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkMetadata {
    pub work_type: WorkType,
    pub status: RecordStatus,
    pub original_title: Option<String>,
    pub release_year: Option<u32>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub cover_asset_id: Option<String>,
    pub rating: Option<f32>,
}

impl Entity {
    /// Create a new Entity representing an entertainment or cultural Work (Anime, Game, Movie, Book, etc.)
    pub fn new_work(
        title: &str,
        work_type: WorkType,
        status: RecordStatus,
        original_title: Option<String>,
        description: Option<String>,
    ) -> Self {
        let meta = WorkMetadata {
            work_type,
            status,
            original_title,
            release_year: None,
            start_date: None,
            end_date: None,
            cover_asset_id: None,
            rating: None,
        };

        Self {
            id: format!("work_{}", &Uuid::new_v4().to_string()[..8]),
            entity_type: work_type.as_str().to_string(),
            title: title.to_string(),
            description,
            properties: json!({
                "work": meta,
                "domain": "work",
                "status": status.as_str(),
                "work_type": work_type.as_str(),
            }),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    /// Create a new Entity representing a Person or Character
    pub fn new_person(name: &str, subtype: Option<&str>, description: Option<String>) -> Self {
        Self {
            id: format!("per_{}", &Uuid::new_v4().to_string()[..8]),
            entity_type: "person".to_string(),
            title: name.to_string(),
            description,
            properties: json!({
                "domain": "person",
                "subtype": subtype.unwrap_or("person"),
            }),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    /// Create a new Entity representing a Project
    pub fn new_project(title: &str, description: Option<String>) -> Self {
        Self {
            id: format!("proj_{}", &Uuid::new_v4().to_string()[..8]),
            entity_type: "project".to_string(),
            title: title.to_string(),
            description,
            properties: json!({
                "domain": "project",
                "status": "active",
            }),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    /// Extract Work metadata if this entity represents a work
    pub fn as_work_metadata(&self) -> Option<WorkMetadata> {
        if let Some(work_val) = self.properties.get("work") {
            if let Ok(meta) = serde_json::from_value::<WorkMetadata>(work_val.clone()) {
                return Some(meta);
            }
        }

        // Fallback: deduce from entity_type if it matches known work types
        let parsed_type = WorkType::parse(&self.entity_type);
        if parsed_type != WorkType::Other || self.entity_type == "other" || self.entity_type == "work" {
            let status = self.properties.get("status")
                .and_then(|v| v.as_str())
                .map(RecordStatus::parse)
                .unwrap_or_default();
            return Some(WorkMetadata {
                work_type: parsed_type,
                status,
                original_title: None,
                release_year: None,
                start_date: None,
                end_date: None,
                cover_asset_id: None,
                rating: None,
            });
        }

        None
    }
}

/// Fully aggregated unified digital footprint for a Work / Personal Record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkSummary {
    pub entity: Entity,
    pub work_metadata: Option<WorkMetadata>,
    pub relations: Vec<Relation>,
    pub linked_assets: Vec<Asset>,
    pub linked_memories: Vec<Memory>,
    pub linked_collections: Vec<Collection>,
    pub external_references: Vec<ExternalReference>,
}
