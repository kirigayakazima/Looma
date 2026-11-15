use std::path::Path;
use std::sync::Arc;
use chrono::Utc;
use serde_json::json;
use uuid::Uuid;
use base64::Engine;

use crate::error::{LoomaError, LoomaResult};
use crate::events::{DomainEvent, EventBus, EventEnvelope};
use crate::models::*;
use crate::services::*;

pub trait VaultRepository:
    VaultService
    + AssetService
    + EntityService
    + RelationService
    + MemoryService
    + CollectionService
    + TimelineService
    + BackupService
    + AuditService
    + ExternalReferenceService
    + ExperienceService
    + SourceRootService
    + SearchService
    + GameActivityService
    + Send
    + Sync
{
}

impl<T> VaultRepository for T where
    T: VaultService
        + AssetService
        + EntityService
        + RelationService
        + MemoryService
        + CollectionService
        + TimelineService
        + BackupService
        + AuditService
        + ExternalReferenceService
        + ExperienceService
        + SourceRootService
        + SearchService
        + GameActivityService
        + Send
        + Sync
{
}

/// Normalizes a directory path string for stable cross-platform deduplication and storage.
pub fn normalize_directory_path(raw: &str) -> String {
    let s = raw.trim();
    // Replace all backslashes with forward slashes
    let mut normalized = s.replace('\\', "/");
    // Strip Windows extended-length prefix \\?\ or //?/
    if let Some(stripped) = normalized.strip_prefix("//?/") {
        normalized = stripped.to_string();
    }
    // Check if Windows UNC network path (starts with //)
    let is_unc = normalized.starts_with("//");
    let body = if is_unc {
        &normalized[2..]
    } else {
        &normalized[..]
    };

    // Remove consecutive duplicate slashes in the path body
    let mut cleaned = String::with_capacity(normalized.len());
    let mut prev_slash = false;
    for c in body.chars() {
        if c == '/' {
            if !prev_slash {
                cleaned.push(c);
                prev_slash = true;
            }
        } else {
            cleaned.push(c);
            prev_slash = false;
        }
    }

    let mut result = if is_unc {
        format!("//{}", cleaned)
    } else {
        cleaned
    };

    // Uppercase drive letter on Windows (e.g. "e:/..." -> "E:/...")
    if result.len() >= 2 && result.as_bytes()[1] == b':' {
        let drive = result[0..1].to_ascii_uppercase();
        result = format!("{}{}", drive, &result[1..]);
    }
    // Trim trailing slash unless it's root like "E:/" or "/" or UNC root like "//server/share"
    if !is_unc && result.len() > 3 && result.ends_with('/') {
        result.pop();
    } else if is_unc && result.len() > 2 && result.ends_with('/') {
        result.pop();
    }
    result
}

/// Checks if `child_path` is strictly equal to or inside `root_path`.
/// Handles Windows case-insensitivity and segment boundaries correctly.
pub fn is_path_under_root(child_path: &str, root_path: &str) -> bool {
    let child_norm = normalize_directory_path(child_path);
    let root_norm = normalize_directory_path(root_path);

    #[cfg(target_os = "windows")]
    let (c, r) = (child_norm.to_lowercase(), root_norm.to_lowercase());
    #[cfg(not(target_os = "windows"))]
    let (c, r) = (child_norm, root_norm);

    if c == r {
        return true;
    }

    if c.starts_with(&r) {
        if r.ends_with('/') {
            return true;
        }
        if c.as_bytes().get(r.len()) == Some(&b'/') {
            return true;
        }
    }
    false
}

/// Computes the relative subpath of `target_path` within `base_path`.
/// Returns `Some(relative_path)` if `target_path` is strictly equal to or inside `base_path`,
/// or `None` otherwise. Handles Windows case-insensitivity and path segment boundaries.
pub fn compute_relative_subpath(base_path: &str, target_path: &str) -> Option<String> {
    let base_norm = normalize_directory_path(base_path);
    let target_norm = normalize_directory_path(target_path);

    #[cfg(target_os = "windows")]
    let matches = target_norm.to_lowercase().starts_with(&base_norm.to_lowercase());
    #[cfg(not(target_os = "windows"))]
    let matches = target_norm.starts_with(&base_norm);

    if !matches {
        return None;
    }

    if target_norm.len() == base_norm.len() {
        return Some(String::new());
    }

    let remainder = &target_norm[base_norm.len()..];
    if remainder.starts_with('/') {
        Some(remainder.trim_start_matches('/').to_string())
    } else {
        None
    }
}

/// The unified Core Application Facade for Looma.
/// Desktop, CLI, and MCP MUST interact with this facade instead of directly manipulating database or internal state.
#[derive(Clone)]
pub struct LoomaCore {
    repo: Arc<dyn VaultRepository>,
    event_bus: EventBus,
    scan_engine: Arc<std::sync::RwLock<Option<Arc<dyn ScanEngine>>>>,
}

use std::sync::atomic::{AtomicUsize, Ordering};

static CONCURRENT_THUMBNAIL_JOBS: AtomicUsize = AtomicUsize::new(0);
const MAX_CONCURRENT_THUMBNAIL_DECODES: usize = 4;

struct ThumbnailThrottleGuard;

impl ThumbnailThrottleGuard {
    fn acquire() -> Self {
        while CONCURRENT_THUMBNAIL_JOBS.fetch_add(1, Ordering::SeqCst) >= MAX_CONCURRENT_THUMBNAIL_DECODES {
            CONCURRENT_THUMBNAIL_JOBS.fetch_sub(1, Ordering::SeqCst);
            std::thread::sleep(std::time::Duration::from_millis(15));
        }
        ThumbnailThrottleGuard
    }
}

impl Drop for ThumbnailThrottleGuard {
    fn drop(&mut self) {
        CONCURRENT_THUMBNAIL_JOBS.fetch_sub(1, Ordering::SeqCst);
    }
}

impl LoomaCore {
    pub fn new(repo: Arc<dyn VaultRepository>) -> Self {
        Self {
            repo,
            event_bus: EventBus::new(),
            scan_engine: Arc::new(std::sync::RwLock::new(None)),
        }
    }

    pub fn with_event_bus(repo: Arc<dyn VaultRepository>, event_bus: EventBus) -> Self {
        Self {
            repo,
            event_bus,
            scan_engine: Arc::new(std::sync::RwLock::new(None)),
        }
    }

    pub fn with_scan_engine(self, engine: Arc<dyn ScanEngine>) -> Self {
        *self.scan_engine.write().unwrap() = Some(engine);
        self
    }

    pub fn set_scan_engine(&self, engine: Arc<dyn ScanEngine>) {
        *self.scan_engine.write().unwrap() = Some(engine);
    }

    pub fn repository(&self) -> &Arc<dyn VaultRepository> {
        &self.repo
    }

    pub fn event_bus(&self) -> &EventBus {
        &self.event_bus
    }

    pub fn subscribe_events<F>(&self, handler: F)
    where
        F: Fn(&EventEnvelope) + Send + Sync + 'static,
    {
        self.event_bus.subscribe(handler);
    }

    fn emit_event(&self, actor: &str, event: DomainEvent) {
        self.event_bus.publish(EventEnvelope::new(actor, event));
    }

    // --- Audit Log ---
    pub fn record_audit(
        &self,
        actor: &str,
        operation: &str,
        target_type: &str,
        target_id: &str,
        result: &str,
        details: serde_json::Value,
    ) -> LoomaResult<()> {
        let log = AuditLog {
            id: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            actor: actor.to_string(),
            operation: operation.to_string(),
            target_type: target_type.to_string(),
            target_id: target_id.to_string(),
            result: result.to_string(),
            details,
        };
        self.repo.record_audit(&log)
    }

    pub fn list_recent_audits(&self, limit: usize) -> LoomaResult<Vec<AuditLog>> {
        self.repo.list_recent_audits(limit)
    }

    // --- Vault Meta & Stats ---
    pub fn get_vault_info(&self) -> LoomaResult<VaultInfo> {
        self.repo.get_vault_info()
    }

    pub fn get_vault_stats(&self) -> LoomaResult<VaultStats> {
        self.repo.get_vault_stats()
    }

    // --- Assets ---
    pub fn list_assets(&self, limit: usize, offset: usize) -> LoomaResult<Vec<Asset>> {
        self.repo.list_assets(limit, offset)
    }

    pub fn get_asset(&self, id: &str) -> LoomaResult<Option<Asset>> {
        self.repo.get_asset_by_id(id)
    }

    pub fn get_asset_by_path(&self, path: &str) -> LoomaResult<Option<Asset>> {
        self.repo.get_asset_by_path(path)
    }

    pub fn list_assets_by_prefix(&self, prefix: &str) -> LoomaResult<Vec<Asset>> {
        self.repo.list_assets_by_prefix(prefix)
    }

    pub fn count_assets(&self) -> LoomaResult<u64> {
        self.repo.count_assets()
    }

    pub fn count_assets_with_filter(&self, filter: &AssetFilter) -> LoomaResult<u64> {
        self.repo.count_assets_with_filter(filter)
    }

    pub fn search_vault(&self, query: &str, category: Option<&str>, limit: usize) -> LoomaResult<VaultSearchResults> {
        self.repo.search_vault(query, category, limit)
    }

    pub fn count_entities(&self) -> LoomaResult<u64> {
        self.repo.count_entities()
    }

    pub fn upsert_asset(&self, actor: &str, asset: &Asset) -> LoomaResult<()> {
        self.repo.upsert_asset(asset)?;
        self.record_audit(
            actor,
            "asset.upsert",
            "asset",
            &asset.id,
            "success",
            json!({ "path": asset.path, "kind": format!("{:?}", asset.kind) }),
        )?;
        self.emit_event(
            actor,
            DomainEvent::AssetCreated {
                id: asset.id.clone(),
                path: asset.path.clone(),
            },
        );
        Ok(())
    }

    pub fn query_assets(&self, filter: &AssetFilter) -> LoomaResult<Vec<Asset>> {
        self.repo.query_assets(filter)
    }

    pub fn delete_asset(&self, actor: &str, id: &str) -> LoomaResult<bool> {
        let deleted = self.repo.delete_asset(id)?;
        if deleted {
            self.record_audit(actor, "asset.delete", "asset", id, "success", json!({}))?;
            self.emit_event(actor, DomainEvent::AssetDeleted { id: id.to_string() });
        }
        Ok(deleted)
    }

    // --- Entities ---
    pub fn list_entities(&self, filter: &EntityFilter) -> LoomaResult<Vec<Entity>> {
        self.repo.list_entities(filter)
    }

    pub fn get_entity(&self, id: &str) -> LoomaResult<Option<Entity>> {
        self.repo.get_entity_by_id(id)
    }

    pub fn create_entity(&self, actor: &str, entity: &Entity) -> LoomaResult<()> {
        if entity.title.trim().is_empty() {
            return Err(LoomaError::Validation("Entity title cannot be empty".to_string()));
        }
        self.repo.create_entity(entity)?;
        self.record_audit(
            actor,
            "entity.create",
            "entity",
            &entity.id,
            "success",
            json!({ "title": entity.title, "type": entity.entity_type }),
        )?;
        self.emit_event(
            actor,
            DomainEvent::EntityCreated {
                id: entity.id.clone(),
                title: entity.title.clone(),
            },
        );
        Ok(())
    }

    pub fn update_entity(&self, actor: &str, entity: &Entity) -> LoomaResult<()> {
        if entity.title.trim().is_empty() {
            return Err(LoomaError::Validation("Entity title cannot be empty".to_string()));
        }

        // Fetch existing entity to verify existence and check for Work domain protection
        let existing = self.repo.get_entity_by_id(&entity.id)?
            .ok_or_else(|| LoomaError::NotFound(format!("Entity not found: {}", entity.id)))?;

        let is_work = existing.as_work_metadata().is_some()
            || existing.entity_type == "work"
            || existing.properties.get("domain").and_then(|v| v.as_str()) == Some("work");

        let mut final_entity = entity.clone();

        if is_work {
            if let Some(work_val) = entity.properties.get("work") {
                // Must be a valid JSON Object, not a scalar string or number
                if !work_val.is_object() {
                    return Err(LoomaError::Validation(
                        "Invalid properties.work: expected a JSON object representing WorkMetadata, found non-object scalar".to_string()
                    ));
                }
                // Must deserialize to WorkMetadata
                if let Err(e) = serde_json::from_value::<WorkMetadata>(work_val.clone()) {
                    return Err(LoomaError::Validation(format!(
                        "Invalid properties.work payload: {e}"
                    )));
                }
            } else {
                // Caller did not provide "work", e.g. generic entity editor modifying title/description/custom fields.
                // PRESERVE the existing WorkMetadata and work domain attributes!
                if let Some(existing_work) = existing.properties.get("work") {
                    if let Some(obj) = final_entity.properties.as_object_mut() {
                        obj.insert("work".to_string(), existing_work.clone());
                        if !obj.contains_key("domain") {
                            obj.insert("domain".to_string(), serde_json::json!("work"));
                        }
                        if !obj.contains_key("status") {
                            if let Some(st) = existing.properties.get("status") {
                                obj.insert("status".to_string(), st.clone());
                            }
                        }
                        if !obj.contains_key("work_type") {
                            if let Some(wt) = existing.properties.get("work_type") {
                                obj.insert("work_type".to_string(), wt.clone());
                            }
                        }
                    }
                }
            }
        }

        // CAS check: if caller passed an entity with an older updated_at than what is in DB
        if entity.updated_at < existing.updated_at {
            return Err(LoomaError::Validation(format!(
                "Stale update conflict for entity '{}': loaded at {}, but current DB version is {}",
                entity.id, entity.updated_at, existing.updated_at
            )));
        }

        final_entity.updated_at = Utc::now();

        let cas_ok = self.repo.update_entity_cas(&final_entity, &existing.updated_at)?;
        if !cas_ok {
            return Err(LoomaError::Validation(format!(
                "Concurrent update conflict: entity '{}' was modified concurrently by another transaction",
                final_entity.id
            )));
        }
        self.record_audit(
            actor,
            "entity.update",
            "entity",
            &final_entity.id,
            "success",
            json!({ "title": final_entity.title, "type": final_entity.entity_type }),
        )?;
        self.emit_event(
            actor,
            DomainEvent::EntityUpdated {
                id: final_entity.id.clone(),
            },
        );
        Ok(())
    }

    pub fn delete_entity(&self, actor: &str, id: &str) -> LoomaResult<bool> {
        let deleted = self.repo.delete_entity(id)?;
        if deleted {
            self.record_audit(actor, "entity.delete", "entity", id, "success", json!({}))?;
            self.emit_event(actor, DomainEvent::EntityDeleted { id: id.to_string() });
        }
        Ok(deleted)
    }

    // --- Personal Records / Work Domain (Expansion Pack) ---
    pub fn create_work(
        &self,
        actor: &str,
        title: &str,
        work_type: WorkType,
        status: RecordStatus,
        original_title: Option<String>,
        description: Option<String>,
    ) -> LoomaResult<Entity> {
        let entity = Entity::new_work(title, work_type, status, original_title, description);
        self.create_entity(actor, &entity)?;
        Ok(entity)
    }

    pub fn update_work_status(
        &self,
        actor: &str,
        entity_id: &str,
        status: RecordStatus,
    ) -> LoomaResult<Entity> {
        let mut entity = self.get_entity(entity_id)?
            .ok_or_else(|| LoomaError::NotFound(format!("Work entity not found: {entity_id}")))?;
        
        let mut props = entity.properties.clone();
        if let Some(obj) = props.as_object_mut() {
            obj.insert("status".to_string(), json!(status.as_str()));
            if let Some(work_val) = obj.get_mut("work") {
                if let Some(work_obj) = work_val.as_object_mut() {
                    work_obj.insert("status".to_string(), json!(status));
                }
            }
        }
        entity.properties = props;
        entity.updated_at = Utc::now();
        self.update_entity(actor, &entity)?;
        Ok(entity)
    }

    pub fn update_work_progress(
        &self,
        actor: &str,
        entity_id: &str,
        position: f64,
        position_type: Option<ProgressPositionType>,
        total_positions: Option<f64>,
        unit: Option<String>,
    ) -> LoomaResult<Entity> {
        let mut entity = self.get_entity(entity_id)?
            .ok_or_else(|| LoomaError::NotFound(format!("Work entity not found: {entity_id}")))?;

        let mut props = entity.properties.clone();
        if let Some(obj) = props.as_object_mut() {
            let existing_status = entity.as_work_metadata().map(|m| m.status).unwrap_or_else(|| {
                entity.properties.get("status")
                    .and_then(|v| v.as_str())
                    .map(RecordStatus::parse)
                    .unwrap_or(RecordStatus::Planned)
            });

            let mut work_meta: WorkMetadata = if let Some(w) = obj.get("work") {
                serde_json::from_value(w.clone()).unwrap_or_else(|_| WorkMetadata {
                    work_type: WorkType::parse(&entity.entity_type),
                    status: existing_status,
                    ..Default::default()
                })
            } else {
                WorkMetadata {
                    work_type: WorkType::parse(&entity.entity_type),
                    status: existing_status,
                    ..Default::default()
                }
            };

            let p_type = position_type.unwrap_or_else(|| {
                work_meta.progress.as_ref().map(|p| p.position_type.clone()).unwrap_or_else(|| {
                    match work_meta.work_type {
                        WorkType::Anime | WorkType::TvSeries => ProgressPositionType::Episode,
                        WorkType::Manga => ProgressPositionType::Chapter,
                        WorkType::Book | WorkType::Novel => ProgressPositionType::Chapter,
                        WorkType::Movie | WorkType::Documentary => ProgressPositionType::Minute,
                        _ => ProgressPositionType::Custom("step".to_string()),
                    }
                })
            });

            let total = total_positions.or_else(|| work_meta.progress.as_ref().and_then(|p| p.total_positions));
            let unit_str = unit.or_else(|| work_meta.progress.as_ref().and_then(|p| p.unit.clone()));

            let progress = WorkProgress {
                position,
                position_type: p_type,
                total_positions: total,
                unit: unit_str,
                updated_at: Utc::now(),
            };

            work_meta.progress = Some(progress.clone());
            obj.insert("work".to_string(), json!(work_meta));
            obj.insert("progress".to_string(), json!(progress));
        }

        entity.properties = props;
        entity.updated_at = Utc::now();
        self.update_entity(actor, &entity)?;

        self.record_audit(
            actor,
            "work.progress_update",
            "entity",
            entity_id,
            "success",
            json!({ "position": position }),
        )?;

        self.emit_event(
            actor,
            DomainEvent::EntityUpdated { id: entity_id.to_string() },
        );

        Ok(entity)
    }

    pub fn update_work_aliases(
        &self,
        actor: &str,
        entity_id: &str,
        aliases: Vec<String>,
    ) -> LoomaResult<Entity> {
        let mut entity = self.get_entity(entity_id)?
            .ok_or_else(|| LoomaError::NotFound(format!("Work entity not found: {entity_id}")))?;

        entity.set_aliases(aliases.clone());
        self.update_entity(actor, &entity)?;

        self.record_audit(
            actor,
            "work.aliases_update",
            "entity",
            entity_id,
            "success",
            json!({ "aliases": aliases }),
        )?;

        self.emit_event(
            actor,
            DomainEvent::EntityUpdated { id: entity_id.to_string() },
        );

        Ok(entity)
    }

    pub fn set_primary_external_reference(
        &self,
        actor: &str,
        entity_id: &str,
        reference_id: &str,
    ) -> LoomaResult<Vec<ExternalReference>> {
        let res: LoomaResult<Vec<ExternalReference>> = (|| {
            // Pre-validation 1: Entity exists
            if self.get_entity(entity_id)?.is_none() {
                return Err(LoomaError::NotFound(format!("Entity not found: {entity_id}")));
            }

            // Pre-validation 2: Reference exists and belongs to entity
            let r = self.get_external_reference(reference_id)?
                .ok_or_else(|| LoomaError::NotFound(format!("External reference not found: {reference_id}")))?;
            if r.entity_id.as_deref() != Some(entity_id) {
                return Err(LoomaError::Validation(format!(
                    "External reference {reference_id} does not belong to entity {entity_id}"
                )));
            }

            // Execute atomic transaction in repository
            self.repo.set_primary_external_reference(entity_id, reference_id)?;

            self.list_external_references(Some(entity_id))
        })();

        match res {
            Ok(updated) => {
                self.record_audit(
                    actor,
                    "work.set_primary_external_reference",
                    "entity",
                    entity_id,
                    "success",
                    json!({ "primary_reference_id": reference_id }),
                )?;

                self.emit_event(
                    actor,
                    DomainEvent::EntityUpdated {
                        id: entity_id.to_string(),
                    },
                );

                Ok(updated)
            }
            Err(e) => {
                let _ = self.record_audit(
                    actor,
                    "work.set_primary_external_reference",
                    "entity",
                    entity_id,
                    "failure",
                    json!({
                        "target_reference_id": reference_id,
                        "error": e.to_string(),
                    }),
                );
                Err(e)
            }
        }
    }

    pub fn update_work_cover_asset(
        &self,
        actor: &str,
        entity_id: &str,
        cover_asset_id: Option<String>,
    ) -> LoomaResult<Entity> {
        let mut entity = self.get_entity(entity_id)?
            .ok_or_else(|| LoomaError::NotFound(format!("Work entity not found: {entity_id}")))?;

        entity.set_cover_asset_id(cover_asset_id.as_deref());
        self.update_entity(actor, &entity)?;

        self.record_audit(
            actor,
            "work.cover_update",
            "entity",
            entity_id,
            "success",
            json!({ "cover_asset_id": cover_asset_id }),
        )?;

        self.emit_event(
            actor,
            DomainEvent::EntityUpdated { id: entity_id.to_string() },
        );

        Ok(entity)
    }

    /// Cleans raw candidate or rip folder names for accurate Steam store search.
    pub fn clean_game_title_for_search(raw: &str) -> String {
        let mut s = String::new();
        let mut in_bracket = false;
        for c in raw.chars() {
            if c == '[' || c == '【' || c == '(' {
                in_bracket = true;
            } else if c == ']' || c == '】' || c == ')' {
                in_bracket = false;
            } else if !in_bracket {
                s.push(c);
            }
        }

        // Replace +, _, - with space first
        s = s.replace('+', " ")
             .replace('_', " ")
             .replace('-', " ");

        // Strip version tokens like v1.2, v0.0.9, v1.0 before dot replacement
        let pre_words = s.split_whitespace().collect::<Vec<_>>();
        let mut pre_kept = Vec::new();
        for word in pre_words {
            let lower = word.to_lowercase();
            // If token starts with 'v' or 'ver' followed by a digit (e.g. v1.2, v0.0.9, v2)
            if (lower.starts_with('v') || lower.starts_with("ver"))
                && lower.chars().skip_while(|c| c.is_alphabetic()).next().map_or(false, |c| c.is_ascii_digit())
            {
                continue;
            }
            pre_kept.push(word);
        }
        s = pre_kept.join(" ");

        // Now replace . with space for remaining words (e.g. Shapez.2 -> Shapez 2)
        s = s.replace('.', " ");

        // Case-insensitive phrase stripping for common edition prefixes/suffixes
        let lower_initial = s.to_lowercase();
        let phrases_to_remove = [
            "game of the yorha edition",
            "game of the year edition",
            "game of the year",
            "definitive edition",
            "legendary edition",
            "enhanced edition",
            "special edition",
            "digital deluxe",
            "deluxe edition",
            "gold edition",
        ];
        for phrase in phrases_to_remove {
            if let Some(idx) = lower_initial.find(phrase) {
                s.replace_range(idx..idx + phrase.len(), " ");
                break;
            }
        }

        let remove_keywords = [
            "3dmgame", "chs", "cht", "green", "repack", "goldberg", "build",
            "launcher", "edition", "yorha", "remastered", "test",
            "demo", "version", "v0", "v1", "v2", "v3", "v4", "v5", "v6", "v7", "v8", "v9",
            "测试版", "客户端", "整合包", "单机"
        ];

        let words: Vec<&str> = s.split_whitespace().collect();
        let mut kept_words = Vec::new();
        for word in words {
            let lower = word.to_lowercase();
            if remove_keywords.iter().any(|&kw| lower == kw || lower.starts_with(kw)) {
                continue;
            }
            kept_words.push(word);
        }

        // If the last word is a lone 0 leftover from version splitting (e.g. v1.0 -> v1 0)
        if let Some(&"0") = kept_words.last() {
            if kept_words.len() > 1 {
                kept_words.pop();
            }
        }

        let result = kept_words.join(" ").trim().to_string();
        if result.is_empty() {
            raw.trim().to_string()
        } else {
            result
        }
    }

    /// Percent encodes a string for HTTP query params without external dependencies.
    pub fn percent_encode(input: &str) -> String {
        let mut encoded = String::new();
        for b in input.bytes() {
            if b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.' || b == b'~' {
                encoded.push(b as char);
            } else if b == b' ' {
                encoded.push('+');
            } else {
                encoded.push_str(&format!("%{:02X}", b));
            }
        }
        encoded
    }

    pub fn set_work_cover_from_bytes(
        &self,
        actor: &str,
        work_id: &str,
        bytes: &[u8],
        user_selected: bool,
        provider: Option<&str>,
    ) -> LoomaResult<Asset> {
        let _work = self.get_entity(work_id)?
            .ok_or_else(|| LoomaError::NotFound(format!("Work entity '{work_id}' not found")))?;

        if bytes.len() > 50 * 1024 * 1024 {
            return Err(LoomaError::Validation("Cover image exceeds 50MB safety limit".to_string()));
        }

        let cursor = std::io::Cursor::new(bytes);
        let reader = image::ImageReader::new(cursor)
            .with_guessed_format()
            .map_err(|e| LoomaError::Validation(format!("Unsupported image format: {e}")))?;

        let format = reader.format()
            .ok_or_else(|| LoomaError::Validation("Unknown image format".to_string()))?;

        let (width, height) = reader.into_dimensions()
            .map_err(|e| LoomaError::Validation(format!("Corrupted or unsupported image header: {e}")))?;

        if (width as u64) * (height as u64) > 64_000_000 {
            return Err(LoomaError::Validation("Image resolution exceeds 64 megapixels limit".to_string()));
        }

        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        let hash = format!("{:x}", hasher.finalize());

        let (ext, mime_type) = match format {
            image::ImageFormat::Png => ("png", "image/png"),
            image::ImageFormat::Jpeg => ("jpg", "image/jpeg"),
            image::ImageFormat::WebP => ("webp", "image/webp"),
            image::ImageFormat::Gif => ("gif", "image/gif"),
            _ => ("bin", "application/octet-stream"),
        };

        let aspect_ratio = if (width as f64 / height as f64 - 1.0).abs() < 0.15 {
            "1:1"
        } else if height > width {
            "2:3"
        } else {
            "landscape"
        };

        let vault_info = self.get_vault_info()?;
        let covers_dir = if vault_info.vault_path.as_os_str() == ":memory:" {
            std::env::temp_dir().join("looma_test_media").join("covers")
        } else {
            vault_info.vault_path.join(".looma").join("media").join("covers")
        };
        std::fs::create_dir_all(&covers_dir)?;

        let file_name = format!("{hash}.{ext}");
        let dest_path = covers_dir.join(&file_name);
        if !dest_path.exists() {
            std::fs::write(&dest_path, bytes)?;
        }

        let rel_path = format!(".looma/media/covers/{file_name}");
        let asset_id = format!("cover_{}", &hash[..16]);
        let now = Utc::now();
        let asset = Asset {
            id: asset_id.clone(),
            kind: AssetKind::Image,
            source: AssetSource::Managed,
            path: Some(rel_path),
            size: Some(bytes.len() as u64),
            hash: Some(hash.clone()),
            mime_type: Some(mime_type.to_string()),
            metadata: json!({
                "role": "work_cover",
                "user_selected": user_selected,
                "provider": provider.unwrap_or("unknown"),
                "aspect_ratio": aspect_ratio,
                "width": width,
                "height": height,
                "content_sha256": hash,
            }),
            status: AssetStatus::Active,
            created_at: now,
            modified_at: now,
            indexed_at: now,
        };
        self.upsert_asset(actor, &asset)?;
        self.update_work_cover_asset(actor, work_id, Some(asset_id))?;

        Ok(asset)
    }

    pub fn set_work_cover_from_file(
        &self,
        actor: &str,
        work_id: &str,
        file_path: &Path,
        user_selected: bool,
    ) -> LoomaResult<Asset> {
        if !file_path.exists() {
            return Err(LoomaError::NotFound(format!("File does not exist: {}", file_path.display())));
        }
        let file_meta = file_path.metadata()?;
        if file_meta.len() > 50 * 1024 * 1024 {
            return Err(LoomaError::Validation("Cover image exceeds 50MB safety limit".to_string()));
        }
        let bytes = std::fs::read(file_path)?;
        self.set_work_cover_from_bytes(actor, work_id, &bytes, user_selected, Some("local_file"))
    }

    /// Fast non-blocking health check for local or remote OpenAI-compatible AI reverse proxy.
    pub fn check_ai_service_status(
        &self,
        endpoint: Option<&str>,
        api_key: Option<&str>,
    ) -> AiServiceStatus {
        let ep = endpoint.unwrap_or("http://localhost:8081/v1").trim_end_matches('/');
        let key = api_key.unwrap_or("sk-gemini");
        let model = "gemini-flash-lite";

        let start = std::time::Instant::now();
        let models_url = format!("{ep}/models");

        match ureq::get(&models_url)
            .timeout(std::time::Duration::from_millis(1500))
            .set("Authorization", &format!("Bearer {key}"))
            .call()
        {
            Ok(resp) => {
                if resp.status() == 200 {
                    let latency = start.elapsed().as_millis() as u64;
                    AiServiceStatus {
                        online: true,
                        endpoint: ep.to_string(),
                        model: model.to_string(),
                        latency_ms: Some(latency),
                        message: Some("Local AI reverse proxy connected".to_string()),
                    }
                } else {
                    AiServiceStatus {
                        online: false,
                        endpoint: ep.to_string(),
                        model: model.to_string(),
                        latency_ms: None,
                        message: Some(format!("HTTP error {}", resp.status())),
                    }
                }
            }
            Err(e) => AiServiceStatus {
                online: false,
                endpoint: ep.to_string(),
                model: model.to_string(),
                latency_ms: None,
                message: Some(format!("Unreachable: {e}")),
            },
        }
    }

    /// Analyzes an unnormalized or messy game folder name/title using local or remote LLM.
    pub fn analyze_game_with_ai(
        &self,
        raw_title: &str,
        endpoint: Option<&str>,
        api_key: Option<&str>,
        model: Option<&str>,
    ) -> LoomaResult<AiGameAnalysis> {
        let ep = endpoint.unwrap_or("http://localhost:8081/v1").trim_end_matches('/');
        let key = api_key.unwrap_or("sk-gemini");
        let chosen_model = model.unwrap_or("gemini-flash-lite");

        let prompt = format!(
            "You are a video game catalog expert. Analyze the following messy game title or directory name from a user's library:\n\
            \"{raw_title}\"\n\n\
            Identify the exact commercial game. Deduce its official primary title (in Chinese if popular, else English), English title, Steam AppID (if available on Steam), and Nintendo Switch Title ID (16-char hex like 01007EF00011E000, if on Switch).\n\
            Respond ONLY with a valid JSON object matching this schema:\n\
            {{\n\
              \"canonical_title\": \"official title\",\n\
              \"english_title\": \"English title or null\",\n\
              \"steam_appid\": \"Steam AppID numeric string or null\",\n\
              \"switch_title_id\": \"16-char hex Title ID or null\"\n\
            }}\n\
            Do not include any code block markdown like ```json, only the raw JSON."
        );

        let body = json!({
            "model": chosen_model,
            "messages": [
                {
                    "role": "user",
                    "content": prompt,
                }
            ],
            "temperature": 0.2
        });

        let chat_url = format!("{ep}/chat/completions");
        let resp = ureq::post(&chat_url)
            .timeout(std::time::Duration::from_secs(18))
            .set("Authorization", &format!("Bearer {key}"))
            .set("Content-Type", "application/json")
            .send_string(&body.to_string())
            .map_err(|e| LoomaError::Validation(format!("AI service request failed: {e}")))?;

        if resp.status() != 200 {
            return Err(LoomaError::Validation(format!("AI service returned HTTP {}", resp.status())));
        }

        let val: serde_json::Value = serde_json::from_reader(resp.into_reader())
            .map_err(|e| LoomaError::Validation(format!("Failed to parse AI JSON response: {e}")))?;

        let content = val.get("choices")
            .and_then(|c| c.as_array())
            .and_then(|a| a.first())
            .and_then(|c| c.get("message"))
            .and_then(|m| m.get("content"))
            .and_then(|cnt| cnt.as_str())
            .ok_or_else(|| LoomaError::Validation("Empty or invalid choices from AI".to_string()))?;

        let cleaned = content.trim();
        let cleaned = if cleaned.starts_with("```") {
            let without_start = cleaned.trim_start_matches('`');
            let without_lang = if without_start.starts_with("json") {
                &without_start[4..]
            } else {
                without_start
            };
            without_lang.trim_end_matches('`').trim()
        } else {
            cleaned
        };

        let analysis: AiGameAnalysis = serde_json::from_str(cleaned)
            .map_err(|e| LoomaError::Validation(format!("Failed to deserialize AI game analysis: {e}. Raw content: {cleaned}")))?;

        Ok(analysis)
    }

    pub fn download_and_set_work_cover(
        &self,
        actor: &str,
        work_id: &str,
        url: &str,
        user_selected: bool,
    ) -> LoomaResult<Asset> {
        let resp = ureq::get(url)
            .timeout(std::time::Duration::from_secs(12))
            .set("User-Agent", "Looma/0.6 (+https://github.com/kirigayakazima/Looma)")
            .call()
            .map_err(|e| LoomaError::Validation(format!("Failed to fetch cover from URL: {e}")))?;

        if resp.status() != 200 {
            return Err(LoomaError::Validation(format!("Cover image URL returned status {}", resp.status())));
        }

        use std::io::Read;
        let mut bytes = Vec::new();
        resp.into_reader()
            .take(50 * 1024 * 1024 + 1)
            .read_to_end(&mut bytes)?;

        if bytes.len() > 50 * 1024 * 1024 {
            return Err(LoomaError::Validation("Downloaded cover image exceeds 50MB safety limit".to_string()));
        }

        let provider = if url.contains("nlib.cc") {
            "nlib"
        } else if url.contains("steam") {
            "steam"
        } else {
            "online"
        };

        self.set_work_cover_from_bytes(actor, work_id, &bytes, user_selected, Some(provider))
    }

    pub fn fetch_cover_candidates(
        &self,
        work_id: &str,
        search_term: Option<&str>,
    ) -> LoomaResult<Vec<CoverCandidate>> {
        let summary = self.get_work_summary(work_id)?
            .ok_or_else(|| LoomaError::NotFound(format!("Work '{work_id}' not found")))?;
        let work = &summary.entity;
        let mut candidates = Vec::new();

        let explicit_term = search_term.map(|s| s.trim()).filter(|s| !s.is_empty());

        // 1. Check if term is a 16-hex Switch Title ID or work has switch activities
        let mut tids = Vec::new();
        if let Some(t) = explicit_term {
            if t.len() == 16 && t.chars().all(|c| c.is_ascii_hexdigit()) {
                tids.push(t.to_uppercase());
            }
        }
        let activities = self.list_game_activities_for_work(work_id).unwrap_or_default();
        for act in activities {
            if act.provider == "switch_nx" {
                let upper = act.external_id.to_uppercase();
                if upper.len() == 16 && !tids.contains(&upper) {
                    tids.push(upper);
                }
            }
        }

        for tid in tids {
            let nlib_meta_url = format!("https://api.nlib.cc/nx/{tid}");
            let mut game_name = None;
            if let Ok(resp) = ureq::get(&nlib_meta_url).timeout(std::time::Duration::from_secs(5)).call() {
                if resp.status() == 200 {
                    if let Ok(val) = serde_json::from_reader::<_, serde_json::Value>(resp.into_reader()) {
                        if let Some(n) = val.get("name").and_then(|v| v.as_str()) {
                            game_name = Some(n.to_string());
                        }
                    }
                }
            }
            let label_prefix = game_name.unwrap_or_else(|| work.title.clone());
            candidates.push(CoverCandidate {
                provider: "nlib".to_string(),
                title: format!("{label_prefix} (Switch 官方方形图标)"),
                image_url: format!("https://api.nlib.cc/nx/{tid}/icon"),
                aspect_ratio: "1:1".to_string(),
                description: Some(format!("512×512 高清图标 | Title ID: {tid}")),
            });
            candidates.push(CoverCandidate {
                provider: "nlib".to_string(),
                title: format!("{label_prefix} (Switch 官方横版横幅)"),
                image_url: format!("https://api.nlib.cc/nx/{tid}/banner/720p"),
                aspect_ratio: "landscape".to_string(),
                description: Some(format!("720p 横幅 | Title ID: {tid}")),
            });
        }

        // 2. Check Steam AppID from term or external references
        let mut appids = Vec::new();
        if let Some(t) = explicit_term {
            if t.chars().all(|c| c.is_ascii_digit()) && t.len() >= 3 && t.len() <= 9 {
                appids.push(t.to_string());
            }
        }
        for r in &summary.external_references {
            if r.provider == "steam" || r.url.contains("store.steampowered.com/app/") {
                if let Some(part) = r.url.split("/app/").nth(1) {
                    if let Some(id_str) = part.split('/').next() {
                        if id_str.chars().all(|c| c.is_ascii_digit()) && !appids.contains(&id_str.to_string()) {
                            appids.push(id_str.to_string());
                        }
                    }
                }
            }
        }

        for appid in appids {
            candidates.push(CoverCandidate {
                provider: "steam".to_string(),
                title: format!("{} (Steam 官方竖版海报)", work.title),
                image_url: format!("https://shared.steamstatic.com/store_item_assets/steam/apps/{appid}/library_600x900.jpg"),
                aspect_ratio: "2:3".to_string(),
                description: Some(format!("600×900 官方海报 | AppID: {appid}")),
            });
            candidates.push(CoverCandidate {
                provider: "steam".to_string(),
                title: format!("{} (Steam 官方横幅)", work.title),
                image_url: format!("https://shared.steamstatic.com/store_item_assets/steam/apps/{appid}/header.jpg"),
                aspect_ratio: "landscape".to_string(),
                description: Some(format!("Steam Header | AppID: {appid}")),
            });
        }

        // 3. Online keyword search on Steam
        let query_text = explicit_term
            .map(|s| s.to_string())
            .unwrap_or_else(|| Self::clean_game_title_for_search(&work.title));

        if query_text.len() >= 2 && !query_text.chars().all(|c| c.is_ascii_hexdigit()) {
            let encoded = Self::percent_encode(&query_text);
            let search_url = format!("https://store.steampowered.com/api/storesearch/?term={encoded}&cc=US");
            if let Ok(resp) = ureq::get(&search_url).timeout(std::time::Duration::from_secs(6)).call() {
                if resp.status() == 200 {
                    if let Ok(val) = serde_json::from_reader::<_, serde_json::Value>(resp.into_reader()) {
                        if let Some(items) = val.get("items").and_then(|v| v.as_array()) {
                            for item in items.iter().take(6) {
                                if let (Some(id), Some(name)) = (
                                    item.get("id").and_then(|v| v.as_u64()),
                                    item.get("name").and_then(|v| v.as_str()),
                                ) {
                                    let capsule_url = format!("https://shared.steamstatic.com/store_item_assets/steam/apps/{id}/library_600x900.jpg");
                                    if !candidates.iter().any(|c| c.image_url == capsule_url) {
                                        candidates.push(CoverCandidate {
                                            provider: "steam".to_string(),
                                            title: name.to_string(),
                                            image_url: capsule_url,
                                            aspect_ratio: "2:3".to_string(),
                                            description: Some(format!("Steam 600×900 海报 | AppID: {id}")),
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // 4. AI-assisted fallback if no candidates found and AI service is online
        if candidates.is_empty() {
            let ai_status = self.check_ai_service_status(None, None);
            if ai_status.online {
                if let Ok(analysis) = self.analyze_game_with_ai(&work.title, None, None, None) {
                    if let Some(appid) = &analysis.steam_appid {
                        let id_clean = appid.trim();
                        if id_clean.chars().all(|c| c.is_ascii_digit()) && !id_clean.is_empty() {
                            let capsule_url = format!("https://shared.steamstatic.com/store_item_assets/steam/apps/{id_clean}/library_600x900.jpg");
                            candidates.push(CoverCandidate {
                                provider: "steam".to_string(),
                                title: format!("{} (AI 识别: {})", analysis.canonical_title, work.title),
                                image_url: capsule_url,
                                aspect_ratio: "2:3".to_string(),
                                description: Some(format!("AI 识别 Steam AppID: {id_clean}")),
                            });
                        }
                    }

                    if let Some(tid) = &analysis.switch_title_id {
                        let upper = tid.trim().to_uppercase();
                        if upper.len() == 16 && upper.chars().all(|c| c.is_ascii_hexdigit()) {
                            candidates.push(CoverCandidate {
                                provider: "nlib".to_string(),
                                title: format!("{} (AI 识别 Switch 图标)", analysis.canonical_title),
                                image_url: format!("https://api.nlib.cc/nx/{upper}/icon"),
                                aspect_ratio: "1:1".to_string(),
                                description: Some(format!("AI 识别 Switch Title ID: {upper}")),
                            });
                        }
                    }

                    if !analysis.canonical_title.trim().is_empty() {
                        let encoded = Self::percent_encode(&analysis.canonical_title);
                        let search_url = format!("https://store.steampowered.com/api/storesearch/?term={encoded}&cc=US");
                        if let Ok(resp) = ureq::get(&search_url).timeout(std::time::Duration::from_secs(6)).call() {
                            if resp.status() == 200 {
                                if let Ok(val) = serde_json::from_reader::<_, serde_json::Value>(resp.into_reader()) {
                                    if let Some(items) = val.get("items").and_then(|v| v.as_array()) {
                                        for item in items.iter().take(4) {
                                            if let (Some(id), Some(name)) = (
                                                item.get("id").and_then(|v| v.as_u64()),
                                                item.get("name").and_then(|v| v.as_str()),
                                            ) {
                                                let capsule_url = format!("https://shared.steamstatic.com/store_item_assets/steam/apps/{id}/library_600x900.jpg");
                                                if !candidates.iter().any(|c| c.image_url == capsule_url) {
                                                    candidates.push(CoverCandidate {
                                                        provider: "steam".to_string(),
                                                        title: format!("{} (AI 检索)", name),
                                                        image_url: capsule_url,
                                                        aspect_ratio: "2:3".to_string(),
                                                        description: Some(format!("AI 匹配 Steam | AppID: {id}")),
                                                    });
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(candidates)
    }

    pub fn batch_auto_match_game_covers(
        &self,
        actor: &str,
    ) -> LoomaResult<AutoMatchResult> {
        let all_works = self.list_works(Some("game"), None)?;
        let mut matched_count = 0;
        let mut skipped_count = 0;
        let mut failed_count = 0;
        let mut matched_titles = Vec::new();

        for work in all_works {
            let meta = work.as_work_metadata();
            if let Some(m) = &meta {
                if m.cover_asset_id.is_some() {
                    skipped_count += 1;
                    continue;
                }
            }

            // 1. Try Switch Title ID
            let activities = self.list_game_activities_for_work(&work.id).unwrap_or_default();
            let switch_tid = activities.iter().find_map(|act| {
                if act.provider == "switch_nx" && act.external_id.len() == 16 {
                    Some(act.external_id.to_uppercase())
                } else {
                    None
                }
            });

            if let Some(tid) = switch_tid {
                let icon_url = format!("https://api.nlib.cc/nx/{tid}/icon");
                if self.download_and_set_work_cover(actor, &work.id, &icon_url, false).is_ok() {
                    matched_count += 1;
                    matched_titles.push(work.title.clone());
                    continue;
                }
            }

            // 2. Try Steam AppID from External Reference
            let summary = self.get_work_summary(&work.id).ok().flatten();
            let steam_appid = summary.as_ref().and_then(|s| {
                s.external_references.iter().find_map(|r| {
                    if r.provider == "steam" {
                        r.url.split("/app/").nth(1)?.split('/').next().map(|s| s.to_string())
                    } else {
                        None
                    }
                })
            });

            if let Some(appid) = steam_appid {
                let capsule_url = format!("https://shared.steamstatic.com/store_item_assets/steam/apps/{appid}/library_600x900.jpg");
                if self.download_and_set_work_cover(actor, &work.id, &capsule_url, false).is_ok() {
                    matched_count += 1;
                    matched_titles.push(work.title.clone());
                    continue;
                }
            }

            // 3. Try Auto Search on Steam by cleaned title
            let cleaned = Self::clean_game_title_for_search(&work.title);
            if cleaned.len() >= 2 {
                let encoded = Self::percent_encode(&cleaned);
                let search_url = format!("https://store.steampowered.com/api/storesearch/?term={encoded}&cc=US");
                let mut found = false;
                if let Ok(resp) = ureq::get(&search_url).timeout(std::time::Duration::from_secs(5)).call() {
                    if resp.status() == 200 {
                        if let Ok(val) = serde_json::from_reader::<_, serde_json::Value>(resp.into_reader()) {
                            if let Some(items) = val.get("items").and_then(|v| v.as_array()) {
                                if let Some(first) = items.first() {
                                    if let Some(app_id) = first.get("id").and_then(|v| v.as_u64()) {
                                        let capsule_url = format!("https://shared.steamstatic.com/store_item_assets/steam/apps/{app_id}/library_600x900.jpg");
                                        if self.download_and_set_work_cover(actor, &work.id, &capsule_url, false).is_ok() {
                                            matched_count += 1;
                                            matched_titles.push(work.title.clone());
                                            found = true;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                if found {
                    continue;
                }
            }

            // 4. Try AI-Assisted Matching for messy/obscure names if AI service is online
            let mut ai_matched = false;
            let ai_status = self.check_ai_service_status(None, None);
            if ai_status.online {
                if let Ok(analysis) = self.analyze_game_with_ai(&work.title, None, None, None) {
                    // 4.1 Try Steam AppID from AI
                    if let Some(appid) = &analysis.steam_appid {
                        let id_clean = appid.trim();
                        if id_clean.chars().all(|c| c.is_ascii_digit()) && !id_clean.is_empty() {
                            let capsule_url = format!("https://shared.steamstatic.com/store_item_assets/steam/apps/{id_clean}/library_600x900.jpg");
                            if self.download_and_set_work_cover(actor, &work.id, &capsule_url, false).is_ok() {
                                matched_count += 1;
                                matched_titles.push(format!("{} (AI -> {})", work.title, analysis.canonical_title));
                                ai_matched = true;
                            }
                        }
                    }

                    // 4.2 Try Switch Title ID from AI
                    if !ai_matched {
                        if let Some(tid) = &analysis.switch_title_id {
                            let upper = tid.trim().to_uppercase();
                            if upper.len() == 16 && upper.chars().all(|c| c.is_ascii_hexdigit()) {
                                let icon_url = format!("https://api.nlib.cc/nx/{upper}/icon");
                                if self.download_and_set_work_cover(actor, &work.id, &icon_url, false).is_ok() {
                                    matched_count += 1;
                                    matched_titles.push(format!("{} (AI Switch -> {})", work.title, analysis.canonical_title));
                                    ai_matched = true;
                                }
                            }
                        }
                    }

                    // 4.3 Try searching Steam for canonical title from AI
                    if !ai_matched && !analysis.canonical_title.trim().is_empty() {
                        let encoded = Self::percent_encode(&analysis.canonical_title);
                        let search_url = format!("https://store.steampowered.com/api/storesearch/?term={encoded}&cc=US");
                        if let Ok(resp) = ureq::get(&search_url).timeout(std::time::Duration::from_secs(5)).call() {
                            if resp.status() == 200 {
                                if let Ok(val) = serde_json::from_reader::<_, serde_json::Value>(resp.into_reader()) {
                                    if let Some(items) = val.get("items").and_then(|v| v.as_array()) {
                                        if let Some(first) = items.first() {
                                            if let Some(app_id) = first.get("id").and_then(|v| v.as_u64()) {
                                                let capsule_url = format!("https://shared.steamstatic.com/store_item_assets/steam/apps/{app_id}/library_600x900.jpg");
                                                if self.download_and_set_work_cover(actor, &work.id, &capsule_url, false).is_ok() {
                                                    matched_count += 1;
                                                    matched_titles.push(format!("{} (AI -> {})", work.title, analysis.canonical_title));
                                                    ai_matched = true;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if ai_matched {
                continue;
            }

            failed_count += 1;
        }

        Ok(AutoMatchResult {
            matched_count,
            skipped_count,
            failed_count,
            matched_titles,
        })
    }

    pub fn remove_work_cover(
        &self,
        actor: &str,
        work_id: &str,
    ) -> LoomaResult<()> {
        self.update_work_cover_asset(actor, work_id, None)?;
        Ok(())
    }

    pub fn list_works(
        &self,
        work_type: Option<&str>,
        status: Option<&str>,
    ) -> LoomaResult<Vec<Entity>> {
        let all_entities = self.list_entities(&EntityFilter::default())?;
        let filtered = all_entities
            .into_iter()
            .filter(|e| {
                let meta = e.as_work_metadata();
                if meta.is_none() && e.entity_type != "work" && e.properties.get("domain").and_then(|v| v.as_str()) != Some("work") {
                    if WorkType::parse(&e.entity_type) == WorkType::Other && e.entity_type != "other" {
                        return false;
                    }
                }

                if let Some(wt) = work_type {
                    let parsed = WorkType::parse(wt);
                    let e_type = meta.as_ref().map(|m| m.work_type).unwrap_or_else(|| WorkType::parse(&e.entity_type));
                    if e_type != parsed && e.entity_type != wt {
                        return false;
                    }
                }

                if let Some(st) = status {
                    let parsed_st = RecordStatus::parse(st);
                    let e_st = meta.as_ref().map(|m| m.status).unwrap_or_else(|| {
                        e.properties.get("status")
                            .and_then(|v| v.as_str())
                            .map(RecordStatus::parse)
                            .unwrap_or(RecordStatus::Unknown)
                    });
                    if e_st != parsed_st {
                        return false;
                    }
                }

                true
            })
            .collect();
        Ok(filtered)
    }

    /// Search works with broad SQLite filtering followed by strict Rust semantic verification (title, description, original_title, aliases)
    pub fn search_works(
        &self,
        work_type: Option<&str>,
        status: Option<&str>,
        query: Option<&str>,
    ) -> LoomaResult<Vec<Entity>> {
        let mut filter = EntityFilter::default();
        if let Some(q) = query {
            let trimmed = q.trim();
            if !trimmed.is_empty() {
                filter.search_query = Some(trimmed.to_string());
            }
        }
        let candidates = self.list_entities(&filter)?;
        let filtered = candidates
            .into_iter()
            .filter(|e| {
                let meta = e.as_work_metadata();
                if meta.is_none() && e.entity_type != "work" && e.properties.get("domain").and_then(|v| v.as_str()) != Some("work") {
                    if WorkType::parse(&e.entity_type) == WorkType::Other && e.entity_type != "other" {
                        return false;
                    }
                }

                if let Some(wt) = work_type {
                    let parsed = WorkType::parse(wt);
                    let e_type = meta.as_ref().map(|m| m.work_type).unwrap_or_else(|| WorkType::parse(&e.entity_type));
                    if e_type != parsed && e.entity_type != wt {
                        return false;
                    }
                }

                if let Some(st) = status {
                    let parsed_st = RecordStatus::parse(st);
                    let e_st = meta.as_ref().map(|m| m.status).unwrap_or_else(|| {
                        e.properties.get("status")
                            .and_then(|v| v.as_str())
                            .map(RecordStatus::parse)
                            .unwrap_or(RecordStatus::Unknown)
                    });
                    if e_st != parsed_st {
                        return false;
                    }
                }

                if let Some(q) = query {
                    if !e.matches_query(q) {
                        return false;
                    }
                }

                true
            })
            .collect();
        Ok(filtered)
    }

    pub fn get_work_summary(&self, entity_id: &str) -> LoomaResult<Option<WorkSummary>> {
        let entity = match self.get_entity(entity_id)? {
            Some(e) => e,
            None => return Ok(None),
        };

        let work_metadata = entity.as_work_metadata();
        let relations = self.list_relations_for_item(entity_id)?;
        let external_references = self.list_external_references(Some(entity_id))?;

        let mut linked_assets = Vec::new();
        let mut linked_memories = Vec::new();

        for r in &relations {
            let other_id = if r.source_id == entity_id {
                &r.target_id
            } else {
                &r.source_id
            };
            let other_type = if r.source_id == entity_id {
                &r.target_type
            } else {
                &r.source_type
            };

            if other_type == "asset" {
                if let Ok(Some(mut asset)) = self.get_asset(other_id) {
                    if let Some(ref p) = asset.path {
                        let exists = Path::new(p).exists();
                        if !exists && asset.status == AssetStatus::Active {
                            asset.status = AssetStatus::Missing;
                            let _ = self.repo.upsert_asset(&asset);
                        } else if exists && asset.status == AssetStatus::Missing {
                            asset.status = AssetStatus::Active;
                            let _ = self.repo.upsert_asset(&asset);
                        }
                    }
                    linked_assets.push(asset);
                }
            } else if other_type == "memory" {
                if let Ok(Some(memory)) = self.get_memory(other_id) {
                    linked_memories.push(memory);
                }
            }
        }

        let mut linked_collections = Vec::new();
        if let Ok(collections) = self.list_collections() {
            for col in collections {
                if let Ok(items) = self.list_collection_items(&col.id) {
                    if items.iter().any(|item| item.item_id == entity_id) {
                        linked_collections.push(col);
                    }
                }
            }
        }

        let experiences = self.list_experiences_for_work(entity_id).unwrap_or_default();

        Ok(Some(WorkSummary {
            entity,
            work_metadata,
            relations,
            linked_assets,
            linked_memories,
            linked_collections,
            external_references,
            experiences,
        }))
    }

    pub fn link_work_asset(
        &self,
        actor: &str,
        work_id: &str,
        asset_id: &str,
        relation_type: Option<&str>,
    ) -> LoomaResult<Relation> {
        // Enforce Data Integrity Invariant: target asset and source work must exist
        if self.get_asset(asset_id)?.is_none() {
            return Err(LoomaError::NotFound(format!(
                "Cannot link nonexistent asset: {asset_id}"
            )));
        }
        if self.get_entity(work_id)?.is_none() {
            return Err(LoomaError::NotFound(format!(
                "Cannot link nonexistent work entity: {work_id}"
            )));
        }

        let r_type = relation_type.unwrap_or(relation_types::ATTACHES);
        let existing = self.list_relations_for_item(work_id)?;
        if let Some(r) = existing.into_iter().find(|r| {
            r.relation_type == r_type && (
                (r.source_id == work_id && r.target_id == asset_id) ||
                (r.source_id == asset_id && r.target_id == work_id)
            )
        }) {
            return Ok(r);
        }

        let rel = Relation {
            id: format!("rel_{}", &Uuid::new_v4().to_string()[..8]),
            source_id: work_id.to_string(),
            source_type: "entity".to_string(),
            relation_type: r_type.to_string(),
            target_id: asset_id.to_string(),
            target_type: "asset".to_string(),
            metadata: json!({ "domain": "personal_records" }),
            created_at: Utc::now(),
        };
        self.create_relation(actor, &rel)?;
        Ok(rel)
    }

    pub fn ensure_directory_asset(
        &self,
        actor: &str,
        raw_path: &str,
    ) -> LoomaResult<Asset> {
        let norm_path = normalize_directory_path(raw_path);
        if norm_path.is_empty() {
            return Err(LoomaError::Validation("Directory path cannot be empty".to_string()));
        }

        // Case A: Asset already exists in database
        if let Some(existing) = self.get_asset_by_path(&norm_path)? {
            return Ok(existing);
        }

        // Case C: Path does not exist on disk or is not a directory
        let p = Path::new(&norm_path);
        if !p.exists() {
            return Err(LoomaError::Validation(format!(
                "Directory path does not exist: {norm_path}"
            )));
        }
        if !p.is_dir() {
            return Err(LoomaError::Validation(format!(
                "Path is not a directory: {norm_path}"
            )));
        }

        // Case B: Create new Directory Asset
        let dir_name = p
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| norm_path.clone());

        let drive = if norm_path.len() >= 2 && &norm_path[1..2] == ":" {
            norm_path[0..2].to_uppercase()
        } else {
            "Local".to_string()
        };

        let now = Utc::now();
        let new_asset = Asset {
            id: Uuid::new_v4().to_string(),
            kind: AssetKind::Directory,
            source: AssetSource::Local,
            path: Some(norm_path.clone()),
            size: None,
            hash: None,
            mime_type: Some("inode/directory".to_string()),
            metadata: json!({
                "directory_name": dir_name,
                "drive": drive,
            }),
            status: AssetStatus::Active,
            created_at: now,
            modified_at: now,
            indexed_at: now,
        };

        self.upsert_asset(actor, &new_asset)?;
        Ok(new_asset)
    }

    /// Lists all directory assets attached to a Work entity via strict Work -> Directory Asset 'attaches' relations.
    pub fn list_work_directory_attachments(
        &self,
        work_id: &str,
    ) -> LoomaResult<Vec<(Asset, Relation)>> {
        let entity = self.get_entity(work_id)?
            .ok_or_else(|| LoomaError::NotFound(format!("Work entity '{work_id}' not found")))?;

        if entity.as_work_metadata().is_none() && entity.entity_type != "work" {
            return Err(LoomaError::Validation(format!(
                "Entity '{work_id}' is of type '{}', expected a work entity",
                entity.entity_type
            )));
        }

        let relations = self.list_relations_for_source(work_id)?;
        let mut results = Vec::new();

        for rel in relations {
            if rel.relation_type == relation_types::ATTACHES && rel.target_type == "asset" {
                if let Some(asset) = self.get_asset(&rel.target_id)? {
                    if asset.kind == AssetKind::Directory {
                        results.push((asset, rel));
                    }
                }
            }
        }

        Ok(results)
    }

    /// Read-only validation of all invariants and integrity rules for a given Work entity.
    pub fn validate_work_integrity(&self, work_id: &str) -> LoomaResult<IntegrityReport> {
        let mut violations = Vec::new();

        // 1. Work Entity existence and type check
        let entity = match self.get_entity(work_id)? {
            Some(e) => e,
            None => {
                return Ok(IntegrityReport::with_violations(
                    vec![IntegrityViolation::with_context(
                        IntegrityViolationCode::WorkNotFound,
                        format!("Work entity '{work_id}' not found"),
                        Some(work_id.to_string()),
                        None,
                        None,
                    )],
                    0,
                    0,
                    0,
                ));
            }
        };

        if entity.as_work_metadata().is_none() && entity.entity_type != "work" {
            return Ok(IntegrityReport::with_violations(
                vec![IntegrityViolation::with_context(
                    IntegrityViolationCode::NotAWork,
                    format!("Entity '{work_id}' has entity_type '{}', expected a work entity", entity.entity_type),
                    Some(work_id.to_string()),
                    None,
                    None,
                )],
                0,
                0,
                0,
            ));
        }

        // 2. External References Primary Invariant (at most 1 primary)
        let ext_refs = self.list_external_references(Some(work_id))?;
        let primary_count = ext_refs.iter().filter(|r| r.is_primary()).count();
        if primary_count > 1 {
            violations.push(IntegrityViolation::with_context(
                IntegrityViolationCode::DuplicatePrimary,
                format!(
                    "Work '{work_id}' has {primary_count} primary external references, maximum allowed is 1"
                ),
                Some(work_id.to_string()),
                None,
                None,
            ));
        }

        // 3. Relations and Attachment Invariants
        let relations = self.list_relations_for_item(work_id)?;
        let mut seen_identities = std::collections::HashSet::new();
        let mut seen_attached_assets = std::collections::HashSet::new();
        let mut checked_assets = std::collections::HashSet::new();

        for r in &relations {
            // 3a. Self-relation check
            if r.source_id == r.target_id {
                violations.push(IntegrityViolation::with_context(
                    IntegrityViolationCode::SelfRelation,
                    format!(
                        "Relation '{}' has identical source and target endpoint: '{}'",
                        r.id, r.source_id
                    ),
                    Some(work_id.to_string()),
                    Some(r.id.clone()),
                    None,
                ));
            }

            // 3b. Missing endpoints check
            if !self.check_item_exists(&r.source_id, &r.source_type)? {
                violations.push(IntegrityViolation::with_context(
                    IntegrityViolationCode::MissingRelationEndpoint,
                    format!(
                        "Relation '{}' source {} '{}' does not exist",
                        r.id, r.source_type, r.source_id
                    ),
                    Some(work_id.to_string()),
                    Some(r.id.clone()),
                    None,
                ));
            }
            if !self.check_item_exists(&r.target_id, &r.target_type)? {
                violations.push(IntegrityViolation::with_context(
                    IntegrityViolationCode::MissingRelationEndpoint,
                    format!(
                        "Relation '{}' target {} '{}' does not exist",
                        r.id, r.target_type, r.target_id
                    ),
                    Some(work_id.to_string()),
                    Some(r.id.clone()),
                    None,
                ));
            }

            // 3c. Duplicate relation check (Logical Identity: source_id, relation_type, target_id)
            let identity = (r.source_id.clone(), r.relation_type.clone(), r.target_id.clone());
            if !seen_identities.insert(identity.clone()) {
                violations.push(IntegrityViolation::with_context(
                    IntegrityViolationCode::DuplicateRelation,
                    format!(
                        "Duplicate relation detected: source='{}', type='{}', target='{}'",
                        identity.0, identity.1, identity.2
                    ),
                    Some(work_id.to_string()),
                    Some(r.id.clone()),
                    None,
                ));
            }

            // 3d. Attachment checks
            if r.relation_type == relation_types::ATTACHES {
                if r.target_id == work_id {
                    // Reverse attachment detected!
                    violations.push(IntegrityViolation::with_context(
                        IntegrityViolationCode::InvalidAttachmentDirection,
                        format!(
                            "Reverse attachment detected: asset '{}' -> work '{}'",
                            r.source_id, r.target_id
                        ),
                        Some(work_id.to_string()),
                        Some(r.id.clone()),
                        None,
                    ));
                } else if r.source_id == work_id {
                    // Target must be directory asset
                    checked_assets.insert(r.target_id.clone());
                    if let Ok(Some(asset)) = self.get_asset(&r.target_id) {
                        if asset.kind != AssetKind::Directory {
                            violations.push(IntegrityViolation::with_context(
                                IntegrityViolationCode::InvalidAttachmentKind,
                                format!(
                                    "Attachment target asset '{}' is of kind '{:?}', expected Directory",
                                    asset.id, asset.kind
                                ),
                                Some(work_id.to_string()),
                                Some(r.id.clone()),
                                None,
                            ));
                        }
                    }
                    if !seen_attached_assets.insert(r.target_id.clone()) {
                        violations.push(IntegrityViolation::with_context(
                            IntegrityViolationCode::DuplicateRelation,
                            format!(
                                "Work '{work_id}' has duplicate attachment to directory asset '{}'",
                                r.target_id
                            ),
                            Some(work_id.to_string()),
                            Some(r.id.clone()),
                            None,
                        ));
                    }
                }
            }
        }

        Ok(IntegrityReport::with_violations(
            violations,
            relations.len(),
            ext_refs.len(),
            checked_assets.len(),
        ))
    }

    /// Deterministic, read-only preview of repair actions for an integrity violation on a Work entity.
    /// Does NOT perform any writes to the database.
    pub fn preview_work_integrity_repair(&self, work_id: &str) -> LoomaResult<IntegrityRepairPlan> {
        let report = self.validate_work_integrity(work_id)?;
        let mut operations = Vec::new();
        let mut actions = Vec::new();
        let mut has_unrepairable = false;

        if report.valid {
            let plan_id = IntegrityRepairPlan::compute_plan_id(work_id, &report.violations, &[]);
            let _ = self.record_audit(
                "system",
                "integrity_repair_preview",
                "work",
                work_id,
                "success",
                json!({
                    "action": "integrity_repair_preview",
                    "plan_id": plan_id,
                    "write": false,
                    "repairable": false,
                    "operations_count": 0
                }),
            );
            return Ok(IntegrityRepairPlan {
                work_id: work_id.to_string(),
                plan_id,
                generated_at: Utc::now(),
                violations: Vec::new(),
                operations: Vec::new(),
                repairable: false,
                safe: true,
                actions: Vec::new(),
            });
        }

        // 1. Deterministic Primary Resolution (Earliest created_at, then lexicographically smallest id)
        if report.violations.iter().any(|v| v.code == IntegrityViolationCode::DuplicatePrimary) {
            let refs = self.list_external_references(Some(work_id))?;
            let mut primary_refs: Vec<_> = refs.into_iter().filter(|r| r.is_primary()).collect();
            if primary_refs.len() > 1 {
                primary_refs.sort_by(|a, b| a.created_at.cmp(&b.created_at).then_with(|| a.id.cmp(&b.id)));
                let winner = &primary_refs[0];
                for demotee in &primary_refs[1..] {
                    operations.push(IntegrityRepairOperation::RemoveDuplicatePrimary {
                        reference_id: demotee.id.clone(),
                    });
                    actions.push(IntegrityRepairAction {
                        kind: RepairActionKind::DemoteDuplicatePrimary,
                        description: format!("Demote redundant primary flag on reference '{}' (retaining '{}')", demotee.id, winner.id),
                        target_id: demotee.id.clone(),
                        details: json!({
                            "retained_primary_id": winner.id,
                            "demoted_reference_id": demotee.id,
                        }),
                    });
                }
            }
        }

        // 2. Deterministic Duplicate Relation Resolution (Earliest created_at, then lexicographically smallest id)
        if report.violations.iter().any(|v| v.code == IntegrityViolationCode::DuplicateRelation) {
            let source_relations = self.list_relations_for_source(work_id)?;
            let mut grouped: std::collections::HashMap<(String, String, String), Vec<Relation>> = std::collections::HashMap::new();
            for r in source_relations {
                grouped.entry((r.source_id.clone(), r.relation_type.clone(), r.target_id.clone()))
                    .or_default()
                    .push(r);
            }

            for (_key, mut group) in grouped {
                if group.len() > 1 {
                    group.sort_by(|a, b| a.created_at.cmp(&b.created_at).then_with(|| a.id.cmp(&b.id)));
                    let winner = &group[0];
                    for loser in &group[1..] {
                        operations.push(IntegrityRepairOperation::RemoveDuplicateRelation {
                            relation_id: loser.id.clone(),
                        });
                        actions.push(IntegrityRepairAction {
                            kind: RepairActionKind::RemoveDuplicateRelation,
                            description: format!("Remove redundant duplicate relation '{}' (retaining '{}')", loser.id, winner.id),
                            target_id: loser.id.clone(),
                            details: json!({
                                "retained_relation_id": winner.id,
                                "removed_relation_id": loser.id,
                            }),
                        });
                    }
                }
            }
        }

        // 3. Inspect other violation codes
        for v in &report.violations {
            match v.code {
                IntegrityViolationCode::InvalidAttachmentDirection | IntegrityViolationCode::InvalidAttachmentKind => {
                    if let Some(ref rel_id) = v.relation_id {
                        if !operations.iter().any(|op| match op {
                            IntegrityRepairOperation::RemoveInvalidAttachment { relation_id } => relation_id == rel_id,
                            _ => false,
                        }) {
                            operations.push(IntegrityRepairOperation::RemoveInvalidAttachment {
                                relation_id: rel_id.clone(),
                            });
                            actions.push(IntegrityRepairAction {
                                kind: RepairActionKind::DetachInvalidAttachment,
                                description: format!("Detach invalid attachment relation '{}'", rel_id),
                                target_id: rel_id.clone(),
                                details: json!({ "violation_message": v.message }),
                            });
                        }
                    }
                }
                IntegrityViolationCode::SelfRelation => {
                    if let Some(ref rel_id) = v.relation_id {
                        if !operations.iter().any(|op| match op {
                            IntegrityRepairOperation::RemoveSelfRelation { relation_id } => relation_id == rel_id,
                            _ => false,
                        }) {
                            operations.push(IntegrityRepairOperation::RemoveSelfRelation {
                                relation_id: rel_id.clone(),
                            });
                            actions.push(IntegrityRepairAction {
                                kind: RepairActionKind::RemoveSelfRelation,
                                description: format!("Remove self-referencing relation '{}'", rel_id),
                                target_id: rel_id.clone(),
                                details: json!({ "violation_message": v.message }),
                            });
                        }
                    }
                }
                IntegrityViolationCode::MissingRelationEndpoint => {
                    has_unrepairable = true;
                    let rel_id = v.relation_id.clone().unwrap_or_default();
                    actions.push(IntegrityRepairAction {
                        kind: RepairActionKind::InspectMissingEndpoint,
                        description: format!("Inspect dangling relation endpoint for relation '{}'", rel_id),
                        target_id: rel_id,
                        details: json!({ "violation_message": v.message }),
                    });
                }
                IntegrityViolationCode::WorkNotFound | IntegrityViolationCode::NotAWork => {
                    has_unrepairable = true;
                    actions.push(IntegrityRepairAction {
                        kind: RepairActionKind::InspectMissingEntity,
                        description: format!("Entity '{}' missing or not a work entity", work_id),
                        target_id: work_id.to_string(),
                        details: json!({ "violation_message": v.message }),
                    });
                }
                IntegrityViolationCode::DuplicatePrimary | IntegrityViolationCode::DuplicateRelation => {
                    // Already processed with deterministic ordering above
                }
                _ => {
                    has_unrepairable = true;
                }
            }
        }

        // Section 6: If unrepairable violations exist, do not generate automatic repair operations
        if has_unrepairable {
            operations.clear();
        }

        let repairable = !has_unrepairable && !operations.is_empty();
        let safe = !has_unrepairable;

        let plan_id = IntegrityRepairPlan::compute_plan_id(work_id, &report.violations, &operations);

        let _ = self.record_audit(
            "system",
            "integrity_repair_preview",
            "work",
            work_id,
            "success",
            json!({
                "action": "integrity_repair_preview",
                "plan_id": plan_id,
                "write": false,
                "repairable": repairable,
                "operations_count": operations.len()
            }),
        );

        Ok(IntegrityRepairPlan {
            work_id: work_id.to_string(),
            plan_id,
            generated_at: Utc::now(),
            violations: report.violations,
            operations,
            repairable,
            safe,
            actions,
        })
    }

    /// Executes an authorized, transactional repair plan for a Work entity.
    ///
    /// Guardrails:
    /// - Revalidates current plan identity (stale plan protection)
    /// - Executes in a single SQLite transaction via LoomaDb
    /// - Full rollback if any operation fails
    /// - Revalidates integrity after transaction commit
    /// - Audits all outcomes (preview, success, conflict, failure)
    pub fn repair_work_integrity(
        &self,
        actor: &str,
        work_id: &str,
        plan_id: &str,
    ) -> LoomaResult<IntegrityReport> {
        // 1. Preview current repair plan
        let current_plan = self.preview_work_integrity_repair(work_id)?;

        // 2. Validate plan staleness
        if plan_id != current_plan.plan_id {
            let msg = format!(
                "Integrity repair plan is stale. Expected plan: {}, Current plan: {}",
                plan_id, current_plan.plan_id
            );
            let _ = self.record_audit(
                actor,
                "integrity_repair",
                "work",
                work_id,
                "failure",
                json!({
                    "action": "integrity_repair",
                    "reason": "stale_plan",
                    "expected_plan": plan_id,
                    "current_plan": current_plan.plan_id
                }),
            );
            return Err(LoomaError::Conflict(msg));
        }

        // 3. Reject if unrepairable
        if !current_plan.repairable {
            return Err(LoomaError::Validation(format!(
                "Integrity repair plan '{}' contains unrepairable violations",
                plan_id
            )));
        }

        // 4. If no operations needed (already clean)
        if current_plan.operations.is_empty() {
            return self.validate_work_integrity(work_id);
        }

        // 4. Single-transaction repair execution
        if let Err(e) = self.repo.repair_work_integrity(work_id, &current_plan) {
            let _ = self.record_audit(
                actor,
                "integrity_repair",
                "work",
                work_id,
                "failure",
                json!({
                    "action": "integrity_repair",
                    "reason": "transaction_rollback",
                    "error": e.to_string(),
                    "plan_id": plan_id
                }),
            );
            return Err(e);
        }

        // 5. Mandatory post-repair integrity validation
        let post_report = self.validate_work_integrity(work_id)?;
        if !post_report.valid {
            let _ = self.record_audit(
                actor,
                "integrity_repair",
                "work",
                work_id,
                "failure",
                json!({
                    "action": "integrity_repair",
                    "reason": "repair_completed_but_integrity_failed",
                    "plan_id": plan_id,
                    "violations": post_report.violations
                }),
            );
            return Err(LoomaError::Integrity(format!(
                "Work integrity repair completed but post-repair integrity check failed for '{}'",
                work_id
            )));
        }

        // 6. Record successful write audit
        self.record_audit(
            actor,
            "integrity_repair",
            "work",
            work_id,
            "success",
            json!({
                "action": "integrity_repair",
                "plan_id": plan_id,
                "applied_operations": current_plan.operations.len(),
                "write": true
            }),
        )?;

        self.emit_event(
            actor,
            DomainEvent::EntityUpdated {
                id: work_id.to_string(),
            },
        );

        Ok(post_report)
    }

    pub fn link_work_directory(
        &self,
        actor: &str,
        work_id: &str,
        dir_path: &str,
    ) -> LoomaResult<(Asset, Relation)> {
        let asset = self.ensure_directory_asset(actor, dir_path)?;
        let rel = self.link_work_asset(actor, work_id, &asset.id, Some(relation_types::ATTACHES))?;
        Ok((asset, rel))
    }

    /// Atomically relocates a work's directory asset to a new filesystem path without modifying the Work identity.
    pub fn relocate_work_directory(
        &self,
        actor: &str,
        work_id: &str,
        old_asset_id: &str,
        new_dir_path: &str,
    ) -> LoomaResult<(Asset, Relation)> {
        let res: LoomaResult<(Asset, Relation)> = (|| {
            // Pre-validation 1: Work entity must exist
            if self.get_entity(work_id)?.is_none() {
                return Err(LoomaError::NotFound(format!("Work entity not found: {work_id}")));
            }

            // Pre-validation 2: Old asset must exist and be a directory
            let old_asset = self.get_asset(old_asset_id)?
                .ok_or_else(|| LoomaError::NotFound(format!("Old asset not found: {old_asset_id}")))?;
            if old_asset.kind != AssetKind::Directory {
                return Err(LoomaError::Validation(format!(
                    "Asset {old_asset_id} is not a directory asset"
                )));
            }

            // Pre-validation 3: Old asset must be linked to work strictly via ATTACHES relation
            // Direction must be source = work_id, target = old_asset_id
            let existing_source_relations = self.list_relations_for_source(work_id)?;
            let old_rel = existing_source_relations.iter().find(|r| {
                r.target_id == old_asset_id && r.relation_type == relation_types::ATTACHES
            }).ok_or_else(|| LoomaError::Validation(format!(
                "Old asset {old_asset_id} is not linked to work {work_id} via attaches relation"
            )))?;

            // Pre-validation 4: Validate new path exists on disk and is a directory
            let norm_path = normalize_directory_path(new_dir_path);
            if norm_path.is_empty() {
                return Err(LoomaError::Validation("Directory path cannot be empty".to_string()));
            }
            let p = Path::new(&norm_path);
            if !p.exists() {
                return Err(LoomaError::Validation(format!(
                    "Directory path does not exist: {norm_path}"
                )));
            }
            if !p.is_dir() {
                return Err(LoomaError::Validation(format!(
                    "Path is not a directory: {norm_path}"
                )));
            }

            // Case E: New path and old path are identical -> Idempotent success!
            if let Some(ref old_path) = old_asset.path {
                if normalize_directory_path(old_path) == norm_path {
                    return Ok((old_asset, old_rel.clone()));
                }
            }

            // Prepare new asset (reuse if path already in DB, or create new)
            let new_asset = if let Some(existing) = self.get_asset_by_path(&norm_path)? {
                if existing.kind != AssetKind::Directory {
                    return Err(LoomaError::Validation(format!(
                        "Target asset {} is not a directory asset", existing.id
                    )));
                }
                existing
            } else {
                let dir_name = p
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| norm_path.clone());

                let drive = if norm_path.len() >= 2 && &norm_path[1..2] == ":" {
                    norm_path[0..2].to_uppercase()
                } else {
                    "Local".to_string()
                };

                let now = Utc::now();
                Asset {
                    id: Uuid::new_v4().to_string(),
                    kind: AssetKind::Directory,
                    source: AssetSource::Local,
                    path: Some(norm_path.clone()),
                    size: None,
                    hash: None,
                    mime_type: Some("inode/directory".to_string()),
                    metadata: json!({
                        "directory_name": dir_name,
                        "drive": drive,
                    }),
                    status: AssetStatus::Active,
                    created_at: now,
                    modified_at: now,
                    indexed_at: now,
                }
            };

            // Prepare new relation
            let new_rel = Relation {
                id: format!("rel_{}", &Uuid::new_v4().to_string()[..8]),
                source_id: work_id.to_string(),
                source_type: "entity".to_string(),
                relation_type: relation_types::ATTACHES.to_string(),
                target_id: new_asset.id.clone(),
                target_type: "asset".to_string(),
                metadata: json!({ "domain": "personal_records" }),
                created_at: Utc::now(),
            };

            // Check if target is already attached to work
            let existing_target_rel = existing_source_relations.iter().find(|r| {
                r.target_id == new_asset.id && r.relation_type == relation_types::ATTACHES
            });

            // Atomically relocate in DB (single transaction: upsert new asset, delete old relation, insert new relation if needed)
            self.repo.relocate_work_directory(work_id, old_asset_id, &new_asset, &new_rel)?;

            let final_rel = if let Some(existing_r) = existing_target_rel {
                existing_r.clone()
            } else {
                new_rel.clone()
            };

            Ok((new_asset, final_rel))
        })();

        match res {
            Ok((new_asset, final_rel)) => {
                self.record_audit(
                    actor,
                    "work.relocate_directory",
                    "entity",
                    work_id,
                    "success",
                    json!({
                        "old_asset_id": old_asset_id,
                        "new_asset_id": new_asset.id,
                        "new_path": new_asset.path,
                    }),
                )?;

                self.emit_event(
                    actor,
                    DomainEvent::RelationCreated {
                        id: final_rel.id.clone(),
                        source_id: work_id.to_string(),
                        target_id: new_asset.id.clone(),
                    },
                );

                Ok((new_asset, final_rel))
            }
            Err(e) => {
                let _ = self.record_audit(
                    actor,
                    "work.relocate_directory",
                    "entity",
                    work_id,
                    "failure",
                    json!({
                        "old_asset_id": old_asset_id,
                        "new_path": new_dir_path,
                        "error": e.to_string(),
                    }),
                );
                Err(e)
            }
        }
    }

    /// Update type-specific metadata overlay on a Work entity
    pub fn update_work_type_metadata(
        &self,
        actor: &str,
        entity_id: &str,
        meta: Option<TypeSpecificMetadata>,
    ) -> LoomaResult<Entity> {
        let mut entity = self.get_entity(entity_id)?
            .ok_or_else(|| LoomaError::NotFound(format!("Work entity not found: {entity_id}")))?;
        entity.set_type_metadata(meta.clone());
        self.update_entity(actor, &entity)?;
        self.record_audit(
            actor,
            "work.type_metadata_update",
            "entity",
            entity_id,
            "success",
            json!({ "type_metadata": meta }),
        )?;
        self.emit_event(
            actor,
            DomainEvent::EntityUpdated { id: entity_id.to_string() },
        );
        Ok(entity)
    }

    pub fn link_work_memory(
        &self,
        actor: &str,
        work_id: &str,
        memory_id: &str,
    ) -> LoomaResult<Relation> {
        let rel = Relation {
            id: format!("rel_{}", &Uuid::new_v4().to_string()[..8]),
            source_id: memory_id.to_string(),
            source_type: "memory".to_string(),
            relation_type: relation_types::REFERENCED_BY.to_string(),
            target_id: work_id.to_string(),
            target_type: "entity".to_string(),
            metadata: json!({ "domain": "personal_records" }),
            created_at: Utc::now(),
        };
        self.create_relation(actor, &rel)?;
        Ok(rel)
    }

    // --- Relations ---
    pub fn list_relations_for_item(&self, item_id: &str) -> LoomaResult<Vec<Relation>> {
        self.repo.list_relations_for_item(item_id)
    }

    pub fn list_relations_for_source(&self, source_id: &str) -> LoomaResult<Vec<Relation>> {
        self.repo.list_relations_for_source(source_id)
    }

    pub fn list_relations_for_target(&self, target_id: &str) -> LoomaResult<Vec<Relation>> {
        self.repo.list_relations_for_target(target_id)
    }

    fn check_item_exists(&self, item_id: &str, item_type: &str) -> LoomaResult<bool> {
        match item_type {
            "entity" => Ok(self.repo.get_entity_by_id(item_id)?.is_some()),
            "asset" => Ok(self.repo.get_asset_by_id(item_id)?.is_some()),
            "memory" => Ok(self.repo.get_memory_by_id(item_id)?.is_some()),
            _ => {
                if self.repo.get_entity_by_id(item_id)?.is_some() {
                    return Ok(true);
                }
                if self.repo.get_asset_by_id(item_id)?.is_some() {
                    return Ok(true);
                }
                if self.repo.get_memory_by_id(item_id)?.is_some() {
                    return Ok(true);
                }
                Ok(false)
            }
        }
    }

    pub fn create_relation(&self, actor: &str, relation: &Relation) -> LoomaResult<()> {
        let res: LoomaResult<()> = (|| {
            // P1 Integrity Check 1: Self relation is rejected
            if relation.source_id == relation.target_id {
                return Err(LoomaError::Validation(format!(
                    "Self-relation is not allowed: source_id and target_id are both '{}'",
                    relation.source_id
                )));
            }

            // P1 Integrity Check 2: Relation source must exist
            if !self.check_item_exists(&relation.source_id, &relation.source_type)? {
                return Err(LoomaError::NotFound(format!(
                    "Relation source {} '{}' not found",
                    relation.source_type, relation.source_id
                )));
            }

            // P1 Integrity Check 3: Relation target must exist
            if !self.check_item_exists(&relation.target_id, &relation.target_type)? {
                return Err(LoomaError::NotFound(format!(
                    "Relation target {} '{}' not found",
                    relation.target_type, relation.target_id
                )));
            }

            // P1 Integrity Check 4: Duplicate relation protection (idempotent success)
            let existing = self.repo.list_relations_for_source(&relation.source_id)?;
            if existing.iter().any(|r| r.relation_type == relation.relation_type && r.target_id == relation.target_id) {
                return Ok(());
            }

            self.repo.create_relation(relation)?;
            Ok(())
        })();

        match res {
            Ok(()) => {
                self.record_audit(
                    actor,
                    "relation.create",
                    "relation",
                    &relation.id,
                    "success",
                    json!({
                        "source_id": relation.source_id,
                        "target_id": relation.target_id,
                        "relation_type": relation.relation_type,
                    }),
                )?;
                self.emit_event(
                    actor,
                    DomainEvent::RelationCreated {
                        id: relation.id.clone(),
                        source_id: relation.source_id.clone(),
                        target_id: relation.target_id.clone(),
                    },
                );
                Ok(())
            }
            Err(e) => {
                let _ = self.record_audit(
                    actor,
                    "relation.create",
                    "relation",
                    &relation.id,
                    "failure",
                    json!({
                        "source_id": relation.source_id,
                        "target_id": relation.target_id,
                        "relation_type": relation.relation_type,
                        "error": e.to_string(),
                    }),
                );
                Err(e)
            }
        }
    }

    pub fn delete_relation(&self, actor: &str, id: &str) -> LoomaResult<bool> {
        let deleted = self.repo.delete_relation(id)?;
        if deleted {
            self.record_audit(actor, "relation.delete", "relation", id, "success", json!({}))?;
            self.emit_event(actor, DomainEvent::RelationDeleted { id: id.to_string() });
        }
        Ok(deleted)
    }

    pub fn delete_relation_between(&self, actor: &str, source_id: &str, target_id: &str) -> LoomaResult<bool> {
        let deleted = self.repo.delete_relation_between(source_id, target_id)?;
        if deleted {
            self.record_audit(
                actor,
                "relation.delete_between",
                "relation",
                &format!("{source_id}<->{target_id}"),
                "success",
                json!({ "source_id": source_id, "target_id": target_id }),
            )?;
            self.emit_event(actor, DomainEvent::RelationDeleted { id: format!("{source_id}<->{target_id}") });
        }
        Ok(deleted)
    }

    // --- Memories ---
    pub fn list_memories(&self, limit: usize, offset: usize) -> LoomaResult<Vec<Memory>> {
        self.repo.list_memories(limit, offset)
    }

    pub fn get_memory(&self, id: &str) -> LoomaResult<Option<Memory>> {
        self.repo.get_memory_by_id(id)
    }

    pub fn create_memory(&self, actor: &str, memory: &Memory) -> LoomaResult<()> {
        if memory.title.trim().is_empty() {
            return Err(LoomaError::Validation("Memory title cannot be empty".to_string()));
        }
        self.repo.create_memory(memory)?;
        self.record_audit(
            actor,
            "memory.create",
            "memory",
            &memory.id,
            "success",
            json!({ "title": memory.title }),
        )?;
        self.emit_event(
            actor,
            DomainEvent::MemoryCreated {
                id: memory.id.clone(),
                title: memory.title.clone(),
            },
        );
        Ok(())
    }

    pub fn update_memory(&self, actor: &str, memory: &Memory) -> LoomaResult<()> {
        if memory.title.trim().is_empty() {
            return Err(LoomaError::Validation("Memory title cannot be empty".to_string()));
        }
        self.repo.update_memory(memory)?;
        self.record_audit(
            actor,
            "memory.update",
            "memory",
            &memory.id,
            "success",
            json!({ "title": memory.title }),
        )?;
        self.emit_event(
            actor,
            DomainEvent::MemoryUpdated {
                id: memory.id.clone(),
            },
        );
        Ok(())
    }

    pub fn delete_memory(&self, actor: &str, id: &str) -> LoomaResult<bool> {
        let deleted = self.repo.delete_memory(id)?;
        if deleted {
            self.record_audit(actor, "memory.delete", "memory", id, "success", json!({}))?;
            self.emit_event(actor, DomainEvent::MemoryDeleted { id: id.to_string() });
        }
        Ok(deleted)
    }

    // --- Collections ---
    pub fn list_collections(&self) -> LoomaResult<Vec<Collection>> {
        self.repo.list_collections()
    }

    pub fn get_collection(&self, id: &str) -> LoomaResult<Option<Collection>> {
        self.repo.get_collection_by_id(id)
    }

    pub fn create_collection(&self, actor: &str, collection: &Collection) -> LoomaResult<()> {
        if collection.title.trim().is_empty() {
            return Err(LoomaError::Validation("Collection title cannot be empty".to_string()));
        }
        self.repo.create_collection(collection)?;
        self.record_audit(
            actor,
            "collection.create",
            "collection",
            &collection.id,
            "success",
            json!({ "title": collection.title }),
        )?;
        Ok(())
    }

    pub fn delete_collection(&self, actor: &str, id: &str) -> LoomaResult<bool> {
        let deleted = self.repo.delete_collection(id)?;
        if deleted {
            self.record_audit(actor, "collection.delete", "collection", id, "success", json!({}))?;
        }
        Ok(deleted)
    }

    pub fn list_collection_items(&self, collection_id: &str) -> LoomaResult<Vec<CollectionItem>> {
        self.repo.list_items_for_collection(collection_id)
    }

    pub fn add_item_to_collection(&self, actor: &str, collection_id: &str, item_id: &str, item_type: &str) -> LoomaResult<()> {
        self.repo.add_item_to_collection(collection_id, item_id, item_type)?;
        self.record_audit(
            actor,
            "collection.add_item",
            "collection_item",
            item_id,
            "success",
            json!({ "collection_id": collection_id, "item_type": item_type }),
        )?;
        Ok(())
    }

    pub fn remove_item_from_collection(&self, actor: &str, collection_id: &str, item_id: &str) -> LoomaResult<bool> {
        let removed = self.repo.remove_item_from_collection(collection_id, item_id)?;
        if removed {
            self.record_audit(
                actor,
                "collection.remove_item",
                "collection_item",
                item_id,
                "success",
                json!({ "collection_id": collection_id }),
            )?;
        }
        Ok(removed)
    }

    // --- External References (P1) ---
    pub fn list_external_references(&self, entity_id: Option<&str>) -> LoomaResult<Vec<ExternalReference>> {
        self.repo.list_external_references(entity_id)
    }

    pub fn get_external_reference(&self, id: &str) -> LoomaResult<Option<ExternalReference>> {
        self.repo.get_external_reference_by_id(id)
    }

    pub fn create_external_reference(&self, actor: &str, reference: &ExternalReference) -> LoomaResult<()> {
        if reference.url.trim().is_empty() {
            return Err(LoomaError::Validation("External reference URL cannot be empty".to_string()));
        }
        self.repo.create_external_reference(reference)?;
        self.record_audit(
            actor,
            "external_reference.create",
            "external_reference",
            &reference.id,
            "success",
            json!({ "url": reference.url, "provider": reference.provider }),
        )?;
        self.emit_event(
            actor,
            DomainEvent::ExternalReferenceCreated {
                id: reference.id.clone(),
                url: reference.url.clone(),
            },
        );
        Ok(())
    }

    pub fn update_external_reference(&self, actor: &str, reference: &ExternalReference) -> LoomaResult<()> {
        if reference.url.trim().is_empty() {
            return Err(LoomaError::Validation("External reference URL cannot be empty".to_string()));
        }
        self.repo.update_external_reference(reference)?;
        self.record_audit(
            actor,
            "external_reference.update",
            "external_reference",
            &reference.id,
            "success",
            json!({ "url": reference.url, "provider": reference.provider }),
        )?;
        Ok(())
    }

    pub fn delete_external_reference(&self, actor: &str, id: &str) -> LoomaResult<bool> {
        let deleted = self.repo.delete_external_reference(id)?;
        if deleted {
            self.record_audit(actor, "external_reference.delete", "external_reference", id, "success", json!({}))?;
            self.emit_event(actor, DomainEvent::ExternalReferenceDeleted { id: id.to_string() });
        }
        Ok(deleted)
    }

    // --- Timeline ---
    pub fn query_timeline(&self, filter: &TimelineFilter) -> LoomaResult<Vec<TimelineItem>> {
        self.repo.query_timeline(filter)
    }

    // --- Backup & Doctor ---
    pub fn generate_manifest(&self) -> LoomaResult<VaultManifest> {
        self.repo.generate_manifest()
    }

    pub fn export_backup(&self, actor: &str, destination_dir: &Path) -> LoomaResult<BackupResult> {
        let res = self.repo.export_backup(destination_dir)?;
        self.record_audit(
            actor,
            "backup.export",
            "backup",
            &res.backup_path,
            "success",
            json!({ "duration_ms": res.duration_ms }),
        )?;
        self.emit_event(
            actor,
            DomainEvent::BackupCompleted {
                path: res.backup_path.clone(),
                duration_ms: res.duration_ms,
            },
        );
        Ok(res)
    }

    pub fn restore_backup(&self, actor: &str, backup_dir_or_file: &Path) -> LoomaResult<RestoreResult> {
        let res = self.repo.restore_backup(backup_dir_or_file)?;
        self.record_audit(
            actor,
            "backup.restore",
            "backup",
            &backup_dir_or_file.to_string_lossy(),
            "success",
            json!({ "restored_assets": res.restored_assets, "duration_ms": res.duration_ms }),
        )?;
        Ok(res)
    }

    pub fn doctor_inspect(&self) -> LoomaResult<VaultDoctorReport> {
        self.repo.doctor_inspect()
    }

    pub fn doctor_cleanup_missing(&self, actor: &str) -> LoomaResult<usize> {
        let count = self.repo.doctor_cleanup_missing()?;
        self.record_audit(
            actor,
            "doctor.cleanup_missing",
            "system",
            "vault",
            "success",
            json!({ "cleaned_count": count }),
        )?;
        Ok(count)
    }

    // --- Unified Directory Scan ---
    pub fn scan_directory<F>(
        &self,
        path: &Path,
        options: Option<ScanOptions>,
        progress_cb: Option<F>,
    ) -> LoomaResult<ScanSummary>
    where
        F: FnMut(ScanProgress) + Send + 'static,
    {
        let engine_opt = self.scan_engine.read().unwrap().clone();
        let engine = engine_opt.ok_or_else(|| {
            LoomaError::Internal("No ScanEngine registered with LoomaCore".to_string())
        })?;

        let norm_str = normalize_directory_path(&path.to_string_lossy());
        let norm_path = std::path::PathBuf::from(&norm_str);
        if !norm_path.exists() {
            return Err(LoomaError::Validation(format!(
                "Scan path does not exist: {}",
                norm_str
            )));
        }

        // 1. Resolve or inherit SourceRoot
        let all_roots = self.repo.list_source_roots()?;
        let exact_root = all_roots.iter().find(|r| {
            #[cfg(target_os = "windows")]
            { normalize_directory_path(&r.path).eq_ignore_ascii_case(&norm_str) }
            #[cfg(not(target_os = "windows"))]
            { normalize_directory_path(&r.path) == norm_str }
        }).cloned();

        let mut source_root = if let Some(r) = exact_root {
            r
        } else {
            // Check if norm_str is under an existing registered SourceRoot (deepest/longest ancestor)
            let ancestor_root = all_roots
                .iter()
                .filter(|r| is_path_under_root(&norm_str, &r.path))
                .max_by_key(|r| r.path.len())
                .cloned();

            if let Some(parent) = ancestor_root {
                parent
            } else {
                let label = norm_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(&norm_str)
                    .to_string();
                let new_root = SourceRoot {
                    id: format!("root_{}", &Uuid::new_v4().to_string()[..8]),
                    label,
                    path: norm_str.clone(),
                    source_kind: "local".to_string(),
                    enabled: true,
                    state: "online".to_string(),
                    last_successful_scan_at: None,
                    last_seen_at: Some(Utc::now()),
                    settings_json: serde_json::json!({}),
                };
                self.repo.create_source_root(&new_root)?
            }
        };

        // 2. Load and inherit tombstone suppressions for this SourceRoot and all ancestor roots
        let mut opts = options.unwrap_or_default();
        let mut candidate_roots = vec![source_root.clone()];
        for r in &all_roots {
            if is_path_under_root(&norm_str, &r.path) && r.id != source_root.id {
                candidate_roots.push(r.clone());
            }
        }

        for root in &candidate_roots {
            let suppressions = self.repo.list_suppressions(Some(&root.id))?;
            let root_norm = normalize_directory_path(&root.path);
            for sup in suppressions {
                let clean_rel = sup.relative_path.trim_start_matches('/').trim_start_matches('\\');
                let full_sup_path = format!("{}/{}", root_norm.trim_end_matches('/'), clean_rel);
                if let Some(rel_to_scan) = compute_relative_subpath(&norm_str, &full_sup_path) {
                    let target_rel = if rel_to_scan.is_empty() {
                        ".".to_string()
                    } else {
                        rel_to_scan
                    };
                    if !opts.suppressed_relative_paths.iter().any(|p| p.eq_ignore_ascii_case(&target_rel)) {
                        opts.suppressed_relative_paths.push(target_rel);
                    }
                }
            }
        }

        // 3. Delegate to ScanEngine
        let cb_box: Option<ScanProgressCallback> = progress_cb.map(|f| Box::new(f) as ScanProgressCallback);
        let summary = engine.scan_directory(&*self.repo, &norm_path, &opts, cb_box)?;

        // 4. Update SourceRoot scan stats & timestamps
        source_root.last_successful_scan_at = Some(Utc::now());
        source_root.last_seen_at = Some(Utc::now());
        source_root.state = "online".to_string();
        self.repo.update_source_root(&source_root)?;

        // 5. Ensure asset_lifecycle records are linked to this SourceRoot for indexed assets
        let assets_under_root = self.repo.list_assets_by_prefix(&norm_str)?;
        for asset in assets_under_root {
            if let Some(ref p) = asset.path {
                let norm_asset_p = normalize_directory_path(p);
                let rel = compute_relative_subpath(&source_root.path, &norm_asset_p)
                    .unwrap_or_else(|| norm_asset_p.clone());

                let existing_lc_opt = self.repo.get_asset_lifecycle(&asset.id)?;
                if let Some(mut existing_lc) = existing_lc_opt {
                    if existing_lc.root_id.is_none() || existing_lc.relative_path.is_none() {
                        existing_lc.root_id = Some(source_root.id.clone());
                        existing_lc.relative_path = Some(rel);
                    }
                    existing_lc.availability = "online".to_string();
                    existing_lc.last_verified_at = Some(Utc::now());
                    self.repo.set_asset_lifecycle(&existing_lc)?;
                } else {
                    let new_lc = AssetLifecycle {
                        asset_id: asset.id.clone(),
                        root_id: Some(source_root.id.clone()),
                        relative_path: Some(rel),
                        availability: "online".to_string(),
                        visibility: "active".to_string(),
                        tracking: "tracked".to_string(),
                        last_verified_at: Some(Utc::now()),
                        untracked_at: None,
                        untracked_reason: None,
                    };
                    self.repo.set_asset_lifecycle(&new_lc)?;
                }
            }
        }

        // 6. Record audit log
        self.record_audit(
            "system",
            "asset.scan",
            "source_root",
            &source_root.id,
            "success",
            serde_json::json!({
                "path": norm_str,
                "total": summary.total_scanned,
                "new": summary.new_assets,
                "modified": summary.modified_assets,
                "suppressed": summary.suppressed_assets,
                "duration_ms": summary.duration_ms,
            }),
        )?;

        Ok(summary)
    }

    // --- Experience / Footprints ---
    pub fn record_experience(
        &self,
        actor: &str,
        entry: &ExperienceEntry,
        links: &[ExperienceLink],
        request_id: Option<(&str, &str)>,
    ) -> LoomaResult<ExperienceEntry> {
        entry.validate()?;
        for link in links {
            link.validate()?;
        }

        // Validate subject exists
        if self.get_entity(&entry.subject_id)?.is_none() {
            return Err(LoomaError::NotFound(format!(
                "Experience subject entity '{}' not found",
                entry.subject_id
            )));
        }

        let created = self.repo.create_experience(entry, links, request_id)?;

        self.record_audit(
            actor,
            "experience.create",
            "experience",
            &created.id,
            "success",
            json!({
                "subject_id": created.subject_id,
                "event_type": created.event_type,
                "occurred_precision": created.occurred_precision,
                "occurred_value": created.occurred_value,
            }),
        )?;

        self.emit_event(
            actor,
            DomainEvent::ExperienceCreated {
                id: created.id.clone(),
                subject_id: created.subject_id.clone(),
                event_type: created.event_type.clone(),
            },
        );

        Ok(created)
    }

    pub fn complete_work_and_record_experience(
        &self,
        actor: &str,
        work_id: &str,
        mut experience: ExperienceEntry,
        links: &[ExperienceLink],
        request_id: Option<(&str, &str)>,
    ) -> LoomaResult<(Entity, ExperienceEntry)> {
        experience.subject_type = "entity".to_string();
        experience.subject_id = work_id.to_string();
        if experience.event_type.trim().is_empty() {
            experience.event_type = experience_events::COMPLETED.to_string();
        }
        experience.validate()?;

        for link in links {
            link.validate()?;
        }

        // Validate work entity exists and is a work
        let entity = self.get_entity(work_id)?
            .ok_or_else(|| LoomaError::NotFound(format!("Work entity '{work_id}' not found")))?;
        if entity.as_work_metadata().is_none() && entity.entity_type != "work" && entity.properties.get("domain").and_then(|v| v.as_str()) != Some("work") {
            return Err(LoomaError::Validation(format!(
                "Entity '{work_id}' is of type '{}', expected a work entity",
                entity.entity_type
            )));
        }

        let audit = AuditLog {
            id: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            actor: actor.to_string(),
            operation: "work.complete_with_experience".to_string(),
            target_type: "work".to_string(),
            target_id: work_id.to_string(),
            result: "success".to_string(),
            details: json!({
                "experience_id": experience.id,
                "event_type": experience.event_type,
                "occurred_precision": experience.occurred_precision,
                "occurred_value": experience.occurred_value,
            }),
        };

        let (updated_work, created_exp) = self.repo.complete_work_and_record_experience(
            work_id,
            &experience,
            links,
            request_id,
            &audit,
        )?;

        self.emit_event(
            actor,
            DomainEvent::EntityUpdated {
                id: work_id.to_string(),
            },
        );
        self.emit_event(
            actor,
            DomainEvent::ExperienceCreated {
                id: created_exp.id.clone(),
                subject_id: created_exp.subject_id.clone(),
                event_type: created_exp.event_type.clone(),
            },
        );

        Ok((updated_work, created_exp))
    }

    pub fn get_experience(&self, id: &str) -> LoomaResult<Option<ExperienceEntry>> {
        self.repo.get_experience_by_id(id)
    }

    pub fn list_experiences(&self, filter: &ExperienceFilter) -> LoomaResult<Vec<ExperienceEntry>> {
        self.repo.list_experiences(filter)
    }

    pub fn list_experiences_for_work(&self, work_id: &str) -> LoomaResult<Vec<ExperienceEntry>> {
        self.repo.list_experiences(&ExperienceFilter {
            subject_id: Some(work_id.to_string()),
            ..Default::default()
        })
    }

    pub fn update_experience(
        &self,
        actor: &str,
        entry: &ExperienceEntry,
    ) -> LoomaResult<()> {
        entry.validate()?;
        self.repo.update_experience(entry)?;
        self.record_audit(
            actor,
            "experience.update",
            "experience",
            &entry.id,
            "success",
            json!({ "title": entry.title, "event_type": entry.event_type }),
        )?;
        self.emit_event(actor, DomainEvent::ExperienceUpdated { id: entry.id.clone() });
        Ok(())
    }

    pub fn archive_experience(&self, actor: &str, id: &str) -> LoomaResult<bool> {
        let archived = self.repo.archive_experience(id)?;
        if archived {
            self.record_audit(
                actor,
                "experience.archive",
                "experience",
                id,
                "success",
                json!({}),
            )?;
            self.emit_event(actor, DomainEvent::ExperienceArchived { id: id.to_string() });
        }
        Ok(archived)
    }

    // --- Game Activity / Playtime Records ---
    pub fn list_game_activities_for_work(&self, work_id: &str) -> LoomaResult<Vec<GameActivityRecord>> {
        self.repo.list_game_activities_for_work(work_id)
    }

    pub fn get_game_activity(&self, provider: &str, source_key: &str, external_id: &str) -> LoomaResult<Option<GameActivityRecord>> {
        self.repo.get_game_activity(provider, source_key, external_id)
    }

    pub fn upsert_game_activity(&self, record: &GameActivityRecord) -> LoomaResult<()> {
        self.repo.upsert_game_activity(record)
    }

    pub fn delete_game_activity(&self, id: &str) -> LoomaResult<bool> {
        self.repo.delete_game_activity(id)
    }

    pub fn preview_switch_import(
        &self,
        json_content: &str,
        user_id: Option<&str>,
    ) -> LoomaResult<SwitchImportPreview> {
        if json_content.len() > 50 * 1024 * 1024 {
            return Err(LoomaError::Validation("JSON file is too large (exceeds 50MB limit)".to_string()));
        }

        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(json_content.as_bytes());
        let file_sha256 = format!("{:x}", hasher.finalize());

        let export: NxActivityLogExport = serde_json::from_str(json_content)
            .map_err(|e| LoomaError::Validation(format!("Invalid NX Activity Log JSON: {e}")))?;

        if export.users.is_empty() {
            return Err(LoomaError::Validation("No user profiles found in export".to_string()));
        }

        let profiles: Vec<SwitchProfilePreview> = export
            .users
            .iter()
            .map(|u| SwitchProfilePreview {
                user_id: u.id.trim().to_lowercase(),
                user_name: u.name.clone().unwrap_or_else(|| "Unknown".to_string()),
                title_count: u.titles.len(),
            })
            .collect();

        let selected_user = if let Some(uid) = user_id {
            let norm = uid.trim().to_lowercase();
            export
                .users
                .iter()
                .find(|u| u.id.trim().to_lowercase() == norm)
                .ok_or_else(|| LoomaError::NotFound(format!("Profile '{uid}' not found in export")))?
        } else {
            &export.users[0]
        };

        let selected_user_id = selected_user.id.trim().to_lowercase();
        let selected_user_name = selected_user.name.clone().unwrap_or_else(|| "Unknown".to_string());

        let existing_works = self.search_works(None, None, None).unwrap_or_default();

        let mut games = Vec::new();
        let mut new_games_count = 0;
        let mut matched_games_count = 0;
        let mut warning_count = 0;

        for t in &selected_user.titles {
            let external_id = t.id.trim().to_lowercase();
            if external_id.is_empty() {
                continue;
            }
            let title_name = t.name.clone().unwrap_or_else(|| external_id.clone());
            let summary = t.summary.as_ref();
            let playtime_seconds = summary.and_then(|s| s.playtime).unwrap_or(0);
            let playtime_hours = (playtime_seconds as f64) / 3600.0;
            let launches = summary.and_then(|s| s.launches);

            let first_played_at = summary
                .and_then(|s| s.first_played)
                .and_then(|ts| if ts > 0 { chrono::DateTime::from_timestamp(ts, 0) } else { None });
            let last_played_at = summary
                .and_then(|s| s.last_played)
                .and_then(|ts| if ts > 0 { chrono::DateTime::from_timestamp(ts, 0) } else { None });

            let existing_record = self.get_game_activity("switch_nx", &selected_user_id, &external_id)?;

            let mut is_time_decreased = false;
            let mut is_duplicate_snapshot = false;
            let mut warning = None;

            let (
                match_status,
                matched_work_id,
                matched_work_title,
                existing_playtime_seconds,
                existing_peak_seconds,
            ) = if let Some(existing) = existing_record {
                let work_title = self.get_entity(&existing.work_id).ok().flatten().map(|e| e.title);

                if playtime_seconds < existing.playtime_seconds {
                    is_time_decreased = true;
                    warning = Some(format!(
                        "游玩时长较上次减少 ({:.1}h -> {:.1}h)，历史峰值 {:.1}h",
                        existing.playtime_hours(),
                        playtime_hours,
                        (existing.peak_seen_seconds as f64) / 3600.0
                    ));
                } else if playtime_seconds == existing.playtime_seconds
                    && existing.last_import_sha256.as_deref() == Some(&file_sha256)
                {
                    is_duplicate_snapshot = true;
                    warning = Some("与先前导入的快照完全一致".to_string());
                } else if let (Some(exp_ts), Some(prev_exp)) = (export.export_timestamp, existing.source_exported_at) {
                    if exp_ts < prev_exp {
                        warning = Some("导出时间早于已保存记录的导出时间".to_string());
                    }
                }

                matched_games_count += 1;
                (
                    SwitchGameMatchStatus::ExactMapped,
                    Some(existing.work_id),
                    work_title,
                    Some(existing.playtime_seconds),
                    Some(existing.peak_seen_seconds),
                )
            } else {
                let norm_title = title_name.trim().to_lowercase();
                let candidate = existing_works.iter().find(|w| {
                    if w.title.trim().to_lowercase() == norm_title {
                        return true;
                    }
                    if let Some(meta) = w.as_work_metadata() {
                        if meta.aliases.iter().any(|a| a.trim().to_lowercase() == norm_title) {
                            return true;
                        }
                    }
                    false
                });

                if let Some(w) = candidate {
                    matched_games_count += 1;
                    (
                        SwitchGameMatchStatus::SuggestedMatch,
                        Some(w.id.clone()),
                        Some(w.title.clone()),
                        None,
                        None,
                    )
                } else {
                    new_games_count += 1;
                    (
                        SwitchGameMatchStatus::NewGame,
                        None,
                        None,
                        None,
                        None,
                    )
                }
            };

            if warning.is_some() {
                warning_count += 1;
            }

            games.push(SwitchGameCandidatePreview {
                external_id,
                title_name,
                playtime_seconds,
                playtime_hours,
                launches,
                first_played_at,
                last_played_at,
                match_status,
                matched_work_id,
                matched_work_title,
                existing_playtime_seconds,
                existing_peak_seconds,
                is_time_decreased,
                is_duplicate_snapshot,
                warning,
            });
        }

        let total_games = games.len();
        Ok(SwitchImportPreview {
            file_sha256,
            export_timestamp: export.export_timestamp,
            export_version: export.export_version,
            profiles,
            selected_user_id,
            selected_user_name,
            games,
            total_games,
            new_games_count,
            matched_games_count,
            warning_count,
        })
    }

    pub fn commit_switch_import(
        &self,
        actor: &str,
        decision: &SwitchImportDecision,
    ) -> LoomaResult<SwitchImportOutcome> {
        let outcome = self.repo.commit_switch_import(actor, decision)?;
        self.emit_event(
            actor,
            DomainEvent::GameActivityImported {
                count: outcome.imported_records,
                provider: "switch_nx".to_string(),
            },
        );
        Ok(outcome)
    }

    pub fn list_experience_links(&self, experience_id: &str) -> LoomaResult<Vec<ExperienceLink>> {
        self.repo.list_experience_links(experience_id)
    }

    pub fn add_experience_link(
        &self,
        actor: &str,
        link: &ExperienceLink,
    ) -> LoomaResult<()> {
        link.validate()?;
        self.repo.add_experience_link(link)?;
        self.record_audit(
            actor,
            "experience.link",
            "experience",
            &link.experience_id,
            "success",
            json!({ "linked_type": link.linked_type, "linked_id": link.linked_id }),
        )?;
        Ok(())
    }

    pub fn remove_experience_link(
        &self,
        actor: &str,
        experience_id: &str,
        linked_type: &str,
        linked_id: &str,
    ) -> LoomaResult<bool> {
        let removed = self.repo.remove_experience_link(experience_id, linked_type, linked_id)?;
        if removed {
            self.record_audit(
                actor,
                "experience.unlink",
                "experience",
                experience_id,
                "success",
                json!({ "linked_type": linked_type, "linked_id": linked_id }),
            )?;
        }
        Ok(removed)
    }

    // ==========================================
    // SourceRoot & Reference Lifecycle Facade
    // ==========================================

    pub fn create_source_root(
        &self,
        actor: &str,
        label: &str,
        path: &str,
    ) -> LoomaResult<SourceRoot> {
        let norm_path = normalize_directory_path(path);
        let root = SourceRoot::new(uuid::Uuid::new_v4().to_string(), label, norm_path);
        let created = self.repo.create_source_root(&root)?;
        self.record_audit(
            actor,
            "sourceroot.create",
            "source_root",
            &created.id,
            "success",
            json!({ "label": created.label, "path": created.path }),
        )?;
        self.event_bus.publish(crate::events::EventEnvelope::new(
            actor,
            crate::events::DomainEvent::SourceRootCreated {
                id: created.id.clone(),
                path: created.path.clone(),
            },
        ));
        Ok(created)
    }

    pub fn get_source_root(&self, id: &str) -> LoomaResult<Option<SourceRoot>> {
        self.repo.get_source_root(id)
    }

    pub fn list_source_roots(&self) -> LoomaResult<Vec<SourceRoot>> {
        self.repo.list_source_roots()
    }

    pub fn rebind_source_root(
        &self,
        actor: &str,
        id: &str,
        new_path: &str,
    ) -> LoomaResult<SourceRoot> {
        let norm_path = normalize_directory_path(new_path);
        let existing = self
            .repo
            .get_source_root(id)?
            .ok_or_else(|| LoomaError::NotFound(format!("SourceRoot '{id}' not found")))?;
        let old_path = existing.path.clone();

        let rebound = self.repo.rebind_source_root(id, &norm_path)?;
        self.record_audit(
            actor,
            "sourceroot.rebind",
            "source_root",
            id,
            "success",
            json!({ "old_path": old_path, "new_path": norm_path }),
        )?;
        self.event_bus.publish(crate::events::EventEnvelope::new(
            actor,
            crate::events::DomainEvent::SourceRootRebound {
                id: id.to_string(),
                old_path,
                new_path: norm_path,
            },
        ));
        Ok(rebound)
    }

    pub fn inspect_source_root(&self, id: &str) -> LoomaResult<SourceRootInspection> {
        self.repo.inspect_source_root(id)
    }

    pub fn archive_asset(&self, actor: &str, asset_id: &str) -> LoomaResult<()> {
        self.repo.archive_asset(asset_id)?;
        self.record_audit(
            actor,
            "asset.archive",
            "asset",
            asset_id,
            "success",
            json!({}),
        )?;
        self.event_bus.publish(crate::events::EventEnvelope::new(
            actor,
            crate::events::DomainEvent::AssetArchived {
                id: asset_id.to_string(),
            },
        ));
        Ok(())
    }

    pub fn unarchive_asset(&self, actor: &str, asset_id: &str) -> LoomaResult<()> {
        self.repo.unarchive_asset(asset_id)?;
        self.record_audit(
            actor,
            "asset.unarchive",
            "asset",
            asset_id,
            "success",
            json!({}),
        )?;
        self.event_bus.publish(crate::events::EventEnvelope::new(
            actor,
            crate::events::DomainEvent::AssetUnarchived {
                id: asset_id.to_string(),
            },
        ));
        Ok(())
    }

    pub fn untrack_asset(&self, actor: &str, asset_id: &str, reason: Option<&str>) -> LoomaResult<()> {
        self.repo.untrack_asset(asset_id, reason)?;
        self.record_audit(
            actor,
            "asset.untrack",
            "asset",
            asset_id,
            "success",
            json!({ "reason": reason }),
        )?;
        self.event_bus.publish(crate::events::EventEnvelope::new(
            actor,
            crate::events::DomainEvent::AssetUntracked {
                id: asset_id.to_string(),
                reason: reason.map(|s| s.to_string()),
            },
        ));
        Ok(())
    }

    pub fn resume_tracking(&self, actor: &str, asset_id: &str) -> LoomaResult<()> {
        self.repo.resume_tracking(asset_id)?;
        self.record_audit(
            actor,
            "asset.resume_tracking",
            "asset",
            asset_id,
            "success",
            json!({}),
        )?;
        self.event_bus.publish(crate::events::EventEnvelope::new(
            actor,
            crate::events::DomainEvent::AssetTrackingResumed {
                id: asset_id.to_string(),
            },
        ));
        Ok(())
    }

    pub fn get_asset_lifecycle(&self, asset_id: &str) -> LoomaResult<Option<AssetLifecycle>> {
        self.repo.get_asset_lifecycle(asset_id)
    }

    pub fn list_suppressions(&self, root_id: Option<&str>) -> LoomaResult<Vec<AssetScanSuppression>> {
        self.repo.list_suppressions(root_id)
    }

    pub fn is_asset_suppressed(&self, root_id: &str, relative_path: &str) -> LoomaResult<bool> {
        self.repo.is_asset_suppressed(root_id, relative_path)
    }

    pub fn add_suppression(&self, root_id: &str, relative_path: &str, reason: Option<&str>) -> LoomaResult<()> {
        self.repo.add_suppression(root_id, relative_path, reason)
    }

    /// Retrieve or generate an on-demand, bounded WebP thumbnail for an image asset.
    /// Results are persisted in `.looma/cache/thumbnails/<asset_id>_w<max_dimension>.webp`.
    /// The generated thumbnail is written atomically via a tempfile to prevent corrupt cache on cancellation.
    pub fn get_asset_thumbnail(
        &self,
        asset_id: &str,
        max_dimension: u32,
    ) -> LoomaResult<Option<ThumbnailResult>> {
        let asset = match self.get_asset(asset_id)? {
            Some(a) => a,
            None => return Ok(None),
        };

        if asset.kind != AssetKind::Image {
            return Ok(None);
        }

        let source_path_str = match &asset.path {
            Some(p) => p,
            None => return Ok(None),
        };

        let vault_info = self.get_vault_info()?;
        let source_path_buf = if std::path::Path::new(source_path_str).is_absolute() && std::path::Path::new(source_path_str).exists() {
            std::path::PathBuf::from(source_path_str)
        } else {
            let candidate = vault_info.vault_path.join(source_path_str);
            if candidate.exists() {
                candidate
            } else if vault_info.vault_path.as_os_str() == ":memory:" {
                let fname = std::path::Path::new(source_path_str).file_name().unwrap_or_default();
                std::env::temp_dir().join("looma_test_media").join("covers").join(fname)
            } else {
                std::path::PathBuf::from(source_path_str)
            }
        };

        if !source_path_buf.exists() {
            return Ok(None);
        }
        let source_path = &source_path_buf;

        // Bounded safety: if file is larger than 100MB, avoid synchronous full-image memory spikes
        if let Ok(meta) = source_path.metadata() {
            if meta.len() > 100 * 1024 * 1024 {
                return Err(LoomaError::Validation(format!(
                    "Image file size ({} MB) exceeds safety threshold of 100MB for thumbnail generation",
                    meta.len() / (1024 * 1024)
                )));
            }
        }

        let vault_info = self.get_vault_info()?;
        let cache_dir = if vault_info.vault_path.as_os_str() == ":memory:" {
            std::env::temp_dir().join("looma_test_cache").join("thumbnails")
        } else {
            vault_info.vault_path.join(".looma").join("cache").join("thumbnails")
        };
        std::fs::create_dir_all(&cache_dir)?;

        let cache_file_name = format!("{}_w{}.webp", asset.id, max_dimension);
        let cache_path = cache_dir.join(cache_file_name);

        let mut needs_generation = true;
        if cache_path.exists() {
            if let (Ok(meta_src), Ok(meta_cache)) = (source_path.metadata(), cache_path.metadata()) {
                if let (Ok(mtime_src), Ok(mtime_cache)) = (meta_src.modified(), meta_cache.modified()) {
                    if mtime_cache >= mtime_src && meta_cache.len() > 0 {
                        needs_generation = false;
                    }
                }
            }
        }

        let is_newly_generated = needs_generation;
        if needs_generation {
            let _guard = ThumbnailThrottleGuard::acquire();

            let reader = image::ImageReader::open(source_path)
                .map_err(|e| LoomaError::Io(std::io::Error::new(std::io::ErrorKind::Other, format!("Failed to open image {}: {}", source_path.display(), e))))?
                .with_guessed_format()
                .map_err(|e| LoomaError::Io(std::io::Error::new(std::io::ErrorKind::Other, format!("Failed to guess format: {}", e))))?;

            // Bounded safety: Check header dimensions before full decompression
            if let Ok((width, height)) = reader.into_dimensions() {
                let megapixels = (width as u64) * (height as u64);
                if megapixels > 64_000_000 {
                    return Err(LoomaError::Validation(format!(
                        "Image resolution ({}x{} = {} MP) exceeds safety threshold of 64 MP",
                        width, height, megapixels / 1_000_000
                    )));
                }
            }

            let reader = image::ImageReader::open(source_path)
                .map_err(|e| LoomaError::Io(std::io::Error::new(std::io::ErrorKind::Other, format!("Failed to open image {}: {}", source_path.display(), e))))?
                .with_guessed_format()
                .map_err(|e| LoomaError::Io(std::io::Error::new(std::io::ErrorKind::Other, format!("Failed to guess format: {}", e))))?;

            let img = reader.decode()
                .map_err(|e| LoomaError::Validation(format!("Failed to decode image: {}", e)))?;

            let thumb = img.thumbnail(max_dimension, max_dimension);

            let tmp_path = cache_dir.join(format!("{}.tmp", uuid::Uuid::new_v4()));
            thumb.save_with_format(&tmp_path, image::ImageFormat::WebP)
                .map_err(|e| LoomaError::Io(std::io::Error::new(std::io::ErrorKind::Other, format!("Failed to encode/save webp thumbnail: {}", e))))?;

            let _ = std::fs::remove_file(&cache_path);
            std::fs::rename(&tmp_path, &cache_path)
                .map_err(|e| LoomaError::Io(std::io::Error::new(std::io::ErrorKind::Other, format!("Failed to finalize thumbnail file: {}", e))))?;
        }

        let thumb_bytes = std::fs::read(&cache_path)?;
        let b64 = base64::engine::general_purpose::STANDARD.encode(&thumb_bytes);
        let data_url = format!("data:image/webp;base64,{}", b64);

        Ok(Some(ThumbnailResult {
            asset_id: asset_id.to_string(),
            cache_path: cache_path.to_string_lossy().to_string(),
            data_url,
            is_newly_generated,
        }))
    }

    /// Return statistics on thumbnail cache usage (file count, total size, directory).
    pub fn get_thumbnail_cache_stats(&self) -> LoomaResult<ThumbnailCacheStats> {
        let vault_info = self.get_vault_info()?;
        let cache_dir = if vault_info.vault_path.as_os_str() == ":memory:" {
            std::env::temp_dir().join("looma_test_cache").join("thumbnails")
        } else {
            vault_info.vault_path.join(".looma").join("cache").join("thumbnails")
        };

        if !cache_dir.exists() {
            return Ok(ThumbnailCacheStats {
                file_count: 0,
                total_bytes: 0,
                cache_dir: cache_dir.to_string_lossy().to_string(),
            });
        }

        let mut file_count = 0;
        let mut total_bytes = 0;
        if let Ok(entries) = std::fs::read_dir(&cache_dir) {
            for entry in entries.flatten() {
                if let Ok(meta) = entry.metadata() {
                    if meta.is_file() {
                        file_count += 1;
                        total_bytes += meta.len();
                    }
                }
            }
        }

        Ok(ThumbnailCacheStats {
            file_count,
            total_bytes,
            cache_dir: cache_dir.to_string_lossy().to_string(),
        })
    }

    /// Clear all cached thumbnail files and return count of removed files.
    pub fn clear_thumbnail_cache(&self) -> LoomaResult<usize> {
        let vault_info = self.get_vault_info()?;
        let cache_dir = if vault_info.vault_path.as_os_str() == ":memory:" {
            std::env::temp_dir().join("looma_test_cache").join("thumbnails")
        } else {
            vault_info.vault_path.join(".looma").join("cache").join("thumbnails")
        };

        if !cache_dir.exists() {
            return Ok(0);
        }

        let mut removed = 0;
        if let Ok(entries) = std::fs::read_dir(&cache_dir) {
            for entry in entries.flatten() {
                if let Ok(meta) = entry.metadata() {
                    if meta.is_file() {
                        if std::fs::remove_file(entry.path()).is_ok() {
                            removed += 1;
                        }
                    }
                }
            }
        }

        Ok(removed)
    }
}

impl AssetService for LoomaCore {
    fn list_assets(&self, limit: usize, offset: usize) -> LoomaResult<Vec<Asset>> {
        self.repo.list_assets(limit, offset)
    }

    fn query_assets(&self, filter: &AssetFilter) -> LoomaResult<Vec<Asset>> {
        self.repo.query_assets(filter)
    }

    fn get_asset_by_id(&self, id: &str) -> LoomaResult<Option<Asset>> {
        self.repo.get_asset_by_id(id)
    }

    fn get_asset_by_path(&self, path: &str) -> LoomaResult<Option<Asset>> {
        self.repo.get_asset_by_path(path)
    }

    fn list_assets_by_prefix(&self, prefix: &str) -> LoomaResult<Vec<Asset>> {
        self.repo.list_assets_by_prefix(prefix)
    }

    fn upsert_asset(&self, asset: &Asset) -> LoomaResult<()> {
        self.repo.upsert_asset(asset)
    }

    fn delete_asset(&self, id: &str) -> LoomaResult<bool> {
        self.repo.delete_asset(id)
    }

    fn count_assets(&self) -> LoomaResult<u64> {
        self.repo.count_assets()
    }

    fn count_assets_with_filter(&self, filter: &AssetFilter) -> LoomaResult<u64> {
        self.repo.count_assets_with_filter(filter)
    }
}

impl SearchService for LoomaCore {
    fn search_vault(&self, query: &str, category: Option<&str>, limit: usize) -> LoomaResult<VaultSearchResults> {
        self.repo.search_vault(query, category, limit)
    }
}
