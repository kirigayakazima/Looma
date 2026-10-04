use std::path::PathBuf;
use std::sync::Arc;
use base64::Engine;
use directories::ProjectDirs;
use tauri::{Emitter, State};

use looma_core::models::*;
use looma_core::LoomaCore;
use looma_database::LoomaDb;
use looma_scanner::{ScanOptions, ScanProgress, ScanSummary, Scanner};

pub struct AppState {
    pub core: LoomaCore,
}

// -------------------------------------------------------------
// Tauri IPC Commands
// -------------------------------------------------------------

#[tauri::command]
fn get_vault_info(state: State<'_, AppState>) -> Result<VaultInfo, String> {
    state.core.get_vault_info().map_err(|e| e.to_string())
}

#[tauri::command]
fn get_vault_stats(state: State<'_, AppState>) -> Result<VaultStats, String> {
    state.core.get_vault_stats().map_err(|e| e.to_string())
}

#[tauri::command]
fn list_assets(filter: Option<AssetFilter>, state: State<'_, AppState>) -> Result<Vec<Asset>, String> {
    state.core
        .query_assets(&filter.unwrap_or_default())
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_asset(id: String, state: State<'_, AppState>) -> Result<Option<Asset>, String> {
    state.core.get_asset(&id).map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_asset(id: String, state: State<'_, AppState>) -> Result<bool, String> {
    state.core.delete_asset("desktop", &id).map_err(|e| e.to_string())
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
    let core = state.core.clone();
    let root = PathBuf::from(path);
    let options = ScanOptions {
        compute_hash: compute_hash.unwrap_or(true),
        ..Default::default()
    };

    tauri::async_runtime::spawn_blocking(move || {
        let progress_cb = move |progress: ScanProgress| {
            let _ = app_handle.emit("scan-progress", &progress);
        };
        Scanner::scan_directory(&core, &root, &options, Some(progress_cb))
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
    state.core
        .list_entities(&filter.unwrap_or_default())
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_entity(id: String, state: State<'_, AppState>) -> Result<Option<Entity>, String> {
    state.core.get_entity(&id).map_err(|e| e.to_string())
}

#[tauri::command]
fn create_entity(entity: Entity, state: State<'_, AppState>) -> Result<(), String> {
    state.core.create_entity("desktop", &entity).map_err(|e| e.to_string())
}

#[tauri::command]
fn update_entity(entity: Entity, state: State<'_, AppState>) -> Result<(), String> {
    state.core.update_entity("desktop", &entity).map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_entity(id: String, state: State<'_, AppState>) -> Result<bool, String> {
    state.core.delete_entity("desktop", &id).map_err(|e| e.to_string())
}

#[tauri::command]
fn create_relation(relation: Relation, state: State<'_, AppState>) -> Result<(), String> {
    state.core.create_relation("desktop", &relation).map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_relation(id: String, state: State<'_, AppState>) -> Result<bool, String> {
    state.core.delete_relation("desktop", &id).map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_relation_between(source_id: String, target_id: String, state: State<'_, AppState>) -> Result<bool, String> {
    state.core.delete_relation_between("desktop", &source_id, &target_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn list_relations_for_item(item_id: String, state: State<'_, AppState>) -> Result<Vec<Relation>, String> {
    state.core.list_relations_for_item(&item_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn list_memories(limit: Option<usize>, offset: Option<usize>, state: State<'_, AppState>) -> Result<Vec<Memory>, String> {
    state.core
        .list_memories(limit.unwrap_or(50), offset.unwrap_or(0))
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn create_memory(memory: Memory, state: State<'_, AppState>) -> Result<(), String> {
    state.core.create_memory("desktop", &memory).map_err(|e| e.to_string())
}

#[tauri::command]
fn update_memory(memory: Memory, state: State<'_, AppState>) -> Result<(), String> {
    state.core.update_memory("desktop", &memory).map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_memory(id: String, state: State<'_, AppState>) -> Result<bool, String> {
    state.core.delete_memory("desktop", &id).map_err(|e| e.to_string())
}

#[tauri::command]
fn list_collections(state: State<'_, AppState>) -> Result<Vec<Collection>, String> {
    state.core.list_collections().map_err(|e| e.to_string())
}

#[tauri::command]
fn get_collection(id: String, state: State<'_, AppState>) -> Result<Option<Collection>, String> {
    state.core.get_collection(&id).map_err(|e| e.to_string())
}

#[tauri::command]
fn create_collection(collection: Collection, state: State<'_, AppState>) -> Result<(), String> {
    state.core.create_collection("desktop", &collection).map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_collection(id: String, state: State<'_, AppState>) -> Result<bool, String> {
    state.core.delete_collection("desktop", &id).map_err(|e| e.to_string())
}

#[tauri::command]
fn list_collection_items(collection_id: String, state: State<'_, AppState>) -> Result<Vec<CollectionItem>, String> {
    state.core.list_collection_items(&collection_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn add_item_to_collection(collection_id: String, item_id: String, item_type: String, state: State<'_, AppState>) -> Result<(), String> {
    state.core.add_item_to_collection("desktop", &collection_id, &item_id, &item_type).map_err(|e| e.to_string())
}

#[tauri::command]
fn remove_item_from_collection(collection_id: String, item_id: String, state: State<'_, AppState>) -> Result<bool, String> {
    state.core.remove_item_from_collection("desktop", &collection_id, &item_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn query_timeline(filter: Option<TimelineFilter>, state: State<'_, AppState>) -> Result<Vec<TimelineItem>, String> {
    state.core
        .query_timeline(&filter.unwrap_or_default())
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn generate_manifest(state: State<'_, AppState>) -> Result<VaultManifest, String> {
    state.core.generate_manifest().map_err(|e| e.to_string())
}

#[tauri::command]
fn export_backup(destination_dir: Option<String>, state: State<'_, AppState>) -> Result<BackupResult, String> {
    let dest_path = if let Some(dir) = destination_dir {
        PathBuf::from(dir)
    } else {
        resolve_default_vault_path().join("backups")
    };
    std::fs::create_dir_all(&dest_path).map_err(|e| e.to_string())?;
    state.core.export_backup("desktop", &dest_path).map_err(|e| e.to_string())
}

#[tauri::command]
fn restore_backup(backup_path: String, state: State<'_, AppState>) -> Result<RestoreResult, String> {
    let p = PathBuf::from(backup_path);
    state.core.restore_backup("desktop", &p).map_err(|e| e.to_string())
}

#[tauri::command]
fn doctor_inspect(state: State<'_, AppState>) -> Result<VaultDoctorReport, String> {
    state.core.doctor_inspect().map_err(|e| e.to_string())
}

#[tauri::command]
fn doctor_cleanup_missing(state: State<'_, AppState>) -> Result<usize, String> {
    state.core.doctor_cleanup_missing("desktop").map_err(|e| e.to_string())
}

#[tauri::command]
fn get_smart_insights(
    state: State<'_, AppState>,
) -> Result<looma_intelligence::SmartInsightsReport, String> {
    looma_intelligence::IntelligenceEngine::generate_insights(&state.core).map_err(|e| e.to_string())
}

#[tauri::command]
fn apply_smart_suggestion(
    state: State<'_, AppState>,
    suggestion: looma_intelligence::Suggestion,
) -> Result<bool, String> {
    looma_intelligence::IntelligenceEngine::apply_suggestion(&state.core, "desktop", &suggestion)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn list_recent_audits(
    limit: Option<usize>,
    state: State<'_, AppState>,
) -> Result<Vec<AuditLog>, String> {
    state.core.list_recent_audits(limit.unwrap_or(50)).map_err(|e| e.to_string())
}

#[tauri::command]
fn list_external_references(
    entity_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<ExternalReference>, String> {
    state.core.list_external_references(entity_id.as_deref()).map_err(|e| e.to_string())
}

#[tauri::command]
fn create_external_reference(
    reference: ExternalReference,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.core.create_external_reference("desktop", &reference).map_err(|e| e.to_string())
}

#[tauri::command]
fn update_external_reference(
    reference: ExternalReference,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.core.update_external_reference("desktop", &reference).map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_external_reference(
    id: String,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    state.core.delete_external_reference("desktop", &id).map_err(|e| e.to_string())
}

#[tauri::command]
fn list_works(
    work_type: Option<String>,
    status: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<Entity>, String> {
    state.core.list_works(work_type.as_deref(), status.as_deref()).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_work_summary(
    id: String,
    state: State<'_, AppState>,
) -> Result<Option<WorkSummary>, String> {
    state.core.get_work_summary(&id).map_err(|e| e.to_string())
}

#[tauri::command]
fn create_work(
    title: String,
    kind: String,
    status: String,
    original_title: Option<String>,
    description: Option<String>,
    state: State<'_, AppState>,
) -> Result<Entity, String> {
    let work_type = WorkType::parse(&kind);
    let record_status = RecordStatus::parse(&status);
    state.core.create_work("desktop", &title, work_type, record_status, original_title, description)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn update_work_status(
    id: String,
    status: String,
    state: State<'_, AppState>,
) -> Result<Entity, String> {
    let record_status = RecordStatus::parse(&status);
    state.core.update_work_status("desktop", &id, record_status)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn update_work_progress(
    id: String,
    position: f64,
    position_type: Option<String>,
    total_positions: Option<f64>,
    unit: Option<String>,
    state: State<'_, AppState>,
) -> Result<Entity, String> {
    let p_type = position_type.map(|s| ProgressPositionType::parse(&s));
    state.core.update_work_progress("desktop", &id, position, p_type, total_positions, unit)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn open_external_url(url: String) -> Result<(), String> {
    tauri_plugin_opener::open_url(&url, None::<&str>)
        .map_err(|e| e.to_string())
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GameCandidate {
    pub id: String,
    pub path: String,
    pub deduced_title: String,
    pub drive: String,
    pub has_executable: bool,
    pub file_count: usize,
    pub matched_work_id: Option<String>,
    pub matched_work_title: Option<String>,
}

#[tauri::command]
fn update_work_cover_asset(
    id: String,
    cover_asset_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<Entity, String> {
    state.core.update_work_cover_asset("desktop", &id, cover_asset_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn detect_game_candidates(
    root_path: String,
    state: State<'_, AppState>,
) -> Result<Vec<GameCandidate>, String> {
    let p = std::path::Path::new(&root_path);
    if !p.exists() {
        return Err(format!("Path does not exist: {}", root_path));
    }

    let existing_games = state.core.list_works(Some("game"), None)
        .unwrap_or_default();

    let mut candidates = Vec::new();
    let read_dir = match std::fs::read_dir(p) {
        Ok(rd) => rd,
        Err(e) => return Err(format!("Failed to read directory: {e}")),
    };

    for entry in read_dir.flatten() {
        let entry_path = entry.path();
        if entry_path.is_dir() {
            let dir_name = entry.file_name().to_string_lossy().to_string();
            if dir_name.starts_with('.') || dir_name == "$RECYCLE.BIN" || dir_name == "System Volume Information" {
                continue;
            }

            let mut has_exe = false;
            let mut file_count = 0usize;
            if let Ok(sub_rd) = std::fs::read_dir(&entry_path) {
                for sub_entry in sub_rd.flatten() {
                    file_count += 1;
                    if let Some(ext) = sub_entry.path().extension() {
                        if ext.to_string_lossy().eq_ignore_ascii_case("exe") {
                            has_exe = true;
                        }
                    }
                }
            }

            let path_str = entry_path.to_string_lossy().to_string();
            let drive = if path_str.len() >= 2 && &path_str[1..2] == ":" {
                path_str[0..2].to_uppercase()
            } else {
                "Local".to_string()
            };

            let deduced_title = dir_name.replace('_', " ").replace('-', " ");
            let mut matched_work_id = None;
            let mut matched_work_title = None;
            let norm_deduced = deduced_title.to_lowercase().replace(' ', "");

            for w in &existing_games {
                let norm_w = w.title.to_lowercase().replace(' ', "");
                if norm_w == norm_deduced || norm_w.contains(&norm_deduced) || norm_deduced.contains(&norm_w) {
                    matched_work_id = Some(w.id.clone());
                    matched_work_title = Some(w.title.clone());
                    break;
                }
            }

            candidates.push(GameCandidate {
                id: format!("cand_{}", uuid::Uuid::new_v4().to_string()[..8].to_string()),
                path: path_str,
                deduced_title,
                drive,
                has_executable: has_exe,
                file_count,
                matched_work_id,
                matched_work_title,
            });
        }
    }

    Ok(candidates)
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
    let core = LoomaCore::new(Arc::new(db));

    if let Err(e) = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState { core })
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
            get_entity,
            create_entity,
            update_entity,
            delete_entity,
            create_relation,
            delete_relation,
            delete_relation_between,
            list_relations_for_item,
            list_memories,
            create_memory,
            update_memory,
            delete_memory,
            list_collections,
            get_collection,
            create_collection,
            delete_collection,
            list_collection_items,
            add_item_to_collection,
            remove_item_from_collection,
            query_timeline,
            generate_manifest,
            export_backup,
            restore_backup,
            doctor_inspect,
            doctor_cleanup_missing,
            get_smart_insights,
            apply_smart_suggestion,
            list_recent_audits,
            list_external_references,
            create_external_reference,
            update_external_reference,
            delete_external_reference,
            list_works,
            get_work_summary,
            create_work,
            update_work_status,
            update_work_progress,
            update_work_cover_asset,
            detect_game_candidates,
            open_external_url
        ])
        .run(tauri::generate_context!())
    {
        let msg = format!("Failed to run Tauri: {e}");
        let _ = std::fs::write("looma_error.log", &msg);
        panic!("{}", msg);
    }
}
