use std::io::{BufRead, Write};
use std::path::PathBuf;
use chrono::Utc;
use serde_json::{json, Value};

use looma_core::models::*;
use looma_core::services::*;
use looma_database::LoomaDb;
use looma_scanner::{ScanOptions, Scanner};

pub struct McpServer {
    db: LoomaDb,
}

impl McpServer {
    pub fn new(db: LoomaDb) -> Self {
        Self { db }
    }

    pub fn run_stdio(&self) -> Result<(), Box<dyn std::error::Error>> {
        let stdin = std::io::stdin();
        let mut stdout = std::io::stdout();

        for line_res in stdin.lock().lines() {
            let line = line_res?;
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            let request: Value = match serde_json::from_str(trimmed) {
                Ok(v) => v,
                Err(e) => {
                    let err_resp = json!({
                        "jsonrpc": "2.0",
                        "id": null,
                        "error": { "code": -32700, "message": format!("Parse error: {e}") }
                    });
                    writeln!(stdout, "{}", serde_json::to_string(&err_resp)?)?;
                    stdout.flush()?;
                    continue;
                }
            };

            let id = request.get("id").cloned();
            let method = request.get("method").and_then(|m| m.as_str()).unwrap_or("");
            let params = request.get("params").cloned().unwrap_or(Value::Null);

            let response = self.handle_method(method, params, id);
            if let Some(resp) = response {
                writeln!(stdout, "{}", serde_json::to_string(&resp)?)?;
                stdout.flush()?;
            }
        }

        Ok(())
    }

    pub fn handle_method(&self, method: &str, params: Value, id: Option<Value>) -> Option<Value> {
        let req_id = id.unwrap_or(Value::Null);

        match method {
            "initialize" => Some(json!({
                "jsonrpc": "2.0",
                "id": req_id,
                "result": {
                    "protocolVersion": "2024-11-05",
                    "capabilities": {
                        "tools": {}
                    },
                    "serverInfo": {
                        "name": "looma-mcp",
                        "version": "0.1.0"
                    }
                }
            })),

            "notifications/initialized" => None,

            "ping" => Some(json!({
                "jsonrpc": "2.0",
                "id": req_id,
                "result": {}
            })),

            "tools/list" => Some(json!({
                "jsonrpc": "2.0",
                "id": req_id,
                "result": {
                    "tools": self.list_tools()
                }
            })),

            "tools/call" => {
                let tool_name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
                let args = params.get("arguments").cloned().unwrap_or(json!({}));

                let result = self.call_tool(tool_name, args);
                match result {
                    Ok(text) => Some(json!({
                        "jsonrpc": "2.0",
                        "id": req_id,
                        "result": {
                            "content": [
                                { "type": "text", "text": text }
                            ]
                        }
                    })),
                    Err(err_msg) => Some(json!({
                        "jsonrpc": "2.0",
                        "id": req_id,
                        "result": {
                            "isError": true,
                            "content": [
                                { "type": "text", "text": format!("Error: {err_msg}") }
                            ]
                        }
                    })),
                }
            }

            _ => Some(json!({
                "jsonrpc": "2.0",
                "id": req_id,
                "error": {
                    "code": -32601,
                    "message": format!("Method not found: {method}")
                }
            })),
        }
    }

    fn list_tools(&self) -> Vec<Value> {
        vec![
            json!({
                "name": "search_assets",
                "description": "Search indexed digital assets in the Looma Vault by query, kind, or path",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "query": { "type": "string", "description": "Keyword to search filename, path, or metadata" },
                        "kind": { "type": "string", "description": "Optional asset kind: image, video, audio, document, code, archive, model3d" },
                        "limit": { "type": "integer", "description": "Max results to return (default 20)" }
                    }
                }
            }),
            json!({
                "name": "get_asset",
                "description": "Get detailed metadata for a specific indexed asset by ID",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "string", "description": "The asset ID" }
                    },
                    "required": ["id"]
                }
            }),
            json!({
                "name": "scan_directory",
                "description": "Trigger an incremental filesystem scan on a directory under Reference Mode",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Absolute filesystem directory path to scan" },
                        "compute_hash": { "type": "boolean", "description": "Whether to compute streaming SHA-256 (default false)" }
                    },
                    "required": ["path"]
                }
            }),
            json!({
                "name": "search_entities",
                "description": "Search user-defined entity concepts (anime, projects, devices, games, books)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "query": { "type": "string", "description": "Search keyword for entity title or description" },
                        "entity_type": { "type": "string", "description": "Optional filter by type" }
                    }
                }
            }),
            json!({
                "name": "create_entity",
                "description": "Create a new typed entity concept with custom dynamic properties",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "title": { "type": "string", "description": "Title of the entity" },
                        "entity_type": { "type": "string", "description": "Type of entity (e.g. anime, project, hardware, book)" },
                        "description": { "type": "string", "description": "Optional description" },
                        "properties": { "type": "object", "description": "Optional dynamic JSON key-value properties" }
                    },
                    "required": ["title", "entity_type"]
                }
            }),
            json!({
                "name": "list_memories",
                "description": "List personal reflections, logs, and Markdown records from the vault",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "limit": { "type": "integer", "description": "Number of memories to return (default 20)" }
                    }
                }
            }),
            json!({
                "name": "create_memory",
                "description": "Create a new Markdown memory note in the vault",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "title": { "type": "string", "description": "Memory note title" },
                        "content": { "type": "string", "description": "Markdown body content" },
                        "category": { "type": "string", "description": "Category (journal, devlog, note, review)" }
                    },
                    "required": ["title", "content"]
                }
            }),
            json!({
                "name": "create_relation",
                "description": "Create a bidirectional relation between two items (assets, entities, memories)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "source_id": { "type": "string", "description": "ID of source item" },
                        "source_type": { "type": "string", "description": "asset | entity | memory" },
                        "target_id": { "type": "string", "description": "ID of target item" },
                        "target_type": { "type": "string", "description": "asset | entity | memory" },
                        "relation_type": { "type": "string", "description": "Relation descriptor (refers_to, belongs_to, attaches)" }
                    },
                    "required": ["source_id", "source_type", "target_id", "target_type", "relation_type"]
                }
            }),
            json!({
                "name": "get_timeline",
                "description": "Fetch unified chronological stream weaving assets, entities, and memories",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "limit": { "type": "integer", "description": "Max timeline events to return" },
                        "item_type": { "type": "string", "description": "Optional filter: asset | entity | memory" }
                    }
                }
            }),
            json!({
                "name": "vault_doctor",
                "description": "Run health and integrity check on the vault database and reference paths",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "cleanup_missing": { "type": "boolean", "description": "If true, mark missing assets and run VACUUM" }
                    }
                }
            }),
            json!({
                "name": "get_smart_insights",
                "description": "Generate Phase 6 AI smart insights: relation suggestions, cluster proposals, and tag recommendations",
                "inputSchema": {
                    "type": "object",
                    "properties": {}
                }
            })
        ]
    }

    fn call_tool(&self, name: &str, args: Value) -> Result<String, String> {
        match name {
            "search_assets" => {
                let q = args.get("query").and_then(|v| v.as_str()).map(|s| s.to_string());
                let kind_str = args.get("kind").and_then(|v| v.as_str());
                let kind = kind_str.and_then(|k| serde_json::from_str(&format!("\"{}\"", k)).ok());
                let limit = args.get("limit").and_then(|v| v.as_u64()).map(|n| n as usize).unwrap_or(20);

                let filter = AssetFilter {
                    kind,
                    search_query: q,
                    limit: Some(limit),
                    ..Default::default()
                };
                let assets = self.db.query_assets(&filter).map_err(|e| e.to_string())?;
                serde_json::to_string_pretty(&assets).map_err(|e| e.to_string())
            }

            "get_asset" => {
                let id = args.get("id").and_then(|v| v.as_str()).ok_or("Missing id parameter")?;
                let asset = self.db.get_asset_by_id(id).map_err(|e| e.to_string())?;
                match asset {
                    Some(a) => serde_json::to_string_pretty(&a).map_err(|e| e.to_string()),
                    None => Err(format!("Asset not found: {id}")),
                }
            }

            "scan_directory" => {
                let path_str = args.get("path").and_then(|v| v.as_str()).ok_or("Missing path parameter")?;
                let compute_hash = args.get("compute_hash").and_then(|v| v.as_bool()).unwrap_or(false);

                let options = ScanOptions {
                    compute_hash,
                    ..Default::default()
                };
                let summary = Scanner::scan_directory(
                    &self.db,
                    &PathBuf::from(path_str),
                    &options,
                    None::<fn(looma_scanner::ScanProgress)>,
                )
                .map_err(|e| e.to_string())?;
                serde_json::to_string_pretty(&summary).map_err(|e| e.to_string())
            }

            "search_entities" => {
                let q = args.get("query").and_then(|v| v.as_str()).map(|s| s.to_string());
                let entity_type = args.get("entity_type").and_then(|v| v.as_str()).map(|s| s.to_string());

                let filter = EntityFilter {
                    entity_type,
                    search_query: q,
                    ..Default::default()
                };
                let entities = self.db.list_entities(&filter).map_err(|e| e.to_string())?;
                serde_json::to_string_pretty(&entities).map_err(|e| e.to_string())
            }

            "create_entity" => {
                let title = args.get("title").and_then(|v| v.as_str()).ok_or("Missing title")?;
                let entity_type = args.get("entity_type").and_then(|v| v.as_str()).ok_or("Missing entity_type")?;
                let desc = args.get("description").and_then(|v| v.as_str()).map(|s| s.to_string());
                let props = args.get("properties").cloned().unwrap_or(json!({}));

                let entity = Entity {
                    id: format!("ent_{}", &uuid::Uuid::new_v4().to_string()[..8]),
                    entity_type: entity_type.to_string(),
                    title: title.to_string(),
                    description: desc,
                    properties: props,
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                };
                self.db.create_entity(&entity).map_err(|e| e.to_string())?;
                serde_json::to_string_pretty(&entity).map_err(|e| e.to_string())
            }

            "list_memories" => {
                let limit = args.get("limit").and_then(|v| v.as_u64()).map(|n| n as usize).unwrap_or(20);
                let memories = self.db.list_memories(limit, 0).map_err(|e| e.to_string())?;
                serde_json::to_string_pretty(&memories).map_err(|e| e.to_string())
            }

            "create_memory" => {
                let title = args.get("title").and_then(|v| v.as_str()).ok_or("Missing title")?;
                let content = args.get("content").and_then(|v| v.as_str()).ok_or("Missing content")?;
                let category = args.get("category").and_then(|v| v.as_str()).map(|s| s.to_string());

                let memory = Memory {
                    id: format!("mem_{}", &uuid::Uuid::new_v4().to_string()[..8]),
                    title: title.to_string(),
                    content: content.to_string(),
                    category,
                    metadata: json!({}),
                    recorded_at: Utc::now(),
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                };
                self.db.create_memory(&memory).map_err(|e| e.to_string())?;
                serde_json::to_string_pretty(&memory).map_err(|e| e.to_string())
            }

            "create_relation" => {
                let source_id = args.get("source_id").and_then(|v| v.as_str()).ok_or("Missing source_id")?;
                let source_type = args.get("source_type").and_then(|v| v.as_str()).ok_or("Missing source_type")?;
                let target_id = args.get("target_id").and_then(|v| v.as_str()).ok_or("Missing target_id")?;
                let target_type = args.get("target_type").and_then(|v| v.as_str()).ok_or("Missing target_type")?;
                let relation_type = args.get("relation_type").and_then(|v| v.as_str()).ok_or("Missing relation_type")?;

                let relation = Relation {
                    id: format!("rel_{}", &uuid::Uuid::new_v4().to_string()[..8]),
                    source_id: source_id.to_string(),
                    source_type: source_type.to_string(),
                    relation_type: relation_type.to_string(),
                    target_id: target_id.to_string(),
                    target_type: target_type.to_string(),
                    metadata: json!({}),
                    created_at: Utc::now(),
                };
                self.db.create_relation(&relation).map_err(|e| e.to_string())?;
                serde_json::to_string_pretty(&relation).map_err(|e| e.to_string())
            }

            "get_timeline" => {
                let limit = args.get("limit").and_then(|v| v.as_u64()).map(|n| n as usize);
                let item_type = args.get("item_type").and_then(|v| v.as_str()).map(|s| vec![s.to_string()]);

                let filter = TimelineFilter {
                    item_types: item_type,
                    limit,
                    offset: None,
                };
                let items = self.db.query_timeline(&filter).map_err(|e| e.to_string())?;
                serde_json::to_string_pretty(&items).map_err(|e| e.to_string())
            }

            "vault_doctor" => {
                let cleanup = args.get("cleanup_missing").and_then(|v| v.as_bool()).unwrap_or(false);
                let report = self.db.doctor_inspect().map_err(|e| e.to_string())?;
                if cleanup {
                    let cleaned = self.db.doctor_cleanup_missing().map_err(|e| e.to_string())?;
                    Ok(format!(
                        "Vault Doctor Report:\n{}\nCleaned Missing Items: {cleaned}",
                        serde_json::to_string_pretty(&report).unwrap_or_default()
                    ))
                } else {
                    serde_json::to_string_pretty(&report).map_err(|e| e.to_string())
                }
            }

            "get_smart_insights" => {
                let insights = looma_intelligence::IntelligenceEngine::generate_insights(&self.db)
                    .map_err(|e| e.to_string())?;
                serde_json::to_string_pretty(&insights).map_err(|e| e.to_string())
            }

            _ => Err(format!("Unknown tool: {name}")),
        }
    }
}
