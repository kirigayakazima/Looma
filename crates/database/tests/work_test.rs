use std::sync::Arc;
use chrono::Utc;
use looma_core::models::*;
use looma_core::services::AssetService;
use looma_core::LoomaCore;
use looma_database::LoomaDb;
use serde_json::json;

#[test]
fn test_work_domain_lifecycle_and_summary() {
    let db = Arc::new(LoomaDb::in_memory().expect("in-memory db init failed"));
    let core = LoomaCore::new(db.clone());

    // 1. Create Anime Work (Scenario A: BLEACH)
    let anime = core.create_work(
        "test-suite",
        "BLEACH 千年血战篇",
        WorkType::Anime,
        RecordStatus::InProgress,
        Some("BLEACH 千年血戦篇".to_string()),
        Some("黑崎一护重返尸魂界的壮烈战役".to_string()),
    ).expect("failed to create anime work");

    assert_eq!(anime.title, "BLEACH 千年血战篇");
    assert_eq!(anime.entity_type, "anime");
    let meta = anime.as_work_metadata().expect("expected work metadata");
    assert_eq!(meta.work_type, WorkType::Anime);
    assert_eq!(meta.status, RecordStatus::InProgress);
    assert_eq!(meta.original_title.as_deref(), Some("BLEACH 千年血戦篇"));

    // 2. Create Person (Creator: 久保带人)
    let creator = Entity::new_person(
        "久保带人",
        Some("author"),
        Some("日本知名漫画家，《BLEACH》原作者".to_string()),
    );
    core.create_entity("test-suite", &creator).expect("failed to create creator entity");

    // Relate Work -> Creator (CREATED_BY)
    let creator_rel = Relation {
        id: uuid::Uuid::new_v4().to_string(),
        source_id: anime.id.clone(),
        source_type: "entity".to_string(),
        relation_type: relation_types::CREATED_BY.to_string(),
        target_id: creator.id.clone(),
        target_type: "entity".to_string(),
        metadata: json!({}),
        created_at: Utc::now(),
    };
    core.create_relation("test-suite", &creator_rel).expect("failed to link creator");

    // 3. Link Local Asset (Poster / Video Clip)
    let asset = Asset {
        id: "asset-bleach-01".to_string(),
        kind: AssetKind::Image,
        source: AssetSource::Local,
        path: Some("D:/Media/Anime/BLEACH/poster.jpg".to_string()),
        size: Some(2048),
        hash: Some("sha256-bleach-poster".to_string()),
        mime_type: Some("image/jpeg".to_string()),
        metadata: json!({"width": 1920, "height": 1080}),
        status: AssetStatus::Active,
        created_at: Utc::now(),
        modified_at: Utc::now(),
        indexed_at: Utc::now(),
    };
    db.upsert_asset(&asset).expect("upsert asset failed");

    core.link_work_asset("test-suite", &anime.id, &asset.id, Some(relation_types::ATTACHES))
        .expect("failed to link asset to work");

    // 4. Link Memory (Personal Review / Note)
    let memory = Memory {
        id: "mem-bleach-01".to_string(),
        title: "千年血战篇 ep1 观后感".to_string(),
        content: "开篇作画和配乐重置太震撼了，卍解特效完全拉满！".to_string(),
        category: Some("anime_review".to_string()),
        metadata: json!({"rating": 9.5}),
        recorded_at: Utc::now(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    core.create_memory("test-suite", &memory).expect("failed to create memory");

    core.link_work_memory("test-suite", &anime.id, &memory.id)
        .expect("failed to link memory to work");

    // 5. Add External References (Bangumi)
    let ext_bgm = ExternalReference {
        id: "ext-bgm-01".to_string(),
        entity_id: Some(anime.id.clone()),
        provider: "bangumi".to_string(),
        url: "https://bgm.tv/subject/371234".to_string(),
        title: "BLEACH 千年血戦篇 (Bangumi)".to_string(),
        description: Some("Bangumi 条目".to_string()),
        metadata: json!({"score": 8.4}),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    core.create_external_reference("test-suite", &ext_bgm).expect("create ext ref failed");

    // 6. Put into Collection
    let collection = Collection {
        id: "col-2022-fall".to_string(),
        title: "2022秋季追番".to_string(),
        description: Some("2022年10月新番个人追番表".to_string()),
        query: None,
        metadata: json!({"icon": "play-circle"}),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    core.create_collection("test-suite", &collection).expect("create collection failed");

    core.add_item_to_collection("test-suite", &collection.id, &anime.id, "entity")
        .expect("add to collection failed");

    // 7. Verify WorkSummary Aggregation
    let summary = core.get_work_summary(&anime.id).expect("get_work_summary failed").expect("summary missing");
    assert_eq!(summary.entity.id, anime.id);
    let work_meta = summary.work_metadata.as_ref().expect("work_metadata should exist");
    assert_eq!(work_meta.work_type, WorkType::Anime);
    assert_eq!(work_meta.status, RecordStatus::InProgress);

    // Verify relations contain creator
    assert!(summary.relations.iter().any(|r| r.target_id == creator.id && r.relation_type == relation_types::CREATED_BY));

    // Verify linked assets
    assert_eq!(summary.linked_assets.len(), 1);
    assert_eq!(summary.linked_assets[0].id, "asset-bleach-01");

    // Verify linked memories
    assert_eq!(summary.linked_memories.len(), 1);
    assert_eq!(summary.linked_memories[0].id, memory.id);
    assert_eq!(summary.linked_memories[0].title, "千年血战篇 ep1 观后感");

    // Verify linked collections
    assert_eq!(summary.linked_collections.len(), 1);
    assert_eq!(summary.linked_collections[0].title, "2022秋季追番");

    // Verify external references
    assert_eq!(summary.external_references.len(), 1);
    assert_eq!(summary.external_references[0].provider, "bangumi");

    // 8. Update Work Status: InProgress -> Completed
    let updated = core.update_work_status("test-suite", &anime.id, RecordStatus::Completed)
        .expect("failed to update work status");
    let updated_meta = updated.as_work_metadata().unwrap();
    assert_eq!(updated_meta.status, RecordStatus::Completed);

    // 9. Query list_works
    let all_works = core.list_works(None, None).expect("list_works failed");
    assert_eq!(all_works.len(), 1);

    let anime_works = core.list_works(Some("anime"), None).expect("list_works anime failed");
    assert_eq!(anime_works.len(), 1);

    let game_works = core.list_works(Some("game"), None).expect("list_works game failed");
    assert_eq!(game_works.len(), 0);

    let completed_works = core.list_works(None, Some("completed")).expect("list completed failed");
    assert_eq!(completed_works.len(), 1);

    let planned_works = core.list_works(None, Some("planned")).expect("list planned failed");
    assert_eq!(planned_works.len(), 0);

    // 10. Update and verify Work Progress (e.g. Episode 24/24)
    let prog_updated = core.update_work_progress(
        "test-suite",
        &anime.id,
        24.0,
        Some(ProgressPositionType::Episode),
        Some(24.0),
        Some("集".to_string()),
    ).expect("failed to update progress");

    let prog_meta = prog_updated.as_work_metadata().unwrap();
    let prog = prog_meta.progress.unwrap();
    assert_eq!(prog.position, 24.0);
    assert_eq!(prog.position_type, ProgressPositionType::Episode);
    assert_eq!(prog.total_positions, Some(24.0));
    assert_eq!(prog.unit.as_deref(), Some("集"));

    // Check progress is reflected in WorkSummary
    let final_summary = core.get_work_summary(&anime.id).unwrap().unwrap();
    assert_eq!(final_summary.work_metadata.unwrap().progress.unwrap().position, 24.0);
}

#[test]
fn test_game_library_extension_multi_location_and_cover() {
    let db = Arc::new(LoomaDb::in_memory().expect("in-memory db init failed"));
    let core = LoomaCore::new(db.clone());

    // 1. Create a Game Work
    let game = core.create_work(
        "test-suite",
        "NieR:Automata",
        WorkType::Game,
        RecordStatus::InProgress,
        Some("ニーア オートマタ".to_string()),
        Some("探讨存在主义与人造人悲剧命运的动作哲学巨作".to_string()),
    ).expect("failed to create game work");

    assert_eq!(game.entity_type, "game");
    let meta = game.as_work_metadata().expect("expected work metadata");
    assert_eq!(meta.work_type, WorkType::Game);
    assert_eq!(meta.status, RecordStatus::InProgress);

    // 2. Link Multiple Local Directories across different drives (E: and F:)
    let dir_primary = Asset {
        id: "asset_game_dir_e".to_string(),
        kind: AssetKind::Directory,
        source: AssetSource::Local,
        path: Some("E:/Games/NieR_Automata".to_string()),
        size: Some(48 * 1024 * 1024 * 1024),
        hash: None,
        mime_type: Some("inode/directory".to_string()),
        metadata: json!({ "drive": "E:" }),
        status: AssetStatus::Active,
        created_at: Utc::now(),
        modified_at: Utc::now(),
        indexed_at: Utc::now(),
    };
    db.upsert_asset(&dir_primary).expect("failed to index primary game dir");

    let dir_backup = Asset {
        id: "asset_game_dir_f".to_string(),
        kind: AssetKind::Directory,
        source: AssetSource::Local,
        path: Some("F:/Archive/Games/NieR_Automata".to_string()),
        size: Some(48 * 1024 * 1024 * 1024),
        hash: None,
        mime_type: Some("inode/directory".to_string()),
        metadata: json!({ "drive": "F:" }),
        status: AssetStatus::Active,
        created_at: Utc::now(),
        modified_at: Utc::now(),
        indexed_at: Utc::now(),
    };
    db.upsert_asset(&dir_backup).expect("failed to index backup game dir");

    // Link both directories to Game Work
    let rel_e = Relation {
        id: "rel_game_e".to_string(),
        source_id: game.id.clone(),
        source_type: "entity".to_string(),
        relation_type: relation_types::ATTACHES.to_string(),
        target_id: dir_primary.id.clone(),
        target_type: "asset".to_string(),
        metadata: json!({ "description": "Primary installation" }),
        created_at: Utc::now(),
    };
    core.create_relation("test-suite", &rel_e).expect("failed to link primary dir");

    let rel_f = Relation {
        id: "rel_game_f".to_string(),
        source_id: game.id.clone(),
        source_type: "entity".to_string(),
        relation_type: relation_types::ATTACHES.to_string(),
        target_id: dir_backup.id.clone(),
        target_type: "asset".to_string(),
        metadata: json!({ "description": "Cold archive backup" }),
        created_at: Utc::now(),
    };
    core.create_relation("test-suite", &rel_f).expect("failed to link backup dir");

    // 3. Link Cover Asset
    let cover_asset = Asset {
        id: "asset_nier_cover".to_string(),
        kind: AssetKind::Image,
        source: AssetSource::Local,
        path: Some("E:/Games/NieR_Automata/cover.png".to_string()),
        size: Some(1024 * 1024),
        hash: Some("sha256_nier_cover".to_string()),
        mime_type: Some("image/png".to_string()),
        metadata: json!({}),
        status: AssetStatus::Active,
        created_at: Utc::now(),
        modified_at: Utc::now(),
        indexed_at: Utc::now(),
    };
    db.upsert_asset(&cover_asset).expect("failed to index cover image");

    let cover_updated = core.update_work_cover_asset("test-suite", &game.id, Some(cover_asset.id.clone()))
        .expect("failed to update cover asset");
    assert_eq!(
        cover_updated.as_work_metadata().unwrap().cover_asset_id.as_deref(),
        Some("asset_nier_cover")
    );

    // 4. Add External Reference (Steam)
    let steam_ref = ExternalReference {
        id: "ref_steam_nier".to_string(),
        entity_id: Some(game.id.clone()),
        provider: "steam".to_string(),
        title: "Steam Store Page".to_string(),
        url: "https://store.steampowered.com/app/524220/NieRAutomata/".to_string(),
        description: Some("PlatinumGames Action RPG".to_string()),
        metadata: json!({}),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    core.create_external_reference("test-suite", &steam_ref).expect("failed to link steam ref");

    // 5. Verify Unified WorkSummary
    let summary = core.get_work_summary(&game.id).expect("get_work_summary failed").unwrap();
    assert_eq!(summary.entity.title, "NieR:Automata");
    assert_eq!(summary.linked_assets.len(), 2); // Both E: and F: directories
    assert_eq!(summary.external_references.len(), 1);
    assert_eq!(summary.external_references[0].provider, "steam");

    let meta = summary.work_metadata.unwrap();
    assert_eq!(meta.cover_asset_id.as_deref(), Some("asset_nier_cover"));

    // 6. Verify Game Query
    let games = core.list_works(Some("game"), None).expect("failed to list games");
    assert_eq!(games.len(), 1);
    assert_eq!(games[0].title, "NieR:Automata");
}
