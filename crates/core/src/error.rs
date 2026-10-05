use thiserror::Error;

#[derive(Error, Debug)]
pub enum LoomaError {
    #[error("Vault error: {0}")]
    Vault(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Database error: {0}")]
    Database(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Conflict error: {0}")]
    Conflict(String),

    #[error("Integrity error: {0}")]
    Integrity(String),
}

pub type LoomaResult<T> = Result<T, LoomaError>;
