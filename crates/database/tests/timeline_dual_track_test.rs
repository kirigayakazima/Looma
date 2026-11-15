use std::sync::Arc;
use chrono::Utc;
use looma_core::models::*;
use looma_core::services::{AssetService, EntityService, MemoryService, TimelineService};
use looma_core::LoomaCore;
use looma_database::LoomaDb;
use serde_json::json;

#[test]
fn test_timeline_dual_track_separation_and_undated_precision() {
    let db = Arc::new(LoomaDb::in_memory().expect("in-memory db init failed"));
    let core = LoomaCore::new(db.clone());

    // 1. Create a Work entity (system track)
    let work_entity = Entity {
        id: "work-hollow-knight".to_string(),
        entity_type: "work".to_string(),
        title: "空洞骑士 (Hollow Knight)".to_string(),
        description: Some("Team Cherry 银河恶魔城神作".to_string()),
        properties: json!({
            "domain": "game",
            "work": {
                "work_type": "game",
                "status": "completed",
                "progress": {
                    "completion_status": "completed",
                    "play_status": "completed"
                }
            }
        }),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    db.create_entity(&work_entity).expect("create entity");

    // 2. Create an Asset (system track)
    let asset = Asset {
        id: "asset-hk-save".to_string(),
        kind: AssetKind::Document,
        source: AssetSource::Local,
        path: Some("D:/Games/HollowKnight/user1.dat".to_string()),
        size: Some(2048),
        hash: Some("sha256-hksave".to_string()),
        mime_type: Some("application/octet-stream".to_string()),
        metadata: json!({ "file": "user1.dat" }),
        status: AssetStatus::Active,
        created_at: Utc::now(),
        modified_at: Utc::now(),
        indexed_at: Utc::now(),
    };
    db.upsert_asset(&asset).expect("upsert asset");

    // 3. Create a Memory (personal track)
    let memory = Memory {
        id: "mem-hk-reflection".to_string(),
        title: "初入泪水之城的手记".to_string(),
        content: "背景钢琴声响起的时候，那种震撼感无与伦比。".to_string(),
        category: Some("game_reflection".to_string()),
        metadata: json!({ "game": "Hollow Knight" }),
        recorded_at: Utc::now(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    db.create_memory(&memory).expect("create memory");

    // 4. Create an Experience with precision "year" (dated, personal track)
    let exp_dated = ExperienceEntry {
        id: "exp-hk-radiance".to_string(),
        subject_type: "entity".to_string(),
        subject_id: "work-hollow-knight".to_string(),
        event_type: "completed".to_string(),
        title: Some("击败辐光，达成真结局".to_string()),
        occurred_precision: "year".to_string(),
        occurred_value: Some("2023".to_string()),
        note: Some("拼尽全力，终于通关真结局".to_string()),
        context_json: json!({ "platform": "PC", "achievements": 63 }),
        visibility: "normal".to_string(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    core.record_experience("tester", &exp_dated, &[], None).expect("record dated experience");

    // 5. Create an Experience with precision "unknown" (undated, personal track)
    let exp_undated = ExperienceEntry {
        id: "exp-hk-demo".to_string(),
        subject_type: "entity".to_string(),
        subject_id: "work-hollow-knight".to_string(),
        event_type: "played".to_string(),
        title: Some("多年前初次体验试玩版".to_string()),
        occurred_precision: "unknown".to_string(),
        occurred_value: None,
        note: Some("具体年份记不清了，在朋友电脑上玩过一段时间试玩 Demo".to_string()),
        context_json: json!({ "is_occurred_time_unknown": true }),
        visibility: "normal".to_string(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    core.record_experience("tester", &exp_undated, &[], None).expect("record undated experience");

    // --- VERIFICATION 1: Default timeline query (track = "personal" by default) ---
    let default_filter = TimelineFilter::default();
    let personal_items = db.query_timeline(&default_filter).expect("query timeline default");

    assert_eq!(personal_items.len(), 3, "Personal track must only return memories and experiences");
    for item in &personal_items {
        assert_eq!(
            item.track.as_deref(),
            Some("personal"),
            "Items in personal track must have track='personal'"
        );
        assert!(
            item.item_type == "memory" || item.item_type == "experience",
            "Item type must be memory or experience, got {}",
            item.item_type
        );
    }

    // Check precision metadata
    let dated_item = personal_items.iter().find(|i| i.id == "exp-hk-radiance").expect("found dated item");
    assert_eq!(dated_item.occurred_precision.as_deref(), Some("year"));

    let undated_item = personal_items.iter().find(|i| i.id == "exp-hk-demo").expect("found undated item");
    assert_eq!(undated_item.occurred_precision.as_deref(), Some("unknown"));

    let memory_item = personal_items.iter().find(|i| i.id == "mem-hk-reflection").expect("found memory item");
    assert_eq!(memory_item.occurred_precision.as_deref(), Some("instant"));

    // --- VERIFICATION 2: System track query (track = "system") ---
    let system_filter = TimelineFilter {
        track: Some("system".to_string()),
        ..Default::default()
    };
    let system_items = db.query_timeline(&system_filter).expect("query timeline system");

    assert_eq!(system_items.len(), 2, "System track must only return assets and entities");
    for item in &system_items {
        assert_eq!(
            item.track.as_deref(),
            Some("system"),
            "Items in system track must have track='system'"
        );
        assert!(
            item.item_type == "asset" || item.item_type == "entity",
            "Item type must be asset or entity, got {}",
            item.item_type
        );
    }

    // --- VERIFICATION 3: All tracks combined (track = "all") ---
    let all_filter = TimelineFilter {
        track: Some("all".to_string()),
        ..Default::default()
    };
    let all_items = db.query_timeline(&all_filter).expect("query timeline all");
    assert_eq!(all_items.len(), 5, "All track must return all 5 items across tracks");
}
