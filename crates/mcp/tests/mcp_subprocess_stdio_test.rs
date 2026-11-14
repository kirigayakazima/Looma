use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::sync::Arc;
use tempfile::tempdir;

use looma_core::services::ExperienceService;
use looma_database::LoomaDb;
use serde_json::{json, Value};

#[test]
fn test_mcp_real_subprocess_stdio_e2e_lifecycle() {
    let mcp_bin = env!("CARGO_BIN_EXE_looma-mcp");

    // -------------------------------------------------------------
    // Session 1: Real Subprocess with Default ReadOnly Configuration
    // -------------------------------------------------------------
    let vault_dir1 = tempdir().unwrap();

    let mut child1 = Command::new(mcp_bin)
        .arg("--vault")
        .arg(vault_dir1.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to spawn looma-mcp subprocess");

    let mut stdin1 = child1.stdin.take().expect("Failed to open stdin");
    let mut reader1 = BufReader::new(child1.stdout.take().expect("Failed to open stdout"));

    // 1. Send MCP initialize handshake
    let init_req = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "clientInfo": {
                "name": "looma-subprocess-test-client",
                "version": "1.0.0"
            }
        }
    });
    writeln!(stdin1, "{}", serde_json::to_string(&init_req).unwrap()).unwrap();
    stdin1.flush().unwrap();

    let mut init_line = String::new();
    reader1.read_line(&mut init_line).expect("Failed to read initialize response");
    let init_resp: Value = serde_json::from_str(&init_line).expect("Invalid JSON in initialize response");
    assert_eq!(init_resp["id"], 1);
    assert_eq!(init_resp["result"]["serverInfo"]["name"], "looma-mcp");

    // 2. Send initialized notification
    let initialized_notif = json!({
        "jsonrpc": "2.0",
        "method": "notifications/initialized"
    });
    writeln!(stdin1, "{}", serde_json::to_string(&initialized_notif).unwrap()).unwrap();
    stdin1.flush().unwrap();

    // 3. Query tools/list via stdio
    let list_req = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list",
        "params": {}
    });
    writeln!(stdin1, "{}", serde_json::to_string(&list_req).unwrap()).unwrap();
    stdin1.flush().unwrap();

    let mut list_line = String::new();
    reader1.read_line(&mut list_line).expect("Failed to read tools/list response");
    let list_resp: Value = serde_json::from_str(&list_line).expect("Invalid JSON in tools/list response");
    assert_eq!(list_resp["id"], 2);
    let tools = list_resp["result"]["tools"].as_array().expect("Tools must be an array");
    assert!(!tools.is_empty(), "Subprocess must register available MCP tools");

    // 4. Attempt write operation under default ReadOnly permissions via stdio
    let write_req = json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/call",
        "params": {
            "name": "record_experience",
            "arguments": {
                "subject_id": "game_bloodborne",
                "event_type": "completed",
                "title": "Unauthorized Write Attempt"
            }
        }
    });
    writeln!(stdin1, "{}", serde_json::to_string(&write_req).unwrap()).unwrap();
    stdin1.flush().unwrap();

    let mut write_line = String::new();
    reader1.read_line(&mut write_line).expect("Failed to read tools/call response");
    let write_resp: Value = serde_json::from_str(&write_line).expect("Invalid JSON in write response");
    assert_eq!(write_resp["id"], 3);
    assert_eq!(
        write_resp["result"]["isError"], true,
        "Default ReadOnly subprocess MUST return isError: true on write tool calls!"
    );
    let err_msg = write_resp["result"]["content"][0]["text"].as_str().unwrap_or("");
    assert!(
        err_msg.contains("ContentWrite") && err_msg.contains("ReadOnly"),
        "Error message must specify permission denial: {err_msg}"
    );

    // Drop child1 stdin to trigger EOF and cleanly wait for child exit
    drop(stdin1);
    let _ = child1.wait();

    // 5. Invariant Proof: Verify target DB has ZERO side effects
    let db1 = Arc::new(LoomaDb::open(vault_dir1.path()).unwrap());
    let exps_count = db1.count_experiences().unwrap();
    assert_eq!(exps_count, 0, "Rejected write must have zero side effects on the database!");

    // -------------------------------------------------------------
    // Session 2: Real Subprocess with Explicit --allow-write Flag
    // -------------------------------------------------------------
    let vault_dir2 = tempdir().unwrap();

    let mut child2 = Command::new(mcp_bin)
        .arg("--vault")
        .arg(vault_dir2.path())
        .arg("--allow-write")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to spawn looma-mcp subprocess with --allow-write");

    let mut stdin2 = child2.stdin.take().expect("Failed to open stdin for child2");
    let mut reader2 = BufReader::new(child2.stdout.take().expect("Failed to open stdout for child2"));

    // Handshake
    writeln!(stdin2, "{}", serde_json::to_string(&init_req).unwrap()).unwrap();
    stdin2.flush().unwrap();
    let mut init_line2 = String::new();
    reader2.read_line(&mut init_line2).unwrap();

    writeln!(stdin2, "{}", serde_json::to_string(&initialized_notif).unwrap()).unwrap();
    stdin2.flush().unwrap();

    // Create Work entity first so foreign key constraints are satisfied
    let create_work_req = json!({
        "jsonrpc": "2.0",
        "id": 10,
        "method": "tools/call",
        "params": {
            "name": "create_work",
            "arguments": {
                "title": "Bloodborne",
                "work_type": "game",
                "status": "completed"
            }
        }
    });
    writeln!(stdin2, "{}", serde_json::to_string(&create_work_req).unwrap()).unwrap();
    stdin2.flush().unwrap();

    let mut work_line = String::new();
    reader2.read_line(&mut work_line).unwrap();
    let work_resp: Value = serde_json::from_str(&work_line).unwrap();
    assert_ne!(work_resp["result"]["isError"], true, "create_work must succeed under --allow-write");
    let work_content: Value = serde_json::from_str(work_resp["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    let work_id = work_content["id"].as_str().unwrap();

    // Call record_experience under --allow-write
    let create_exp_req = json!({
        "jsonrpc": "2.0",
        "id": 11,
        "method": "tools/call",
        "params": {
            "name": "record_experience",
            "arguments": {
                "subject_id": work_id,
                "event_type": "completed",
                "title": "Plat Trophy via Subprocess stdio"
            }
        }
    });
    writeln!(stdin2, "{}", serde_json::to_string(&create_exp_req).unwrap()).unwrap();
    stdin2.flush().unwrap();

    let mut exp_line = String::new();
    reader2.read_line(&mut exp_line).unwrap();
    let exp_resp: Value = serde_json::from_str(&exp_line).unwrap();
    assert_ne!(exp_resp["result"]["isError"], true, "create_experience must succeed under --allow-write");

    // Drop child2 stdin and terminate
    drop(stdin2);
    let _ = child2.wait();

    // Verify record in database
    let db2 = Arc::new(LoomaDb::open(vault_dir2.path()).unwrap());
    let exps = db2.list_experiences(&looma_core::models::ExperienceFilter::default()).unwrap();
    assert_eq!(exps.len(), 1, "Authorized subprocess write must persist experience entry");
    assert_eq!(exps[0].title.as_deref(), Some("Plat Trophy via Subprocess stdio"));
}
