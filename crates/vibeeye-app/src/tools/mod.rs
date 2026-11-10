//! Tool definitions for VibeEye
//!
//! Concrete tool implementations for browser operations.

#[cfg(feature = "servo")]
pub mod browse;
#[cfg(feature = "servo")]
pub mod common;
#[cfg(feature = "servo")]
pub mod extract;
#[cfg(feature = "servo")]
pub mod snapshot;

#[cfg(all(test, feature = "servo"))]
mod tests;

#[cfg(feature = "servo")]
pub use browse::{BrowseInput, BrowseOutput, BrowseTool};
#[cfg(feature = "servo")]
pub use extract::{ExtractInput, ExtractOutput, ExtractTool};
#[cfg(feature = "servo")]
pub use snapshot::{SnapshotInput, SnapshotOutput, SnapshotTool};
