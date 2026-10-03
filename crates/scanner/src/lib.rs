pub mod classifier;
pub mod engine;
pub mod hasher;
pub mod options;

pub use classifier::classify_file;
pub use engine::{normalize_path, Scanner};
pub use hasher::compute_sha256;
pub use options::{ScanOptions, ScanProgress, ScanSummary};
