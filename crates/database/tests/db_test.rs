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

    // 5. Relation Service (Bi-directional)
    let relation = Relation {
        id: "rel-1".to_string(),
        source_id: "asset-1".to_string(),
        source_type: "asset".to_string(),
        target_id: "ent-1".to_string(),
        target_type: "entity".to_string(),
        relation_type: "belongs_to".to_string(),
        metadata: json!({ "note": "cover art" }),
        created_at: Utc::now(),
    };
    db.create_relation(&relation).expect("create relation failed");

    // Fetch relations for source (asset-1)
    let rels_for_asset = db.list_relations_for_item("asset-1").expect("list relations for asset failed");
    assert_eq!(rels_for_asset.len(), 1);
    assert_eq!(rels_for_asset[0].target_id, "ent-1");

    // Fetch relations for target (ent-1) - verifying bi-directional lookup
    let rels_for_entity = db.list_relations_for_item("ent-1").expect("list relations for entity failed");
    assert_eq!(rels_for_entity.len(), 1);
    assert_eq!(rels_for_entity[0].source_id, "asset-1");

    // Delete relation between items
    db.delete_relation_between("asset-1", "ent-1").expect("delete relation between failed");
    assert_eq!(db.list_relations_for_item("asset-1").unwrap().len(), 0);

    // 6. Collection Service
    let collection = Collection {
        id: "col-1".to_string(),
        title: "Favorites".to_string(),
        description: Some("Curated personal favorites".to_string()),
        query: None,
        metadata: json!({}),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    db.create_collection(&collection).expect("create collection failed");
    assert_eq!(db.list_collections().expect("list collections failed").len(), 1);

    // Add items to collection
    db.add_item_to_collection("col-1", "asset-1", "asset").expect("add asset to collection failed");
    db.add_item_to_collection("col-1", "ent-1", "entity").expect("add entity to collection failed");

    let col_items = db.list_items_for_collection("col-1").expect("list items failed");
    assert_eq!(col_items.len(), 2);

    // Remove item and delete collection
    db.remove_item_from_collection("col-1", "asset-1").expect("remove item failed");
    assert_eq!(db.list_items_for_collection("col-1").unwrap().len(), 1);

    db.delete_collection("col-1").expect("delete collection failed");
    assert_eq!(db.list_collections().expect("list collections failed").len(), 0);
    assert_eq!(db.list_items_for_collection("col-1").unwrap().len(), 0);

    // 7. Entity Update & Delete
    let mut updated_entity = fetched;
    updated_entity.title = "BLEACH: Thousand-Year Blood War".to_string();
    db.update_entity(&updated_entity).expect("update entity failed");
    let re_fetched = db.get_entity_by_id("ent-1").unwrap().unwrap();
    assert_eq!(re_fetched.title, "BLEACH: Thousand-Year Blood War");

    db.delete_entity("ent-1").expect("delete entity failed");
    assert!(db.get_entity_by_id("ent-1").unwrap().is_none());
}
