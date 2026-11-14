use std::sync::Arc;
use tempfile::tempdir;

use looma_core::LoomaCore;
use looma_database::LoomaDb;
use looma_mcp::{McpPermissionLevel, McpServer};
use looma_scanner::Scanner;

fn setup_test_server(permission: Option<McpPermissionLevel>) -> McpServer {
    let vault_dir = tempdir().unwrap();
    let db = Arc::new(LoomaDb::open(vault_dir.path()).unwrap());
    let scanner = Arc::new(Scanner);
    let core = LoomaCore::new(db).with_scan_engine(scanner);

    match permission {
        Some(perm) => McpServer::with_permission(core, perm),
        None => McpServer::new(core), // Tests default constructor
    }
}

fn assert_tool_error(res: &serde_json::Value, expected_substring: &str) {
    let is_err = res["result"]["isError"].as_bool().unwrap_or(false) || res.get("error").is_some();
    assert!(is_err, "Response was expected to be an error: {res:?}");
    let text = if let Some(content) = res["result"]["content"].as_array() {
        content[0]["text"].as_str().unwrap_or("")
    } else if let Some(err) = res.get("error") {
        err["message"].as_str().unwrap_or("")
    } else {
        ""
    };
    assert!(
        text.contains(expected_substring),
        "Expected error containing '{expected_substring}', got: '{text}'"
    );
}

fn assert_tool_success(res: &serde_json::Value) {
    let is_err = res["result"]["isError"].as_bool().unwrap_or(false);
    assert!(!is_err, "Response was expected to be a success: {res:?}");
    assert!(res.get("error").is_none(), "Response contained error: {res:?}");
}

#[test]
fn test_mcp_default_is_readonly() {
    let server = setup_test_server(None);

    // ReadOnly should allow search_assets and list_experiences
    let search_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "search_assets",
            "arguments": {
                "query": "test"
            }
        }
    });
    let res = server.handle_json_str(&search_req.to_string()).unwrap();
    assert_tool_success(&res);

    let list_exp_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": {
            "name": "list_experiences",
            "arguments": {}
        }
    });
    let res_exp = server.handle_json_str(&list_exp_req.to_string()).unwrap();
    assert_tool_success(&res_exp);

    // ReadOnly must REJECT create_entity
    let create_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/call",
        "params": {
            "name": "create_entity",
            "arguments": {
                "title": "Test Work",
                "entity_type": "work"
            }
        }
    });
    let res = server.handle_json_str(&create_req.to_string()).unwrap();
    assert_tool_error(&res, "Permission Denied");

    // ReadOnly must REJECT record_experience
    let exp_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 4,
        "method": "tools/call",
        "params": {
            "name": "record_experience",
            "arguments": {
                "subject_id": "ent_test",
                "event_type": "completed"
            }
        }
    });
    let res = server.handle_json_str(&exp_req.to_string()).unwrap();
    assert_tool_error(&res, "Permission Denied");

    // ReadOnly must REJECT scan_directory
    let scan_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 5,
        "method": "tools/call",
        "params": {
            "name": "scan_directory",
            "arguments": {
                "path": "D:/Games"
            }
        }
    });
    let res = server.handle_json_str(&scan_req.to_string()).unwrap();
    assert_tool_error(&res, "Permission Denied");

    // ReadOnly must REJECT delete_asset
    let del_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 6,
        "method": "tools/call",
        "params": {
            "name": "delete_asset",
            "arguments": {
                "id": "asset_1"
            }
        }
    });
    let res = server.handle_json_str(&del_req.to_string()).unwrap();
    assert_tool_error(&res, "Permission Denied");
}

#[test]
fn test_mcp_metadata_write_permission_tier() {
    let server = setup_test_server(Some(McpPermissionLevel::MetadataWrite));

    // MetadataWrite: create_entity is permitted
    let create_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 10,
        "method": "tools/call",
        "params": {
            "name": "create_entity",
            "arguments": {
                "title": "Hollow Knight",
                "entity_type": "work"
            }
        }
    });
    let res = server.handle_json_str(&create_req.to_string()).unwrap();
    assert_tool_success(&res);

    // MetadataWrite: record_experience (ContentWrite required) is rejected
    let exp_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 11,
        "method": "tools/call",
        "params": {
            "name": "record_experience",
            "arguments": {
                "subject_id": "ent_test",
                "event_type": "completed"
            }
        }
    });
    let res = server.handle_json_str(&exp_req.to_string()).unwrap();
    assert_tool_error(&res, "Permission Denied");

    // MetadataWrite: scan_directory (ContentWrite required) is rejected
    let scan_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 12,
        "method": "tools/call",
        "params": {
            "name": "scan_directory",
            "arguments": {
                "path": "D:/Games"
            }
        }
    });
    let res = server.handle_json_str(&scan_req.to_string()).unwrap();
    assert_tool_error(&res, "Permission Denied");
}

#[test]
fn test_mcp_content_write_permission_tier() {
    let server = setup_test_server(Some(McpPermissionLevel::ContentWrite));

    // ContentWrite: create_entity permitted
    let create_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 20,
        "method": "tools/call",
        "params": {
            "name": "create_entity",
            "arguments": {
                "title": "Bloodborne",
                "entity_type": "work"
            }
        }
    });
    let res = server.handle_json_str(&create_req.to_string()).unwrap();
    assert_tool_success(&res);
    let content_text = res["result"]["content"][0]["text"].as_str().unwrap();
    let ent: serde_json::Value = serde_json::from_str(content_text).unwrap();
    let entity_id = ent["id"].as_str().unwrap();

    // ContentWrite: record_experience permitted for existing entity
    let exp_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 21,
        "method": "tools/call",
        "params": {
            "name": "record_experience",
            "arguments": {
                "subject_id": entity_id,
                "event_type": "completed",
                "occurred_precision": "year",
                "occurred_value": "2015"
            }
        }
    });
    let res = server.handle_json_str(&exp_req.to_string()).unwrap();
    assert_tool_success(&res);

    // ContentWrite: delete_asset (FullDestructive required) is rejected
    let del_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 22,
        "method": "tools/call",
        "params": {
            "name": "delete_asset",
            "arguments": {
                "id": "asset_1"
            }
        }
    });
    let res = server.handle_json_str(&del_req.to_string()).unwrap();
    assert_tool_error(&res, "Permission Denied");
}

#[test]
fn test_mcp_full_destructive_permission_tier() {
    let server = setup_test_server(Some(McpPermissionLevel::FullDestructive));

    // FullDestructive: delete_asset is allowed through permission check
    let del_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 30,
        "method": "tools/call",
        "params": {
            "name": "delete_asset",
            "arguments": {
                "id": "nonexistent_asset"
            }
        }
    });
    let res = server.handle_json_str(&del_req.to_string()).unwrap();
    assert_tool_success(&res);
}
