use thiserror::Error;

/// Storage-specific error types
#[derive(Debug, Error)]
pub enum StorageError {
    #[error("Connection error: {0}")]
    Connection(String),

    #[error("Not connected to database")]
    NotConnected,

    #[error("Query execution failed: {0}")]
    QueryFailed(String),

    #[error("Schema error: {0}")]
    Schema(String),

    #[error("Migration error: {0}")]
    Migration(String),

    #[error("Record not found: {0}")]
    NotFound(String),

    #[error("Duplicate record: {0}")]
    Duplicate(String),

    #[error("Invalid data: {0}")]
    InvalidData(String),

    #[error("Transaction error: {0}")]
    Transaction(String),

    #[error("Vector search error: {0}")]
    VectorSearch(String),

    #[error("Graph operation error: {0}")]
    GraphOperation(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Deserialization error: {0}")]
    Deserialization(String),

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("Authentication failed: {0}")]
    Authentication(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Operation not supported: {0}")]
    NotSupported(String),
}

// Note: From<StorageError> for Paper2CodesError is automatically generated
// by the #[from] attribute in error/mod.rs

pub type StorageResult<T> = std::result::Result<T, StorageError>;
