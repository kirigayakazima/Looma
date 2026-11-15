use std::sync::Arc;
use looma_core::models::*;
use looma_core::LoomaCore;
use looma_database::LoomaDb;
use serde_json::json;

#[test]
fn test_work_lifecycle_unification_and_precision() {
    let db = Arc::new(LoomaDb::in_memory().expect("in-memory db init failed"));
    let core = LoomaCore::new(db.clone());

    // 1. Create a Game Work with aliases and original title
    let game = core.create_work(
        "test-suite",
        "艾尔登法环",
        WorkType::Game,
        RecordStatus::InProgress,
        Some("ELDEN RING".to_string()),
        Some("FromSoftware open-world action RPG".to_string()),
    ).expect("failed to create game work");

    // Add aliases to the game using with_aliases
    let game_with_aliases = game.with_aliases(vec!["老头环".to_string(), "法环".to_string(), "ER".to_string()]);
    core.update_entity("test-suite", &game_with_aliases).expect("failed to update game with aliases");

    // Verify aliases are persisted in summary
    let summary = core.get_work_summary(&game_with_aliases.id).expect("get_work_summary failed").unwrap();
    assert_eq!(summary.work_metadata.unwrap().aliases, vec!["老头环", "法环", "ER"]);

    // 2. Link a game directory asset to the game
    let temp_dir = tempfile::tempdir().expect("failed to create tempdir");
    let game_dir = temp_dir.path().join("EldenRing");
    std::fs::create_dir_all(&game_dir).expect("failed to create game dir");
    let game_dir_str = game_dir.to_string_lossy().to_string();

    let (linked_asset, _rel) = core.link_work_directory("test-suite", &game_with_aliases.id, &game_dir_str).expect("link_work_directory failed");
    let expected_norm_path = looma_core::normalize_directory_path(&game_dir_str);
    assert_eq!(linked_asset.path.as_deref(), Some(expected_norm_path.as_str()));

    let summary = core.get_work_summary(&game_with_aliases.id).expect("get_work_summary failed").unwrap();
    assert_eq!(summary.linked_assets.len(), 1);
    assert_eq!(summary.linked_assets[0].path.as_deref(), Some(expected_norm_path.as_str()));

    // 3. Test Experience recording with flexible precisions
    // 3a. 'unknown' precision (no date)
    let exp_unknown = ExperienceEntry::new(&game_with_aliases.id, experience_events::STARTED)
        .with_title("初入交界地")
        .with_occurred_unknown()
        .with_context(json!({ "platform": "PC", "class": "Vagabond" }));
    let created_unknown = core.record_experience("test-suite", &exp_unknown, &[], None).expect("record_experience failed");
    assert_eq!(created_unknown.occurred_precision, "unknown");
    assert!(created_unknown.occurred_value.is_none());

    // 3b. 'year' precision
    let exp_year = ExperienceEntry::new(&game_with_aliases.id, experience_events::PROGRESS_MILESTONE)
        .with_title("击败碎星拉塔恩")
        .with_occurred_year(2022)
        .with_context(json!({ "level": 65 }));
    let created_year = core.record_experience("test-suite", &exp_year, &[], None).expect("record_experience failed");
    assert_eq!(created_year.occurred_precision, "year");
    assert_eq!(created_year.occurred_value.as_deref(), Some("2022"));

    // 3c. 'month' precision
    let exp_month = ExperienceEntry::new(&game_with_aliases.id, experience_events::PROGRESS_MILESTONE)
        .with_title("抵达王城罗德尔")
        .with_occurred_month(2022, 4)
        .with_context(json!({ "level": 90 }));
    let created_month = core.record_experience("test-suite", &exp_month, &[], None).expect("record_experience failed");
    assert_eq!(created_month.occurred_precision, "month");
    assert_eq!(created_month.occurred_value.as_deref(), Some("2022-04"));

    // 3d. 'day' precision
    let exp_day = ExperienceEntry::new(&game_with_aliases.id, experience_events::COMPLETED)
        .with_title("达成星世纪结局")
        .with_occurred_day(2022, 5, 15)
        .with_context(json!({ "playthrough": 1, "ending": "Age of the Stars" }));
    let created_day = core.record_experience("test-suite", &exp_day, &[], None).expect("record_experience failed");
    assert_eq!(created_day.occurred_precision, "day");
    assert_eq!(created_day.occurred_value.as_deref(), Some("2022-05-15"));

    // 4. Verify all experiences appear in work summary
    let summary = core.get_work_summary(&game_with_aliases.id).expect("get_work_summary failed").unwrap();
    assert_eq!(summary.experiences.len(), 4);

    // 5. Create a generic work (Anime) to prove lifecycle unification
    let anime = core.create_work(
        "test-suite",
        "葬送的芙莉莲",
        WorkType::Anime,
        RecordStatus::Completed,
        Some("Sousou no Frieren".to_string()),
        Some("Madhouse fantasy anime".to_string()),
    ).expect("failed to create anime work");

    let exp_anime = ExperienceEntry::new(&anime.id, experience_events::COMPLETED)
        .with_title("追番完结")
        .with_occurred_day(2024, 3, 22);
    core.record_experience("test-suite", &exp_anime, &[], None).expect("record anime experience failed");

    let anime_summary = core.get_work_summary(&anime.id).expect("anime summary failed").unwrap();
    assert_eq!(anime_summary.experiences.len(), 1);
    assert_eq!(anime_summary.experiences[0].title.as_deref(), Some("追番完结"));
    assert_eq!(anime_summary.work_metadata.unwrap().status, RecordStatus::Completed);
}
