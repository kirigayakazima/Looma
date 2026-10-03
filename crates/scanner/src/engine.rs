use std::path::Path;
use std::time::Instant;
use chrono::{DateTime, Utc};
use serde_json::json;
use uuid::Uuid;
use walkdir::WalkDir;

use looma_core::error::{LoomaError, LoomaResult};
use looma_core::models::{Asset, AssetSource, AssetStatus};
use looma_core::services::AssetService;

use crate::classifier::classify_file;
use crate::hasher::compute_sha256;
use crate::options::{ScanOptions, ScanProgress, ScanSummary};

/// Check if directory name should be ignored during vault indexing.
fn is_ignored_directory(name: &str) -> bool {
    matches!(
        name,
        ".git" | ".looma" | "node_modules" | "target" | ".svn" | ".hg" | ".DS_Store" | "$RECYCLE.BIN" | "System Volume Information" | ".vscode" | ".idea"
    )
}

/// Normalizes path string for consistent database storage across platforms.
pub fn normalize_path(path: &Path) -> String {
    let s = path.to_string_lossy().to_string();
    // Strip Windows extended-length prefix \\?\ if present
    if let Some(stripped) = s.strip_prefix(r"\\?\") {
        stripped.to_string()
    } else {
        s
    }
}

/// The main filesystem scanner for Looma.
pub struct Scanner;

impl Scanner {
    /// Scans a directory incrementally, recording assets into the database in Reference Mode.
    pub fn scan_directory<S, F>(
        service: &S,
        root: &Path,
        options: &ScanOptions,
        mut progress_cb: Option<F>,
    ) -> LoomaResult<ScanSummary>
    where
        S: AssetService,
        F: FnMut(ScanProgress),
    {
        if !root.exists() {
            return Err(LoomaError::Validation(format!(
                "Scan path does not exist: {:?}",
                root
            )));
        }

        let start_time = Instant::now();
        let root_str = normalize_path(root);

        let mut total_scanned = 0usize;
        let mut new_assets = 0usize;
        let mut modified_assets = 0usize;
        let mut unchanged_assets = 0usize;
        let mut errors = Vec::new();

        let mut walker = WalkDir::new(root).follow_links(options.follow_symlinks);
        if !options.recursive {
            walker = walker.max_depth(1);
        }

        let mut it = walker.into_iter();
        while let Some(entry_res) = it.next() {
            let entry = match entry_res {
                Ok(e) => e,
                Err(err) => {
                    errors.push(format!("WalkDir error: {err}"));
                    continue;
                }
            };

            let file_name = entry.file_name().to_string_lossy();

            // Skip hidden or system subdirectories
            if entry.file_type().is_dir() {
                if entry.depth() > 0 && (options.ignore_hidden && file_name.starts_with('.') || is_ignored_directory(&file_name)) {
                    it.skip_current_dir();
                }
                continue;
            }

            // Skip hidden files if requested
            if options.ignore_hidden && file_name.starts_with('.') {
                continue;
            }

            // Only process regular files
            if !entry.file_type().is_file() {
                continue;
            }

            let file_path = entry.path();
            let norm_path = normalize_path(file_path);

            let metadata = match entry.metadata() {
                Ok(m) => m,
                Err(e) => {
                    errors.push(format!("Failed to read metadata for {:?}: {e}", norm_path));
                    continue;
                }
            };

            let file_size = metadata.len();
            let mtime: DateTime<Utc> = metadata
                .modified()
                .ok()
                .map(|t| t.into())
                .unwrap_or_else(Utc::now);
            let ctime: DateTime<Utc> = metadata
                .created()
                .ok()
                .map(|t| t.into())
                .unwrap_or(mtime);

            total_scanned += 1;

            // Check if existing record exists in database
            match service.get_asset_by_path(&norm_path)? {
                Some(mut existing) => {
                    // Check if unchanged (same size and mtime within 1 second)
                    let same_size = existing.size == Some(file_size);
                    let same_mtime = (mtime.timestamp() - existing.modified_at.timestamp()).abs() <= 1;

                    if same_size && same_mtime && existing.status == AssetStatus::Active {
                        unchanged_assets += 1;
                    } else {
                        // File modified or restored from missing
                        let (kind, mime) = classify_file(file_path);
                        let hash = if options.compute_hash {
                            if let Some(max_sz) = options.max_hash_file_size {
                                if file_size <= max_sz {
                                    compute_sha256(file_path).ok()
                                } else {
                                    None
                                }
                            } else {
                                compute_sha256(file_path).ok()
                            }
                        } else {
                            existing.hash
                        };

                        existing.kind = kind;
                        existing.size = Some(file_size);
                        existing.hash = hash;
                        existing.mime_type = mime;
                        existing.status = AssetStatus::Active;
                        existing.modified_at = mtime;
                        existing.indexed_at = Utc::now();

                        service.upsert_asset(&existing)?;
                        modified_assets += 1;
                    }
                }
                None => {
                    // New asset discovery
                    let (kind, mime) = classify_file(file_path);
                    let hash = if options.compute_hash {
                        if let Some(max_sz) = options.max_hash_file_size {
                            if file_size <= max_sz {
                                compute_sha256(file_path).ok()
                            } else {
                                None
                            }
                        } else {
                            compute_sha256(file_path).ok()
                        }
                    } else {
                        None
                    };

                    let ext = file_path
                        .extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or("")
                        .to_lowercase();

                    let parent_str = file_path
                        .parent()
                        .map(normalize_path)
                        .unwrap_or_default();

                    let meta_val = json!({
                        "file_name": file_name.to_string(),
                        "extension": ext,
                        "directory": parent_str,
                    });

                    let new_asset = Asset {
                        id: Uuid::new_v4().to_string(),
                        kind,
                        source: AssetSource::Local,
                        path: Some(norm_path.clone()),
                        size: Some(file_size),
                        hash,
                        mime_type: mime,
                        metadata: meta_val,
                        status: AssetStatus::Active,
                        created_at: ctime,
                        modified_at: mtime,
                        indexed_at: Utc::now(),
                    };

                    service.upsert_asset(&new_asset)?;
                    new_assets += 1;
                }
            }

            // Periodic progress notification
            if total_scanned % 20 == 0 || total_scanned == 1 {
                if let Some(ref mut cb) = progress_cb {
                    cb(ScanProgress {
                        total_discovered: total_scanned,
                        new_assets,
                        modified_assets,
                        unchanged_assets,
                        current_file: Some(norm_path),
                    });
                }
            }
        }

        // Check for missing assets that previously belonged to this folder
        let mut missing_assets = 0usize;
        if let Ok(existing_in_dir) = service.list_assets_by_prefix(&root_str) {
            for mut asset in existing_in_dir {
                if let Some(ref p) = asset.path {
                    if !Path::new(p).exists() && asset.status == AssetStatus::Active {
                        asset.status = AssetStatus::Missing;
                        asset.indexed_at = Utc::now();
                        if let Err(e) = service.upsert_asset(&asset) {
                            errors.push(format!("Failed to mark asset as missing {}: {e}", p));
                        } else {
                            missing_assets += 1;
                        }
                    }
                }
            }
        }

        let duration_ms = start_time.elapsed().as_millis() as u64;

        // Final progress report
        if let Some(ref mut cb) = progress_cb {
            cb(ScanProgress {
                total_discovered: total_scanned,
                new_assets,
                modified_assets,
                unchanged_assets,
                current_file: None,
            });
        }

        Ok(ScanSummary {
            root_path: root_str,
            duration_ms,
            total_scanned,
            new_assets,
            modified_assets,
            unchanged_assets,
            missing_assets,
            errors,
        })
    }
}
