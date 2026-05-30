//! Hierarchical crawl configuration (global → domain → subdomain)
//!
//! Loads from `~/.config/vibe-eye/crawl.toml` or an explicit `--config` path.
//! Merge order: CLI flags → subdomain → domain → global.

pub mod crawl;
pub mod embeddings;

pub use crawl::{CrawlConfig, CrawlProfile};
pub use embeddings::{EmbeddingConfig, interpolate_env_vars};
