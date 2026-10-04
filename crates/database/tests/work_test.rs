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

#[test]
fn test_game_library_patch_data_integrity_and_directory_assets() {
    let temp_root = std::env::temp_dir().join(format!("looma_patch_test_{}", uuid::Uuid::new_v4()));
    let game_a_dir = temp_root.join("Games").join("GameA");
    let game_b_dir = temp_root.join("Games").join("GameB");
    let game_c_dir = temp_root.join("Backup").join("GameA");
    std::fs::create_dir_all(&game_a_dir).expect("failed to create game_a_dir");
    std::fs::create_dir_all(&game_b_dir).expect("failed to create game_b_dir");
    std::fs::create_dir_all(&game_c_dir).expect("failed to create game_c_dir");

    let db_path = temp_root.join("vault_db");
    std::fs::create_dir_all(&db_path).expect("failed to create db dir");

    let game_a_str = game_a_dir.to_string_lossy().to_string();
    let game_c_str = game_c_dir.to_string_lossy().to_string();

    let (created_game_id, created_asset_a_id, created_asset_c_id) = {
        let db = Arc::new(LoomaDb::open(&db_path).expect("open db failed"));
        let core = LoomaCore::new(db.clone());

        // Test 1: Link Unindexed Directory -> creates real AssetKind::Directory and Relation
        let game = core.create_work(
            "test",
            "Elden Ring",
            WorkType::Game,
            RecordStatus::InProgress,
            None,
            None,
        ).expect("failed to create game");

        let (asset_a, rel_a) = core.link_work_directory("test", &game.id, &game_a_str)
            .expect("failed to link unindexed directory");

        assert_eq!(asset_a.kind, AssetKind::Directory);
        assert_eq!(asset_a.status, AssetStatus::Active);
        assert_eq!(rel_a.source_id, game.id);
        assert_eq!(rel_a.target_id, asset_a.id);

        // Verify target asset actually exists in database
        let fetched_asset = core.get_asset(&asset_a.id).expect("get_asset failed").expect("asset should exist");
        assert_eq!(fetched_asset.id, asset_a.id);

        // Test 2 & 3: Idempotent ensure_directory_asset & Path normalization
        let slash_variant = format!("{}/", game_a_str.replace('/', "\\"));
        let asset_a_again = core.ensure_directory_asset("test", &slash_variant)
            .expect("ensure directory idempotent failed");
        assert_eq!(asset_a_again.id, asset_a.id, "same path should return exact same asset");

        // Test 4: Multiple Locations for the same Game
        let (asset_c, _rel_c) = core.link_work_directory("test", &game.id, &game_c_str)
            .expect("failed to link second location");
        assert_ne!(asset_a.id, asset_c.id);

        let summary = core.get_work_summary(&game.id).expect("summary failed").unwrap();
        assert_eq!(summary.linked_assets.len(), 2);
        assert!(summary.linked_assets.iter().any(|a| a.id == asset_a.id));
        assert!(summary.linked_assets.iter().any(|a| a.id == asset_c.id));

        // Test 8: Data Integrity Invariant - No Orphan Relations
        let fake_asset_id = "asset_dir_nonexistent_12345";
        let orphan_attempt = core.link_work_asset("test", &game.id, fake_asset_id, None);
        assert!(orphan_attempt.is_err(), "Linking nonexistent asset MUST fail");

        // Test Case C: Nonexistent directory path fails
        let nonexistent_path = temp_root.join("NonexistentDir_XYZ").to_string_lossy().to_string();
        let nonexist_attempt = core.ensure_directory_asset("test", &nonexistent_path);
        assert!(nonexist_attempt.is_err(), "Ensuring nonexistent path MUST fail");

        (game.id, asset_a.id, asset_c.id)
    };

    // Test 6: Restart Persistence (reopen DB from disk)
    {
        let db = Arc::new(LoomaDb::open(&db_path).expect("reopen db failed"));
        let core = LoomaCore::new(db.clone());

        let games = core.list_works(Some("game"), None).expect("list works failed");
        assert_eq!(games.len(), 1);
        assert_eq!(games[0].id, created_game_id);

        let summary = core.get_work_summary(&created_game_id).expect("summary failed").unwrap();
        assert_eq!(summary.linked_assets.len(), 2);
        assert!(summary.linked_assets.iter().any(|a| a.id == created_asset_a_id));
        assert!(summary.linked_assets.iter().any(|a| a.id == created_asset_c_id));

        // Test 7: Missing Directory Detection
        // Remove game_a_dir from disk to simulate missing/disconnected drive
        std::fs::remove_dir_all(&game_a_dir).expect("failed to remove game_a_dir");

        let summary_after_remove = core.get_work_summary(&created_game_id).expect("summary failed").unwrap();
        // Work still exists!
        assert_eq!(summary_after_remove.entity.id, created_game_id);
        // Relations still exist!
        assert_eq!(summary_after_remove.linked_assets.len(), 2);
        // The deleted directory is marked as Missing!
        let missing_asset = summary_after_remove.linked_assets.iter().find(|a| a.id == created_asset_a_id)
            .expect("asset a should still be linked");
        assert_eq!(missing_asset.status, AssetStatus::Missing);

        // Intact directory remains Active
        let active_asset = summary_after_remove.linked_assets.iter().find(|a| a.id == created_asset_c_id)
            .expect("asset c should still be linked");
        assert_eq!(active_asset.status, AssetStatus::Active);
    }

    // Cleanup test temp directory
    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
fn test_anime_work_lifecycle_without_local_asset() {
    // Verifies Guardrails Section 4, 30: Work can exist purely as a digital record without any local asset.
    let db = Arc::new(LoomaDb::in_memory().expect("in-memory db init failed"));
    let core = LoomaCore::new(db.clone());

    // 1. Create Anime Work (Frieren) with no local assets
    let anime = core.create_work(
        "test-suite",
        "葬送的芙莉莲",
        WorkType::Anime,
        RecordStatus::InProgress,
        Some("葬送のフリーレン".to_string()),
        Some("千年精灵魔法使追寻生命意义的旅行".to_string()),
    ).expect("failed to create anime work");

    // 2. Set type-specific metadata (season, studio)
    let anime_meta = TypeSpecificMetadata::Anime(AnimeSpecificMeta {
        season: Some("2023-10".to_string()),
        total_episodes: Some(28),
        anime_format: Some("tv".to_string()),
        studio: Some("Madhouse".to_string()),
        broadcast_day: Some("Friday".to_string()),
    });
    core.update_work_type_metadata("test-suite", &anime.id, Some(anime_meta.clone()))
        .expect("update type metadata failed");

    // 3. Add ExternalReference (Bangumi & Official website)
    let ref_bgm = ExternalReference {
        id: "ref-frieren-bgm".to_string(),
        entity_id: Some(anime.id.clone()),
        provider: "bangumi".to_string(),
        title: "Bangumi 条目".to_string(),
        url: "https://bgm.tv/subject/399999".to_string(),
        description: None,
        metadata: json!({"score": 8.9}),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    core.create_external_reference("test-suite", &ref_bgm).expect("create ext ref failed");

    // 4. Update Progress to Episode 16
    core.update_work_progress(
        "test-suite",
        &anime.id,
        16.0,
        Some(ProgressPositionType::Episode),
        Some(28.0),
        Some("集".to_string()),
    ).expect("update progress failed");

    // 5. Add Memory note
    let note = Memory {
        id: "mem-frieren-01".to_string(),
        title: "第16集观后感：长寿种与短命种的羁绊".to_string(),
        content: "克拉夫特与芙莉莲在暴风雪木屋中的长谈太具有神性了。".to_string(),
        category: Some("anime_review".to_string()),
        metadata: json!({"rating": 9.8}),
        recorded_at: Utc::now(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    core.create_memory("test-suite", &note).expect("create memory failed");
    core.link_work_memory("test-suite", &anime.id, &note.id).expect("link memory failed");

    // 6. Assert complete WorkProfile
    let summary = core.get_work_summary(&anime.id).expect("get summary failed").unwrap();
    assert_eq!(summary.entity.title, "葬送的芙莉莲");
    assert_eq!(summary.entity.entity_type, "anime");

    // Invariant: Zero local assets
    assert_eq!(summary.linked_assets.len(), 0, "Work without local assets MUST have 0 assets");

    // Invariant: Progress, references, memories all present
    let work_meta = summary.work_metadata.expect("work_metadata must exist");
    assert_eq!(work_meta.work_type, WorkType::Anime);
    assert_eq!(work_meta.status, RecordStatus::InProgress);
    assert_eq!(work_meta.type_metadata, Some(anime_meta));

    let progress = work_meta.progress.expect("progress must exist");
    assert_eq!(progress.position, 16.0);
    assert_eq!(progress.total_positions, Some(28.0));
    assert_eq!(progress.unit.as_deref(), Some("集"));

    assert_eq!(summary.external_references.len(), 1);
    assert_eq!(summary.external_references[0].provider, "bangumi");

    assert_eq!(summary.linked_memories.len(), 1);
    assert_eq!(summary.linked_memories[0].title, "第16集观后感：长寿种与短命种的羁绊");
}

#[test]
fn test_manga_work_lifecycle_with_unknown_total() {
    // Verifies Guardrails Section 7: Ongoing works with total_positions = None (serialized manga)
    let db = Arc::new(LoomaDb::in_memory().expect("in-memory db init failed"));
    let core = LoomaCore::new(db.clone());

    // 1. Create Manga Work (Chainsaw Man Part 2)
    let manga = core.create_work(
        "test-suite",
        "电锯人 第二部",
        WorkType::Manga,
        RecordStatus::InProgress,
        Some("チェンソーマン 第二部".to_string()),
        Some("藤本树连载中的黑暗奇幻少年漫画".to_string()),
    ).expect("failed to create manga work");

    let manga_meta = TypeSpecificMetadata::Manga(MangaSpecificMeta {
        total_volumes: Some(18),
        total_chapters: None, // ongoing, unknown total
        author: Some("藤本树".to_string()),
        magazine: Some("少年JUMP+".to_string()),
    });
    core.update_work_type_metadata("test-suite", &manga.id, Some(manga_meta.clone()))
        .expect("update type metadata failed");

    // 2. Track ongoing progress without total
    core.update_work_progress(
        "test-suite",
        &manga.id,
        142.0,
        Some(ProgressPositionType::Chapter),
        None, // total_positions is explicitly None!
        Some("话".to_string()),
    ).expect("update progress failed");

    // 3. Step forward +1 (143话)
    core.update_work_progress(
        "test-suite",
        &manga.id,
        143.0,
        Some(ProgressPositionType::Chapter),
        None,
        Some("话".to_string()),
    ).expect("step progress failed");

    let summary = core.get_work_summary(&manga.id).expect("get summary failed").unwrap();
    let meta = summary.work_metadata.unwrap();
    assert_eq!(meta.work_type, WorkType::Manga);
    assert_eq!(meta.type_metadata, Some(manga_meta));

    let prog = meta.progress.unwrap();
    assert_eq!(prog.position, 143.0);
    assert_eq!(prog.total_positions, None, "Ongoing serialized manga total positions must remain None");
    assert_eq!(prog.unit.as_deref(), Some("话"));
}

#[test]
fn test_book_work_lifecycle_and_type_metadata() {
    // Verifies Book type with ISBN, author, translator, and page tracking
    let db = Arc::new(LoomaDb::in_memory().expect("in-memory db init failed"));
    let core = LoomaCore::new(db.clone());

    let book = core.create_work(
        "test-suite",
        "Rust程序设计",
        WorkType::Book,
        RecordStatus::InProgress,
        Some("Programming Rust 2nd Edition".to_string()),
        None,
    ).expect("failed to create book");

    let book_meta = TypeSpecificMetadata::Book(BookSpecificMeta {
        isbn: Some("9787111685876".to_string()),
        author: Some("Jim Blandy, Jason Orendorff".to_string()),
        translator: Some("陈小杰".to_string()),
        publisher: Some("机械工业出版社".to_string()),
        total_pages: Some(650),
    });
    core.update_work_type_metadata("test-suite", &book.id, Some(book_meta.clone())).unwrap();

    core.update_work_progress(
        "test-suite",
        &book.id,
        280.0,
        Some(ProgressPositionType::Page),
        Some(650.0),
        Some("页".to_string()),
    ).unwrap();

    let summary = core.get_work_summary(&book.id).unwrap().unwrap();
    let meta = summary.work_metadata.unwrap();
    assert_eq!(meta.work_type, WorkType::Book);
    assert_eq!(meta.type_metadata, Some(book_meta));

    let prog = meta.progress.unwrap();
    assert_eq!(prog.position, 280.0);
    assert_eq!(prog.total_positions, Some(650.0));
    assert_eq!(prog.position_type, ProgressPositionType::Page);
    assert_eq!(prog.unit.as_deref(), Some("页"));
}

#[test]
fn test_work_hybrid_local_and_external_with_restart_persistence() {
    // Verifies Guardrails Section 31, 32: 1 Work with multiple local directories + external references + reopen DB
    let temp_root = std::env::temp_dir().join(format!("looma_hybrid_test_{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&temp_root).expect("create temp root failed");

    let db_path = temp_root.join("hybrid_test.db");
    let loc_a = temp_root.join("BLEACH_Season1");
    let loc_b = temp_root.join("BLEACH_Archive");
    std::fs::create_dir_all(&loc_a).unwrap();
    std::fs::create_dir_all(&loc_b).unwrap();

    let work_id: String;
    let asset_a_id: String;
    let asset_b_id: String;

    {
        let db = Arc::new(LoomaDb::open(&db_path).unwrap());
        let core = LoomaCore::new(db.clone());

        let work = core.create_work(
            "test",
            "BLEACH",
            WorkType::Anime,
            RecordStatus::Completed,
            Some("BLEACH".to_string()),
            None,
        ).unwrap();
        work_id = work.id.clone();

        // 2 local directories linked to the SAME work
        let (ast_a, _) = core.link_work_directory("test", &work.id, &loc_a.to_string_lossy()).unwrap();
        let (ast_b, _) = core.link_work_directory("test", &work.id, &loc_b.to_string_lossy()).unwrap();
        asset_a_id = ast_a.id.clone();
        asset_b_id = ast_b.id.clone();

        // 2 external references
        let ref_1 = ExternalReference {
            id: "ref-h1".to_string(),
            entity_id: Some(work.id.clone()),
            provider: "bangumi".to_string(),
            title: "Bangumi".to_string(),
            url: "https://bgm.tv/subject/1".to_string(),
            description: None,
            metadata: json!({}),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let ref_2 = ExternalReference {
            id: "ref-h2".to_string(),
            entity_id: Some(work.id.clone()),
            provider: "official".to_string(),
            title: "Official".to_string(),
            url: "https://bleach-anime.com".to_string(),
            description: None,
            metadata: json!({}),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        core.create_external_reference("test", &ref_1).unwrap();
        core.create_external_reference("test", &ref_2).unwrap();
    }

    // Reopen DB from disk
    {
        let db = Arc::new(LoomaDb::open(&db_path).unwrap());
        let core = LoomaCore::new(db.clone());

        let summary = core.get_work_summary(&work_id).unwrap().unwrap();
        assert_eq!(summary.entity.id, work_id);
        assert_eq!(summary.linked_assets.len(), 2, "1 Work MUST maintain 2 directory assets across restart");
        assert!(summary.linked_assets.iter().any(|a| a.id == asset_a_id));
        assert!(summary.linked_assets.iter().any(|a| a.id == asset_b_id));

        assert_eq!(summary.external_references.len(), 2, "1 Work MUST maintain 2 external references across restart");
    }

    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
fn test_missing_asset_integrity_and_generic_relocate() {
    // Verifies Guardrails Section 33, 34: Missing folder does not delete Work, and relocate preserves Work Identity
    let temp_root = std::env::temp_dir().join(format!("looma_relocate_test_{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&temp_root).expect("create temp root failed");

    let old_dir = temp_root.join("Movie_Old");
    let new_dir = temp_root.join("Movie_New");
    std::fs::create_dir_all(&old_dir).unwrap();
    std::fs::create_dir_all(&new_dir).unwrap();

    let db = Arc::new(LoomaDb::in_memory().unwrap());
    let core = LoomaCore::new(db.clone());

    let movie = core.create_work(
        "test",
        "奥本海默",
        WorkType::Movie,
        RecordStatus::Completed,
        Some("Oppenheimer".to_string()),
        None,
    ).unwrap();

    // Link old directory
    let (old_asset, _) = core.link_work_directory("test", &movie.id, &old_dir.to_string_lossy()).unwrap();
    let old_asset_id = old_asset.id.clone();

    // 1. Delete folder from disk -> triggers Missing status on asset, BUT does NOT delete Work or Relation!
    std::fs::remove_dir_all(&old_dir).unwrap();

    let summary_missing = core.get_work_summary(&movie.id).unwrap().unwrap();
    assert_eq!(summary_missing.entity.id, movie.id, "Work MUST NOT be deleted when asset path is missing");
    assert_eq!(summary_missing.linked_assets.len(), 1, "Relation MUST NOT be deleted when asset is missing");
    assert_eq!(summary_missing.linked_assets[0].status, AssetStatus::Missing);

    // 2. Perform generic relocation
    let (new_asset, new_rel) = core.relocate_work_directory(
        "test",
        &movie.id,
        &old_asset_id,
        &new_dir.to_string_lossy(),
    ).expect("relocation must succeed");

    assert_eq!(new_rel.relation_type, relation_types::ATTACHES);

    // 3. Confirm Work identity is preserved and new asset is active
    let summary_relocated = core.get_work_summary(&movie.id).unwrap().unwrap();
    assert_eq!(summary_relocated.entity.id, movie.id, "Work ID must remain identical after relocation");
    assert_eq!(summary_relocated.linked_assets.len(), 1);
    assert_eq!(summary_relocated.linked_assets[0].id, new_asset.id);
    assert_eq!(summary_relocated.linked_assets[0].status, AssetStatus::Active);

    let _ = std::fs::remove_dir_all(&temp_root);
}
