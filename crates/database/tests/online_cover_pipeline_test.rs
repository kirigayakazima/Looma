use std::sync::Arc;
use looma_core::models::*;
use looma_core::LoomaCore;
use looma_database::LoomaDb;

#[test]
fn test_clean_game_title_for_search() {
    assert_eq!(
        LoomaCore::clean_game_title_for_search("NieR+Automata+Game+of+the+YoRHa+Edition+v1.0"),
        "NieR Automata"
    );
    assert_eq!(
        LoomaCore::clean_game_title_for_search("[0 chapter] 3DMGAME Final Fantasy VII Remake.CHS.Green"),
        "Final Fantasy VII Remake"
    );
    assert_eq!(
        LoomaCore::clean_game_title_for_search("Rusty Lake Roots v1.2"),
        "Rusty Lake Roots"
    );
    assert_eq!(
        LoomaCore::clean_game_title_for_search("Shapez.2 v0.0.9"),
        "Shapez 2"
    );
}

#[test]
fn test_percent_encode() {
    assert_eq!(LoomaCore::percent_encode("The Witcher 3"), "The+Witcher+3");
    assert_eq!(LoomaCore::percent_encode("Zelda"), "Zelda");
}

#[test]
fn test_online_cover_fetch_and_managed_persistence() {
    let db = LoomaDb::in_memory().expect("in-memory db failed");
    let core = LoomaCore::new(Arc::new(db));

    let work = core
        .create_work(
            "tester",
            "The Witcher 3",
            WorkType::Game,
            RecordStatus::InProgress,
            None,
            None,
        )
        .expect("create_work failed");

    // 1. Fetch cover from Steam CDN (The Witcher 3 capsule)
    let steam_capsule_url = "https://shared.steamstatic.com/store_item_assets/steam/apps/292030/library_600x900.jpg";
    let asset = core
        .download_and_set_work_cover("tester", &work.id, steam_capsule_url, true)
        .expect("download_and_set_work_cover failed");

    assert_eq!(asset.source, AssetSource::Managed);
    assert_eq!(asset.kind, AssetKind::Image);
    assert!(asset.size.unwrap() > 10_000, "Should have downloaded real image bytes");
    assert!(asset.path.as_ref().unwrap().starts_with(".looma/media/covers/"));

    let meta = asset.metadata.as_object().unwrap();
    assert_eq!(meta.get("role").unwrap(), "work_cover");
    assert_eq!(meta.get("provider").unwrap(), "steam");
    assert_eq!(meta.get("aspect_ratio").unwrap(), "2:3");

    // Verify work is updated with cover_asset_id
    let updated_work = core.get_entity(&work.id).unwrap().unwrap();
    let work_meta = updated_work.as_work_metadata().unwrap();
    assert_eq!(work_meta.cover_asset_id, Some(asset.id.clone()));

    // Verify thumbnail generation works from downloaded managed cover
    let thumb_res = core.get_asset_thumbnail(&asset.id, 200).expect("thumbnail failed").unwrap();
    assert!(thumb_res.data_url.starts_with("data:image/"));

    // 2. Test candidate fetching with Switch Title ID
    let zelda_work = core
        .create_work(
            "tester",
            "塞尔达：旷野之息",
            WorkType::Game,
            RecordStatus::Completed,
            None,
            None,
        )
        .expect("create zelda work failed");

    // Link a Switch Game Activity record
    let now = chrono::Utc::now();
    let act = GameActivityRecord {
        id: "act_zelda_botw".to_string(),
        work_id: zelda_work.id.clone(),
        provider: "switch_nx".to_string(),
        source_key: "switch_user_1".to_string(),
        external_id: "01007EF00011E000".to_string(),
        source_label: Some("Switch OLED".to_string()),
        title_hint: Some("The Legend of Zelda: Breath of the Wild".to_string()),
        playtime_seconds: 72000,
        peak_seen_seconds: 72000,
        launch_count: Some(50),
        first_played_at: Some(now),
        last_played_at: Some(now),
        source_exported_at: None,
        last_import_sha256: None,
        updated_at: now,
    };
    core.upsert_game_activity(&act).expect("upsert activity failed");

    let candidates = core.fetch_cover_candidates(&zelda_work.id, None).expect("fetch candidates failed");
    assert!(!candidates.is_empty(), "Should return candidates for Zelda");

    let nlib_candidate = candidates.iter().find(|c| c.provider == "nlib" && c.aspect_ratio == "1:1");
    assert!(nlib_candidate.is_some(), "Should find Nlib 1:1 square icon candidate");
    let c = nlib_candidate.unwrap();
    assert!(c.image_url.contains("01007EF00011E000/icon"));
}
