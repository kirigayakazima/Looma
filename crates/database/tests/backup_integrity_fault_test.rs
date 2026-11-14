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

    // 4. Fault Injection 2: Missing manifest.json
    let empty_backup_dir = tempdir().unwrap();
    let missing_err = fresh_core.restore_backup("tester", empty_backup_dir.path());
    assert!(missing_err.is_err());
    match missing_err.err().unwrap() {
        LoomaError::Validation(msg) => assert!(msg.contains("manifest.json is required")),
        other => panic!("Expected Validation error for missing manifest, got: {other:?}"),
    }

    // Fault Injection 3: Missing vault_dump.json with manifest present
    let missing_dump_dir = tempdir().unwrap();
    fs::copy(backup_path.join("manifest.json"), missing_dump_dir.path().join("manifest.json")).unwrap();
    let missing_dump_err = fresh_core.restore_backup("tester", missing_dump_dir.path());
    assert!(missing_dump_err.is_err());
    match missing_dump_err.err().unwrap() {
        LoomaError::Vault(msg) => assert!(msg.contains("vault_dump.json not found")),
        other => panic!("Expected Vault error for missing vault_dump.json, got: {other:?}"),
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

    // 6. Fault Injection 4: Atomic Rollback on Mid-Restore Failure
    // Target database initially has a pre-existing record:
    let rollback_vault_dir = tempdir().unwrap();
    let rollback_db = Arc::new(LoomaDb::open(rollback_vault_dir.path()).unwrap());
    let rollback_core = LoomaCore::new(rollback_db.clone());
    let initial_work = rollback_core
        .create_work("author", "Initial Unrelated Game", WorkType::Game, RecordStatus::Completed, None, None)
        .unwrap();

    // Prepare a malicious/corrupted backup where entities succeed but experiences fail (e.g. invalid foreign key or precision)
    let bad_dump_dir = tempdir().unwrap();
    let mut bad_dump: VaultExportDump = serde_json::from_slice(&fs::read(&dump_file).unwrap()).unwrap();
    // Add a new entity that would succeed if inserted first:
    let new_entity = Entity {
        id: "ent_should_not_exist".to_string(),
        entity_type: "game".to_string(),
        title: "Ghost Game Should Be Rolled Back".to_string(),
        description: None,
        properties: serde_json::json!({}),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    bad_dump.entities.push(new_entity);

    // Inject an invalid experience entry that violates check constraint (invalid precision enum)
    let bad_exp = ExperienceEntry {
        id: "exp_bad_fk".to_string(),
        subject_type: "work".to_string(),
        subject_id: "non_existent_subject_id_123456".to_string(),
        event_type: "completed".to_string(),
        title: Some("Invalid FK Experience".to_string()),
        note: None,
        occurred_value: None,
        occurred_precision: "invalid_precision_enum".to_string(), // Violates CHECK constraint in schema!
        context_json: serde_json::json!({}),
        visibility: "public".to_string(),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    bad_dump.experiences.push(bad_exp);

    let bad_dump_bytes = serde_json::to_vec_pretty(&bad_dump).unwrap();
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(&bad_dump_bytes);
    let bad_dump_hash = format!("{:x}", hasher.finalize());

    let mut bad_manifest = backup_res.manifest.clone();
    bad_manifest.checksums.insert("vault_dump.json".to_string(), bad_dump_hash);

    fs::write(bad_dump_dir.path().join("vault_dump.json"), &bad_dump_bytes).unwrap();
    fs::write(bad_dump_dir.path().join("manifest.json"), serde_json::to_string_pretty(&bad_manifest).unwrap()).unwrap();

    // Attempt restore: MUST FAIL
    let failed_restore = rollback_core.restore_backup("tester", bad_dump_dir.path());
    assert!(failed_restore.is_err(), "Restore with invalid experience constraint must fail");

    // CRITICAL ATOMICITY CHECK:
    // 1. Initial work must STILL exist untouched:
    let check_initial = rollback_core.get_work_summary(&initial_work.id).unwrap();
    assert!(check_initial.is_some(), "Pre-existing initial record must remain intact");

    // 2. Entity that appeared before the failure in the dump MUST NOT exist (it was rolled back!):
    let check_ghost = rollback_core.get_work_summary("ent_should_not_exist").unwrap();
    assert!(check_ghost.is_none(), "Atomic rollback verification failed: ghost entity was committed despite restore failure!");

    // 3. Total entity count in DB must be exactly 1:
    let all_entities = rollback_core.list_works(None, None).unwrap();
    assert_eq!(all_entities.len(), 1, "Database was polluted with partial restore data!");
}
