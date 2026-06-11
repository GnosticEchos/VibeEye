//! Hierarchical crawl configuration (global → domain → subdomain)
//!
//! Loads from `~/.config/vibe-eye/crawl.toml` or an explicit `--config` path.
//! Merge order: CLI flags → subdomain → domain → global.

pub mod crawl;
pub mod embeddings;

pub use crawl::{CliConfig, CrawlConfig, CrawlProfile, HelpTreeConfig, TextThemeConfig};
pub use embeddings::{EmbeddingConfig, interpolate_env_vars};

/// Resolve the SurrealDB connection URL.
///
/// Priority:
/// 1. `VIBEYE_DB_URL` environment variable
/// 2. `db_url` field in `~/.config/vibe-eye/crawl.toml` (global profile)
/// 3. `surrealdb_path` field in config (backwards compat)
/// 4. Default embedded SurrealKV at `~/.local/share/vibe-eye/db`
pub fn resolve_db_url() -> String {
    if let Ok(url) = std::env::var("VIBEYE_DB_URL") {
        return url;
    }

    if let Ok(config) = CrawlConfig::load(None) {
        if let Some(url) = config.global.db_url {
            return url;
        }
        if let Some(path) = config.global.surrealdb_path {
            return format!("surrealkv://{path}");
        }
    }

    let db_path = dirs::data_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("vibe-eye")
        .join("db");
    format!("surrealkv://{}", db_path.display())
}
