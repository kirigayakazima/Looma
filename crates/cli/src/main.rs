use std::path::PathBuf;
use std::sync::Arc;
use chrono::Utc;
use clap::{Args, Parser, Subcommand};
use directories::ProjectDirs;
use serde_json::json;

use looma_core::models::*;
use looma_core::LoomaCore;
use looma_database::LoomaDb;
use looma_mcp::McpServer;
use looma_scanner::{ScanOptions, Scanner};

#[derive(Parser)]
#[command(name = "looma")]
#[command(author = "Looma Team")]
#[command(version = "0.2.0")]
#[command(about = "Looma - Local-First Personal Digital Vault CLI", long_about = None)]
struct Cli {
    #[arg(short, long, help = "Custom vault root directory path")]
    vault: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    #[command(about = "Display vault status, statistics, and storage paths")]
    Status,

    #[command(about = "Trigger incremental scanner on a directory")]
    Scan(ScanArgs),

    #[command(about = "Search assets, entities, or memories")]
    Search(SearchArgs),

    #[command(about = "Asset management commands")]
    Asset {
        #[command(subcommand)]
        action: AssetCommands,
    },

    #[command(about = "Entity concept management commands")]
    Entity {
        #[command(subcommand)]
        action: EntityCommands,
    },

    #[command(about = "Personal Records & Works management (Anime, Game, Movie, Book, Project, Music)")]
    Work {
        #[command(subcommand)]
        action: WorkCommands,
    },

    #[command(about = "Memory notes management commands")]
    Memory {
        #[command(subcommand)]
        action: MemoryCommands,
    },

    #[command(about = "List collection groupings")]
    Collection,

    #[command(about = "View unified chronological timeline")]
    Timeline(TimelineArgs),

    #[command(about = "Manage external references (GitHub, Bilibili, Steam, Web)")]
    Ref {
        #[command(subcommand)]
        action: RefCommands,
    },

    #[command(about = "Display recent audit trail of operations")]
    Audit {
        #[arg(short, long, default_value_t = 30, help = "Number of audit entries to show")]
        limit: usize,
    },

    #[command(about = "Vault backup, export, and disaster recovery")]
    Backup(BackupArgs),

    #[command(about = "Inspect vault health, check missing files and compact database")]
    Doctor(DoctorArgs),

    #[command(about = "Generate AI smart insights and relationship suggestions")]
    Suggest(SuggestArgs),

    #[command(about = "Run Model Context Protocol (MCP) server over stdio for AI agents")]
    Mcp,
}

#[derive(Args)]
struct SuggestArgs {
    #[arg(short, long, help = "Filter suggestion type: relation, tag, cluster")]
    kind: Option<String>,

    #[arg(long, help = "Automatically apply all high-confidence suggestions")]
    auto_apply: bool,
}

#[derive(Args)]
struct ScanArgs {
    #[arg(help = "Filesystem path to scan")]
    path: PathBuf,

    #[arg(short = 'H', long, help = "Compute streaming SHA-256 hash")]
    hash: bool,
}

#[derive(Args)]
struct SearchArgs {
    #[arg(help = "Search query keyword")]
    query: String,

    #[arg(short, long, help = "Filter by kind: image, video, audio, document, code, archive, model3d")]
    kind: Option<String>,
}

#[derive(Subcommand)]
enum AssetCommands {
    #[command(about = "List indexed assets")]
    List {
        #[arg(short, long, help = "Filter by kind")]
        kind: Option<String>,
        #[arg(short, long, default_value_t = 20, help = "Max assets to list")]
        limit: usize,
    },
    #[command(about = "Get detailed asset metadata by ID")]
    Get {
        #[arg(help = "Asset ID")]
        id: String,
    },
    #[command(about = "Delete an indexed asset record from the vault")]
    Delete {
        #[arg(help = "Asset ID")]
        id: String,
    },
}

#[derive(Subcommand)]
enum EntityCommands {
    #[command(about = "List entity concepts")]
    List {
        #[arg(short = 't', long, help = "Filter by entity type")]
        entity_type: Option<String>,
    },
    #[command(about = "Get entity details by ID")]
    Get {
        #[arg(help = "Entity ID")]
        id: String,
    },
    #[command(about = "Create a new typed entity concept")]
    Create {
        #[arg(help = "Entity title")]
        title: String,
        #[arg(short = 't', long, default_value = "concept", help = "Entity type")]
        entity_type: String,
        #[arg(short, long, help = "Optional description")]
        desc: Option<String>,
    },
    #[command(about = "Delete an entity concept by ID")]
    Delete {
        #[arg(help = "Entity ID")]
        id: String,
    },
}

#[derive(Subcommand)]
enum MemoryCommands {
    #[command(about = "List memory notes")]
    List {
        #[arg(short, long, default_value_t = 20, help = "Max memories to list")]
        limit: usize,
    },
    #[command(about = "Create a new memory note")]
    Create {
        #[arg(help = "Note title")]
        title: String,
        #[arg(help = "Markdown body content")]
        content: String,
        #[arg(short, long, default_value = "journal", help = "Category")]
        category: String,
    },
    #[command(about = "Delete a memory note by ID")]
    Delete {
        #[arg(help = "Memory ID")]
        id: String,
    },
}

#[derive(Subcommand)]
enum WorkCommands {
    #[command(about = "List registered works")]
    List {
        #[arg(short = 't', long, help = "Filter by type (anime, game, movie, tv_series, music, book, novel, etc.)")]
        kind: Option<String>,
        #[arg(short, long, help = "Filter by status (planned, in_progress, completed, paused, dropped, revisit)")]
        status: Option<String>,
    },
    #[command(about = "Register a new personal work")]
    Create {
        #[arg(help = "Title of the work")]
        title: String,
        #[arg(short = 't', long, default_value = "anime", help = "Work type (anime, game, movie, book, project, etc.)")]
        kind: String,
        #[arg(short, long, default_value = "planned", help = "Status (planned, in_progress, completed, paused, dropped, revisit, archived)")]
        status: String,
        #[arg(long, help = "Original native title")]
        orig: Option<String>,
        #[arg(short, long, help = "Description or synopsis")]
        desc: Option<String>,
        #[arg(long, value_delimiter = ',', help = "Alternative titles or aliases (comma-separated)")]
        aliases: Vec<String>,
    },
    #[command(about = "Add an alias title to a work")]
    Alias {
        #[arg(help = "Work Entity ID")]
        id: String,
        #[arg(help = "Alias to add")]
        alias: String,
    },
    #[command(about = "Update consumption status of a work")]
    Status {
        #[arg(help = "Work Entity ID")]
        id: String,
        #[arg(help = "New status (planned, in_progress, completed, paused, dropped, revisit, archived)")]
        status: String,
    },
    #[command(about = "Update consumption progress (episode, chapter, page, percentage)")]
    Progress {
        #[arg(help = "Work Entity ID")]
        id: String,
        #[arg(help = "Current position (e.g. 24 or 520)")]
        position: f64,
        #[arg(short = 't', long, help = "Position type: episode, chapter, page, minute, percentage, custom")]
        kind: Option<String>,
        #[arg(long, help = "Total positions (e.g. 24 episodes total)")]
        total: Option<f64>,
        #[arg(short, long, help = "Custom unit (e.g. 集, 话, 页, %)")]
        unit: Option<String>,
    },
    #[command(about = "Inspect full personal record and digital footprint for a work")]
    Show {
        #[arg(help = "Work Entity ID")]
        id: String,
    },
    #[command(about = "Link a work to an asset or memory")]
    Relate {
        #[arg(help = "Work Entity ID")]
        work_id: String,
        #[arg(help = "Target Item ID (Asset ID or Memory ID)")]
        target_id: String,
        #[arg(short, long, default_value = "referenced_by", help = "Relation descriptor (attaches, referenced_by, created_by, etc.)")]
        relation: String,
    },
    #[command(about = "Ensure a local directory asset and link it to a work")]
    LinkDir {
        #[arg(help = "Work Entity ID")]
        work_id: String,
        #[arg(help = "Filesystem directory path")]
        path: PathBuf,
    },
    #[command(about = "Relocate a work's directory asset to a new filesystem path")]
    RelocateDir {
        #[arg(help = "Work Entity ID")]
        work_id: String,
        #[arg(help = "Old Asset ID to replace")]
        old_asset_id: String,
        #[arg(help = "New directory path")]
        new_path: PathBuf,
    },
    #[command(about = "Validate integrity and invariants of a work entity")]
    Check {
        #[arg(help = "Work Entity ID")]
        id: String,
    },
}

#[derive(Subcommand)]
enum RefCommands {
    #[command(about = "List external references")]
    List {
        #[arg(short, long, help = "Filter by entity ID")]
        entity_id: Option<String>,
    },
    #[command(about = "Add an external reference")]
    Add {
        #[arg(short, long, help = "Provider (github, bilibili, steam, website, document)")]
        provider: String,
        #[arg(short, long, help = "Target URL")]
        url: String,
        #[arg(short, long, help = "Display title")]
        title: Option<String>,
        #[arg(short, long, help = "Associated Entity ID")]
        entity_id: Option<String>,
        #[arg(long, help = "Mark this reference as primary entry point")]
        primary: bool,
    },
    #[command(about = "Set an external reference as primary for its entity")]
    Primary {
        #[arg(help = "Work Entity ID")]
        entity_id: String,
        #[arg(help = "Reference ID to mark as primary")]
        reference_id: String,
    },
    #[command(about = "Delete an external reference by ID")]
    Delete {
        #[arg(help = "Reference ID")]
        id: String,
    },
}

#[derive(Args)]
struct TimelineArgs {
    #[arg(short, long, help = "Filter by type: asset | entity | memory")]
    item_type: Option<String>,
    #[arg(short, long, default_value_t = 30, help = "Max items to display")]
    limit: usize,
}

#[derive(Args)]
struct BackupArgs {
    #[arg(short, long, help = "Export backup package to destination directory")]
    export: Option<PathBuf>,

    #[arg(short, long, help = "Restore vault database from backup directory")]
    restore: Option<PathBuf>,
}

#[derive(Args)]
struct DoctorArgs {
    #[arg(short, long, help = "Mark missing files and run SQLite VACUUM compaction")]
    clean: bool,
}

fn resolve_default_vault_path(custom: Option<PathBuf>) -> PathBuf {
    if let Some(p) = custom {
        return p;
    }
    if let Some(proj_dirs) = ProjectDirs::from("com", "looma", "Looma") {
        let data_dir = proj_dirs.data_dir();
        std::fs::create_dir_all(data_dir).ok();
        data_dir.to_path_buf()
    } else {
        PathBuf::from("./looma_vault")
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let vault_path = resolve_default_vault_path(cli.vault);

    // Mcp command does not print headers so that stdio remains clean for JSON-RPC
    if let Commands::Mcp = cli.command {
        let db = LoomaDb::open(&vault_path)?;
        let core = LoomaCore::new(Arc::new(db));
        let server = McpServer::new(core);
        server.run_stdio()?;
        return Ok(());
    }

    println!("======================================================");
    println!("  Looma Digital Vault CLI (v0.2.0 - Core Facade)      ");
    println!("  Vault Root: {}", vault_path.display());
    println!("======================================================\n");

    let db = LoomaDb::open(&vault_path)?;
    let core = LoomaCore::new(Arc::new(db));

    match cli.command {
        Commands::Status => {
            let info = core.get_vault_info()?;
            let stats = core.get_vault_stats()?;
            println!("Vault Name:       {}", info.name);
            println!("Architecture:     v{}", info.version);
            println!("Database:         {}", info.database_path.display());
            println!("Indexed Assets:   {}", stats.total_assets);
            println!("Typed Entities:   {}", stats.total_entities);
            println!("Memories Count:   {}", stats.total_memories);
            println!("Collections:      {}", stats.total_collections);
            println!("Database Size:    {:.2} MB", stats.database_size_bytes as f64 / (1024.0 * 1024.0));
        }

        Commands::Scan(args) => {
            println!("Scanning directory: {}", args.path.display());
            println!("Compute SHA-256:    {}", args.hash);
            let options = ScanOptions {
                compute_hash: args.hash,
                ..Default::default()
            };
            let summary = Scanner::scan_directory(
                &core,
                &args.path,
                &options,
                Some(|prog: looma_scanner::ScanProgress| {
                    print!(
                        "\rDiscovered: {} | New: {} | Modified: {} | Unchanged: {}",
                        prog.total_discovered,
                        prog.new_assets,
                        prog.modified_assets,
                        prog.unchanged_assets
                    );
                    std::io::Write::flush(&mut std::io::stdout()).ok();
                }),
            )?;
            println!();
            println!("\nScan Summary:");
            println!("  Total files:     {}", summary.total_scanned);
            println!("  New indexed:     {}", summary.new_assets);
            println!("  Updated:         {}", summary.modified_assets);
            println!("  Unchanged:       {}", summary.unchanged_assets);
            println!("  Missing/deleted: {}", summary.missing_assets);
        }

        Commands::Search(args) => {
            let kind = args.kind.and_then(|k| serde_json::from_str(&format!("\"{}\"", k)).ok());
            let filter = AssetFilter {
                search_query: Some(args.query.clone()),
                kind,
                limit: Some(25),
                ..Default::default()
            };
            let assets = core.query_assets(&filter)?;
            println!("Found {} assets matching '{}':\n", assets.len(), args.query);
            for a in assets {
                let name = a.path.as_deref().unwrap_or(&a.id);
                println!("  [{:?}] {} (ID: {})", a.kind, name, a.id);
            }
        }

        Commands::Asset { action } => match action {
            AssetCommands::List { kind, limit } => {
                let kind_enum = kind.and_then(|k| serde_json::from_str(&format!("\"{}\"", k)).ok());
                let filter = AssetFilter {
                    kind: kind_enum,
                    limit: Some(limit),
                    ..Default::default()
                };
                let assets = core.query_assets(&filter)?;
                println!("Indexed Assets (showing up to {}):\n", limit);
                for a in assets {
                    let name = a.path.as_deref().unwrap_or(&a.id);
                    println!("  - [{:?}] {}", a.kind, name);
                }
            }
            AssetCommands::Get { id } => {
                if let Some(a) = core.get_asset(&id)? {
                    println!("{}", serde_json::to_string_pretty(&a)?);
                } else {
                    println!("Asset not found: {}", id);
                }
            }
            AssetCommands::Delete { id } => {
                let deleted = core.delete_asset("cli", &id)?;
                if deleted {
                    println!("Asset deleted: {}", id);
                } else {
                    println!("Asset not found: {}", id);
                }
            }
        },

        Commands::Entity { action } => match action {
            EntityCommands::List { entity_type } => {
                let filter = EntityFilter {
                    entity_type,
                    ..Default::default()
                };
                let entities = core.list_entities(&filter)?;
                println!("Entity Concepts ({}):\n", entities.len());
                for e in entities {
                    println!("  * [{}] {} (ID: {})", e.entity_type, e.title, e.id);
                    if let Some(d) = e.description {
                        println!("    {}", d);
                    }
                }
            }
            EntityCommands::Get { id } => {
                if let Some(e) = core.get_entity(&id)? {
                    println!("{}", serde_json::to_string_pretty(&e)?);
                } else {
                    println!("Entity not found: {}", id);
                }
            }
            EntityCommands::Create { title, entity_type, desc } => {
                let entity = Entity {
                    id: format!("ent_{}", &uuid::Uuid::new_v4().to_string()[..8]),
                    entity_type,
                    title: title.clone(),
                    description: desc,
                    properties: json!({}),
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                };
                core.create_entity("cli", &entity)?;
                println!("Created entity: {} (ID: {})", title, entity.id);
            }
            EntityCommands::Delete { id } => {
                let deleted = core.delete_entity("cli", &id)?;
                if deleted {
                    println!("Entity deleted: {}", id);
                } else {
                    println!("Entity not found: {}", id);
                }
            }
        },

        Commands::Work { action } => match action {
            WorkCommands::List { kind, status } => {
                let works = core.list_works(kind.as_deref(), status.as_deref())?;
                println!("Personal Records & Works ({}):\n", works.len());
                for w in works {
                    let meta = w.as_work_metadata();
                    let st = meta.as_ref().map(|m| m.status.as_str()).unwrap_or("unknown");
                    let prog_str = meta.as_ref().and_then(|m| m.progress.as_ref()).map(|p| {
                        let u = p.unit.as_deref().unwrap_or(p.position_type.as_str());
                        if let Some(t) = p.total_positions {
                            format!(" [{:.0}/{:.0} {}]", p.position, t, u)
                        } else {
                            format!(" [{:.0} {}]", p.position, u)
                        }
                    }).unwrap_or_default();
                    println!("  * [{:^10}] [{:^11}]{} {} (ID: {})", w.entity_type, st, prog_str, w.title, w.id);
                    if let Some(d) = &w.description {
                        println!("      {}", d);
                    }
                }
            }
            WorkCommands::Create { title, kind, status, orig, desc, aliases } => {
                let work_type = WorkType::parse(&kind);
                let record_status = RecordStatus::parse(&status);
                let mut work = core.create_work("cli", &title, work_type, record_status, orig, desc)?;
                if !aliases.is_empty() {
                    work = core.update_work_aliases("cli", &work.id, aliases)?;
                }
                println!("Registered new work: {} [{}] ({}) ID: {}", title, work_type.as_str(), record_status.as_str(), work.id);
            }
            WorkCommands::Alias { id, alias } => {
                let entity = core.get_entity(&id)?
                    .ok_or_else(|| looma_core::LoomaError::NotFound(format!("Work entity not found: {id}")))?;
                let mut existing_aliases = entity.as_work_metadata().map(|m| m.aliases).unwrap_or_default();
                if !existing_aliases.contains(&alias) {
                    existing_aliases.push(alias.clone());
                }
                let updated = core.update_work_aliases("cli", &id, existing_aliases)?;
                let aliases_now = updated.as_work_metadata().map(|m| m.aliases).unwrap_or_default();
                println!("Updated aliases for {}: {:?}", updated.title, aliases_now);
            }
            WorkCommands::Status { id, status } => {
                let record_status = RecordStatus::parse(&status);
                let updated = core.update_work_status("cli", &id, record_status)?;
                println!("Updated work status: {} -> [{}]", updated.title, record_status.as_str());
            }
            WorkCommands::Progress { id, position, kind, total, unit } => {
                let p_type = kind.map(|k| ProgressPositionType::parse(&k));
                let updated = core.update_work_progress("cli", &id, position, p_type, total, unit)?;
                let meta = updated.as_work_metadata();
                let prog_display = meta.and_then(|m| m.progress).map(|p| {
                    let u = p.unit.as_deref().unwrap_or(p.position_type.as_str());
                    if let Some(t) = p.total_positions {
                        format!("{:.0}/{:.0} {}", p.position, t, u)
                    } else {
                        format!("{:.0} {}", p.position, u)
                    }
                }).unwrap_or_else(|| format!("{position}"));
                println!("Updated work progress: {} -> [{}]", updated.title, prog_display);
            }
            WorkCommands::Show { id } => {
                if let Some(summary) = core.get_work_summary(&id)? {
                    let w = &summary.entity;
                    let meta = summary.work_metadata.as_ref();
                    let st = meta.map(|m| m.status.as_str()).unwrap_or("unknown");
                    println!("======================================================");
                    println!("  Work: {} [{}]", w.title, w.entity_type);
                    println!("  ID:     {}", w.id);
                    println!("  Status: {}", st);
                    if let Some(m) = meta {
                        if !m.aliases.is_empty() {
                            println!("  Aliases: {}", m.aliases.join(", "));
                        }
                        if let Some(p) = &m.progress {
                            let u = p.unit.as_deref().unwrap_or(p.position_type.as_str());
                            if let Some(t) = p.total_positions {
                                println!("  Progress: {:.0} / {:.0} {} ({})", p.position, t, u, p.position_type.as_str());
                            } else {
                                println!("  Progress: {:.0} {} ({})", p.position, u, p.position_type.as_str());
                            }
                        }
                        if let Some(orig) = &m.original_title {
                            println!("  Native: {}", orig);
                        }
                    }
                    if let Some(d) = &w.description {
                        println!("  Description: {}", d);
                    }
                    println!("------------------------------------------------------");
                    println!("  Linked Assets ({})", summary.linked_assets.len());
                    for a in &summary.linked_assets {
                        let path = a.path.as_deref().unwrap_or(&a.id);
                        println!("    - [{:?}] {}", a.kind, path);
                    }
                    println!("  Linked Memories ({})", summary.linked_memories.len());
                    for m in &summary.linked_memories {
                        println!("    - [{}] {}", m.recorded_at.format("%Y-%m-%d"), m.title);
                    }
                    println!("  Linked Collections ({})", summary.linked_collections.len());
                    for c in &summary.linked_collections {
                        println!("    - # {}", c.title);
                    }
                    println!("  External References ({})", summary.external_references.len());
                    for r in &summary.external_references {
                        let primary_tag = if r.is_primary() { " [PRIMARY]" } else { "" };
                        println!("    - [{}] {}{} -> {}", r.provider, r.title, primary_tag, r.url);
                    }
                    println!("======================================================");
                } else {
                    println!("Work not found: {}", id);
                }
            }
            WorkCommands::Relate { work_id, target_id, relation } => {
                if target_id.starts_with("mem_") {
                    let rel = core.link_work_memory("cli", &work_id, &target_id)?;
                    println!("Linked work {} <-> memory {} with [{}] (Rel ID: {})", work_id, target_id, rel.relation_type, rel.id);
                } else {
                    let rel = core.link_work_asset("cli", &work_id, &target_id, Some(&relation))?;
                    println!("Linked work {} <-> asset {} with [{}] (Rel ID: {})", work_id, target_id, rel.relation_type, rel.id);
                }
            }
            WorkCommands::LinkDir { work_id, path } => {
                let path_str = path.to_string_lossy();
                let (asset, rel) = core.link_work_directory("cli", &work_id, &path_str)?;
                println!("Successfully linked directory asset:");
                println!("  Asset ID: {}", asset.id);
                println!("  Path:     {}", asset.path.unwrap_or_default());
                println!("  Relation: {} [{}]", rel.id, rel.relation_type);
            }
            WorkCommands::RelocateDir { work_id, old_asset_id, new_path } => {
                let path_str = new_path.to_string_lossy();
                let (asset, rel) = core.relocate_work_directory("cli", &work_id, &old_asset_id, &path_str)?;
                println!("Successfully relocated work directory asset:");
                println!("  Work ID:      {}", work_id);
                println!("  Old Asset ID: {}", old_asset_id);
                println!("  New Asset ID: {}", asset.id);
                println!("  New Path:     {}", asset.path.unwrap_or_default());
                println!("  New Relation: {} [{}]", rel.id, rel.relation_type);
            }
            WorkCommands::Check { id } => {
                let report = core.validate_work_integrity(&id)?;
                if report.valid {
                    println!("Work Integrity: PASS");
                    println!("No violations found.");
                } else {
                    println!("Work Integrity: FAIL\n");
                    for v in &report.violations {
                        println!("- {}", v.message);
                    }
                    std::process::exit(1);
                }
            }
        },

        Commands::Memory { action } => match action {
            MemoryCommands::List { limit } => {
                let memories = core.list_memories(limit, 0)?;
                println!("Memory Notes ({}):\n", memories.len());
                for m in memories {
                    let cat = m.category.unwrap_or_else(|| "general".to_string());
                    println!("  [{} | {}] {}", m.recorded_at.format("%Y-%m-%d"), cat, m.title);
                }
            }
            MemoryCommands::Create { title, content, category } => {
                let memory = Memory {
                    id: format!("mem_{}", &uuid::Uuid::new_v4().to_string()[..8]),
                    title: title.clone(),
                    content,
                    category: Some(category),
                    metadata: json!({}),
                    recorded_at: Utc::now(),
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                };
                core.create_memory("cli", &memory)?;
                println!("Created memory note: {} (ID: {})", title, memory.id);
            }
            MemoryCommands::Delete { id } => {
                let deleted = core.delete_memory("cli", &id)?;
                if deleted {
                    println!("Memory deleted: {}", id);
                } else {
                    println!("Memory not found: {}", id);
                }
            }
        },

        Commands::Ref { action } => match action {
            RefCommands::List { entity_id } => {
                let refs = core.list_external_references(entity_id.as_deref())?;
                println!("External References ({}):\n", refs.len());
                for r in refs {
                    let primary_tag = if r.is_primary() { " [PRIMARY]" } else { "" };
                    println!("  [{}] {}{} -> {} (ID: {})", r.provider, r.title, primary_tag, r.url, r.id);
                }
            }
            RefCommands::Add { provider, url, title, entity_id, primary } => {
                let ext_ref = ExternalReference {
                    id: format!("ref_{}", &uuid::Uuid::new_v4().to_string()[..8]),
                    entity_id: entity_id.clone(),
                    provider,
                    title: title.unwrap_or_else(|| url.clone()),
                    url: url.clone(),
                    description: None,
                    metadata: json!({}),
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                }.with_primary(primary);
                core.create_external_reference("cli", &ext_ref)?;
                if primary {
                    if let Some(ref eid) = entity_id {
                        let _ = core.set_primary_external_reference("cli", eid, &ext_ref.id);
                    }
                }
                println!("Created external reference: {} (ID: {}, Primary: {})", ext_ref.url, ext_ref.id, primary);
            }
            RefCommands::Primary { entity_id, reference_id } => {
                let updated = core.set_primary_external_reference("cli", &entity_id, &reference_id)?;
                println!("Set reference {} as primary for entity {}. Total references: {}", reference_id, entity_id, updated.len());
            }
            RefCommands::Delete { id } => {
                let deleted = core.delete_external_reference("cli", &id)?;
                if deleted {
                    println!("External reference deleted: {}", id);
                } else {
                    println!("External reference not found: {}", id);
                }
            }
        },

        Commands::Audit { limit } => {
            let logs = core.list_recent_audits(limit)?;
            println!("Recent Audit Trail (showing up to {}):\n", limit);
            for log in logs {
                println!(
                    "  {} [{}] {} {} on {} -> {}",
                    log.timestamp.format("%Y-%m-%d %H:%M:%S"),
                    log.actor,
                    log.operation,
                    log.target_type,
                    log.target_id,
                    log.result
                );
            }
        }

        Commands::Collection => {
            let cols = core.list_collections()?;
            println!("Collections ({}):\n", cols.len());
            for c in cols {
                println!("  # {} (ID: {})", c.title, c.id);
                if let Some(d) = c.description {
                    println!("    {}", d);
                }
            }
        }

        Commands::Timeline(args) => {
            let filter = TimelineFilter {
                item_types: args.item_type.map(|t| vec![t]),
                limit: Some(args.limit),
                offset: None,
            };
            let items = core.query_timeline(&filter)?;
            println!("Timeline Stream ({} events):\n", items.len());
            for item in items {
                println!(
                    "  {} [{}] {}",
                    item.timestamp.format("%Y-%m-%d %H:%M"),
                    item.item_type,
                    item.title
                );
            }
        }

        Commands::Backup(args) => {
            if let Some(dest) = args.export {
                println!("Exporting vault backup to {}...", dest.display());
                let res = core.export_backup("cli", &dest)?;
                println!("Backup successful! Path: {}", res.backup_path);
                println!("Duration: {} ms", res.duration_ms);
            } else if let Some(restore_path) = args.restore {
                println!("Restoring vault from {}...", restore_path.display());
                let res = core.restore_backup("cli", &restore_path)?;
                println!("Restore complete!");
                println!("  Restored Assets:      {}", res.restored_assets);
                println!("  Restored Entities:    {}", res.restored_entities);
                println!("  Restored Memories:    {}", res.restored_memories);
                println!("  Restored Collections: {}", res.restored_collections);
                println!("  Duration:             {} ms", res.duration_ms);
            } else {
                let manifest = core.generate_manifest()?;
                println!("Vault Manifest Snapshot:");
                println!("{}", serde_json::to_string_pretty(&manifest)?);
            }
        }

        Commands::Doctor(args) => {
            println!("Running Vault Doctor inspection...");
            let report = core.doctor_inspect()?;
            println!("  SQLite Integrity:      {}", report.integrity_message);
            println!("  Active Assets:         {}", report.active_assets);
            println!("  Total Assets:          {}", report.total_assets);
            println!("  Missing Assets Count:  {}", report.missing_assets.len());
            println!("  Database Size:         {:.2} KB", report.database_size_bytes as f64 / 1024.0);

            if !report.missing_assets.is_empty() {
                println!("\nMissing files:");
                for p in &report.missing_assets {
                    println!("    - {}", p);
                }
            }

            if args.clean {
                println!("\nCleaning missing assets and compacting database (VACUUM)...");
                let cleaned = core.doctor_cleanup_missing("cli")?;
                println!("Successfully cleaned {} missing items and compacted SQLite database!", cleaned);
            }
        }

        Commands::Suggest(args) => {
            println!("Generating Smart Insights & Relationship Suggestions (Phase 6)...");
            let insights = looma_intelligence::IntelligenceEngine::generate_insights(&core)?;

            println!(
                "\nFound {} suggestions in total: {} relations, {} tags, {} clusters\n",
                insights.total_suggestions,
                insights.relation_suggestions.len(),
                insights.tag_suggestions.len(),
                insights.cluster_suggestions.len()
            );

            // Filter if requested
            let filter = args.kind.as_deref().unwrap_or("all");

            if filter == "all" || filter == "relation" {
                if !insights.relation_suggestions.is_empty() {
                    println!("--- Relationship Suggestions ---");
                    for s in &insights.relation_suggestions {
                        println!("  [{:.0}% Confidence] {}", s.confidence * 100.0, s.title);
                        println!("     Reason: {}", s.description);
                    }
                    println!();
                }
            }

            if filter == "all" || filter == "cluster" {
                if !insights.cluster_suggestions.is_empty() {
                    println!("--- Collection Clustering Suggestions ---");
                    for s in &insights.cluster_suggestions {
                        println!("  [{:.0}% Confidence] {}", s.confidence * 100.0, s.title);
                        println!("     Reason: {}", s.description);
                    }
                    println!();
                }
            }

            if filter == "all" || filter == "tag" {
                if !insights.tag_suggestions.is_empty() {
                    println!("--- Smart Tag Suggestions (First 5) ---");
                    for s in insights.tag_suggestions.iter().take(5) {
                        println!("  {} -> [{}]", s.title, s.tags.join(", "));
                    }
                    println!();
                }
            }

            if args.auto_apply {
                println!("Auto-applying high-confidence suggestions...");
                let mut applied_count = 0;
                for s in &insights.relation_suggestions {
                    if s.confidence >= 0.85 {
                        if looma_intelligence::IntelligenceEngine::apply_suggestion(&core, "cli", s)? {
                            applied_count += 1;
                        }
                    }
                }
                for s in &insights.cluster_suggestions {
                    if s.confidence >= 0.85 {
                        if looma_intelligence::IntelligenceEngine::apply_suggestion(&core, "cli", s)? {
                            applied_count += 1;
                        }
                    }
                }
                println!("Applied {} smart suggestions successfully!", applied_count);
            }
        }

        Commands::Mcp => unreachable!(),
    }

    Ok(())
}
