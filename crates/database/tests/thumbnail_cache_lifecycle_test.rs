use std::sync::Arc;
use chrono::Utc;
use looma_core::models::*;
use looma_core::LoomaCore;
use looma_core::AssetService;
use looma_database::LoomaDb;

#[test]
fn test_thumbnail_cache_lifecycle_and_invalidation() {
    let temp_vault = tempfile::tempdir().expect("failed to create tempdir");
    let db = Arc::new(LoomaDb::open(temp_vault.path()).expect("failed to open LoomaDb"));
    let core = LoomaCore::new(db.clone());

    // 1. Create a real test PNG image
    let img_dir = temp_vault.path().join("pictures");
    std::fs::create_dir_all(&img_dir).expect("failed to create img_dir");
    let img_path = img_dir.join("sample_cover.png");

    let mut img_buf = image::RgbaImage::new(400, 300);
    for pixel in img_buf.pixels_mut() {
        *pixel = image::Rgba([120, 80, 200, 255]);
    }
    img_buf.save_with_format(&img_path, image::ImageFormat::Png).expect("failed to save test image");
    let img_path_str = img_path.to_string_lossy().to_string();

    // 2. Register asset in database
    let asset = Asset {
        id: "ast_sample_cover".to_string(),
        kind: AssetKind::Image,
        source: AssetSource::Local,
        path: Some(img_path_str.clone()),
        size: Some(img_path.metadata().unwrap().len()),
        hash: Some("mock_hash_1234567890".to_string()),
        mime_type: Some("image/png".to_string()),
        metadata: serde_json::json!({}),
        status: AssetStatus::Active,
        created_at: Utc::now(),
        modified_at: Utc::now(),
        indexed_at: Utc::now(),
    };
    db.upsert_asset(&asset).expect("upsert_asset failed");

    // 3. First thumbnail generation (cold cache)
    let res1 = core.get_asset_thumbnail("ast_sample_cover", 128)
        .expect("get_asset_thumbnail failed")
        .expect("thumbnail result should be Some");

    assert_eq!(res1.asset_id, "ast_sample_cover");
    assert!(res1.is_newly_generated, "first call should generate new thumbnail");
    assert!(res1.data_url.starts_with("data:image/webp;base64,"));
    assert!(std::path::Path::new(&res1.cache_path).exists());
    assert!(res1.cache_path.ends_with(".webp"));

    // 4. Second thumbnail generation (warm cache hit)
    let res2 = core.get_asset_thumbnail("ast_sample_cover", 128)
        .expect("get_asset_thumbnail failed")
        .expect("thumbnail result should be Some");

    assert!(!res2.is_newly_generated, "second call should hit cache without regenerating");
    assert_eq!(res1.cache_path, res2.cache_path);
    assert_eq!(res1.data_url, res2.data_url);

    // 5. Inspect cache stats
    let stats = core.get_thumbnail_cache_stats().expect("get_thumbnail_cache_stats failed");
    assert!(stats.file_count >= 1);
    assert!(stats.total_bytes > 0);

    // 6. Clear cache
    let cleared = core.clear_thumbnail_cache().expect("clear_thumbnail_cache failed");
    assert!(cleared >= 1);

    let stats_after = core.get_thumbnail_cache_stats().expect("get_thumbnail_cache_stats after clear failed");
    assert_eq!(stats_after.file_count, 0);
    assert_eq!(stats_after.total_bytes, 0);

    // 7. Non-image asset test
    let code_asset = Asset {
        id: "ast_main_rs".to_string(),
        kind: AssetKind::Code,
        source: AssetSource::Local,
        path: Some(img_path_str),
        size: Some(100),
        hash: None,
        mime_type: Some("text/x-rust".to_string()),
        metadata: serde_json::json!({}),
        status: AssetStatus::Active,
        created_at: Utc::now(),
        modified_at: Utc::now(),
        indexed_at: Utc::now(),
    };
    db.upsert_asset(&code_asset).expect("upsert code asset failed");

    let res_code = core.get_asset_thumbnail("ast_main_rs", 128).expect("get thumbnail for code asset");
    assert!(res_code.is_none(), "non-image assets must return None");

    // 8. Missing asset path test
    let missing_asset = Asset {
        id: "ast_missing_file".to_string(),
        kind: AssetKind::Image,
        source: AssetSource::Local,
        path: Some("C:/non/existent/path/here.png".to_string()),
        size: Some(100),
        hash: None,
        mime_type: Some("image/png".to_string()),
        metadata: serde_json::json!({}),
        status: AssetStatus::Missing,
        created_at: Utc::now(),
        modified_at: Utc::now(),
        indexed_at: Utc::now(),
    };
    db.upsert_asset(&missing_asset).expect("upsert missing asset failed");

    let res_missing = core.get_asset_thumbnail("ast_missing_file", 128).expect("get thumbnail for missing asset");
    assert!(res_missing.is_none(), "missing image assets must return None gracefully");
}
