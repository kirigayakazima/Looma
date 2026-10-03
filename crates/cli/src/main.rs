use std::path::PathBuf;
use chrono::Utc;
use clap::{Args, Parser, Subcommand};
use directories::ProjectDirs;
use serde_json::json;

use looma_core::models::*;
use looma_core::services::*;
use looma_database::LoomaDb;
use looma_mcp::McpServer;
use looma_scanner::{ScanOptions, Scanner};

#[derive(Parser)]
#[command(name = "looma")]
#[command(author = "Looma Team")]
#[command(version = "0.1.0")]
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

    #[command(about = "Memory notes management commands")]
    Memory {
        #[command(subcommand)]
        action: MemoryCommands,
    },

    #[command(about = "List collection groupings")]
    Collection,

    #[command(about = "View unified chronological timeline")]
    Timeline(TimelineArgs),

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
        let server = McpServer::new(db);
        server.run_stdio()?;
        return Ok(());
    }

    println!("======================================================");
    println!("  Looma Digital Vault CLI (Phase 5 - Local-First)     ");
    println!("  Vault Root: {}", vault_path.display());
    println!("======================================================\n");

    let db = LoomaDb::open(&vault_path)?;

    match cli.command {
        Commands::Status => {
            let info = db.get_vault_info()?;
            let stats = db.get_vault_stats()?;
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
                &db,
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
            let assets = db.query_assets(&filter)?;
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
                let assets = db.query_assets(&filter)?;
                println!("Indexed Assets (showing up to {}):\n", limit);
                for a in assets {
                    let name = a.path.as_deref().unwrap_or(&a.id);
                    println!("  - [{:?}] {}", a.kind, name);
                }
            }
            AssetCommands::Get { id } => {
                if let Some(a) = db.get_asset_by_id(&id)? {
                    println!("{}", serde_json::to_string_pretty(&a)?);
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
                let entities = db.list_entities(&filter)?;
                println!("Entity Concepts ({}):\n", entities.len());
                for e in entities {
                    println!("  * [{}] {} (ID: {})", e.entity_type, e.title, e.id);
                    if let Some(d) = e.description {
                        println!("    {}", d);
                    }
                }
            }
            EntityCommands::Get { id } => {
                if let Some(e) = db.get_entity_by_id(&id)? {
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
                db.create_entity(&entity)?;
                println!("Created entity: {} (ID: {})", title, entity.id);
            }
        },

        Commands::Memory { action } => match action {
            MemoryCommands::List { limit } => {
                let memories = db.list_memories(limit, 0)?;
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
                db.create_memory(&memory)?;
                println!("Created memory note: {} (ID: {})", title, memory.id);
            }
        },

        Commands::Collection => {
            let cols = db.list_collections()?;
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
            let items = db.query_timeline(&filter)?;
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
                let res = db.export_backup(&dest)?;
                println!("Backup successful! Path: {}", res.backup_path);
                println!("Duration: {} ms", res.duration_ms);
            } else if let Some(restore_path) = args.restore {
                println!("Restoring vault from {}...", restore_path.display());
                let res = db.restore_backup(&restore_path)?;
                println!("Restore complete!");
                println!("  Restored Assets:      {}", res.restored_assets);
                println!("  Restored Entities:    {}", res.restored_entities);
                println!("  Restored Memories:    {}", res.restored_memories);
                println!("  Restored Collections: {}", res.restored_collections);
                println!("  Duration:             {} ms", res.duration_ms);
            } else {
                let manifest = db.generate_manifest()?;
                println!("Vault Manifest Snapshot:");
                println!("{}", serde_json::to_string_pretty(&manifest)?);
            }
        }

        Commands::Doctor(args) => {
            println!("Running Vault Doctor inspection...");
            let report = db.doctor_inspect()?;
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
                let cleaned = db.doctor_cleanup_missing()?;
                println!("Successfully cleaned {} missing items and compacted SQLite database!", cleaned);
            }
        }

        Commands::Suggest(args) => {
            println!("Generating Smart Insights & Relationship Suggestions (Phase 6)...");
            let insights = looma_intelligence::IntelligenceEngine::generate_insights(&db)?;

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
                        if looma_intelligence::IntelligenceEngine::apply_suggestion(&db, s)? {
                            applied_count += 1;
                        }
                    }
                }
                for s in &insights.cluster_suggestions {
                    if s.confidence >= 0.85 {
                        if looma_intelligence::IntelligenceEngine::apply_suggestion(&db, s)? {
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
