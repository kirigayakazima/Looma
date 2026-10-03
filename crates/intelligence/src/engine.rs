use std::collections::{HashMap, HashSet};
use std::path::Path;

use chrono::Utc;
use serde_json::json;
use uuid::Uuid;

use looma_core::{
    Asset, AssetKind, AssetService, Collection, CollectionService,
    EntityFilter, EntityService, LoomaResult, Relation, RelationService,
};
use looma_database::LoomaDb;

use crate::models::{SmartInsightsReport, Suggestion, SuggestionType};

pub struct IntelligenceEngine;

impl IntelligenceEngine {
    /// Generate automatic relation suggestions between assets and entities based on
    /// directory names, file names, type affinities, and keyword occurrences.
    pub fn suggest_relations(db: &LoomaDb, min_confidence: f32) -> LoomaResult<Vec<Suggestion>> {
        let entities = db.list_entities(&EntityFilter::default())?;
        if entities.is_empty() {
            return Ok(Vec::new());
        }

        let assets = db.list_assets(1000, 0)?;
        if assets.is_empty() {
            return Ok(Vec::new());
        }

        let mut suggestions = Vec::new();

        for entity in &entities {
            let entity_title_clean = entity.title.trim().to_lowercase();
            if entity_title_clean.is_empty() {
                continue;
            }

            // Gather already linked assets for this entity to prevent duplicate suggestions
            let existing_target_relations = db.list_relations_for_target(&entity.id)?;
            let existing_source_relations = db.list_relations_for_source(&entity.id)?;

            let mut linked_asset_ids = HashSet::new();
            for r in existing_target_relations.iter().chain(existing_source_relations.iter()) {
                if r.source_type == "asset" {
                    linked_asset_ids.insert(r.source_id.clone());
                }
                if r.target_type == "asset" {
                    linked_asset_ids.insert(r.target_id.clone());
                }
            }

            for asset in &assets {
                if linked_asset_ids.contains(&asset.id) {
                    continue;
                }

                let asset_path_str = match &asset.path {
                    Some(p) => p,
                    None => continue,
                };

                let path_obj = Path::new(asset_path_str);
                let file_name = path_obj
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_string();
                let file_stem = path_obj
                    .file_stem()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_lowercase();
                let parent_dir = path_obj
                    .parent()
                    .and_then(|p| p.file_name())
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_lowercase();
                let full_path_lower = asset_path_str.to_lowercase();

                let mut confidence = 0.0f32;
                let mut reason = String::new();

                // 1. Direct file stem match / prefix match
                if file_stem == entity_title_clean {
                    confidence = 0.98;
                    reason = format!("文件名「{}」与实体名称完全一致", file_name);
                } else if file_stem.starts_with(&entity_title_clean) || file_stem.contains(&entity_title_clean) {
                    confidence = 0.90;
                    reason = format!("文件名包含实体名称「{}」", entity.title);
                } else if parent_dir == entity_title_clean || parent_dir.contains(&entity_title_clean) {
                    // 2. Parent directory match
                    confidence = 0.88;
                    reason = format!("所在文件夹「{}」与实体名称匹配", parent_dir);
                } else if full_path_lower.contains(&entity_title_clean) {
                    // 3. Substring match in full path hierarchy
                    confidence = 0.75;
                    reason = format!("文件路径层级中包含关键词「{}」", entity.title);
                }

                // 4. Affinity boost based on entity type and asset kind
                if confidence > 0.0 {
                    let et_lower = entity.entity_type.to_lowercase();
                    match asset.kind {
                        AssetKind::Image => {
                            if et_lower.contains("wallpaper")
                                || et_lower.contains("photo")
                                || et_lower.contains("picture")
                                || et_lower.contains("壁纸")
                                || et_lower.contains("图片")
                            {
                                confidence = (confidence + 0.05).min(1.0);
                            }
                        }
                        AssetKind::Video => {
                            if et_lower.contains("anime")
                                || et_lower.contains("movie")
                                || et_lower.contains("film")
                                || et_lower.contains("video")
                                || et_lower.contains("动画")
                                || et_lower.contains("动漫")
                            {
                                confidence = (confidence + 0.05).min(1.0);
                            }
                        }
                        AssetKind::Code => {
                            if et_lower.contains("project")
                                || et_lower.contains("repo")
                                || et_lower.contains("code")
                                || et_lower.contains("项目")
                            {
                                confidence = (confidence + 0.05).min(1.0);
                            }
                        }
                        _ => {}
                    }
                }

                if confidence >= min_confidence {
                    suggestions.push(Suggestion {
                        id: Uuid::new_v4().to_string(),
                        suggestion_type: SuggestionType::Relation,
                        title: format!("推荐将「{}」关联到实体「{}」", file_name, entity.title),
                        description: reason,
                        confidence,
                        source_id: asset.id.clone(),
                        source_name: file_name,
                        target_id: Some(entity.id.clone()),
                        target_name: Some(entity.title.clone()),
                        relation_type: Some("referenced_by".to_string()),
                        tags: vec![entity.entity_type.clone()],
                        action_payload: Some(json!({
                            "asset_id": asset.id,
                            "entity_id": entity.id,
                            "relation_type": "referenced_by",
                        })),
                    });
                }
            }
        }

        // Sort by confidence descending
        suggestions.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal));
        Ok(suggestions)
    }

    /// Suggest tags for assets based on their metadata and characteristics
    pub fn suggest_tags(db: &LoomaDb) -> LoomaResult<Vec<Suggestion>> {
        let assets = db.list_assets(500, 0)?;
        let mut suggestions = Vec::new();

        for asset in &assets {
            let asset_path_str = match &asset.path {
                Some(p) => p,
                None => continue,
            };
            let path_obj = Path::new(asset_path_str);
            let file_name = path_obj
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();

            let mut tags = Vec::new();

            // Analyze extension
            if let Some(ext) = path_obj.extension().and_then(|e| e.to_str()) {
                let ext_lower = ext.to_lowercase();
                match ext_lower.as_str() {
                    "jpg" | "jpeg" | "png" | "webp" => tags.push("Image".to_string()),
                    "mp4" | "mkv" | "mov" => tags.push("Video".to_string()),
                    "flac" | "wav" => tags.push("LosslessAudio".to_string()),
                    "mp3" | "m4a" => tags.push("Audio".to_string()),
                    "rs" => tags.push("Rust".to_string()),
                    "ts" | "tsx" => tags.push("TypeScript".to_string()),
                    "py" => tags.push("Python".to_string()),
                    "md" => tags.push("Markdown".to_string()),
                    "pdf" => tags.push("Document".to_string()),
                    _ => {}
                }
            }

            // Analyze path patterns
            let path_lower = asset_path_str.to_lowercase();
            if path_lower.contains("wallpaper") || path_lower.contains("壁纸") {
                tags.push("Wallpaper".to_string());
            }
            if path_lower.contains("screenshot") || path_lower.contains("截图") {
                tags.push("Screenshot".to_string());
            }
            if path_lower.contains("avatar") || path_lower.contains("头像") {
                tags.push("Avatar".to_string());
            }

            // Size heuristics
            if let Some(size) = asset.size {
                if size > 100 * 1024 * 1024 {
                    tags.push("LargeMedia".to_string());
                }
            }

            if !tags.is_empty() {
                suggestions.push(Suggestion {
                    id: Uuid::new_v4().to_string(),
                    suggestion_type: SuggestionType::Tag,
                    title: format!("为「{}」推荐智能标签", file_name),
                    description: format!("根据文件扩展名与路径特征推导出 {} 个建议标签", tags.len()),
                    confidence: 0.85,
                    source_id: asset.id.clone(),
                    source_name: file_name,
                    target_id: None,
                    target_name: None,
                    relation_type: None,
                    tags,
                    action_payload: Some(json!({ "asset_id": asset.id })),
                });
            }
        }

        Ok(suggestions)
    }

    /// Suggest clustering uncollected assets into collections based on parent directories
    pub fn suggest_clusters(db: &LoomaDb) -> LoomaResult<Vec<Suggestion>> {
        let assets = db.list_assets(1000, 0)?;
        let collections = db.list_collections()?;

        let mut existing_col_titles = HashSet::new();
        for c in &collections {
            existing_col_titles.insert(c.title.to_lowercase());
        }

        // Group assets by parent directory
        let mut dir_map: HashMap<String, Vec<&Asset>> = HashMap::new();

        for asset in &assets {
            if let Some(p) = &asset.path {
                let path_obj = Path::new(p);
                if let Some(parent) = path_obj.parent() {
                    let parent_name = parent
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("")
                        .to_string();
                    if !parent_name.is_empty() {
                        dir_map.entry(parent_name).or_default().push(asset);
                    }
                }
            }
        }

        let mut suggestions = Vec::new();

        for (dir_name, group) in dir_map {
            // If at least 2 assets and no collection exists with this title
            if group.len() >= 2 && !existing_col_titles.contains(&dir_name.to_lowercase()) {
                let asset_ids: Vec<String> = group.iter().map(|a| a.id.clone()).collect();
                suggestions.push(Suggestion {
                    id: Uuid::new_v4().to_string(),
                    suggestion_type: SuggestionType::Cluster,
                    title: format!("发现「{}」聚合目录，建议创建新合集", dir_name),
                    description: format!("检测到该目录下有 {} 个数字资产，可聚类为一个专属收藏合集", group.len()),
                    confidence: 0.90,
                    source_id: dir_name.clone(),
                    source_name: dir_name.clone(),
                    target_id: None,
                    target_name: Some(dir_name.clone()),
                    relation_type: None,
                    tags: vec!["Collection".to_string(), format!("{} 资产", group.len())],
                    action_payload: Some(json!({
                        "collection_title": dir_name,
                        "asset_ids": asset_ids,
                    })),
                });
            }
        }

        Ok(suggestions)
    }

    /// Generate unified smart insights report
    pub fn generate_insights(db: &LoomaDb) -> LoomaResult<SmartInsightsReport> {
        let relation_suggestions = Self::suggest_relations(db, 0.70)?;
        let tag_suggestions = Self::suggest_tags(db)?;
        let cluster_suggestions = Self::suggest_clusters(db)?;

        let total_suggestions =
            relation_suggestions.len() + tag_suggestions.len() + cluster_suggestions.len();

        Ok(SmartInsightsReport {
            total_suggestions,
            relation_suggestions,
            tag_suggestions,
            cluster_suggestions,
        })
    }

    /// Apply a specific suggestion by its payload
    pub fn apply_suggestion(
        db: &LoomaDb,
        suggestion: &Suggestion,
    ) -> LoomaResult<bool> {
        match suggestion.suggestion_type {
            SuggestionType::Relation => {
                if let (Some(target_id), Some(relation_type)) =
                    (&suggestion.target_id, &suggestion.relation_type)
                {
                    let relation = Relation {
                        id: Uuid::new_v4().to_string(),
                        source_id: suggestion.source_id.clone(),
                        source_type: "asset".to_string(),
                        relation_type: relation_type.clone(),
                        target_id: target_id.clone(),
                        target_type: "entity".to_string(),
                        metadata: json!({
                            "auto_suggested": true,
                            "confidence": suggestion.confidence,
                            "reason": suggestion.description,
                        }),
                        created_at: Utc::now(),
                    };
                    db.create_relation(&relation)?;
                    return Ok(true);
                }
            }
            SuggestionType::Cluster => {
                if let Some(payload) = &suggestion.action_payload {
                    let col_title = payload
                        .get("collection_title")
                        .and_then(|v| v.as_str())
                        .unwrap_or(&suggestion.source_name);
                    let asset_ids = payload
                        .get("asset_ids")
                        .and_then(|v| v.as_array())
                        .cloned()
                        .unwrap_or_default();

                    let col = Collection {
                        id: Uuid::new_v4().to_string(),
                        title: col_title.to_string(),
                        description: Some(format!("由智能聚类引擎自 {} 个资产自动创建", asset_ids.len())),
                        query: None,
                        metadata: json!({ "auto_clustered": true }),
                        created_at: Utc::now(),
                        updated_at: Utc::now(),
                    };
                    db.create_collection(&col)?;

                    for (_idx, aid_val) in asset_ids.iter().enumerate() {
                        if let Some(aid) = aid_val.as_str() {
                            let _ = db.add_item_to_collection(&col.id, aid, "asset");
                        }
                    }
                    return Ok(true);
                }
            }
            SuggestionType::Tag => {
                // Future: store suggested tags into asset metadata
                return Ok(true);
            }
        }

        Ok(false)
    }
}
