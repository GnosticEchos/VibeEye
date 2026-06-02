use rust_mcp_sdk::macros::{JsonSchema, mcp_tool};
use serde::{Deserialize, Serialize};

// ── Browser tools ──────────────────────────────────────────────────────────

#[mcp_tool(
    name = "browser_navigate",
    description = "Navigate to a URL and load the page"
)]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct NavigateTool {
    /// URL to navigate to
    pub url: String,
    /// Optional wait condition
    #[serde(default)]
    pub wait_until: Option<String>,
}

#[mcp_tool(
    name = "browser_snapshot",
    description = "Return the current page URL, title, and body text"
)]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct SnapshotTool {
    /// URL to capture
    pub url: String,
}

#[mcp_tool(
    name = "browser_extract",
    description = "Extract page content as Markdown, HTML, or plain text"
)]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct ExtractTool {
    /// URL to extract content from
    pub url: String,
    /// Output format: markdown, html, or text
    #[serde(default = "default_format")]
    pub format: String,
}

// ── SurrealDB read-only tools ──────────────────────────────────────────────

#[cfg(feature = "surrealdb")]
#[mcp_tool(
    name = "db_query",
    description = "Full-text search (BM25) over crawled pages in SurrealDB. Returns matching pages with relevance scores. Optional group filter."
)]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct DbQueryTool {
    /// Search query string
    pub query: String,
    /// Group to search (all groups if omitted)
    #[serde(default)]
    pub group: Option<String>,
    /// Maximum results
    #[serde(default = "default_limit")]
    pub limit: u64,
}

#[cfg(feature = "surrealdb")]
#[mcp_tool(
    name = "db_list",
    description = "List all crawl groups stored in SurrealDB."
)]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct DbListTool {}

#[cfg(feature = "surrealdb")]
#[mcp_tool(
    name = "db_status",
    description = "Show statistics (page count, link count, chunk count) for a crawl group."
)]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct DbStatusTool {
    /// Group name
    pub group: String,
}

// ── SurrealDB embedding tools ──────────────────────────────────────────────

#[cfg(feature = "embeddings")]
#[mcp_tool(
    name = "db_vector",
    description = "Vector similarity search over chunk embeddings. Converts the query text to an embedding and finds semantically similar chunks. Optional group filter."
)]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct DbVectorTool {
    /// Search query string
    pub query: String,
    /// Group to search (all groups if omitted)
    #[serde(default)]
    pub group: Option<String>,
    /// Maximum results
    #[serde(default = "default_limit")]
    pub limit: u64,
}

#[cfg(feature = "embeddings")]
#[mcp_tool(
    name = "db_hybrid",
    description = "Hybrid search: BM25 pre-filter followed by vector rerank. Combines keyword and semantic matching for best results. Optional group filter."
)]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct DbHybridTool {
    /// Search query string
    pub query: String,
    /// Group to search (all groups if omitted)
    #[serde(default)]
    pub group: Option<String>,
    /// Maximum final results
    #[serde(default = "default_limit")]
    pub limit: u64,
    /// BM25 candidate pool size
    #[serde(default = "default_bm25_limit")]
    pub bm25_limit: u64,
}

// ── Long-running / destructive tools (CLI-recommended) ─────────────────────

#[cfg(feature = "surrealdb")]
#[mcp_tool(
    name = "db_export",
    description = "Export crawl data for a group to a SurQL file. Recommended: Ask the user to run `vibe-eye db export <path> --group <name>` in their terminal instead. This preserves MCP context."
)]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct DbExportTool {
    /// Group name
    pub group: String,
    /// Output file path
    pub target_path: String,
}

#[cfg(feature = "surrealdb")]
#[mcp_tool(
    name = "db_import",
    description = "Import crawl data from a file or directory into SurrealDB. Recommended: Ask the user to run `vibe-eye db import <path> --group <name>` in their terminal instead. This preserves MCP context."
)]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct DbImportTool {
    /// Group name
    pub group: String,
    /// Source file or directory path
    pub source_path: String,
}

#[cfg(feature = "surrealdb")]
#[mcp_tool(
    name = "crawl",
    description = "Run a BFS web crawl starting from a URL and store results in SurrealDB. Automatically handles JavaScript-rendered pages (SPAs like crates.io, GitHub) by scrolling and settling before extraction. If the page requires authentication or the crawl would exceed ~100 pages, instruct the user to run `vibe-eye crawl <url> --auth` in their terminal instead. This preserves MCP context and avoids blocking the agent session."
)]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct CrawlTool {
    /// Seed URL to start crawling from
    pub url: String,
    /// Maximum crawl depth
    #[serde(default = "default_max_depth")]
    pub max_depth: u32,
    /// Maximum pages to crawl
    #[serde(default = "default_max_pages")]
    pub max_pages: u64,
    /// Target group name (derived from domain if omitted)
    #[serde(default)]
    pub group: Option<String>,
    /// Persist to SurrealDB (default: true)
    #[serde(default = "default_true")]
    pub surrealdb: bool,
    /// Generate embeddings after crawl (requires embeddings feature)
    #[serde(default = "default_false")]
    pub embed: bool,
}

#[cfg(feature = "surrealdb")]
#[mcp_tool(
    name = "db_reset",
    description = "PERMANENTLY DELETE all data for a crawl group. Recommended: Ask the user to run `vibe-eye db reset --group <name>` in their terminal instead. This preserves MCP context and prevents accidental data loss."
)]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct DbResetTool {
    /// Group name
    pub group: String,
}

#[cfg(feature = "surrealdb")]
#[mcp_tool(
    name = "db_reset_all",
    description = "PERMANENTLY DELETE ALL DATA in SurrealDB. Recommended: Ask the user to run `vibe-eye db reset-all` in their terminal instead. This preserves MCP context and prevents catastrophic data loss."
)]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct DbResetAllTool {}

// ── Defaults ───────────────────────────────────────────────────────────────

fn default_format() -> String {
    "markdown".to_string()
}

#[cfg(feature = "surrealdb")]
fn default_limit() -> u64 {
    10
}

#[cfg(feature = "embeddings")]
fn default_bm25_limit() -> u64 {
    50
}

#[cfg(feature = "surrealdb")]
fn default_max_depth() -> u32 {
    3
}

#[cfg(feature = "surrealdb")]
fn default_max_pages() -> u64 {
    100
}

#[cfg(feature = "surrealdb")]
fn default_true() -> bool {
    true
}

#[cfg(feature = "surrealdb")]
fn default_false() -> bool {
    false
}
