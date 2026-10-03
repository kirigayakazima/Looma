use std::path::PathBuf;
use std::sync::Arc;
use directories::ProjectDirs;
use looma_core::LoomaCore;
use looma_database::LoomaDb;
use looma_mcp::{McpPermissionLevel, McpServer};

fn resolve_default_vault_path() -> PathBuf {
    if let Some(proj_dirs) = ProjectDirs::from("com", "looma", "Looma") {
        let data_dir = proj_dirs.data_dir();
        std::fs::create_dir_all(data_dir).ok();
        data_dir.to_path_buf()
    } else {
        PathBuf::from("./looma_vault")
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let mut vault_path = resolve_default_vault_path();
    let mut permission = McpPermissionLevel::ContentWrite;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--allow-destructive" => {
                permission = McpPermissionLevel::FullDestructive;
            }
            "--read-only" => {
                permission = McpPermissionLevel::ReadOnly;
            }
            "--vault" => {
                if i + 1 < args.len() {
                    vault_path = PathBuf::from(&args[i + 1]);
                    i += 1;
                }
            }
            _ => {}
        }
        i += 1;
    }

    let db = LoomaDb::open(&vault_path)?;
    let core = LoomaCore::new(Arc::new(db));
    let server = McpServer::with_permission(core, permission);
    server.run_stdio()?;
    Ok(())
}
