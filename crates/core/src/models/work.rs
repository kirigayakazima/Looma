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
    #[serde(other)]
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
    Archived,
    #[serde(other)]
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
            Self::Archived => "archived",
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
            "archived" | "archive" => Self::Archived,
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
    pub const SPIN_OFF_OF: &str = "spin_off_of";
    pub const REMAKE_OF: &str = "remake_of";
    pub const BASED_ON: &str = "based_on";
    pub const RELATED_TO: &str = "related_to";
    pub const INSPIRED_BY: &str = "inspired_by";
    pub const FEATURES: &str = "features";
    pub const REFERENCED_BY: &str = "referenced_by";
    pub const ATTACHES: &str = "attaches";
}

/// Type of position for tracking consumption progress
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProgressPositionType {
    Episode,
    Chapter,
    Page,
    Minute,
    Percentage,
    Custom(String),
}

impl ProgressPositionType {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Episode => "episode",
            Self::Chapter => "chapter",
            Self::Page => "page",
            Self::Minute => "minute",
            Self::Percentage => "percentage",
            Self::Custom(s) => s.as_str(),
        }
    }

    pub fn parse(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "episode" | "ep" => Self::Episode,
            "chapter" | "ch" => Self::Chapter,
            "page" | "p" => Self::Page,
            "minute" | "min" => Self::Minute,
            "percentage" | "percent" | "%" => Self::Percentage,
            other => Self::Custom(other.to_string()),
        }
    }
}

impl Default for ProgressPositionType {
    fn default() -> Self {
        Self::Episode
    }
}

/// Consumption progress of a personal work (episodes, chapters, pages, etc.)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkProgress {
    pub position: f64,
    pub position_type: ProgressPositionType,
    pub total_positions: Option<f64>,
    pub unit: Option<String>,
    pub updated_at: chrono::DateTime<Utc>,
}

/// Type-specific metadata overlays for personal cultural works.
/// Zero-breaking schema: stored safely in `properties_json.work.type_metadata` without adding dedicated tables.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TypeSpecificMetadata {
    Game(GameSpecificMeta),
    Anime(AnimeSpecificMeta),
    Manga(MangaSpecificMeta),
    Book(BookSpecificMeta),
    Movie(MovieSpecificMeta),
    TvSeries(TvSeriesSpecificMeta),
    Music(MusicSpecificMeta),
    Custom(serde_json::Value),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct GameSpecificMeta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub developer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub publisher: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub engine: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct AnimeSpecificMeta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub season: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_episodes: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anime_format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub studio: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub broadcast_day: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct MangaSpecificMeta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_volumes: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_chapters: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub magazine: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct BookSpecificMeta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub isbn: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub translator: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub publisher: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_pages: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct MovieSpecificMeta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runtime_minutes: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub director: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TvSeriesSpecificMeta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_seasons: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_episodes: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct MusicSpecificMeta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artist: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub album: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub track_number: Option<u32>,
}

/// Strongly-typed view of a Work entity (stored transparently in Entity.properties)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkMetadata {
    pub work_type: WorkType,
    pub status: RecordStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub release_year: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover_asset_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rating: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub progress: Option<WorkProgress>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub type_metadata: Option<TypeSpecificMetadata>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub aliases: Vec<String>,
}

impl Default for WorkMetadata {
    fn default() -> Self {
        Self {
            work_type: WorkType::Other,
            status: RecordStatus::Planned,
            original_title: None,
            release_year: None,
            start_date: None,
            end_date: None,
            cover_asset_id: None,
            rating: None,
            progress: None,
            type_metadata: None,
            aliases: Vec::new(),
        }
    }
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
            progress: None,
            type_metadata: None,
            aliases: Vec::new(),
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

    /// Attach initial progress to this work
    pub fn with_progress(mut self, progress: WorkProgress) -> Self {
        if let Some(obj) = self.properties.as_object_mut() {
            if let Some(work_val) = obj.get_mut("work") {
                if let Some(work_obj) = work_val.as_object_mut() {
                    work_obj.insert("progress".to_string(), json!(progress));
                }
            }
            obj.insert("progress".to_string(), json!(progress));
        }
        self
    }

    /// Attach a cover asset ID to this work
    pub fn with_cover_asset(mut self, asset_id: &str) -> Self {
        self.set_cover_asset_id(Some(asset_id));
        self
    }

    /// Set or clear cover asset ID on work metadata
    pub fn set_cover_asset_id(&mut self, asset_id: Option<&str>) {
        if let Some(obj) = self.properties.as_object_mut() {
            if let Some(work_val) = obj.get_mut("work") {
                if let Some(work_obj) = work_val.as_object_mut() {
                    match asset_id {
                        Some(id) => {
                            work_obj.insert("cover_asset_id".to_string(), json!(id));
                        }
                        None => {
                            work_obj.remove("cover_asset_id");
                        }
                    }
                }
            }
            match asset_id {
                Some(id) => {
                    obj.insert("cover_asset_id".to_string(), json!(id));
                }
                None => {
                    obj.remove("cover_asset_id");
                }
            }
            self.updated_at = Utc::now();
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
                progress: None,
                type_metadata: None,
                aliases: Vec::new(),
            });
        }

        None
    }

    /// Attach aliases to this work
    pub fn with_aliases(mut self, aliases: Vec<String>) -> Self {
        self.set_aliases(aliases);
        self
    }

    /// Set or update aliases on work metadata and properties (trimmed, deduplicated case-insensitively)
    pub fn set_aliases(&mut self, aliases: Vec<String>) {
        let mut seen = std::collections::HashSet::new();
        let mut clean_aliases = Vec::new();
        for a in aliases {
            let trimmed = a.trim();
            if trimmed.is_empty() {
                continue;
            }
            let lower = trimmed.to_lowercase();
            if seen.insert(lower) {
                clean_aliases.push(trimmed.to_string());
            }
        }

        if let Some(obj) = self.properties.as_object_mut() {
            if let Some(work_val) = obj.get_mut("work") {
                if let Some(work_obj) = work_val.as_object_mut() {
                    if clean_aliases.is_empty() {
                        work_obj.remove("aliases");
                    } else {
                        work_obj.insert("aliases".to_string(), json!(clean_aliases));
                    }
                }
            }
            if clean_aliases.is_empty() {
                obj.remove("aliases");
            } else {
                obj.insert("aliases".to_string(), json!(clean_aliases));
            }
            self.updated_at = Utc::now();
        }
    }

    /// Check if this work matches an alias query specifically
    pub fn matches_alias(&self, query: &str) -> bool {
        let q = query.trim().to_lowercase();
        if q.is_empty() {
            return false;
        }
        if let Some(meta) = self.as_work_metadata() {
            for alias in &meta.aliases {
                if alias.to_lowercase().contains(&q) {
                    return true;
                }
            }
        }
        false
    }

    /// Match entity against a search query across title, description, original_title, and aliases
    pub fn matches_query(&self, query: &str) -> bool {
        let q = query.trim().to_lowercase();
        if q.is_empty() {
            return true;
        }
        if self.title.to_lowercase().contains(&q) {
            return true;
        }
        if let Some(desc) = &self.description {
            if desc.to_lowercase().contains(&q) {
                return true;
            }
        }
        if let Some(meta) = self.as_work_metadata() {
            if let Some(orig) = &meta.original_title {
                if orig.to_lowercase().contains(&q) {
                    return true;
                }
            }
            for alias in &meta.aliases {
                if alias.to_lowercase().contains(&q) {
                    return true;
                }
            }
        }
        false
    }

    /// Attach type-specific metadata overlay to this work
    pub fn with_type_metadata(mut self, meta: TypeSpecificMetadata) -> Self {
        self.set_type_metadata(Some(meta));
        self
    }

    /// Set or clear type-specific metadata overlay on work entity
    pub fn set_type_metadata(&mut self, meta: Option<TypeSpecificMetadata>) {
        if let Some(obj) = self.properties.as_object_mut() {
            if let Some(work_val) = obj.get_mut("work") {
                if let Some(work_obj) = work_val.as_object_mut() {
                    match &meta {
                        Some(m) => {
                            work_obj.insert("type_metadata".to_string(), json!(m));
                        }
                        None => {
                            work_obj.remove("type_metadata");
                        }
                    }
                }
            }
            match &meta {
                Some(m) => {
                    obj.insert("type_metadata".to_string(), json!(m));
                }
                None => {
                    obj.remove("type_metadata");
                }
            }
            self.updated_at = Utc::now();
        }
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

/// Integrity violation item detected on a Work entity
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IntegrityViolation {
    pub kind: String,
    pub message: String,
}

impl IntegrityViolation {
    pub fn new(kind: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            message: message.into(),
        }
    }
}

/// Integrity check report summarizing invariant checks for a Work entity
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IntegrityReport {
    pub valid: bool,
    pub violations: Vec<IntegrityViolation>,
}

impl IntegrityReport {
    pub fn ok() -> Self {
        Self {
            valid: true,
            violations: Vec::new(),
        }
    }

    pub fn with_violations(violations: Vec<IntegrityViolation>) -> Self {
        let valid = violations.is_empty();
        Self { valid, violations }
    }
}

/// Architectural alias: A Work Profile represents the complete unified digital footprint for any medium.
pub type WorkProfile = WorkSummary;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_work_metadata_serde_with_type_metadata() {
        let anime_meta = TypeSpecificMetadata::Anime(AnimeSpecificMeta {
            season: Some("2024-10".to_string()),
            total_episodes: Some(24),
            anime_format: Some("tv".to_string()),
            studio: Some("Pierrot".to_string()),
            broadcast_day: Some("Saturday".to_string()),
        });

        let mut work = Entity::new_work(
            "BLEACH 千年血战篇",
            WorkType::Anime,
            RecordStatus::InProgress,
            Some("BLEACH 千年血戦篇".to_string()),
            Some("死神千年血战篇动画化".to_string()),
        ).with_type_metadata(anime_meta.clone());

        let meta = work.as_work_metadata().expect("as_work_metadata should succeed");
        assert_eq!(meta.work_type, WorkType::Anime);
        assert_eq!(meta.status, RecordStatus::InProgress);
        assert_eq!(meta.type_metadata, Some(anime_meta));

        // Test progress without total_positions (ongoing/serialized work)
        work = work.with_progress(WorkProgress {
            position: 12.0,
            position_type: ProgressPositionType::Episode,
            total_positions: None,
            unit: Some("集".to_string()),
            updated_at: Utc::now(),
        });

        let meta2 = work.as_work_metadata().unwrap();
        assert_eq!(meta2.progress.as_ref().unwrap().position, 12.0);
        assert_eq!(meta2.progress.as_ref().unwrap().total_positions, None);
    }

    #[test]
    fn test_manga_and_book_type_metadata_overlays() {
        let manga_meta = TypeSpecificMetadata::Manga(MangaSpecificMeta {
            total_volumes: Some(18),
            total_chapters: None, // ongoing serialized manga
            author: Some("藤本树".to_string()),
            magazine: Some("少年JUMP+".to_string()),
        });

        let manga = Entity::new_work(
            "电锯人",
            WorkType::Manga,
            RecordStatus::InProgress,
            Some("チェンソーマン".to_string()),
            None,
        ).with_type_metadata(manga_meta.clone());

        let meta = manga.as_work_metadata().unwrap();
        assert_eq!(meta.work_type, WorkType::Manga);
        assert_eq!(meta.type_metadata, Some(manga_meta));

        let book_meta = TypeSpecificMetadata::Book(BookSpecificMeta {
            isbn: Some("9787111544937".to_string()),
            author: Some("Randal E. Bryant".to_string()),
            translator: Some("龚奕利".to_string()),
            publisher: Some("机械工业出版社".to_string()),
            total_pages: Some(737),
        });

        let book = Entity::new_work(
            "深入理解计算机系统",
            WorkType::Book,
            RecordStatus::Planned,
            Some("Computer Systems: A Programmer's Perspective".to_string()),
            None,
        ).with_type_metadata(book_meta.clone());

        let b_meta = book.as_work_metadata().unwrap();
        assert_eq!(b_meta.work_type, WorkType::Book);
        assert_eq!(b_meta.type_metadata, Some(book_meta));
    }

    #[test]
    fn test_backward_compatibility_without_type_metadata() {
        // Plain json without type_metadata field deserializes cleanly to None
        let legacy_json = json!({
            "work_type": "game",
            "status": "completed",
            "original_title": "Elden Ring",
            "rating": 9.8
        });

        let parsed: WorkMetadata = serde_json::from_value(legacy_json).expect("legacy json must parse");
        assert_eq!(parsed.work_type, WorkType::Game);
        assert_eq!(parsed.status, RecordStatus::Completed);
        assert_eq!(parsed.type_metadata, None);
    }
}
