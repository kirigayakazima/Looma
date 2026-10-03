use std::fs::File;
use std::io::{self, Read};
use std::path::Path;
use sha2::{Digest, Sha256};

const BUFFER_SIZE: usize = 64 * 1024; // 64 KB streaming buffer

/// Calculates the SHA-256 hex digest of a file in streaming chunks.
pub fn compute_sha256(path: &Path) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; BUFFER_SIZE];

    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    let result = hasher.finalize();
    Ok(hex::encode(result))
}
