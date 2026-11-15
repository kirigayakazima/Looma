use looma_core::models::*;
use looma_core::LoomaCore;
use looma_database::LoomaDb;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::tempdir;

fn create_sample_png(path: &std::path::Path, width: u32, height: u32) {
    let mut imgbuf = image::RgbImage::new(width, height);
    for (x, y, pixel) in imgbuf.enumerate_pixels_mut() {
        let r = (0.3 * x as f32) as u8;
        let b = (0.3 * y as f32) as u8;
        *pixel = image::Rgb([r, 120, b]);
    }
    imgbuf.save(path).unwrap();
}

fn setup_test_vault() -> (tempfile::TempDir, LoomaCore, PathBuf) {
    let dir = tempdir().unwrap();
    let vault_path = dir.path().to_path_buf();
    let db = Arc::new(LoomaDb::open(&vault_path).unwrap());
    let core = LoomaCore::new(db);
    (dir, core, vault_path)
}

#[test]
fn test_managed_cover_lifecycle_and_backup_restore() {
    let (_vault_dir, core, vault_path) = setup_test_vault();

    // 1. Create a game work
    let work = core.create_work(
        "tester",
        "Elden Ring",
        WorkType::Game,
        RecordStatus::InProgress,
        Some("ELDEN RING".to_string()),
        Some("Action RPG".to_string()),
    ).unwrap();
    assert_eq!(work.as_work_metadata().unwrap().cover_asset_id, None);

    // 2. Prepare an external sample image (vertical 2:3 poster, 300x450)
    let temp_img_dir = tempdir().unwrap();
    let sample_img_path = temp_img_dir.path().join("elden_ring_poster.png");
    create_sample_png(&sample_img_path, 300, 450);

    // 3. Set cover from file
    let asset = core.set_work_cover_from_file(
        "tester",
        &work.id,
        &sample_img_path,
        true, // user_selected = true
    ).unwrap();

    assert_eq!(asset.kind, AssetKind::Image);
    assert_eq!(asset.source, AssetSource::Managed);
    assert_eq!(asset.metadata["role"], "work_cover");
    assert_eq!(asset.metadata["user_selected"], true);
    assert_eq!(asset.metadata["aspect_ratio"], "2:3");

    // Verify file copied to .looma/media/covers/<hash>.png
    let covers_dir = vault_path.join(".looma").join("media").join("covers");
    let cover_file = covers_dir.join(format!("{}.png", asset.hash.as_ref().unwrap()));
    assert!(cover_file.exists());
    assert_eq!(fs::metadata(&cover_file).unwrap().len(), asset.size.unwrap());

    // Verify work now points to cover asset
    let updated_work = core.get_entity(&work.id).unwrap().unwrap();
    assert_eq!(updated_work.as_work_metadata().unwrap().cover_asset_id.as_deref(), Some(asset.id.as_str()));

    // 4. Test Thumbnail Generation from Managed Cover
    let thumb = core.get_asset_thumbnail(&asset.id, 200).unwrap();
    assert!(thumb.is_some(), "Thumbnail generation for managed cover should succeed");
    let thumb_res = thumb.unwrap();
    assert!(thumb_res.data_url.len() > 50);

    // 5. Export Backup (must bundle managed cover bytes and checksums)
    let export_dir = tempdir().unwrap();
    let backup_res = core.export_backup("tester", export_dir.path()).unwrap();
    let backup_path = PathBuf::from(&backup_res.backup_path);

    let rel_cover_key = format!("media/covers/{}.png", asset.hash.as_ref().unwrap());
    assert!(backup_res.manifest.checksums.contains_key(&rel_cover_key), "Manifest must contain cover checksum");
    let bundled_cover = backup_path.join("media").join("covers").join(format!("{}.png", asset.hash.as_ref().unwrap()));
    assert!(bundled_cover.exists(), "Backup directory must contain actual cover file bytes");

    // 6. Fault Injection: Tamper with bundled cover file bytes -> restore must reject
    let original_bytes = fs::read(&bundled_cover).unwrap();
    let mut tampered_bytes = original_bytes.clone();
    tampered_bytes[10] ^= 0xFF; // flip bits
    fs::write(&bundled_cover, &tampered_bytes).unwrap();

    let (_dir_tampered, core_tampered, _path_tampered) = setup_test_vault();
    let restore_err = core_tampered.restore_backup("tester", &backup_path);
    assert!(restore_err.is_err(), "Restoring tampered cover should fail checksum validation");
    let err_str = restore_err.unwrap_err().to_string();
    assert!(err_str.contains("Checksum mismatch for backup cover"), "Error should explain checksum mismatch: {err_str}");

    // Revert to valid bytes
    fs::write(&bundled_cover, &original_bytes).unwrap();

    // 7. Restore to completely new Vault
    let (_dir_new, core_new, path_new) = setup_test_vault();
    let restore_res = core_new.restore_backup("tester", &backup_path).unwrap();
    assert!(restore_res.restored_assets >= 1);
    assert!(restore_res.restored_entities >= 1);

    // Verify restored Vault has the cover on disk
    let restored_cover = path_new.join(".looma").join("media").join("covers").join(format!("{}.png", asset.hash.as_ref().unwrap()));
    assert!(restored_cover.exists(), "Restored vault must have restored cover file");
    assert_eq!(fs::read(&restored_cover).unwrap(), original_bytes);

    // Verify restored Work still links to cover Asset
    let restored_work = core_new.get_entity(&work.id).unwrap().unwrap();
    assert_eq!(restored_work.as_work_metadata().unwrap().cover_asset_id.as_deref(), Some(asset.id.as_str()));

    // Verify thumbnail generation in restored vault works
    let thumb_new = core_new.get_asset_thumbnail(&asset.id, 200).unwrap();
    assert!(thumb_new.is_some(), "Thumbnail generation in restored vault must succeed");

    // 8. Cover Removal: unlinks from work, but preserves Asset and disk file
    core.remove_work_cover("tester", &work.id).unwrap();
    let unlinked_work = core.get_entity(&work.id).unwrap().unwrap();
    assert_eq!(unlinked_work.as_work_metadata().unwrap().cover_asset_id, None);
    assert!(cover_file.exists(), "Underlying cover file must not be prematurely deleted");
    assert!(core.get_asset(&asset.id).unwrap().is_some(), "Underlying asset record must remain");
}
