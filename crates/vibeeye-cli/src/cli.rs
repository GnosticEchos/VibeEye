//! CLI argument parsing for vibe-eye

use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

/// VibeEye - Headless browser for agentic content extraction
#[derive(Parser, Debug)]
#[command(name = "vibe-eye")]
#[command(about = "VibeEye - Headless browser for agentic content extraction")]
#[command(version)]
pub struct Cli {
    #[command(flatten)]
    pub help_tree: crate::help_tree::HelpTreeArgs,

    /// Enable verbose debug logging
    #[arg(short, long)]
    pub verbose: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Navigate to a URL
    Navigate {
        /// URL to navigate to
        url: String,
    },

    /// Capture a page snapshot (URL, title, body, HTML)
    Snapshot {
        /// URL to capture
        url: String,
    },

    /// Extract page content as Markdown, HTML, or text
    Extract {
        /// URL to extract content from
        url: String,

        /// Output format: markdown, html, or text
        #[arg(short, long, default_value = "markdown")]
        format: String,
    },

    /// Fetch a list of URLs without link discovery or BFS crawling
    Batch {
        /// File containing URLs to fetch (one per line, use `-` for stdin)
        urls_file: PathBuf,

        /// Output format: markdown, html, or text
        #[arg(short, long)]
        format: Option<String>,

        /// Directory to write per-page files (JSON Lines to stdout if omitted)
        #[arg(short, long, value_name = "DIR")]
        output: Option<PathBuf>,

        /// Maximum concurrent fetches
        #[arg(long)]
        concurrency: Option<usize>,

        /// Per-page timeout in seconds
        #[arg(long)]
        timeout: Option<u64>,

        /// Settle time for SPA content in milliseconds
        #[arg(long)]
        settle_ms: Option<u64>,

        /// Persist results to SurrealDB
        #[cfg(feature = "surrealdb")]
        #[arg(long)]
        surrealdb: bool,

        /// Target group name (required when using --surrealdb)
        #[cfg(feature = "surrealdb")]
        #[arg(long)]
        group: Option<String>,

        /// Generate embeddings after fetch (requires --surrealdb)
        #[cfg(feature = "embeddings")]
        #[arg(long)]
        embed: bool,
    },

    /// BFS crawl a website starting from a seed URL
    Crawl {
        /// Seed URL to start crawling from
        url: String,

        /// Path to a custom TOML config file
        #[arg(long, value_name = "FILE")]
        config: Option<PathBuf>,

        /// Maximum crawl depth
        #[arg(long)]
        max_depth: Option<u32>,

        /// Maximum pages to crawl (0 = unlimited)
        #[arg(long)]
        max_pages: Option<usize>,

        /// Output format: markdown, html, or text
        #[arg(short, long)]
        format: Option<String>,

        /// Directory to write per-page files (JSON Lines to stdout if omitted)
        #[arg(short, long, value_name = "DIR")]
        output: Option<PathBuf>,

        /// File containing seed URLs (one per line, use `-` for stdin)
        #[arg(long, value_name = "FILE")]
        urls_file: Option<PathBuf>,

        /// File to append discovered URLs to (one per line)
        #[arg(long, value_name = "FILE")]
        output_urls: Option<PathBuf>,

        /// Respect robots.txt
        #[arg(long)]
        respect_robots: Option<bool>,

        /// Requests per second per host
        #[arg(long)]
        requests_per_second: Option<f64>,

        /// Maximum concurrent fetches
        #[arg(long)]
        concurrency: Option<usize>,

        /// Restrict to same origin
        #[arg(long)]
        same_origin: Option<bool>,

        /// Per-page timeout in seconds
        #[arg(long)]
        timeout: Option<u64>,

        /// Pre-seed from sitemap.xml
        #[arg(long)]
        sitemap: Option<bool>,

        /// Persist crawl results to SurrealDB
        #[cfg(feature = "surrealdb")]
        #[arg(long)]
        surrealdb: bool,

        /// Generate embeddings after crawl (requires --surrealdb)
        #[cfg(feature = "embeddings")]
        #[arg(long)]
        embed: bool,

        /// Enable DevTools server on a random port for debugging
        #[arg(long)]
        devtools: bool,
    },

    /// Import crawl data into SurrealDB
    #[cfg(feature = "embeddings")]
    Import {
        /// Source file or directory
        source: PathBuf,

        /// Target group name
        #[arg(short, long)]
        group: String,
    },

    /// Export crawl data from SurrealDB
    #[cfg(feature = "embeddings")]
    Export {
        /// Output file path
        target: PathBuf,

        /// Group to export (all groups if omitted)
        #[arg(short, long)]
        group: Option<String>,
    },

    /// SurrealDB database management
    #[cfg(feature = "surrealdb")]
    Db {
        #[command(subcommand)]
        command: DbCommands,
    },
}

/// Query result output format.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum OutputFormat {
    Json,
    Table,
    Markdown,
}

#[derive(Subcommand, Debug)]
#[cfg(feature = "surrealdb")]
pub enum DbCommands {
    /// List all crawl groups
    List,

    /// Show stats for a group
    Status {
        /// Group name
        group: String,
    },

    /// Full-text search (BM25) over a group or all groups
    Query {
        /// Search query string
        query: String,

        /// Group to search (all groups if omitted)
        #[arg(short, long)]
        group: Option<String>,

        /// Maximum results
        #[arg(short, long, default_value = "10")]
        limit: usize,

        /// Output format
        #[arg(short, long, default_value = "json")]
        format: OutputFormat,
    },

    /// Vector similarity search over chunk embeddings
    #[cfg(feature = "embeddings")]
    Vector {
        /// Search query string
        query: String,

        /// Group to search (all groups if omitted)
        #[arg(short, long)]
        group: Option<String>,

        /// Maximum results
        #[arg(short, long, default_value = "10")]
        limit: usize,

        /// Output format
        #[arg(short, long, default_value = "json")]
        format: OutputFormat,
    },

    /// Hybrid search: BM25 pre-filter + vector rerank
    #[cfg(feature = "embeddings")]
    Hybrid {
        /// Search query string
        query: String,

        /// Group to search (all groups if omitted)
        #[arg(short, long)]
        group: Option<String>,

        /// Maximum final results
        #[arg(short, long, default_value = "10")]
        limit: usize,

        /// BM25 candidate pool size
        #[arg(long, default_value = "50")]
        bm25_limit: usize,

        /// Output format
        #[arg(short, long, default_value = "json")]
        format: OutputFormat,
    },

    /// Remove all data for a group
    Reset {
        /// Group name
        group: String,
    },

    /// Remove all data (all groups)
    ResetAll,
}
