use std::io::{BufRead, Write};
use std::path::PathBuf;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use looma_core::models::*;
use looma_core::LoomaCore;
use looma_scanner::{ScanOptions, Scanner};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpPermissionLevel {
    /// Read-only access to assets, entities, memories, timeline, and stats
    ReadOnly = 0,
    /// Can create/update entities, relations, external references
    MetadataWrite = 1,
    /// Can create/update memories, trigger scans
    ContentWrite = 2,
    /// Can perform destructive operations: delete items, cleanup missing
    FullDestructive = 3,
}

pub struct McpServer {
    core: LoomaCore,
    permission: McpPermissionLevel,
}

impl McpServer {
    pub fn new(core: LoomaCore) -> Self {
        Self {
            core,
            permission: McpPermissionLevel::ContentWrite,
        }
    }

    pub fn with_permission(core: LoomaCore, permission: McpPermissionLevel) -> Self {
        Self { core, permission }
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
                        "tools": {},
                        "resources": {}
                    },
                    "serverInfo": {
                        "name": "looma-mcp",
                        "version": "0.2.0"
                    }
                }
            })),

            "notifications/initialized" => None,

            "ping" => Some(json!({
                "jsonrpc": "2.0",
                "id": req_id,
                "result": {}
            })),

            "resources/list" => Some(json!({
                "jsonrpc": "2.0",
                "id": req_id,
                "result": {
                    "resources": self.list_resources()
                }
            })),

            "resources/read" => {
                let uri = params.get("uri").and_then(|u| u.as_str()).unwrap_or("");
                match self.read_resource(uri) {
                    Ok((mime_type, text)) => Some(json!({
                        "jsonrpc": "2.0",
                        "id": req_id,
                        "result": {
                            "contents": [
                                {
                                    "uri": uri,
                                    "mimeType": mime_type,
                                    "text": text
                                }
                            ]
                        }
                    })),
                    Err(err_msg) => Some(json!({
                        "jsonrpc": "2.0",
                        "id": req_id,
                        "error": {
                            "code": -32002,
                            "message": format!("Resource read failed: {err_msg}")
                        }
                    })),
                }
            }

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

    fn list_resources(&self) -> Vec<Value> {
        vec![
            json!({
                "uri": "looma://stats",
                "name": "Vault Statistics & Meta",
                "description": "Current vault storage size, total indexed assets, entities, and memories",
                "mimeType": "application/json"
            }),
            json!({
                "uri": "looma://timeline",
                "name": "Unified Timeline Activity",
                "description": "Chronological activity stream weaving assets, entities, and memory records",
                "mimeType": "application/json"
            }),
            json!({
                "uri": "looma://entities",
                "name": "Entity Catalog",
                "description": "All typed entity concepts registered in the vault",
                "mimeType": "application/json"
            }),
            json!({
                "uri": "looma://memories",
                "name": "Memories & Reflection Notes",
                "description": "Personal reflection notes, devlogs, and markdown entries",
                "mimeType": "application/json"
            }),
            json!({
                "uri": "looma://works",
                "name": "Personal Works & Cultural Records",
                "description": "Catalog of all registered works (Anime, Games, Movies, Books, Projects, Music)",
                "mimeType": "application/json"
            }),
            json!({
                "uri": "looma://audit",
                "name": "Recent Audit Trail",
                "description": "Recent 50 operations recorded across all actors (desktop, cli, mcp)",
                "mimeType": "application/json"
            }),
        ]
    }

    fn read_resource(&self, uri: &str) -> Result<(&'static str, String), String> {
        if uri == "looma://stats" {
            let stats = self.core.get_vault_stats().map_err(|e| e.to_string())?;
            let info = self.core.get_vault_info().map_err(|e| e.to_string())?;
            let combined = json!({
                "info": info,
                "stats": stats,
            });
            let s = serde_json::to_string_pretty(&combined).map_err(|e| e.to_string())?;
            return Ok(("application/json", s));
        }

        if uri == "looma://timeline" {
            let items = self.core.query_timeline(&TimelineFilter {
                limit: Some(50),
                ..Default::default()
            }).map_err(|e| e.to_string())?;
            let s = serde_json::to_string_pretty(&items).map_err(|e| e.to_string())?;
            return Ok(("application/json", s));
        }

        if uri == "looma://entities" {
            let entities = self.core.list_entities(&EntityFilter::default()).map_err(|e| e.to_string())?;
            let s = serde_json::to_string_pretty(&entities).map_err(|e| e.to_string())?;
            return Ok(("application/json", s));
        }

        if uri == "looma://works" || uri.starts_with("looma://works?") {
            let works = self.core.list_works(None, None).map_err(|e| e.to_string())?;
            let s = serde_json::to_string_pretty(&works).map_err(|e| e.to_string())?;
            return Ok(("application/json", s));
        }

        if uri == "looma://memories" {
            let memories = self.core.list_memories(50, 0).map_err(|e| e.to_string())?;
            let s = serde_json::to_string_pretty(&memories).map_err(|e| e.to_string())?;
            return Ok(("application/json", s));
        }

        if uri == "looma://audit" {
            let audits = self.core.list_recent_audits(50).map_err(|e| e.to_string())?;
            let s = serde_json::to_string_pretty(&audits).map_err(|e| e.to_string())?;
            return Ok(("application/json", s));
        }

        if let Some(id) = uri.strip_prefix("looma://asset/") {
            let asset = self.core.get_asset(id).map_err(|e| e.to_string())?
                .ok_or_else(|| format!("Asset not found: {id}"))?;
            let relations = self.core.list_relations_for_item(id).unwrap_or_default();
            let payload = json!({
                "asset": asset,
                "relations": relations,
            });
            let s = serde_json::to_string_pretty(&payload).map_err(|e| e.to_string())?;
            return Ok(("application/json", s));
        }

        let entity_id_opt = uri.strip_prefix("looma://entity/").or_else(|| uri.strip_prefix("looma://work/"));
        if let Some(id) = entity_id_opt {
            if let Some(summary) = self.core.get_work_summary(id).map_err(|e| e.to_string())? {
                let s = serde_json::to_string_pretty(&summary).map_err(|e| e.to_string())?;
                return Ok(("application/json", s));
            } else {
                return Err(format!("Entity/Work not found: {id}"));
            }
        }

        if let Some(id) = uri.strip_prefix("looma://memory/") {
            let memory = self.core.get_memory(id).map_err(|e| e.to_string())?
                .ok_or_else(|| format!("Memory not found: {id}"))?;
            let s = serde_json::to_string_pretty(&memory).map_err(|e| e.to_string())?;
            return Ok(("application/json", s));
        }

        Err(format!("Unsupported resource URI: {uri}"))
    }

    fn list_tools(&self) -> Vec<Value> {
        vec![
            // Read tools
            json!({
                "name": "search_assets",
                "description": "[Read] Search indexed digital assets in the Looma Vault by query, kind, or path",
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
                "description": "[Read] Get detailed metadata and relations for a specific indexed asset by ID",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "string", "description": "The asset ID" }
                    },
                    "required": ["id"]
                }
            }),
            json!({
                "name": "search_entities",
                "description": "[Read] Search user-defined entity concepts and personal records (anime, game, movie, book, project, person)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "query": { "type": "string", "description": "Search keyword for entity title or description" },
                        "entity_type": { "type": "string", "description": "Optional filter by type (anime, game, movie, book, project, person, work, etc.)" },
                        "status": { "type": "string", "description": "Optional status filter (planned, in_progress, completed, paused, dropped, revisit)" }
                    }
                }
            }),
            json!({
                "name": "get_entity",
                "description": "[Read] Get an entity with all linked relations and external references by ID",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "string", "description": "The entity ID" }
                    },
                    "required": ["id"]
                }
            }),
            json!({
                "name": "list_memories",
                "description": "[Read] List personal reflections, logs, and Markdown records from the vault",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "limit": { "type": "integer", "description": "Number of memories to return (default 20)" }
                    }
                }
            }),
            json!({
                "name": "get_memory",
                "description": "[Read] Fetch memory note content and metadata by ID",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "string", "description": "Memory ID" }
                    },
                    "required": ["id"]
                }
            }),
            json!({
                "name": "get_timeline",
                "description": "[Read] Fetch unified chronological stream weaving assets, entities, and memories",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "limit": { "type": "integer", "description": "Max timeline events to return" },
                        "item_type": { "type": "string", "description": "Optional filter: asset | entity | memory" }
                    }
                }
            }),
            json!({
                "name": "list_external_references",
                "description": "[Read] List external references (GitHub, Bilibili, Steam, Web) optionally filtered by entity",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "entity_id": { "type": "string", "description": "Optional entity ID filter" }
                    }
                }
            }),
            json!({
                "name": "get_smart_insights",
                "description": "[Read] Generate AI smart insights: relation suggestions, cluster proposals, and tag recommendations",
                "inputSchema": {
                    "type": "object",
                    "properties": {}
                }
            }),

            // Metadata write tools
            json!({
                "name": "create_entity",
                "description": "[MetadataWrite] Create a new typed entity concept or personal work (anime, game, book, movie, project, person)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "title": { "type": "string", "description": "Title or name of the entity/work" },
                        "entity_type": { "type": "string", "description": "Type of entity (anime, game, movie, tv_series, music, book, novel, project, person, concept)" },
                        "status": { "type": "string", "description": "Optional personal consumption status (planned, in_progress, completed, paused, dropped, revisit)" },
                        "description": { "type": "string", "description": "Optional description" },
                        "original_title": { "type": "string", "description": "Optional original native title" },
                        "release_year": { "type": "integer", "description": "Optional release year" },
                        "properties": { "type": "object", "description": "Optional dynamic JSON key-value properties" }
                    },
                    "required": ["title", "entity_type"]
                }
            }),
            json!({
                "name": "update_entity",
                "description": "[MetadataWrite] Update an existing typed entity concept",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "string", "description": "Entity ID to update" },
                        "title": { "type": "string", "description": "Updated title" },
                        "entity_type": { "type": "string", "description": "Updated entity type" },
                        "description": { "type": "string", "description": "Updated description" },
                        "properties": { "type": "object", "description": "Updated properties" }
                    },
                    "required": ["id", "title", "entity_type"]
                }
            }),
            json!({
                "name": "update_work_progress",
                "description": "[MetadataWrite] Update consumption progress for a work entity (episode, chapter, page, or percentage)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "string", "description": "The work/entity ID" },
                        "position": { "type": "number", "description": "Current position (e.g. 24.0 for episode 24)" },
                        "position_type": { "type": "string", "description": "Optional position type: episode, chapter, page, minute, percentage, custom" },
                        "total_positions": { "type": "number", "description": "Optional total positions (e.g. 24.0 episodes)" },
                        "unit": { "type": "string", "description": "Optional unit display string, e.g. '集', '话', '页'" }
                    },
                    "required": ["id", "position"]
                }
            }),
            json!({
                "name": "create_relation",
                "description": "[MetadataWrite] Create a relation between two vault items (assets, entities, memories)",
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
                "name": "create_external_reference",
                "description": "[MetadataWrite] Attach an external URL reference (GitHub repo, Bilibili video, Steam app, Web page) to an entity",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "entity_id": { "type": "string", "description": "Parent entity ID to link" },
                        "provider": { "type": "string", "description": "Provider: github | bilibili | steam | website | document" },
                        "url": { "type": "string", "description": "External target URL" },
                        "title": { "type": "string", "description": "Optional human-readable title" },
                        "external_id": { "type": "string", "description": "Optional provider-native ID (e.g. repo name or video bvid)" },
                        "is_primary": { "type": "boolean", "description": "Whether this reference is the primary entry point for the work" },
                        "metadata": { "type": "object", "description": "Optional provider specific metadata" }
                    },
                    "required": ["entity_id", "provider", "url"]
                }
            }),
            json!({
                "name": "set_primary_external_reference",
                "description": "[MetadataWrite] Set a specific external reference as the primary reference for a work entity",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "entity_id": { "type": "string", "description": "Work Entity ID" },
                        "reference_id": { "type": "string", "description": "External reference ID to mark as primary" }
                    },
                    "required": ["entity_id", "reference_id"]
                }
            }),
            json!({
                "name": "ensure_directory_asset",
                "description": "[MetadataWrite] Ensure a directory path is registered as a Directory Asset in the vault",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Filesystem directory path" }
                    },
                    "required": ["path"]
                }
            }),
            json!({
                "name": "create_work",
                "description": "[MetadataWrite] Register a new Work entity across any cultural medium (game, anime, manga, book, movie, tv_series, project, etc.)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "title": { "type": "string", "description": "Work title" },
                        "work_type": { "type": "string", "description": "Medium type: game | anime | manga | book | movie | tv_series | music | project | other" },
                        "status": { "type": "string", "description": "Status: planned | in_progress | completed | paused | dropped | revisit | archived" },
                        "original_title": { "type": "string", "description": "Optional native original title" },
                        "description": { "type": "string", "description": "Optional synopsis or description" },
                        "aliases": { "type": "array", "items": { "type": "string" }, "description": "Optional list of alias titles or alternative names" }
                    },
                    "required": ["title", "work_type"]
                }
            }),
            json!({
                "name": "link_work_directory",
                "description": "[MetadataWrite] Ensure a directory asset and link it to a work entity",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "work_id": { "type": "string", "description": "Work Entity ID" },
                        "path": { "type": "string", "description": "Filesystem directory path" }
                    },
                    "required": ["work_id", "path"]
                }
            }),
            json!({
                "name": "relocate_work_directory",
                "description": "[MetadataWrite] Relocate a work's directory asset to a new filesystem path without losing Work identity",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "work_id": { "type": "string", "description": "Work Entity ID" },
                        "old_asset_id": { "type": "string", "description": "ID of previous/missing directory asset" },
                        "new_path": { "type": "string", "description": "New filesystem directory path" }
                    },
                    "required": ["work_id", "old_asset_id", "new_path"]
                }
            }),
            json!({
                "name": "check_work_integrity",
                "description": "[ReadOnly] Validate data integrity, attachment direction, and invariants for a work entity",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "work_id": { "type": "string", "description": "Work Entity ID" }
                    },
                    "required": ["work_id"]
                }
            }),
            json!({
                "name": "preview_work_integrity_repair",
                "description": "[ReadOnly] Generate a deterministic preview of repair actions for an integrity violation on a Work entity",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "work_id": { "type": "string", "description": "Work Entity ID" }
                    },
                    "required": ["work_id"]
                }
            }),
            json!({
                "name": "repair_work_integrity",
                "description": "[MetadataWrite] Execute an authorized, single-transaction integrity repair plan for a Work entity",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "work_id": { "type": "string", "description": "Work Entity ID" },
                        "plan_id": { "type": "string", "description": "Deterministic Plan ID generated by preview_work_integrity_repair" }
                    },
                    "required": ["work_id", "plan_id"]
                }
            }),

            // Content write tools
            json!({
                "name": "create_memory",
                "description": "[ContentWrite] Create a new Markdown memory note in the vault",
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
                "name": "update_memory",
                "description": "[ContentWrite] Update an existing Markdown memory note",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "string", "description": "Memory ID to update" },
                        "title": { "type": "string", "description": "Updated note title" },
                        "content": { "type": "string", "description": "Updated Markdown body" },
                        "category": { "type": "string", "description": "Updated category" }
                    },
                    "required": ["id", "title", "content"]
                }
            }),
            json!({
                "name": "scan_directory",
                "description": "[ContentWrite] Trigger an incremental filesystem scan on a directory under Reference Mode",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Absolute filesystem directory path to scan" },
                        "compute_hash": { "type": "boolean", "description": "Whether to compute streaming SHA-256 (default false)" }
                    },
                    "required": ["path"]
                }
            }),

            // Destructive write tools
            json!({
                "name": "delete_asset",
                "description": "[DestructiveWrite] Delete an indexed asset record from the vault (does not delete original file)",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "string", "description": "Asset ID to delete" }
                    },
                    "required": ["id"]
                }
            }),
            json!({
                "name": "delete_entity",
                "description": "[DestructiveWrite] Delete a typed entity concept and all its cascade references",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "string", "description": "Entity ID to delete" }
                    },
                    "required": ["id"]
                }
            }),
            json!({
                "name": "delete_memory",
                "description": "[DestructiveWrite] Delete a memory note record",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "string", "description": "Memory ID to delete" }
                    },
                    "required": ["id"]
                }
            }),
            json!({
                "name": "delete_relation",
                "description": "[DestructiveWrite] Remove a relation by ID",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "string", "description": "Relation ID to delete" }
                    },
                    "required": ["id"]
                }
            }),
            json!({
                "name": "delete_external_reference",
                "description": "[DestructiveWrite] Remove an external reference link by ID",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "string", "description": "External reference ID to delete" }
                    },
                    "required": ["id"]
                }
            }),
            json!({
                "name": "vault_doctor",
                "description": "[Read / DestructiveWrite] Inspect vault health and optionally prune missing files",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "cleanup_missing": { "type": "boolean", "description": "If true (requires DestructiveWrite permission), mark missing assets and run VACUUM" }
                    }
                }
            })
        ]
    }

    fn check_permission(&self, required: McpPermissionLevel, op_name: &str) -> Result<(), String> {
        if self.permission < required {
            return Err(format!(
                "Permission Denied: Operation '{op_name}' requires '{required:?}' permission, but server is running with '{:?}'. Pass '--allow-destructive' to authorize.",
                self.permission
            ));
        }
        Ok(())
    }

    fn call_tool(&self, name: &str, args: Value) -> Result<String, String> {
        match name {
            // Read tools
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
                let assets = self.core.query_assets(&filter).map_err(|e| e.to_string())?;
                serde_json::to_string_pretty(&assets).map_err(|e| e.to_string())
            }

            "get_asset" => {
                let id = args.get("id").and_then(|v| v.as_str()).ok_or("Missing id parameter")?;
                let asset = self.core.get_asset(id).map_err(|e| e.to_string())?;
                match asset {
                    Some(a) => {
                        let relations = self.core.list_relations_for_item(id).unwrap_or_default();
                        let payload = json!({
                            "asset": a,
                            "relations": relations,
                        });
                        serde_json::to_string_pretty(&payload).map_err(|e| e.to_string())
                    }
                    None => Err(format!("Asset not found: {id}")),
                }
            }

            "search_entities" => {
                let q = args.get("query").and_then(|v| v.as_str()).map(|s| s.to_string());
                let entity_type = args.get("entity_type").and_then(|v| v.as_str()).map(|s| s.to_string());
                let status_filter = args.get("status").and_then(|v| v.as_str());

                let filter = EntityFilter {
                    entity_type,
                    search_query: q,
                    ..Default::default()
                };
                let mut entities = self.core.list_entities(&filter).map_err(|e| e.to_string())?;
                if let Some(status) = status_filter {
                    let parsed = RecordStatus::parse(status);
                    entities.retain(|e| {
                        let st = e.as_work_metadata()
                            .map(|m| m.status)
                            .unwrap_or_else(|| {
                                e.properties.get("status")
                                    .and_then(|v| v.as_str())
                                    .map(RecordStatus::parse)
                                    .unwrap_or(RecordStatus::Unknown)
                            });
                        st == parsed
                    });
                }
                serde_json::to_string_pretty(&entities).map_err(|e| e.to_string())
            }

            "get_entity" => {
                let id = args.get("id").and_then(|v| v.as_str()).ok_or("Missing id parameter")?;
                if let Some(summary) = self.core.get_work_summary(id).map_err(|e| e.to_string())? {
                    serde_json::to_string_pretty(&summary).map_err(|e| e.to_string())
                } else {
                    Err(format!("Entity not found: {id}"))
                }
            }

            "list_memories" => {
                let limit = args.get("limit").and_then(|v| v.as_u64()).map(|n| n as usize).unwrap_or(20);
                let memories = self.core.list_memories(limit, 0).map_err(|e| e.to_string())?;
                serde_json::to_string_pretty(&memories).map_err(|e| e.to_string())
            }

            "get_memory" => {
                let id = args.get("id").and_then(|v| v.as_str()).ok_or("Missing id parameter")?;
                let memory = self.core.get_memory(id).map_err(|e| e.to_string())?;
                match memory {
                    Some(m) => serde_json::to_string_pretty(&m).map_err(|e| e.to_string()),
                    None => Err(format!("Memory not found: {id}")),
                }
            }

            "get_timeline" => {
                let limit = args.get("limit").and_then(|v| v.as_u64()).map(|n| n as usize);
                let item_type = args.get("item_type").and_then(|v| v.as_str()).map(|s| vec![s.to_string()]);

                let filter = TimelineFilter {
                    item_types: item_type,
                    limit,
                    offset: None,
                };
                let items = self.core.query_timeline(&filter).map_err(|e| e.to_string())?;
                serde_json::to_string_pretty(&items).map_err(|e| e.to_string())
            }

            "list_external_references" => {
                let entity_id = args.get("entity_id").and_then(|v| v.as_str());
                let refs = self.core.list_external_references(entity_id).map_err(|e| e.to_string())?;
                serde_json::to_string_pretty(&refs).map_err(|e| e.to_string())
            }

            "get_smart_insights" => {
                let insights = looma_intelligence::IntelligenceEngine::generate_insights(&self.core)
                    .map_err(|e| e.to_string())?;
                serde_json::to_string_pretty(&insights).map_err(|e| e.to_string())
            }

            // Metadata write tools
            "create_entity" => {
                self.check_permission(McpPermissionLevel::MetadataWrite, "create_entity")?;
                let title = args.get("title").and_then(|v| v.as_str()).ok_or("Missing title")?;
                let entity_type = args.get("entity_type").and_then(|v| v.as_str()).ok_or("Missing entity_type")?;
                let desc = args.get("description").and_then(|v| v.as_str()).map(|s| s.to_string());
                let mut props = args.get("properties").cloned().unwrap_or(json!({}));
                if let Some(status_str) = args.get("status").and_then(|v| v.as_str()) {
                    props["status"] = json!(status_str);
                }

                let work_type = WorkType::parse(entity_type);
                if work_type != WorkType::Other || entity_type == "work" {
                    let status = props.get("status")
                        .and_then(|v| v.as_str())
                        .map(RecordStatus::parse)
                        .unwrap_or(RecordStatus::Planned);
                    props["domain"] = json!("work");
                    props["status"] = json!(status.as_str());
                    props["work"] = json!(WorkMetadata {
                        work_type,
                        status,
                        original_title: args.get("original_title").and_then(|v| v.as_str()).map(|s| s.to_string()),
                        release_year: args.get("release_year").and_then(|v| v.as_u64()).map(|n| n as u32),
                        start_date: args.get("start_date").and_then(|v| v.as_str()).map(|s| s.to_string()),
                        end_date: args.get("end_date").and_then(|v| v.as_str()).map(|s| s.to_string()),
                        cover_asset_id: None,
                        rating: args.get("rating").and_then(|v| v.as_f64()).map(|f| f as f32),
                        progress: None,
                        type_metadata: None,
                        aliases: Vec::new(),
                    });
                }

                let entity = Entity {
                    id: format!("ent_{}", &uuid::Uuid::new_v4().to_string()[..8]),
                    entity_type: entity_type.to_string(),
                    title: title.to_string(),
                    description: desc,
                    properties: props,
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                };
                self.core.create_entity("mcp", &entity).map_err(|e| e.to_string())?;
                serde_json::to_string_pretty(&entity).map_err(|e| e.to_string())
            }

            "update_entity" => {
                self.check_permission(McpPermissionLevel::MetadataWrite, "update_entity")?;
                let id = args.get("id").and_then(|v| v.as_str()).ok_or("Missing id")?;
                let existing = self.core.get_entity(id).map_err(|e| e.to_string())?
                    .ok_or_else(|| format!("Entity not found: {id}"))?;
                let title = args.get("title").and_then(|v| v.as_str()).unwrap_or(&existing.title);
                let entity_type = args.get("entity_type").and_then(|v| v.as_str()).unwrap_or(&existing.entity_type);
                let desc = match args.get("description") {
                    Some(v) => v.as_str().map(|s| s.to_string()),
                    None => existing.description,
                };
                let props = args.get("properties").cloned().unwrap_or(existing.properties);

                let updated = Entity {
                    id: existing.id,
                    entity_type: entity_type.to_string(),
                    title: title.to_string(),
                    description: desc,
                    properties: props,
                    created_at: existing.created_at,
                    updated_at: Utc::now(),
                };
                self.core.update_entity("mcp", &updated).map_err(|e| e.to_string())?;
                serde_json::to_string_pretty(&updated).map_err(|e| e.to_string())
            }

            "update_work_progress" => {
                self.check_permission(McpPermissionLevel::MetadataWrite, "update_work_progress")?;
                let id = args.get("id").and_then(|v| v.as_str()).ok_or("Missing id")?;
                let position = args.get("position").and_then(|v| v.as_f64()).ok_or("Missing position")?;
                let p_type = args.get("position_type").and_then(|v| v.as_str()).map(ProgressPositionType::parse);
                let total = args.get("total_positions").and_then(|v| v.as_f64());
                let unit = args.get("unit").and_then(|v| v.as_str()).map(|s| s.to_string());

                let updated = self.core.update_work_progress("mcp", id, position, p_type, total, unit)
                    .map_err(|e| e.to_string())?;
                serde_json::to_string_pretty(&updated).map_err(|e| e.to_string())
            }

            "create_relation" => {
                self.check_permission(McpPermissionLevel::MetadataWrite, "create_relation")?;
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
                self.core.create_relation("mcp", &relation).map_err(|e| e.to_string())?;
                serde_json::to_string_pretty(&relation).map_err(|e| e.to_string())
            }

            "create_external_reference" => {
                self.check_permission(McpPermissionLevel::MetadataWrite, "create_external_reference")?;
                let entity_id = args.get("entity_id").and_then(|v| v.as_str()).map(|s| s.to_string());
                let provider = args.get("provider").and_then(|v| v.as_str()).ok_or("Missing provider")?;
                let url = args.get("url").and_then(|v| v.as_str()).ok_or("Missing url")?;
                let title = args.get("title").and_then(|v| v.as_str()).unwrap_or(url);
                let desc = args.get("description").and_then(|v| v.as_str()).map(|s| s.to_string());
                let is_primary = args.get("is_primary").and_then(|v| v.as_bool()).unwrap_or(false);
                let mut meta = args.get("metadata").cloned().unwrap_or(json!({}));
                if is_primary {
                    if let Some(obj) = meta.as_object_mut() {
                        obj.insert("is_primary".to_string(), json!(true));
                    }
                }

                let ext_ref = ExternalReference {
                    id: format!("ref_{}", &uuid::Uuid::new_v4().to_string()[..8]),
                    entity_id,
                    provider: provider.to_string(),
                    title: title.to_string(),
                    url: url.to_string(),
                    description: desc,
                    metadata: meta,
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                };
                self.core.create_external_reference("mcp", &ext_ref).map_err(|e| e.to_string())?;
                serde_json::to_string_pretty(&ext_ref).map_err(|e| e.to_string())
            }

            "set_primary_external_reference" => {
                self.check_permission(McpPermissionLevel::MetadataWrite, "set_primary_external_reference")?;
                let entity_id = args.get("entity_id").and_then(|v| v.as_str()).ok_or("Missing entity_id")?;
                let reference_id = args.get("reference_id").and_then(|v| v.as_str()).ok_or("Missing reference_id")?;
                let updated = self.core.set_primary_external_reference("mcp", entity_id, reference_id).map_err(|e| e.to_string())?;
                serde_json::to_string_pretty(&updated).map_err(|e| e.to_string())
            }

            "ensure_directory_asset" => {
                self.check_permission(McpPermissionLevel::MetadataWrite, "ensure_directory_asset")?;
                let path = args.get("path").and_then(|v| v.as_str()).ok_or("Missing path")?;
                let asset = self.core.ensure_directory_asset("mcp", path).map_err(|e| e.to_string())?;
                serde_json::to_string_pretty(&asset).map_err(|e| e.to_string())
            }

            "create_work" => {
                self.check_permission(McpPermissionLevel::MetadataWrite, "create_work")?;
                let title = args.get("title").and_then(|v| v.as_str()).ok_or("Missing title")?;
                let work_type_str = args.get("work_type").and_then(|v| v.as_str()).ok_or("Missing work_type")?;
                let status_str = args.get("status").and_then(|v| v.as_str()).unwrap_or("planned");
                let orig_title = args.get("original_title").and_then(|v| v.as_str()).map(|s| s.to_string());
                let desc = args.get("description").and_then(|v| v.as_str()).map(|s| s.to_string());
                let aliases: Option<Vec<String>> = args.get("aliases")
                    .and_then(|v| v.as_array())
                    .map(|arr| arr.iter().filter_map(|s| s.as_str().map(|str_val| str_val.to_string())).collect());

                let work_type = WorkType::parse(work_type_str);
                let status = RecordStatus::parse(status_str);

                let mut work = self.core.create_work("mcp", title, work_type, status, orig_title, desc).map_err(|e| e.to_string())?;
                if let Some(alias_list) = aliases {
                    if !alias_list.is_empty() {
                        work = self.core.update_work_aliases("mcp", &work.id, alias_list).map_err(|e| e.to_string())?;
                    }
                }
                serde_json::to_string_pretty(&work).map_err(|e| e.to_string())
            }

            "link_work_directory" => {
                self.check_permission(McpPermissionLevel::MetadataWrite, "link_work_directory")?;
                let work_id = args.get("work_id").and_then(|v| v.as_str()).ok_or("Missing work_id")?;
                let path = args.get("path").and_then(|v| v.as_str()).ok_or("Missing path")?;
                let (asset, rel) = self.core.link_work_directory("mcp", work_id, path).map_err(|e| e.to_string())?;
                let res = json!({ "asset": asset, "relation": rel });
                serde_json::to_string_pretty(&res).map_err(|e| e.to_string())
            }

            "relocate_work_directory" => {
                self.check_permission(McpPermissionLevel::MetadataWrite, "relocate_work_directory")?;
                let work_id = args.get("work_id").and_then(|v| v.as_str()).ok_or("Missing work_id")?;
                let old_asset_id = args.get("old_asset_id").and_then(|v| v.as_str()).ok_or("Missing old_asset_id")?;
                let new_path = args.get("new_path").and_then(|v| v.as_str()).ok_or("Missing new_path")?;
                let (asset, rel) = self.core.relocate_work_directory("mcp", work_id, old_asset_id, new_path).map_err(|e| e.to_string())?;
                let res = json!({ "asset": asset, "relation": rel });
                serde_json::to_string_pretty(&res).map_err(|e| e.to_string())
            }

            "check_work_integrity" => {
                self.check_permission(McpPermissionLevel::ReadOnly, "check_work_integrity")?;
                let work_id = args.get("work_id").and_then(|v| v.as_str()).ok_or("Missing work_id")?;
                let report = self.core.validate_work_integrity(work_id).map_err(|e| e.to_string())?;
                serde_json::to_string_pretty(&report).map_err(|e| e.to_string())
            }

            "preview_work_integrity_repair" => {
                self.check_permission(McpPermissionLevel::ReadOnly, "preview_work_integrity_repair")?;
                let work_id = args.get("work_id").and_then(|v| v.as_str()).ok_or("Missing work_id")?;
                let plan = self.core.preview_work_integrity_repair(work_id).map_err(|e| e.to_string())?;
                serde_json::to_string_pretty(&plan).map_err(|e| e.to_string())
            }

            "repair_work_integrity" => {
                self.check_permission(McpPermissionLevel::MetadataWrite, "repair_work_integrity")?;
                let work_id = args.get("work_id").and_then(|v| v.as_str()).ok_or("Missing work_id")?;
                let plan_id = args.get("plan_id").and_then(|v| v.as_str()).ok_or("Missing plan_id")?;
                let preview = self.core.preview_work_integrity_repair(work_id).map_err(|e| e.to_string())?;
                let applied_ops = preview.operations.len();
                let report = self.core.repair_work_integrity("mcp", work_id, plan_id).map_err(|e| e.to_string())?;
                let resp = json!({
                    "success": report.valid,
                    "plan_id": plan_id,
                    "applied_operations": applied_ops,
                    "integrity": {
                        "valid": report.valid,
                        "violations": report.violations
                    }
                });
                serde_json::to_string_pretty(&resp).map_err(|e| e.to_string())
            }

            // Content write tools
            "create_memory" => {
                self.check_permission(McpPermissionLevel::ContentWrite, "create_memory")?;
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
                self.core.create_memory("mcp", &memory).map_err(|e| e.to_string())?;
                serde_json::to_string_pretty(&memory).map_err(|e| e.to_string())
            }

            "update_memory" => {
                self.check_permission(McpPermissionLevel::ContentWrite, "update_memory")?;
                let id = args.get("id").and_then(|v| v.as_str()).ok_or("Missing id")?;
                let existing = self.core.get_memory(id).map_err(|e| e.to_string())?
                    .ok_or_else(|| format!("Memory not found: {id}"))?;
                let title = args.get("title").and_then(|v| v.as_str()).unwrap_or(&existing.title);
                let content = args.get("content").and_then(|v| v.as_str()).unwrap_or(&existing.content);
                let category = match args.get("category") {
                    Some(v) => v.as_str().map(|s| s.to_string()),
                    None => existing.category,
                };

                let updated = Memory {
                    id: existing.id,
                    title: title.to_string(),
                    content: content.to_string(),
                    category,
                    metadata: existing.metadata,
                    recorded_at: existing.recorded_at,
                    created_at: existing.created_at,
                    updated_at: Utc::now(),
                };
                self.core.update_memory("mcp", &updated).map_err(|e| e.to_string())?;
                serde_json::to_string_pretty(&updated).map_err(|e| e.to_string())
            }

            "scan_directory" => {
                self.check_permission(McpPermissionLevel::ContentWrite, "scan_directory")?;
                let path_str = args.get("path").and_then(|v| v.as_str()).ok_or("Missing path parameter")?;
                let compute_hash = args.get("compute_hash").and_then(|v| v.as_bool()).unwrap_or(false);

                let options = ScanOptions {
                    compute_hash,
                    ..Default::default()
                };
                let summary = Scanner::scan_directory(
                    &self.core,
                    &PathBuf::from(path_str),
                    &options,
                    None::<fn(looma_scanner::ScanProgress)>,
                )
                .map_err(|e| e.to_string())?;
                serde_json::to_string_pretty(&summary).map_err(|e| e.to_string())
            }

            // Destructive write tools
            "delete_asset" => {
                self.check_permission(McpPermissionLevel::FullDestructive, "delete_asset")?;
                let id = args.get("id").and_then(|v| v.as_str()).ok_or("Missing id")?;
                let deleted = self.core.delete_asset("mcp", id).map_err(|e| e.to_string())?;
                Ok(json!({ "deleted": deleted, "id": id }).to_string())
            }

            "delete_entity" => {
                self.check_permission(McpPermissionLevel::FullDestructive, "delete_entity")?;
                let id = args.get("id").and_then(|v| v.as_str()).ok_or("Missing id")?;
                let deleted = self.core.delete_entity("mcp", id).map_err(|e| e.to_string())?;
                Ok(json!({ "deleted": deleted, "id": id }).to_string())
            }

            "delete_memory" => {
                self.check_permission(McpPermissionLevel::FullDestructive, "delete_memory")?;
                let id = args.get("id").and_then(|v| v.as_str()).ok_or("Missing id")?;
                let deleted = self.core.delete_memory("mcp", id).map_err(|e| e.to_string())?;
                Ok(json!({ "deleted": deleted, "id": id }).to_string())
            }

            "delete_relation" => {
                self.check_permission(McpPermissionLevel::FullDestructive, "delete_relation")?;
                let id = args.get("id").and_then(|v| v.as_str()).ok_or("Missing id")?;
                let deleted = self.core.delete_relation("mcp", id).map_err(|e| e.to_string())?;
                Ok(json!({ "deleted": deleted, "id": id }).to_string())
            }

            "delete_external_reference" => {
                self.check_permission(McpPermissionLevel::FullDestructive, "delete_external_reference")?;
                let id = args.get("id").and_then(|v| v.as_str()).ok_or("Missing id")?;
                let deleted = self.core.delete_external_reference("mcp", id).map_err(|e| e.to_string())?;
                Ok(json!({ "deleted": deleted, "id": id }).to_string())
            }

            "vault_doctor" => {
                let cleanup = args.get("cleanup_missing").and_then(|v| v.as_bool()).unwrap_or(false);
                let report = self.core.doctor_inspect().map_err(|e| e.to_string())?;
                if cleanup {
                    self.check_permission(McpPermissionLevel::FullDestructive, "vault_doctor(cleanup_missing=true)")?;
                    let cleaned = self.core.doctor_cleanup_missing("mcp").map_err(|e| e.to_string())?;
                    Ok(format!(
                        "Vault Doctor Report:\n{}\nCleaned Missing Items: {cleaned}",
                        serde_json::to_string_pretty(&report).unwrap_or_default()
                    ))
                } else {
                    serde_json::to_string_pretty(&report).map_err(|e| e.to_string())
                }
            }

            _ => Err(format!("Unknown tool: {name}")),
        }
    }
}
