//! App-level error types
//!
//! Re-exports the unified `VibeError` from `vibeeye_core` to ensure
//! a single source of truth for all error handling across the workspace.

pub use vibeeye_core::{Result, VibeError as Error};
