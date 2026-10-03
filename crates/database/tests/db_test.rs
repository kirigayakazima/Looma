use chrono::Utc;
use looma_core::models::*;
use looma_core::services::*;
use looma_database::LoomaDb;
use serde_json::json;

#[test]
fn test_in_memory_db_and_services() {
    let db = LoomaDb::in_memory().expect("in-memory db init failed");

    // 1. Vault Info
    let info = db.get_vault_info().expect("vault info failed");
    assert!(info.is_initialized);

    // 2. Entity Service
    let entity = Entity {
        id: "ent-1".to_string(),
        entity_type: "anime".to_string(),
        title: "BLEACH".to_string(),
        description: Some("Thousand-Year Blood War".to_string()),
        properties: json!({ "status": "watching", "rating": 5 }),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    db.create_entity(&entity).expect("create entity failed");

    let count = db.count_entities().expect("count entities failed");
    assert_eq!(count, 1);

    let fetched = db.get_entity_by_id("ent-1").expect("get entity failed").expect("entity not found");
    assert_eq!(fetched.title, "BLEACH");

    // 3. Asset Service
    let asset = Asset {
        id: "asset-1".to_string(),
        kind: AssetKind::Image,
        source: AssetSource::Local,
        path: Some("D:/Anime/BLEACH/poster.png".to_string()),
        size: Some(1024),
        hash: Some("abcdef123456".to_string()),
        mime_type: Some("image/png".to_string()),
        metadata: json!({ "width": 1920, "height": 1080 }),
        status: AssetStatus::Active,
        created_at: Utc::now(),
        modified_at: Utc::now(),
        indexed_at: Utc::now(),
    };
    db.upsert_asset(&asset).expect("upsert asset failed");
    assert_eq!(db.count_assets().expect("count assets failed"), 1);

    // 4. Memory Service
    let memory = Memory {
        id: "mem-1".to_string(),
        title: "Initial thought".to_string(),
        content: "Weaving personal vault with Looma".to_string(),
        category: Some("journal".to_string()),
        metadata: json!({}),
        recorded_at: Utc::now(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    db.create_memory(&memory).expect("create memory failed");
    let mems = db.list_memories(10, 0).expect("list memories failed");
    assert_eq!(mems.len(), 1);
}
