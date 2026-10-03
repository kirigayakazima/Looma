use std::fs;
use tempfile::tempdir;

use looma_core::models::{AssetKind, AssetStatus};
use looma_core::services::AssetService;
use looma_database::LoomaDb;
use looma_scanner::{classify_file, compute_sha256, ScanOptions, Scanner};

#[test]
fn test_classifier() {
    let (kind, mime) = classify_file(std::path::Path::new("test.png"));
    assert_eq!(kind, AssetKind::Image);
    assert_eq!(mime.as_deref(), Some("image/png"));

    let (kind, mime) = classify_file(std::path::Path::new("document.pdf"));
    assert_eq!(kind, AssetKind::Document);
    assert_eq!(mime.as_deref(), Some("application/pdf"));

    let (kind, mime) = classify_file(std::path::Path::new("code.rs"));
    assert_eq!(kind, AssetKind::Code);
    assert_eq!(mime.as_deref(), Some("text/x-rust"));

    let (kind, mime) = classify_file(std::path::Path::new("movie.mp4"));
    assert_eq!(kind, AssetKind::Video);
    assert_eq!(mime.as_deref(), Some("video/mp4"));

    let (kind, mime) = classify_file(std::path::Path::new("music.mp3"));
    assert_eq!(kind, AssetKind::Audio);
    assert_eq!(mime.as_deref(), Some("audio/mpeg"));

    let (kind, mime) = classify_file(std::path::Path::new("archive.zip"));
    assert_eq!(kind, AssetKind::Archive);
    assert_eq!(mime.as_deref(), Some("application/zip"));
}

#[test]
fn test_hasher() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("sample.txt");
    fs::write(&file_path, "hello looma vault").unwrap();

    let hash = compute_sha256(&file_path).unwrap();
    assert_eq!(hash.len(), 64); // SHA-256 is 64 hex characters
}

#[test]
fn test_incremental_scanner() {
    let dir = tempdir().unwrap();
    let root = dir.path();

    // Create test files
    let file1 = root.join("photo1.jpg");
    let file2 = root.join("notes.md");
    fs::write(&file1, "binary photo content 1").unwrap();
    fs::write(&file2, "# Title\nMy first note").unwrap();

    let db = LoomaDb::in_memory().unwrap();
    let options = ScanOptions::default();

    // Pass 1: First scan should discover 2 new assets
    let summary1 = Scanner::scan_directory(&db, root, &options, None::<fn(_)>).unwrap();
    assert_eq!(summary1.total_scanned, 2);
    assert_eq!(summary1.new_assets, 2);
    assert_eq!(summary1.unchanged_assets, 0);
    assert_eq!(summary1.modified_assets, 0);
    assert_eq!(db.count_assets().unwrap(), 2);

    // Pass 2: Second scan with no changes should report 2 unchanged
    let summary2 = Scanner::scan_directory(&db, root, &options, None::<fn(_)>).unwrap();
    eprintln!("Pass 2 summary: {:?}", summary2);
    assert_eq!(summary2.total_scanned, 2);
    assert_eq!(summary2.new_assets, 0);
    assert_eq!(summary2.unchanged_assets, 2);
    assert_eq!(summary2.modified_assets, 0);

    // Pass 3: Modify file1
    std::thread::sleep(std::time::Duration::from_millis(1100)); // Ensure mtime changes
    fs::write(&file1, "updated binary photo content 1").unwrap();

    let summary3 = Scanner::scan_directory(&db, root, &options, None::<fn(_)>).unwrap();
    assert_eq!(summary3.total_scanned, 2);
    assert_eq!(summary3.new_assets, 0);
    assert_eq!(summary3.unchanged_assets, 1);
    assert_eq!(summary3.modified_assets, 1);

    // Pass 4: Delete file2 -> should be detected as missing
    fs::remove_file(&file2).unwrap();
    let summary4 = Scanner::scan_directory(&db, root, &options, None::<fn(_)>).unwrap();
    assert_eq!(summary4.total_scanned, 1);
    assert_eq!(summary4.missing_assets, 1);

    let missing_asset = db.get_asset_by_path(&looma_scanner::normalize_path(&file2)).unwrap().unwrap();
    assert_eq!(missing_asset.status, AssetStatus::Missing);
}
