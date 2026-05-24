//! App-level error types

use thiserror::Error;

/// Error types for vibeeye-app operations
#[derive(Error, Debug)]
pub enum AppError {
    #[error("Browser error: {0}")]
    Browser(String),

    #[error("Navigation failed: {0}")]
    Navigation(String),

    #[error("Content extraction error: {0}")]
    Extraction(String),

    #[error("Tool execution error: {0}")]
    ToolExecution(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error(transparent)]
    Core(#[from] vibeeye_core::VibeError),

    #[error(transparent)]
    Serde(#[from] serde_json::Error),
}

/// Result type alias for app operations
pub type Result<T> = std::result::Result<T, AppError>;
