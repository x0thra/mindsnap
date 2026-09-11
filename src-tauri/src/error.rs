use thiserror::Error;

#[derive(Error, Debug)]
pub enum MindsnapError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Platform error: {0}")]
    Platform(String),

    #[error("Notification error: {0}")]
    Notification(String),

    #[error("Lock concurrency error: {0}")]
    Lock(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Unknown error: {0}")]
    Unknown(String),
}

pub type Result<T> = std::result::Result<T, MindsnapError>;
