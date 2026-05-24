use thiserror::Error;

/// Core error types for VibeEye
#[derive(Error, Debug)]
pub enum VibeError {
    #[error("Browser engine error: {0}")]
    Engine(String),

    #[error("Navigation failed: {0}")]
    Navigation(String),

    #[error("Content extraction error: {0}")]
    Extraction(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("MCP protocol error: {0}")]
    Mcp(String),
}

/// Result type alias for VibeEye operations
pub type Result<T> = std::result::Result<T, VibeError>;
