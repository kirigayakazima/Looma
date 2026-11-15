use std::sync::Arc;
use looma_core::models::*;
use looma_core::services::{AssetService, EntityService, RelationService};
use looma_core::LoomaCore;
use looma_database::LoomaDb;
use looma_intelligence::{IntelligenceEngine, Suggestion, SuggestionType};
use serde_json::json;

#[test]
fn test_batch_apply_accuracy_and_tag_rejection() {
    let db = Arc::new(LoomaDb::in_memory().expect("in-memory db init failed"));
    let core = LoomaCore::new(db.clone());

    // Setup base data: 1 entity and 1 asset
    let entity = Entity {
        id: "elden-ring".to_string(),
        entity_type: "game".to_string(),
        title: "Elden Ring".to_string(),
        description: None,
        properties: json!({}),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    db.create_entity(&entity).expect("create entity");

    let asset = Asset {
        id: "asset-wallpaper-1".to_string(),
        kind: AssetKind::Image,
        source: AssetSource::Local,
        path: Some("D:/Games/EldenRing/wallpaper.png".to_string()),
        size: Some(1024),
        hash: Some("hash123".to_string()),
        mime_type: Some("image/png".to_string()),
        metadata: json!({}),
        status: AssetStatus::Active,
        created_at: chrono::Utc::now(),
        modified_at: chrono::Utc::now(),
        indexed_at: chrono::Utc::now(),
    };
    db.upsert_asset(&asset).expect("upsert asset");

    // 1. Single apply_suggestion on Tag must fail with Validation error
    let tag_suggestion = Suggestion {
        id: "sug-tag-1".to_string(),
        suggestion_type: SuggestionType::Tag,
        title: "Auto-tag suggestion".to_string(),
        description: "Tagging asset with souls".to_string(),
        confidence: 0.95,
        source_id: asset.id.clone(),
        source_name: "test".to_string(),
        target_id: None,
        target_name: None,
        relation_type: None,
        tags: vec!["souls".to_string()],
        action_payload: None,
    };

    let tag_res = IntelligenceEngine::apply_suggestion(&core, "test", &tag_suggestion);
    assert!(
        tag_res.is_err(),
        "Tag suggestion must be rejected in v0.5 with Validation error"
    );
    let err_str = tag_res.unwrap_err().to_string();
    assert!(
        err_str.contains("Tag 建议自动写入功能尚未开放"),
        "Error message must clearly explain tag rejection: {}",
        err_str
    );

    // 2. Prepare a mixed batch of suggestions
    // a. Valid Relation suggestion
    let valid_relation = Suggestion {
        id: "sug-rel-valid".to_string(),
        suggestion_type: SuggestionType::Relation,
        title: "Link Elden Ring wallpaper".to_string(),
        description: "Auto link".to_string(),
        confidence: 0.92,
        source_id: asset.id.clone(),
        source_name: "wallpaper.png".to_string(),
        target_id: Some(entity.id.clone()),
        target_name: Some(entity.title.clone()),
        relation_type: Some("features".to_string()),
        tags: vec![],
        action_payload: None,
    };

    // b. Invalid Relation suggestion (target entity does not exist)
    let invalid_relation = Suggestion {
        id: "sug-rel-invalid".to_string(),
        suggestion_type: SuggestionType::Relation,
        title: "Link non-existent entity".to_string(),
        description: "Broken link".to_string(),
        confidence: 0.88,
        source_id: "non-existent-asset".to_string(),
        source_name: "missing.png".to_string(),
        target_id: Some("non-existent-entity".to_string()),
        target_name: Some("missing".to_string()),
        relation_type: Some("features".to_string()),
        tags: vec![],
        action_payload: None,
    };

    // c. Valid Cluster suggestion
    let valid_cluster = Suggestion {
        id: "sug-cluster-valid".to_string(),
        suggestion_type: SuggestionType::Cluster,
        title: "Group Souls Artwork".to_string(),
        description: "Create collection".to_string(),
        confidence: 0.86,
        source_id: "cluster-root".to_string(),
        source_name: "魂系壁纸集".to_string(),
        target_id: None,
        target_name: None,
        relation_type: None,
        tags: vec![],
        action_payload: Some(json!({
            "collection_title": "魂系壁纸集",
            "asset_ids": [asset.id.clone()]
        })),
    };

    // d. Tag suggestion (should be skipped honestly in batch)
    let batch_tag = tag_suggestion.clone();

    let batch = vec![valid_relation, invalid_relation, valid_cluster, batch_tag];

    // 3. Execute batch apply
    let result = IntelligenceEngine::apply_batch_suggestions(&core, "test", &batch);

    // 4. Verify batch accounting accuracy
    assert_eq!(result.total_attempted, 4, "Total items must be 4");
    assert_eq!(result.success_count, 2, "Exactly 2 items must succeed (relation + cluster)");
    assert_eq!(result.failure_count, 1, "Exactly 1 item must fail (invalid relation)");
    assert_eq!(result.skipped_count, 1, "Exactly 1 item must be skipped (tag)");
    assert_eq!(result.errors.len(), 2, "Itemized errors must contain 2 entries (1 fail + 1 skip)");
    assert!(result.errors.iter().any(|e| e.contains("标签自动写入功能尚未开放")));

    // 5. Verify database mutations
    let relations = db.list_relations_for_source(&asset.id).expect("list relations");
    assert_eq!(relations.len(), 1, "Relation from successful suggestion must exist");
    assert_eq!(relations[0].target_id, entity.id);

    let collections = core.list_collections().expect("list collections");
    let souls_col = collections.iter().find(|c| c.title == "魂系壁纸集");
    assert!(souls_col.is_some(), "Collection from successful cluster must exist");
    let items = core.list_collection_items(&souls_col.unwrap().id).expect("list items");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].item_id, asset.id);
}
