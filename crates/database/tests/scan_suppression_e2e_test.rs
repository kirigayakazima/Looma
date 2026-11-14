use std::fs;
use std::sync::Arc;
use tempfile::tempdir;

use looma_core::models::*;
use looma_core::LoomaCore;
use looma_database::LoomaDb;
use looma_mcp::{McpPermissionLevel, McpServer};
use looma_scanner::Scanner;

#[test]
fn test_untrack_tombstone_scan_loop_and_lifecycle_invariants() {
    let vault_dir = tempdir().unwrap();
    let db = Arc::new(LoomaDb::open(vault_dir.path()).unwrap());
    let scanner = Arc::new(Scanner);
    let core = LoomaCore::new(db.clone()).with_scan_engine(scanner.clone());

    // 1. Prepare scan root on disk
    let media_dir = tempdir().unwrap();
    let game_path = media_dir.path().join("game.exe");
    let notes_path = media_dir.path().join("notes.txt");
    let dlc_dir = media_dir.path().join("dlc");
    fs::create_dir_all(&dlc_dir).unwrap();
    let patch_path = dlc_dir.join("patch.bin");

    fs::write(&game_path, b"GAME_BINARY_DATA").unwrap();
    fs::write(&notes_path, b"MY_NOTES_DATA").unwrap();
    fs::write(&patch_path, b"DLC_PATCH_DATA").unwrap();

    // 2. Perform initial scan via unified LoomaCore
    let summary1 = core
        .scan_directory(media_dir.path(), None, None::<fn(ScanProgress)>)
        .expect("Initial scan must succeed");
    assert_eq!(summary1.new_assets, 3);
    assert_eq!(summary1.suppressed_assets, 0);

    // Verify SourceRoot was auto-registered
    let roots = core.list_source_roots().unwrap();
    assert_eq!(roots.len(), 1);
    let root = &roots[0];
    assert_eq!(root.state, "online");

    // Fetch assets
    let game_norm = looma_core::facade::normalize_directory_path(&game_path.to_string_lossy());
    let notes_norm = looma_core::facade::normalize_directory_path(&notes_path.to_string_lossy());
    let game_asset = core.get_asset_by_path(&game_norm).unwrap().expect("game asset must exist");
    let notes_asset = core.get_asset_by_path(&notes_norm).unwrap().expect("notes asset must exist");

    // Verify initial lifecycle tracking
    let game_lc = core.get_asset_lifecycle(&game_asset.id).unwrap().expect("lifecycle must exist");
    assert_eq!(game_lc.tracking, "tracked");
    assert_eq!(game_lc.availability, "online");

    // 3. User Untracks game.exe
    core.untrack_asset("test-user", &game_asset.id, Some("Suppressed by user")).unwrap();
    let game_lc_after = core.get_asset_lifecycle(&game_asset.id).unwrap().unwrap();
    assert_eq!(game_lc_after.tracking, "untracked");
    assert!(game_lc_after.untracked_at.is_some());

    // User Archives notes.txt
    core.archive_asset("test-user", &notes_asset.id).unwrap();
    let notes_asset_archived = core.get_asset(&notes_asset.id).unwrap().unwrap();
    assert_eq!(notes_asset_archived.status, AssetStatus::Archived);

    // 4. Perform second scan via LoomaCore
    // Proves: game.exe must NOT be re-indexed as active, and notes.txt must remain Archived!
    let summary2 = core
        .scan_directory(media_dir.path(), None, None::<fn(ScanProgress)>)
        .expect("Second scan must succeed");

    assert_eq!(summary2.new_assets, 0);
    assert_eq!(summary2.suppressed_assets, 1, "game.exe must be counted as suppressed");

    // Invariant check: notes.txt must STILL be Archived, not reverted to Active!
    let notes_check = core.get_asset(&notes_asset.id).unwrap().unwrap();
    assert_eq!(
        notes_check.status,
        AssetStatus::Archived,
        "Scanning must never revert an Archived asset to Active!"
    );

    // 5. Test Case-Insensitive Suppression matching
    // Untrack dlc/patch.bin with uppercase relative path
    let patch_norm = looma_core::facade::normalize_directory_path(&patch_path.to_string_lossy());
    let patch_asset = core.get_asset_by_path(&patch_norm).unwrap().expect("patch asset must exist");
    core.untrack_asset("test-user", &patch_asset.id, Some("Case test")).unwrap();

    let summary3 = core
        .scan_directory(media_dir.path(), None, None::<fn(ScanProgress)>)
        .expect("Third scan must succeed");
    assert_eq!(summary3.suppressed_assets, 2, "Both game.exe and patch.bin must be suppressed");

    // 6. Test MCP Scan Endpoint invokes the unified core logic
    let mcp = McpServer::with_permission(core.clone(), McpPermissionLevel::ContentWrite);
    let mcp_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "scan_directory",
            "arguments": {
                "path": media_dir.path().to_string_lossy()
            }
        }
    });

    let mcp_res = mcp.handle_json_str(&mcp_req.to_string()).expect("MCP response required");
    let content_text = mcp_res["result"]["content"][0]["text"].as_str().unwrap();
    let mcp_summary: ScanSummary = serde_json::from_str(content_text).unwrap();
    assert_eq!(mcp_summary.suppressed_assets, 2, "MCP scan must load tombstones from Core and suppress untracked files");

    // 7. Resume tracking of game.exe and re-scan
    core.resume_tracking("test-user", &game_asset.id).unwrap();
    let summary4 = core
        .scan_directory(media_dir.path(), None, None::<fn(ScanProgress)>)
        .expect("Fourth scan must succeed");
    assert_eq!(summary4.suppressed_assets, 1, "Only patch.bin is suppressed now");
}

#[test]
fn test_subdirectory_scan_inherits_parent_tombstones_and_boundary_isolation() {
    let vault_dir = tempdir().unwrap();
    let db = Arc::new(LoomaDb::open(vault_dir.path()).unwrap());
    let scanner = Arc::new(Scanner);
    let core = LoomaCore::new(db.clone()).with_scan_engine(scanner.clone());

    // 1. Directory layout:
    // base_dir/
    //   Games/
    //     Bloodborne/
    //       ebitas.exe   (to be untracked on Games)
    //       normal.bin   (tracked)
    //   GamesBackup/
    //     backup.iso     (should never match Games prefix)
    let base_dir = tempdir().unwrap();
    let games_dir = base_dir.path().join("Games");
    let games_backup_dir = base_dir.path().join("GamesBackup");
    let bloodborne_dir = games_dir.join("Bloodborne");

    fs::create_dir_all(&bloodborne_dir).unwrap();
    fs::create_dir_all(&games_backup_dir).unwrap();

    let ebitas_file = bloodborne_dir.join("ebitas.exe");
    let normal_file = bloodborne_dir.join("normal.bin");
    let backup_file = games_backup_dir.join("backup.iso");

    fs::write(&ebitas_file, b"EBITAS_DATA").unwrap();
    fs::write(&normal_file, b"NORMAL_DATA").unwrap();
    fs::write(&backup_file, b"BACKUP_DATA").unwrap();

    // 2. Register Games as a registered SourceRoot
    let games_norm = looma_core::facade::normalize_directory_path(&games_dir.to_string_lossy());
    let root = core
        .create_source_root("test", "My Games", &games_norm)
        .expect("Create parent SourceRoot must succeed");

    // Add tombstone suppression on parent SourceRoot for subfolder path: "Bloodborne/ebitas.exe"
    core.add_suppression(&root.id, "Bloodborne/ebitas.exe", Some("Untracked secret"))
        .expect("Add suppression on parent root must succeed");

    // 3. Scan ONLY the subdirectory Bloodborne
    let sub_summary = core
        .scan_directory(&bloodborne_dir, None, None::<fn(ScanProgress)>)
        .expect("Subdirectory scan must succeed");

    // Proves: Tombstone from parent SourceRoot ("Bloodborne/ebitas.exe") was transformed to "ebitas.exe"
    // and successfully applied during the subfolder scan!
    assert_eq!(sub_summary.new_assets, 1, "Only normal.bin should be newly indexed");
    assert_eq!(sub_summary.suppressed_assets, 1, "ebitas.exe must be suppressed via inherited tombstone!");

    // Verify ebitas.exe was NEVER indexed in assets table
    let ebitas_norm = looma_core::facade::normalize_directory_path(&ebitas_file.to_string_lossy());
    assert!(
        core.get_asset_by_path(&ebitas_norm).unwrap().is_none(),
        "Suppressed file must not exist in assets index"
    );

    // Verify normal.bin was indexed and correctly linked to parent SourceRoot
    let normal_norm = looma_core::facade::normalize_directory_path(&normal_file.to_string_lossy());
    let normal_asset = core.get_asset_by_path(&normal_norm).unwrap().expect("normal.bin must exist");
    let normal_lc = core.get_asset_lifecycle(&normal_asset.id).unwrap().expect("lifecycle must exist");
    assert_eq!(normal_lc.root_id.as_deref(), Some(root.id.as_str()), "Asset in subfolder must inherit parent SourceRoot ID");
    assert_eq!(normal_lc.relative_path.as_deref(), Some("Bloodborne/normal.bin"), "Relative path must be relative to parent SourceRoot");

    // Proves: Direct subfolder scan did not fragment or create a duplicate SourceRoot
    let all_roots = core.list_source_roots().unwrap();
    assert_eq!(all_roots.len(), 1, "Scanning subdirectory of existing root must not create duplicate root");

    // 4. Test boundary isolation between Games and GamesBackup
    // Scan GamesBackup to index its file
    let backup_summary = core
        .scan_directory(&games_backup_dir, None, None::<fn(ScanProgress)>)
        .expect("GamesBackup scan must succeed");
    assert_eq!(backup_summary.new_assets, 1);

    // Query list_assets_by_prefix for Games: MUST NOT contain GamesBackup/backup.iso
    let games_assets = core.list_assets_by_prefix(&games_norm).unwrap();
    assert_eq!(games_assets.len(), 1, "Only normal.bin should match Games prefix");
    assert_eq!(games_assets[0].id, normal_asset.id);
    for a in &games_assets {
        let p = a.path.as_deref().unwrap_or("");
        assert!(
            !p.contains("GamesBackup"),
            "Critical prefix boundary failure: GamesBackup asset leaked into Games query! Path: {p}"
        );
    }
}

#[test]
fn test_scan_directory_propagates_lifecycle_persistence_error() {
    let vault_dir = tempdir().unwrap();
    let db = Arc::new(LoomaDb::open(vault_dir.path()).unwrap());
    let scanner = Arc::new(Scanner);
    let core = LoomaCore::new(db).with_scan_engine(scanner);

    // Prepare a scan folder with a new file
    let scan_dir = tempdir().unwrap();
    let sample_file = scan_dir.path().join("asset.dat");
    fs::write(&sample_file, b"SAMPLE_PAYLOAD").unwrap();

    // Fault Injection: Inject an SQLite trigger that aborts INSERT into asset_lifecycle
    // This simulates database constraints, lock contention, disk corruption, or lifecycle schema failures
    let db_path = vault_dir.path().join(".looma").join("vault.db");
    let raw_conn = rusqlite::Connection::open(&db_path).unwrap();
    raw_conn
        .execute(
            "CREATE TRIGGER fail_lifecycle_insert BEFORE INSERT ON asset_lifecycle
             BEGIN
                 SELECT RAISE(ABORT, 'Simulated lifecycle disk corruption');
             END;",
            [],
        )
        .unwrap();
    drop(raw_conn);

    // Execute scan: under the previous bug, `let _ = self.repo.set_asset_lifecycle(&new_lc)` would
    // swallow this error and report success. Now, it MUST propagate the database error!
    let scan_res = core.scan_directory(scan_dir.path(), None, None::<fn(ScanProgress)>);
    assert!(
        scan_res.is_err(),
        "scan_directory must fail when asset_lifecycle persistence fails!"
    );

    let err_str = format!("{:?}", scan_res.err().unwrap());
    assert!(
        err_str.contains("Simulated lifecycle disk corruption"),
        "Underlying database error must be propagated to caller: {err_str}"
    );
}
