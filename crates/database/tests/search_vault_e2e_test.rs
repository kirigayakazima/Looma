use std::sync::Arc;
use chrono::Utc;
use looma_core::models::*;
use looma_core::services::{AssetService, MemoryService};
use looma_core::LoomaCore;
use looma_database::LoomaDb;
use serde_json::json;

#[test]
fn test_search_vault_and_asset_pagination_e2e() {
    let db = Arc::new(LoomaDb::in_memory().expect("in-memory db init failed"));
    let core = LoomaCore::new(db.clone());

    // 1. Verify dynamic vault version matches Cargo package version, not hardcoded 0.2.0
    let vault_info = core.get_vault_info().expect("get vault info");
    assert_eq!(
        vault_info.version,
        env!("CARGO_PKG_VERSION"),
        "Vault version must dynamically reflect CARGO_PKG_VERSION"
    );

    // 2. Insert 100 Assets with varied paths and kinds
    for i in 0..100 {
        let (name, kind) = if i < 40 {
            (format!("bloodborne_concept_art_{:03}.png", i), AssetKind::Image)
        } else if i < 70 {
            (format!("elden_ring_ost_track_{:03}.flac", i), AssetKind::Audio)
        } else {
            (format!("sekiro_guide_doc_{:03}.pdf", i), AssetKind::Document)
        };

        let asset = Asset {
            id: format!("asset-{:03}", i),
            kind: kind.clone(),
            source: AssetSource::Local,
            path: Some(format!("D:/Games/Souls/{}", name)),
            size: Some(1024 * (i as u64 + 1)),
            hash: Some(format!("hash_{:03}", i)),
            mime_type: Some(match kind {
                AssetKind::Image => "image/png".to_string(),
                AssetKind::Audio => "audio/flac".to_string(),
                _ => "application/pdf".to_string(),
            }),
            metadata: json!({ "index": i }),
            status: AssetStatus::Active,
            created_at: Utc::now(),
            modified_at: Utc::now(),
            indexed_at: Utc::now(),
        };
        db.upsert_asset(&asset).expect("upsert asset");
    }

    // 3. Test count_assets and count_assets_with_filter
    let total_assets = core.count_assets().expect("count assets");
    assert_eq!(total_assets, 100);

    let image_filter = AssetFilter {
        kind: Some(AssetKind::Image),
        ..Default::default()
    };
    let image_count = core.count_assets_with_filter(&image_filter).expect("count image assets");
    assert_eq!(image_count, 40);

    let search_filter = AssetFilter {
        search_query: Some("elden_ring".to_string()),
        ..Default::default()
    };
    let elden_count = core.count_assets_with_filter(&search_filter).expect("count elden assets");
    assert_eq!(elden_count, 30);

    // 4. Test pagination LIMIT & OFFSET in query_assets
    let page1 = core
        .query_assets(&AssetFilter {
            limit: Some(25),
            offset: Some(0),
            ..Default::default()
        })
        .expect("page 1");
    assert_eq!(page1.len(), 25);

    let page2 = core
        .query_assets(&AssetFilter {
            limit: Some(25),
            offset: Some(25),
            ..Default::default()
        })
        .expect("page 2");
    assert_eq!(page2.len(), 25);
    assert_ne!(page1[0].id, page2[0].id, "Page 1 and Page 2 must not overlap");

    // 5. Populate Entities, Collections, Memories
    let bb_entity = Entity::new_work(
        "Bloodborne (血源诅咒)",
        WorkType::Game,
        RecordStatus::Completed,
        Some("Bloodborne".to_string()),
        Some("FromSoftware 维多利亚克苏鲁风动作游戏".to_string()),
    );
    core.create_entity("test", &bb_entity).expect("create bloodborne entity");

    let er_entity = Entity::new_work(
        "Elden Ring (艾尔登法环)",
        WorkType::Game,
        RecordStatus::InProgress,
        Some("Elden Ring".to_string()),
        Some("FromSoftware 动作角色扮演游戏".to_string()),
    );
    core.create_entity("test", &er_entity).expect("create elden ring entity");

    let collection = Collection {
        id: "col-fromsoft".to_string(),
        title: "FromSoftware 经典神作".to_string(),
        description: Some("收录血源、老头环、只狼相关资产与经历".to_string()),
        query: None,
        metadata: json!({}),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    core.create_collection("test", &collection).expect("create collection");

    let memory = Memory {
        id: "mem-bb-plat".to_string(),
        title: "血源诅咒白金达成纪念".to_string(),
        content: "终于击败苏美鲁女王，完成全奖杯！亚楠的日出令人难忘。".to_string(),
        category: Some("gaming".to_string()),
        metadata: json!({ "tags": ["bloodborne", "platinum"] }),
        recorded_at: Utc::now(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    db.create_memory(&memory).expect("create memory");

    // 6. Test search_vault across all categories
    let search_all = core
        .search_vault("bloodborne", None, 50)
        .expect("search bloodborne");
    assert!(search_all.total_matches >= 42, "Must find at least 40 assets + 1 entity + 1 memory");
    assert!(search_all.items.iter().any(|i| i.category == "asset"));
    assert!(search_all.items.iter().any(|i| i.category == "entity"));
    assert!(search_all.items.iter().any(|i| i.category == "memory"));

    // 7. Test search_vault with category filter
    let search_assets_only = core
        .search_vault("bloodborne", Some("asset"), 10)
        .expect("search asset category");
    assert_eq!(search_assets_only.items.len(), 10, "Limit must cap results to 10");
    assert!(search_assets_only.items.iter().all(|i| i.category == "asset"));

    let search_entities_only = core
        .search_vault("法环", Some("entity"), 10)
        .expect("search entity category");
    assert_eq!(search_entities_only.items.len(), 1);
    assert_eq!(search_entities_only.items[0].category, "entity");
    assert_eq!(search_entities_only.items[0].id, er_entity.id);
}
