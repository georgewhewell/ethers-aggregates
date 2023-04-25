//
use thiserror::*;

#[derive(Debug, Error)]
pub enum AggregateError {
    #[error("Error: {0}")]
    Error(String),

    #[error("IO Error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Serde Error: {0}")]
    SerdeError(#[from] serde_json::Error),
}
