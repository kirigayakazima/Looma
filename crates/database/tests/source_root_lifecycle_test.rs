use std::path::Path;
use std::sync::Arc;
use chrono::Utc;
use serde_json::json;
use tempfile::tempdir;

use looma_core::models::*;
use looma_core::services::*;
use looma_core::LoomaCore;
use looma_database::LoomaDb;
use looma_scanner::engine::Scanner;
use looma_scanner::options::ScanOptions;

fn helper_create_asset(id: &str, kind: AssetKind, path: Option<String>) -> Asset {
    Asset {
        id: id.to_string(),
        kind,
        path,
        size: Some(1024),
        hash: None,
        mime_type: None,
        source: AssetSource::Local,
        status: AssetStatus::Active,
        metadata: json!({}),
        created_at: Utc::now(),
        modified_at: Utc::now(),
        indexed_at: Utc::now(),
    }
}

#[test]
fn test_source_root_crud_and_inspect() {
    let vault_dir = tempdir().unwrap();
    let db = Arc::new(LoomaDb::open(vault_dir.path()).unwrap());
    let core = LoomaCore::new(db.clone());

    let media_dir = tempdir().unwrap();
    let root_path = media_dir.path().to_str().unwrap().replace('\\', "/");

    let created = core.create_source_root("test-actor", "Media Root", &root_path).unwrap();
    assert_eq!(created.label, "Media Root");
    assert_eq!(created.state, "online");

    let fetched = core.get_source_root(&created.id).unwrap().unwrap();
    assert_eq!(fetched.label, "Media Root");

    let list = core.list_source_roots().unwrap();
    assert_eq!(list.len(), 1);

    let inspection = core.inspect_source_root(&created.id).unwrap();
    assert!(inspection.exists);
    assert!(inspection.is_dir);
    assert!(inspection.accessible);
    assert_eq!(inspection.tracked_assets, 0);
}

#[test]
fn test_t08_work_and_experience_intact_when_directory_offline() {
    let vault_dir = tempdir().unwrap();
    let db = Arc::new(LoomaDb::open(vault_dir.path()).unwrap());
    let core = LoomaCore::new(db.clone());

    let work = core.create_work("test", "Bloodborne", WorkType::Game, RecordStatus::InProgress, None, None).unwrap();

    let exp = ExperienceEntry::new(&work.id, experience_events::COMPLETED)
        .with_title("完成首通");
    core.record_experience("test", &exp, &[], None).unwrap();

    // Verify work and experience intact even if physical storage is offline
    let summary = core.get_work_summary(&work.id).unwrap().unwrap();
    assert_eq!(summary.entity.id, work.id);
    assert_eq!(summary.experiences.len(), 1);
    assert_eq!(summary.experiences[0].event_type, "completed");
}

#[test]
fn test_t09_doctor_safe_isolation_for_offline_roots() {
    let vault_dir = tempdir().unwrap();
    let db = Arc::new(LoomaDb::open(vault_dir.path()).unwrap());

    // Create an offline root (non-existent path)
    let offline_root = SourceRoot::new("root_offline", "Offline External Drive", "X:/UnpluggedDrive/Games");
    db.create_source_root(&offline_root).unwrap();

    // Asset registered under this offline drive
    let asset = helper_create_asset("asset_offline_game", AssetKind::Directory, Some("X:/UnpluggedDrive/Games/EldenRing".to_string()));
    db.upsert_asset(&asset).unwrap();

    // Normal active asset
    let local_file = vault_dir.path().join("local.txt");
    std::fs::write(&local_file, "hello").unwrap();
    let local_asset = helper_create_asset("asset_local", AssetKind::Document, Some(local_file.to_str().unwrap().replace('\\', "/")));
    db.upsert_asset(&local_asset).unwrap();

    // Run doctor inspect: offline root's asset MUST NOT be marked missing per Section 9.4 and T09!
    let report = db.doctor_inspect().unwrap();
    assert_eq!(report.total_assets, 2);
    assert_eq!(report.missing_assets.len(), 0, "Assets under offline root must NOT be treated as missing");

    let cleaned = db.doctor_cleanup_missing().unwrap();
    assert_eq!(cleaned, 0);

    // Verify offline asset is still active in database
    let fetched = db.get_asset_by_id("asset_offline_game").unwrap().unwrap();
    assert_eq!(fetched.status, AssetStatus::Active);
}

#[test]
fn test_t11_rebind_source_root() {
    let vault_dir = tempdir().unwrap();
    let db = Arc::new(LoomaDb::open(vault_dir.path()).unwrap());
    let core = LoomaCore::new(db.clone());

    let root_e_dir = tempdir().unwrap();
    let root_e_path = root_e_dir.path().to_str().unwrap().replace('\\', "/");
    let file_e = root_e_dir.path().join("game.exe");
    std::fs::write(&file_e, "binary content").unwrap();

    let root = core.create_source_root("test", "Games Drive", &root_e_path).unwrap();

    let asset = helper_create_asset("asset_game_exe", AssetKind::Code, Some(format!("{root_e_path}/game.exe")));
    db.upsert_asset(&asset).unwrap();

    let mut lc = AssetLifecycle::new("asset_game_exe");
    lc.root_id = Some(root.id.clone());
    lc.relative_path = Some("game.exe".to_string());
    lc.availability = "online".to_string();
    db.set_asset_lifecycle(&lc).unwrap();

    // Also link to work and experience
    let work = core.create_work("test", "NieR: Automata", WorkType::Game, RecordStatus::InProgress, None, None).unwrap();
    let rel = Relation {
        id: "rel_work_asset".to_string(),
        source_id: work.id.clone(),
        source_type: "entity".to_string(),
        relation_type: "attaches".to_string(),
        target_id: "asset_game_exe".to_string(),
        target_type: "asset".to_string(),
        metadata: json!({}),
        created_at: Utc::now(),
    };
    db.create_relation(&rel).unwrap();

    let exp = ExperienceEntry::new(&work.id, experience_events::COMPLETED);
    core.record_experience("test", &exp, &[], None).unwrap();

    // Now rebind root to G: (represented by new temp dir)
    let root_g_dir = tempdir().unwrap();
    let root_g_path = root_g_dir.path().to_str().unwrap().replace('\\', "/");
    let file_g = root_g_dir.path().join("game.exe");
    std::fs::write(&file_g, "binary content in G").unwrap();

    let rebound = core.rebind_source_root("test", &root.id, &root_g_path).unwrap();
    assert_eq!(rebound.path, root_g_path);
    assert_eq!(rebound.state, "online");

    // Asset path must be updated to G:
    let updated_asset = db.get_asset_by_id("asset_game_exe").unwrap().unwrap();
    assert_eq!(updated_asset.path.unwrap(), format!("{root_g_path}/game.exe"));

    // Asset lifecycle availability must be online
    let updated_lc = core.get_asset_lifecycle("asset_game_exe").unwrap().unwrap();
    assert_eq!(updated_lc.availability, "online");

    // Work relation and Experience remain 100% intact
    let rels = db.list_relations_for_source(&work.id).unwrap();
    assert_eq!(rels.len(), 1);
    assert_eq!(rels[0].target_id, "asset_game_exe");

    let exps = core.list_experiences(&ExperienceFilter {
        subject_id: Some(work.id.clone()),
        ..Default::default()
    }).unwrap();
    assert_eq!(exps.len(), 1);
}

#[test]
fn test_t12_t13_untrack_asset_suppression_and_resume() {
    let vault_dir = tempdir().unwrap();
    let db = Arc::new(LoomaDb::open(vault_dir.path()).unwrap());
    let core = LoomaCore::new(db.clone());

    let media_dir = tempdir().unwrap();
    let root_path = media_dir.path().to_str().unwrap().replace('\\', "/");
    let file_path = media_dir.path().join("deleted_junk.tmp");
    std::fs::write(&file_path, "junk data").unwrap();

    let root = core.create_source_root("test", "Tmp Root", &root_path).unwrap();

    let asset = helper_create_asset("asset_junk", AssetKind::Document, Some(format!("{root_path}/deleted_junk.tmp")));
    db.upsert_asset(&asset).unwrap();

    let mut lc = AssetLifecycle::new("asset_junk");
    lc.root_id = Some(root.id.clone());
    lc.relative_path = Some("deleted_junk.tmp".to_string());
    db.set_asset_lifecycle(&lc).unwrap();

    // T12: User untracks asset -> creates suppression tombstone
    core.untrack_asset("test", "asset_junk", Some("User removed from vault")).unwrap();

    let updated_lc = core.get_asset_lifecycle("asset_junk").unwrap().unwrap();
    assert_eq!(updated_lc.tracking, "untracked");
    assert_eq!(updated_lc.untracked_reason.as_deref(), Some("User removed from vault"));

    let is_suppressed = db.is_asset_suppressed(&root.id, "deleted_junk.tmp").unwrap();
    assert!(is_suppressed, "Suppression tombstone must be active");

    // Scanner with suppression list must NOT re-index or revive this file
    let suppressions = core.list_suppressions(Some(&root.id)).unwrap();
    let suppressed_paths: Vec<String> = suppressions.into_iter().map(|s| s.relative_path).collect();
    let options = ScanOptions {
        suppressed_relative_paths: suppressed_paths,
        ..Default::default()
    };

    let summary = Scanner::scan_directory(&*db, media_dir.path(), &options, None::<fn(_)>).unwrap();
    assert_eq!(summary.suppressed_assets, 1, "Scanner must skip suppressed file");

    // T13: Resume tracking -> suppression is cleared
    core.resume_tracking("test", "asset_junk").unwrap();
    let resumed_lc = core.get_asset_lifecycle("asset_junk").unwrap().unwrap();
    assert_eq!(resumed_lc.tracking, "tracked");
    assert_eq!(resumed_lc.untracked_reason, None);

    let is_still_suppressed = db.is_asset_suppressed(&root.id, "deleted_junk.tmp").unwrap();
    assert!(!is_still_suppressed, "Suppression must be removed on resume");
}

#[test]
fn test_t14_archive_and_unarchive_asset() {
    let vault_dir = tempdir().unwrap();
    let db = Arc::new(LoomaDb::open(vault_dir.path()).unwrap());
    let core = LoomaCore::new(db.clone());

    let dummy_file = vault_dir.path().join("archived_doc.pdf");
    std::fs::write(&dummy_file, "pdf bytes").unwrap();

    let asset = helper_create_asset("asset_pdf", AssetKind::Document, Some(dummy_file.to_str().unwrap().replace('\\', "/")));
    db.upsert_asset(&asset).unwrap();

    // Archive asset
    core.archive_asset("test", "asset_pdf").unwrap();
    let fetched = db.get_asset_by_id("asset_pdf").unwrap().unwrap();
    assert_eq!(fetched.status, AssetStatus::Archived);

    let lc = core.get_asset_lifecycle("asset_pdf").unwrap().unwrap();
    assert_eq!(lc.visibility, "archived");

    // Original file on disk must be untouched!
    assert!(dummy_file.exists());

    // Unarchive asset
    core.unarchive_asset("test", "asset_pdf").unwrap();
    let fetched_again = db.get_asset_by_id("asset_pdf").unwrap().unwrap();
    assert_eq!(fetched_again.status, AssetStatus::Active);

    let lc_again = core.get_asset_lifecycle("asset_pdf").unwrap().unwrap();
    assert_eq!(lc_again.visibility, "active");
}

#[test]
fn test_t18_backup_and_restore_roots_and_suppressions() {
    let vault_dir = tempdir().unwrap();
    let db = Arc::new(LoomaDb::open(vault_dir.path()).unwrap());
    let core = LoomaCore::new(db.clone());

    let root = core.create_source_root("test", "Backup Root", "D:/Media").unwrap();
    db.add_suppression(&root.id, "junk/cache.tmp", Some("ignore cache")).unwrap();

    let work = core.create_work("test", "Backup Game", WorkType::Game, RecordStatus::InProgress, None, None).unwrap();
    let exp = ExperienceEntry::new(&work.id, "completed");
    core.record_experience("test", &exp, &[], None).unwrap();

    let backup_dir = tempdir().unwrap();
    let res = core.export_backup("test", backup_dir.path()).unwrap();
    assert!(Path::new(&res.backup_path).join("manifest.json").exists());
    assert!(Path::new(&res.backup_path).join("vault_dump.json").exists());

    // Restore to fresh vault
    let fresh_vault_dir = tempdir().unwrap();
    let fresh_db = Arc::new(LoomaDb::open(fresh_vault_dir.path()).unwrap());
    let fresh_core = LoomaCore::new(fresh_db.clone());
    let restore_res = fresh_core.restore_backup("test", &Path::new(&res.backup_path)).unwrap();

    assert_eq!(restore_res.restored_experiences, 1);
    assert_eq!(restore_res.restored_source_roots, 1);
    assert_eq!(restore_res.restored_asset_suppressions, 1);

    let restored_root = fresh_core.get_source_root(&root.id).unwrap().unwrap();
    assert_eq!(restored_root.label, "Backup Root");

    let is_suppressed = fresh_db.is_asset_suppressed(&root.id, "junk/cache.tmp").unwrap();
    assert!(is_suppressed);
}
