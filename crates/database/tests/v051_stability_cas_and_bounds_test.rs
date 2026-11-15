use std::sync::Arc;
use chrono::Utc;
use looma_core::models::*;
use looma_core::services::AssetService;
use looma_core::LoomaCore;
use looma_database::LoomaDb;
use serde_json::json;

#[test]
fn test_entity_cas_optimistic_concurrency_and_conflict_rejection() {
    let db = Arc::new(LoomaDb::in_memory().expect("in-memory db init failed"));
    let core = LoomaCore::new(db.clone());

    // 1. Create an entity
    let original = Entity {
        id: "ent-elden-ring".to_string(),
        entity_type: "work".to_string(),
        title: "艾尔登法环 (Elden Ring)".to_string(),
        description: Some("FromSoftware ARPG".to_string()),
        properties: json!({
            "domain": "game",
            "work": {
                "work_type": "game",
                "status": "in_progress"
            }
        }),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    core.create_entity("test", &original).expect("create entity");

    // Fetch entity snapshot A and snapshot B concurrently
    let snapshot_a = core.get_entity("ent-elden-ring").expect("get").expect("found");
    let mut snapshot_b = core.get_entity("ent-elden-ring").expect("get").expect("found");

    // Process A updates title successfully
    let mut update_a = snapshot_a.clone();
    update_a.title = "艾尔登法环：黄金树幽影".to_string();
    core.update_entity("worker-a", &update_a).expect("worker A update should succeed");

    // Verify DB updated_at is now strictly newer than snapshot_b's updated_at
    let current_in_db = core.get_entity("ent-elden-ring").expect("get").expect("found");
    assert_eq!(current_in_db.title, "艾尔登法环：黄金树幽影");
    assert!(current_in_db.updated_at > snapshot_b.updated_at);

    // Process B tries to submit its stale snapshot (without reloading)
    snapshot_b.title = "艾尔登法环 (Stale Overwrite Attempt)".to_string();
    let err = core.update_entity("worker-b", &snapshot_b).expect_err("stale update must be rejected");

    let err_msg = err.to_string();
    assert!(
        err_msg.contains("Stale update conflict") || err_msg.contains("Concurrent update conflict"),
        "Error message must indicate optimistic lock conflict, got: {}",
        err_msg
    );

    // Verify DB record was NOT corrupted by stale snapshot B
    let final_db = core.get_entity("ent-elden-ring").expect("get").expect("found");
    assert_eq!(final_db.title, "艾尔登法环：黄金树幽影");
}

#[test]
fn test_thumbnail_safety_rejects_extreme_dimensions() {
    let db = Arc::new(LoomaDb::in_memory().expect("in-memory db init failed"));
    let core = LoomaCore::new(db.clone());

    // Create a temporary giant image file (e.g. 9000x9000 = 81 megapixels)
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let giant_img_path = temp_dir.path().join("extreme_resolution.png");

    // Write a dummy PNG with 9000x9000 header
    // Using image crate to write header
    let img = image::RgbImage::new(9000, 9000);
    img.save(&giant_img_path).expect("save giant image");

    let asset = Asset {
        id: "asset-giant".to_string(),
        kind: AssetKind::Image,
        source: AssetSource::Local,
        path: Some(giant_img_path.to_string_lossy().to_string()),
        size: Some(1024),
        hash: Some("sha256-giant".to_string()),
        mime_type: Some("image/png".to_string()),
        metadata: json!({}),
        status: AssetStatus::Active,
        created_at: Utc::now(),
        modified_at: Utc::now(),
        indexed_at: Utc::now(),
    };
    db.upsert_asset(&asset).expect("upsert asset");

    // Generating thumbnail must be rejected safely by the 64MP limit without full memory decompression
    let err = core.get_asset_thumbnail("asset-giant", 320).expect_err("giant image must be rejected");
    assert!(err.to_string().contains("exceeds safety threshold"));
}
