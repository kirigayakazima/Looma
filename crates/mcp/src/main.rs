use std::path::PathBuf;
use directories::ProjectDirs;
use looma_database::LoomaDb;
use looma_mcp::McpServer;

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
    let vault_path = resolve_default_vault_path();
    let db = LoomaDb::open(&vault_path)?;
    let server = McpServer::new(db);
    server.run_stdio()?;
    Ok(())
}
