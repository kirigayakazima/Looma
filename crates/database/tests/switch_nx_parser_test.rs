use looma_core::models::*;
use looma_core::LoomaCore;
use looma_database::LoomaDb;
use std::sync::Arc;
use tempfile::tempdir;

fn setup_test_core() -> (tempfile::TempDir, LoomaCore) {
    let dir = tempdir().unwrap();
    let db = Arc::new(LoomaDb::open(dir.path()).unwrap());
    let core = LoomaCore::new(db);
    (dir, core)
}

#[test]
fn test_switch_nx_single_profile_and_cjk_parsing() {
    let (_dir, core) = setup_test_core();

    let json_content = r#"{
        "exportTimestamp": 1700000000,
        "exportVersion": "1.4.0",
        "users": [
            {
                "id": "0123456789abcdef0123456789abcdef",
                "name": "主玩家",
                "titles": [
                    {
                        "id": "01007EF00011E000",
                        "name": "塞尔达传说 旷野之息",
                        "summary": {
                            "firstPlayed": 1500000000,
                            "lastPlayed": 1690000000,
                            "launches": 150,
                            "playtime": 306000
                        },
                        "events": []
                    },
                    {
                        "id": "0100000000010000",
                        "name": "Super Mario Odyssey",
                        "summary": {
                            "firstPlayed": 0,
                            "lastPlayed": 0,
                            "launches": 20,
                            "playtime": 72000
                        }
                    }
                ]
            }
        ]
    }"#;

    let preview = core.preview_switch_import(json_content, None).unwrap();

    assert_eq!(preview.profiles.len(), 1);
    assert_eq!(preview.selected_user_id, "0123456789abcdef0123456789abcdef");
    assert_eq!(preview.selected_user_name, "主玩家");
    assert_eq!(preview.total_games, 2);

    // Zelda check: normalized 16-hex, CJK title, 306000s = 85.0h
    let zelda = &preview.games[0];
    assert_eq!(zelda.external_id, "01007ef00011e000");
    assert_eq!(zelda.title_name, "塞尔达传说 旷野之息");
    assert_eq!(zelda.playtime_seconds, 306000);
    assert!((zelda.playtime_hours - 85.0).abs() < 0.001);
    assert_eq!(zelda.launches, Some(150));
    assert!(zelda.first_played_at.is_some());
    assert!(zelda.last_played_at.is_some());
    assert_eq!(zelda.match_status, SwitchGameMatchStatus::NewGame);

    // Mario check: zero first/lastPlayed maps to None (not epoch 1970!)
    let mario = &preview.games[1];
    assert_eq!(mario.external_id, "0100000000010000");
    assert_eq!(mario.title_name, "Super Mario Odyssey");
    assert_eq!(mario.first_played_at, None);
    assert_eq!(mario.last_played_at, None);
    assert_eq!(mario.playtime_seconds, 72000);
    assert!((mario.playtime_hours - 20.0).abs() < 0.001);
}

#[test]
fn test_switch_nx_multi_profile_selection() {
    let (_dir, core) = setup_test_core();

    let json_content = r#"{
        "exportTimestamp": 1700000000,
        "exportVersion": 2,
        "users": [
            {
                "id": "11111111111111111111111111111111",
                "name": "MainUser",
                "titles": [
                    {
                        "id": "0100000000010000",
                        "name": "Mario Odyssey",
                        "summary": { "playtime": 36000 }
                    }
                ]
            },
            {
                "id": "22222222222222222222222222222222",
                "name": "AltAccount",
                "titles": [
                    {
                        "id": "0100123000456000",
                        "name": "Pokemon Scarlet",
                        "summary": { "playtime": 18000 }
                    }
                ]
            }
        ]
    }"#;

    // 1. Default selection picks first user
    let p1 = core.preview_switch_import(json_content, None).unwrap();
    assert_eq!(p1.profiles.len(), 2);
    assert_eq!(p1.selected_user_id, "11111111111111111111111111111111");
    assert_eq!(p1.games.len(), 1);
    assert_eq!(p1.games[0].title_name, "Mario Odyssey");

    // 2. Explicit selection of AltAccount
    let p2 = core.preview_switch_import(json_content, Some("22222222222222222222222222222222")).unwrap();
    assert_eq!(p2.selected_user_id, "22222222222222222222222222222222");
    assert_eq!(p2.games.len(), 1);
    assert_eq!(p2.games[0].title_name, "Pokemon Scarlet");

    // 3. Non-existent profile fails gracefully
    let err = core.preview_switch_import(json_content, Some("nonexistent_id"));
    assert!(err.is_err());
}

#[test]
fn test_switch_nx_matching_and_warnings() {
    let (_dir, core) = setup_test_core();

    // Create an existing Work entity titled "Hollow Knight" with an alias
    let work = core.create_work(
        "test-actor",
        "Hollow Knight",
        WorkType::Game,
        RecordStatus::InProgress,
        None,
        Some("Metroidvania masterpiece".to_string()),
    ).unwrap();

    let json_v1 = r#"{
        "exportTimestamp": 1700000000,
        "exportVersion": "1.0",
        "users": [
            {
                "id": "0123456789abcdef0123456789abcdef",
                "name": "Player1",
                "titles": [
                    {
                        "id": "0100d07000536000",
                        "name": "Hollow Knight",
                        "summary": {
                            "playtime": 180000,
                            "launches": 50
                        }
                    }
                ]
            }
        ]
    }"#;

    // 1. Preview v1: should suggest matching existing Work "Hollow Knight"
    let p1 = core.preview_switch_import(json_v1, None).unwrap();
    assert_eq!(p1.games[0].match_status, SwitchGameMatchStatus::SuggestedMatch);
    assert_eq!(p1.games[0].matched_work_id, Some(work.id.clone()));
    assert_eq!(p1.games[0].matched_work_title, Some("Hollow Knight".to_string()));

    // Commit v1 import linked to the existing Work
    let decision_v1 = SwitchImportDecision {
        file_sha256: p1.file_sha256.clone(),
        selected_user_id: p1.selected_user_id.clone(),
        user_label: Some("Switch (OLED)".to_string()),
        source_exported_at: p1.export_timestamp,
        items: vec![
            SwitchImportItemDecision {
                external_id: p1.games[0].external_id.clone(),
                title_hint: p1.games[0].title_name.clone(),
                playtime_seconds: p1.games[0].playtime_seconds,
                launch_count: p1.games[0].launches,
                first_played_at: p1.games[0].first_played_at,
                last_played_at: p1.games[0].last_played_at,
                action: SwitchImportAction::LinkExistingWork { work_id: work.id.clone() },
            }
        ],
    };
    let outcome_v1 = core.commit_switch_import("test-actor", &decision_v1).unwrap();
    assert_eq!(outcome_v1.imported_records, 1);
    assert_eq!(outcome_v1.created_works, 0);

    // 2. Duplicate snapshot preview: same file -> ExactMapped + is_duplicate_snapshot = true
    let p_dup = core.preview_switch_import(json_v1, None).unwrap();
    assert_eq!(p_dup.games[0].match_status, SwitchGameMatchStatus::ExactMapped);
    assert!(p_dup.games[0].is_duplicate_snapshot);

    // 3. Playtime decreased warning (e.g. system reset from 50h down to 2h)
    let json_decreased = r#"{
        "exportTimestamp": 1700010000,
        "users": [
            {
                "id": "0123456789abcdef0123456789abcdef",
                "name": "Player1",
                "titles": [
                    {
                        "id": "0100d07000536000",
                        "name": "Hollow Knight",
                        "summary": {
                            "playtime": 7200,
                            "launches": 5
                        }
                    }
                ]
            }
        ]
    }"#;
    let p_dec = core.preview_switch_import(json_decreased, None).unwrap();
    let game_dec = &p_dec.games[0];
    assert_eq!(game_dec.match_status, SwitchGameMatchStatus::ExactMapped);
    assert!(game_dec.is_time_decreased);
    assert_eq!(game_dec.existing_peak_seconds, Some(180000));
    assert!(game_dec.warning.as_ref().unwrap().contains("较上次减少"));

    // Commit decreased import: peak_seen_seconds must remain 180000 (50h)!
    let decision_dec = SwitchImportDecision {
        file_sha256: p_dec.file_sha256.clone(),
        selected_user_id: p_dec.selected_user_id.clone(),
        user_label: Some("Switch (OLED)".to_string()),
        source_exported_at: p_dec.export_timestamp,
        items: vec![
            SwitchImportItemDecision {
                external_id: game_dec.external_id.clone(),
                title_hint: game_dec.title_name.clone(),
                playtime_seconds: game_dec.playtime_seconds,
                launch_count: game_dec.launches,
                first_played_at: game_dec.first_played_at,
                last_played_at: game_dec.last_played_at,
                action: SwitchImportAction::LinkExistingWork { work_id: work.id.clone() },
            }
        ],
    };
    let outcome_dec = core.commit_switch_import("test-actor", &decision_dec).unwrap();
    assert_eq!(outcome_dec.imported_records, 1);
    assert_eq!(outcome_dec.updated_records, 1);

    let record = core.get_game_activity("switch_nx", "0123456789abcdef0123456789abcdef", "0100d07000536000").unwrap().unwrap();
    assert_eq!(record.playtime_seconds, 7200); // latest current value is 2h
    assert_eq!(record.peak_seen_seconds, 180000); // peak preserved at 50h!
}

#[test]
fn test_switch_nx_lenient_fields_and_error_handling() {
    let (_dir, core) = setup_test_core();

    // 1. Bad JSON
    let err = core.preview_switch_import("{ not valid json", None);
    assert!(err.is_err());

    // 2. Empty users array
    let empty_users = r#"{ "exportTimestamp": 100, "users": [] }"#;
    let err_empty = core.preview_switch_import(empty_users, None);
    assert!(err_empty.is_err());

    // 3. Lenient floats and string representations
    let lenient_json = r#"{
        "exportTimestamp": "1700000000",
        "exportVersion": 1,
        "users": [
            {
                "id": "user123",
                "titles": [
                    {
                        "id": "0100abc000def000",
                        "summary": {
                            "playtime": 3600.0,
                            "launches": "10"
                        }
                    }
                ]
            }
        ]
    }"#;
    let preview = core.preview_switch_import(lenient_json, None).unwrap();
    assert_eq!(preview.games[0].playtime_seconds, 3600);
    assert_eq!(preview.games[0].launches, Some(10));
}
