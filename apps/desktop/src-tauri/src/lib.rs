use std::path::PathBuf;
use base64::Engine;
use directories::ProjectDirs;
use tauri::{Emitter, State};

use looma_core::models::*;
use looma_core::services::*;
use looma_database::LoomaDb;
use looma_scanner::{ScanOptions, ScanProgress, ScanSummary, Scanner};

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
fn list_assets(filter: Option<AssetFilter>, state: State<'_, AppState>) -> Result<Vec<Asset>, String> {
    state.db
        .query_assets(&filter.unwrap_or_default())
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_asset(id: String, state: State<'_, AppState>) -> Result<Option<Asset>, String> {
    state.db.get_asset_by_id(&id).map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_asset(id: String, state: State<'_, AppState>) -> Result<bool, String> {
    state.db.delete_asset(&id).map_err(|e| e.to_string())
}

#[tauri::command]
fn pick_folder() -> Result<Option<String>, String> {
    let folder = rfd::FileDialog::new().pick_folder();
    Ok(folder.map(|p| p.to_string_lossy().to_string()))
}

#[tauri::command]
async fn scan_directory(
    path: String,
    compute_hash: Option<bool>,
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<ScanSummary, String> {
    let db = state.db.clone();
    let root = PathBuf::from(path);
    let options = ScanOptions {
        compute_hash: compute_hash.unwrap_or(true),
        ..Default::default()
    };

    tauri::async_runtime::spawn_blocking(move || {
        let progress_cb = move |progress: ScanProgress| {
            let _ = app_handle.emit("scan-progress", &progress);
        };
        Scanner::scan_directory(&db, &root, &options, Some(progress_cb))
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())
}

#[tauri::command]
fn open_in_file_manager(path: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let mut cmd = std::process::Command::new("explorer");
        // Windows explorer /select,"path" opens explorer with the file selected
        cmd.arg(format!("/select,{}", path));
        cmd.spawn().map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "macos")]
    {
        let mut cmd = std::process::Command::new("open");
        cmd.arg("-R").arg(&path);
        cmd.spawn().map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "linux")]
    {
        let p = std::path::Path::new(&path);
        let parent = p.parent().unwrap_or(p);
        let mut cmd = std::process::Command::new("xdg-open");
        cmd.arg(parent);
        cmd.spawn().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn read_asset_preview(path: String) -> Result<String, String> {
    let p = std::path::Path::new(&path);
    if !p.exists() {
        return Err("File not found".to_string());
    }
    let meta = std::fs::metadata(p).map_err(|e| e.to_string())?;
    if meta.len() > 30 * 1024 * 1024 {
        return Err("File too large for direct inline preview".to_string());
    }
    let bytes = std::fs::read(p).map_err(|e| e.to_string())?;
    let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
    let mime = match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "bmp" => "image/bmp",
        "ico" => "image/x-icon",
        _ => "application/octet-stream",
    };
    let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
    Ok(format!("data:{mime};base64,{encoded}"))
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
            get_asset,
            delete_asset,
            pick_folder,
            scan_directory,
            open_in_file_manager,
            read_asset_preview,
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
