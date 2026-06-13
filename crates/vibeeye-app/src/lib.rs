//! VibeEye App Library
//!
//! Shared library containing browser logic, content extraction,
//! and the TypedTool/Tool traits for CLI/MCP parity.

pub mod batch;
pub mod browser;
#[cfg(feature = "embeddings")]
pub mod chunk;
pub mod config;
pub mod crawl;
#[cfg(feature = "surrealdb")]
pub mod db;
pub mod discovery;
#[cfg(feature = "embeddings")]
pub mod embed;
pub mod error;
pub mod extraction;
#[cfg(feature = "embeddings")]
pub mod progress;
pub mod tool_registry;
pub mod tools;

pub use discovery::{Tool, ToolAdapter, TypedTool};
pub use error::{Error, Result};
pub use tool_registry::ToolRegistry;
pub use tools::{BrowseInput, BrowseOutput, BrowseTool};
pub use tools::{ExtractInput, ExtractOutput, ExtractTool};
pub use tools::{SnapshotInput, SnapshotOutput, SnapshotTool};

// Re-export core types for convenience
pub use vibeeye_core::{BrowserContext, ContentFormat, NavigationState, RenderedBuffer, Viewport};
