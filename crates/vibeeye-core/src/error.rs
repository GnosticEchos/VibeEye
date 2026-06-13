use thiserror::Error;

/// Unified error types for all VibeEye operations
#[derive(Error, Debug)]
pub enum VibeError {
    #[error("Browser engine error: {0}")]
    Browser(String),

    #[error("Navigation failed: {0}")]
    Navigation(String),

    #[error("Content extraction error: {0}")]
    Extraction(String),

    #[error("Tool execution error: {0}")]
    ToolExecution(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("MCP protocol error: {0}")]
    Mcp(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// Result type alias for VibeEye operations
pub type Result<T> = std::result::Result<T, VibeError>;
