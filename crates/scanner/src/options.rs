use serde::{Deserialize, Serialize};

/// Options configuring the behavior of a scan operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanOptions {
    /// Whether to scan subdirectories recursively. Default: true.
    pub recursive: bool,
    /// Whether to calculate SHA-256 content hashes. Default: true.
    pub compute_hash: bool,
    /// Whether to ignore hidden files and directories (names starting with '.' or system hidden). Default: true.
    pub ignore_hidden: bool,
    /// Whether to follow symlinks. Default: false (safety against cycles).
    pub follow_symlinks: bool,
    /// Maximum file size to compute hash for (in bytes). Files larger than this will have hash = None.
    /// Default: 500 MB (500 * 1024 * 1024). Set to None to hash all files.
    pub max_hash_file_size: Option<u64>,
}

impl Default for ScanOptions {
    fn default() -> Self {
        Self {
            recursive: true,
            compute_hash: true,
            ignore_hidden: true,
            follow_symlinks: false,
            max_hash_file_size: Some(500 * 1024 * 1024), // 500 MB
        }
    }
}

/// Real-time progress update emitted during scanning.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanProgress {
    pub total_discovered: usize,
    pub new_assets: usize,
    pub modified_assets: usize,
    pub unchanged_assets: usize,
    pub current_file: Option<String>,
}

/// Final summary returned after a directory scan finishes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanSummary {
    pub root_path: String,
    pub duration_ms: u64,
    pub total_scanned: usize,
    pub new_assets: usize,
    pub modified_assets: usize,
    pub unchanged_assets: usize,
    pub missing_assets: usize,
    pub errors: Vec<String>,
}
