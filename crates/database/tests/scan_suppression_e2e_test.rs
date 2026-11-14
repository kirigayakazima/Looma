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
