use std::path::PathBuf;
use directories::ProjectDirs;
use tauri::State;

use looma_core::models::*;
use looma_core::services::*;
use looma_database::LoomaDb;

pub struct AppState {
    pub db: LoomaDb,
}

// -------------------------------------------------------------
// Tauri IPC Commands
// -------------------------------------------------------------

#[tauri::command]
fn get_vault_info(state: State<'_, AppState>) -> Result<VaultInfo, String> {
    state.db.get_vault_info().map_err(|e| e.to_string())
}

#[tauri::command]
fn get_vault_stats(state: State<'_, AppState>) -> Result<VaultStats, String> {
    state.db.get_vault_stats().map_err(|e| e.to_string())
}

#[tauri::command]
fn list_assets(limit: Option<usize>, offset: Option<usize>, state: State<'_, AppState>) -> Result<Vec<Asset>, String> {
    state.db
        .list_assets(limit.unwrap_or(50), offset.unwrap_or(0))
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn list_entities(filter: Option<EntityFilter>, state: State<'_, AppState>) -> Result<Vec<Entity>, String> {
    state.db
        .list_entities(&filter.unwrap_or_default())
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn create_entity(entity: Entity, state: State<'_, AppState>) -> Result<(), String> {
    state.db.create_entity(&entity).map_err(|e| e.to_string())
}

#[tauri::command]
fn list_memories(limit: Option<usize>, offset: Option<usize>, state: State<'_, AppState>) -> Result<Vec<Memory>, String> {
    state.db
        .list_memories(limit.unwrap_or(50), offset.unwrap_or(0))
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn create_memory(memory: Memory, state: State<'_, AppState>) -> Result<(), String> {
    state.db.create_memory(&memory).map_err(|e| e.to_string())
}

#[tauri::command]
fn list_collections(state: State<'_, AppState>) -> Result<Vec<Collection>, String> {
    state.db.list_collections().map_err(|e| e.to_string())
}

fn resolve_default_vault_path() -> PathBuf {
    if let Some(proj_dirs) = ProjectDirs::from("com", "looma", "Looma") {
        let data_dir = proj_dirs.data_dir();
        std::fs::create_dir_all(data_dir).ok();
        data_dir.to_path_buf()
    } else {
        PathBuf::from("./looma_vault")
    }
}

pub fn run() {
    tracing_subscriber::fmt::init();
    let vault_path = resolve_default_vault_path();
    let db = match LoomaDb::open(&vault_path) {
        Ok(db) => db,
        Err(e) => {
            let msg = format!("Failed to open LoomaDb at {:?}: {e}", vault_path);
            let _ = std::fs::write("looma_error.log", &msg);
            panic!("{}", msg);
        }
    };

    if let Err(e) = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState { db })
        .invoke_handler(tauri::generate_handler![
            get_vault_info,
            get_vault_stats,
            list_assets,
            list_entities,
            create_entity,
            list_memories,
            create_memory,
            list_collections
        ])
        .run(tauri::generate_context!())
    {
        let msg = format!("Failed to run Tauri: {e}");
        let _ = std::fs::write("looma_error.log", &msg);
        panic!("{}", msg);
    }
}
