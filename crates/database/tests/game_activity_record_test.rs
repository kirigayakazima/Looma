use std::sync::Arc;
use chrono::Utc;
use looma_core::models::*;
use looma_core::LoomaCore;
use looma_database::LoomaDb;

#[test]
fn test_game_activity_crud_multi_profile_cascade_and_backup_restore() {
    let db = Arc::new(LoomaDb::in_memory().expect("in-memory db init failed"));
    let core = LoomaCore::new(db.clone());

    // 1. Create a Work entity (Zelda: Tears of the Kingdom)
    let work = core
        .create_work(
            "tester",
            "塞尔达传说：王国之泪",
            WorkType::Game,
            RecordStatus::InProgress,
            Some("The Legend of Zelda: Tears of the Kingdom".to_string()),
            Some("任天堂开放世界神作".to_string()),
        )
        .expect("create work");

    // 2. Insert GameActivityRecord for Switch Profile A
    let record_a = GameActivityRecord {
        id: "act-totk-profile-a".to_string(),
        work_id: work.id.clone(),
        provider: "switch_nx".to_string(),
        source_key: "user_128bit_hex_a".to_string(),
        external_id: "0100f2c0115b6000".to_string(),
        source_label: Some("Switch (主账号)".to_string()),
        title_hint: Some("Tears of the Kingdom".to_string()),
        playtime_seconds: 85 * 3600, // 85 hours
        peak_seen_seconds: 85 * 3600,
        launch_count: Some(42),
        first_played_at: Some(Utc::now()),
        last_played_at: Some(Utc::now()),
        source_exported_at: Some(1731672000),
        last_import_sha256: Some("sha256-test-export-v1".to_string()),
        updated_at: Utc::now(),
    };
    core.upsert_game_activity(&record_a).expect("upsert activity a");

    // 3. Verify retrieval
    let list = core.list_game_activities_for_work(&work.id).expect("list activities");
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].playtime_hours(), 85.0);
    assert_eq!(list[0].launch_count, Some(42));

    // 4. Test Upsert update (incremental playtime from 85h to 90h)
    let mut updated_a = record_a.clone();
    updated_a.playtime_seconds = 90 * 3600;
    updated_a.peak_seen_seconds = 90 * 3600;
    updated_a.launch_count = Some(45);
    updated_a.updated_at = Utc::now();
    core.upsert_game_activity(&updated_a).expect("update activity a");

    let list_after_update = core.list_game_activities_for_work(&work.id).expect("list activities");
    assert_eq!(list_after_update.len(), 1, "Upsert must update in place without duplicating");
    assert_eq!(list_after_update[0].playtime_hours(), 90.0);
    assert_eq!(list_after_update[0].launch_count, Some(45));

    // 5. Test Multi-Profile: Same game, different Switch Profile B
    let record_b = GameActivityRecord {
        id: "act-totk-profile-b".to_string(),
        work_id: work.id.clone(),
        provider: "switch_nx".to_string(),
        source_key: "user_128bit_hex_b".to_string(),
        external_id: "0100f2c0115b6000".to_string(),
        source_label: Some("Switch (副账号/试玩)".to_string()),
        title_hint: Some("Tears of the Kingdom".to_string()),
        playtime_seconds: 10 * 3600, // 10 hours
        peak_seen_seconds: 10 * 3600,
        launch_count: Some(5),
        first_played_at: Some(Utc::now()),
        last_played_at: Some(Utc::now()),
        source_exported_at: Some(1731672000),
        last_import_sha256: Some("sha256-test-export-v1".to_string()),
        updated_at: Utc::now(),
    };
    core.upsert_game_activity(&record_b).expect("upsert activity b");

    let list_multi = core.list_game_activities_for_work(&work.id).expect("list activities");
    assert_eq!(list_multi.len(), 2, "Both profiles must coexist cleanly");
    assert!(list_multi.iter().any(|r| r.source_key == "user_128bit_hex_a" && r.playtime_hours() == 90.0));
    assert!(list_multi.iter().any(|r| r.source_key == "user_128bit_hex_b" && r.playtime_hours() == 10.0));

    // 6. Test Backup & Restore includes game activities
    let temp_backup_dir = tempfile::tempdir().expect("backup dir");
    let backup_res = core.export_backup("tester", temp_backup_dir.path()).expect("export backup");

    // Restore into a fresh in-memory database
    let restored_db = Arc::new(LoomaDb::in_memory().expect("in-memory db init"));
    let restored_core = LoomaCore::new(restored_db.clone());
    let restore_res = restored_core
        .restore_backup("tester", std::path::Path::new(&backup_res.backup_path))
        .expect("restore backup");

    assert!(restore_res.restored_entities >= 1);
    let restored_activities = restored_core
        .list_game_activities_for_work(&work.id)
        .expect("list restored activities");
    assert_eq!(restored_activities.len(), 2, "Backup restore must recover both game activities identically");
    assert!(restored_activities.iter().any(|r| r.source_key == "user_128bit_hex_a" && r.playtime_hours() == 90.0));
    assert!(restored_activities.iter().any(|r| r.source_key == "user_128bit_hex_b" && r.playtime_hours() == 10.0));

    // 7. Test Cascade Delete: Deleting the Work entity cascades to its game activity records
    core.delete_entity("tester", &work.id).expect("delete work");
    let after_delete = core.list_game_activities_for_work(&work.id).expect("list activities");
    assert_eq!(after_delete.len(), 0, "Deleting work must cascade delete its activity records");
}
