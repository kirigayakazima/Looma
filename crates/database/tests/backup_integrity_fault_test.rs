use std::fs;
use std::sync::Arc;
use tempfile::tempdir;

use looma_core::error::LoomaError;
use looma_core::models::*;
use looma_core::LoomaCore;
use looma_database::LoomaDb;

#[test]
fn test_backup_integrity_and_fault_injection() {
    let vault_dir = tempdir().unwrap();
    let db = Arc::new(LoomaDb::open(vault_dir.path()).unwrap());
    let core = LoomaCore::new(db.clone());

    // 1. Populate rich vault data
    let work = core
        .create_work("author", "Elden Ring", WorkType::Game, RecordStatus::Completed, None, None)
        .unwrap();

    let exp = ExperienceEntry::new(&work.id, experience_events::COMPLETED)
        .with_title("Elden Ring 100% Achievement")
        .with_occurred_day(2026, 2, 25);
    let recorded_exp = core.record_experience("author", &exp, &[], None).unwrap();

    let root = core
        .create_source_root("author", "Steam Games", "D:/SteamLibrary")
        .unwrap();

    core.add_suppression(&root.id, "common/temp.log", Some("Ignore log")).unwrap();

    // 2. Export backup
    let export_dir = tempdir().unwrap();
    let backup_res = core.export_backup("author", export_dir.path()).unwrap();
    let backup_path = std::path::PathBuf::from(&backup_res.backup_path);

    // Verify manifest schema_version is 4 (all 4 migrations applied)
    assert_eq!(backup_res.manifest.schema_version, 4);
    assert!(backup_res.manifest.checksums.contains_key("vault_dump.json"));

    // 3. Fault Injection 1: Tamper with vault_dump.json content
    let dump_file = backup_path.join("vault_dump.json");
    let mut dump_bytes = fs::read(&dump_file).unwrap();
    // Invert one byte in the middle of dump
    let mid = dump_bytes.len() / 2;
    dump_bytes[mid] ^= 0xFF;

    let tampered_dir = tempdir().unwrap();
    let tampered_backup = tampered_dir.path().join("corrupted_backup");
    fs::create_dir_all(&tampered_backup).unwrap();
    fs::copy(backup_path.join("manifest.json"), tampered_backup.join("manifest.json")).unwrap();
    fs::write(tampered_backup.join("vault_dump.json"), &dump_bytes).unwrap();

    let fresh_vault_dir = tempdir().unwrap();
    let fresh_db = Arc::new(LoomaDb::open(fresh_vault_dir.path()).unwrap());
    let fresh_core = LoomaCore::new(fresh_db);

    let restore_err = fresh_core.restore_backup("tester", &tampered_backup);
    assert!(
        restore_err.is_err(),
        "Restoring tampered vault_dump.json must be rejected by checksum verification!"
    );
    match restore_err.err().unwrap() {
        LoomaError::Validation(msg) => {
            assert!(msg.contains("checksum mismatch"), "Error message was: {msg}");
        }
        other => panic!("Expected Validation error, got: {other:?}"),
    }

    // 4. Fault Injection 2: Missing vault_dump.json
    let empty_backup_dir = tempdir().unwrap();
    let missing_err = fresh_core.restore_backup("tester", empty_backup_dir.path());
    assert!(missing_err.is_err());
    match missing_err.err().unwrap() {
        LoomaError::Vault(msg) => assert!(msg.contains("vault_dump.json not found")),
        other => panic!("Expected Vault error, got: {other:?}"),
    }

    // 5. Restore original valid backup into fresh vault
    let restore_res = fresh_core.restore_backup("tester", &backup_path).expect("Valid backup must restore cleanly");
    assert_eq!(restore_res.restored_entities, 1);
    assert_eq!(restore_res.restored_experiences, 1);
    assert_eq!(restore_res.restored_source_roots, 1);
    assert_eq!(restore_res.restored_asset_suppressions, 1);

    // Verify restored records are intact
    let restored_work = fresh_core.get_work_summary(&work.id).unwrap().expect("Work must be restored");
    assert_eq!(restored_work.entity.title, "Elden Ring");

    let restored_exp = fresh_core.get_experience(&recorded_exp.id).unwrap().expect("Experience must be restored");
    assert_eq!(restored_exp.title.as_deref(), Some("Elden Ring 100% Achievement"));
    assert_eq!(restored_exp.occurred_precision, "day");
    assert_eq!(restored_exp.occurred_value.as_deref(), Some("2026-02-25"));

    let restored_roots = fresh_core.list_source_roots().unwrap();
    assert_eq!(restored_roots.len(), 1);
    assert_eq!(restored_roots[0].id, root.id);

    let is_sup = fresh_core.is_asset_suppressed(&root.id, "common/temp.log").unwrap();
    assert!(is_sup, "Suppression must be preserved upon restore");
}
