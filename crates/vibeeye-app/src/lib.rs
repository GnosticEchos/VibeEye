//! VibeEye App Library
//!
//! Shared library containing browser logic, content extraction,
//! and the SonarDiscovery trait for CLI/MCP parity.

pub mod browser;
pub mod config;
pub mod crawl;
pub mod discovery;
pub mod error;
pub mod extraction;
pub mod tool_registry;
pub mod tools;

pub use discovery::{CapabilityProvider, SonarDiscovery, Tool};
pub use error::{AppError, Result};
pub use tool_registry::ToolRegistry;
pub use tools::{BrowseInput, BrowseOutput, BrowseTool};
pub use tools::{ExtractInput, ExtractOutput, ExtractTool};
pub use tools::{SnapshotInput, SnapshotOutput, SnapshotTool};

// Re-export core types for convenience
pub use vibeeye_core::{BrowserContext, ContentFormat, NavigationState, RenderedBuffer, Viewport};
