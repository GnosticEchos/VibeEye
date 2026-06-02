use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use surrealdb::types::RecordId;

/// A crawled page stored in SurrealDB.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageRecord {
    /// SurrealDB record ID (e.g. page:wvjq... or page:['group','url']).
    pub id: Option<RecordId>,
    /// Logical group name (e.g. "surrealdb_com").
    pub group: String,
    /// Canonical URL of the page.
    pub url: String,
    /// Page title from `<title>` or first H1.
    pub title: String,
    /// Distilled markdown content.
    pub content: String,
    /// BFS depth from start URL.
    pub depth: i32,
    /// Content format (e.g. "markdown", "html").
    pub format: String,
    /// When the page was crawled.
    pub crawled_at: DateTime<Utc>,
    /// Structured metadata (JSON-LD, Open Graph, IndexedDB dump).
    pub meta: Option<serde_json::Value>,
}

/// A discovered link (graph edge) between two pages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkRecord {
    pub group: String,
    /// Source page record ID.
    pub from_page: RecordId,
    /// Target page record ID.
    pub to_page: RecordId,
    pub anchor_text: Option<String>,
    pub discovered_at: DateTime<Utc>,
}

/// Result of a BM25 text query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    pub url: String,
    pub title: String,
    pub snippet: String,
    pub score: Option<f64>,
}

/// Minimal stats about a crawl group.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DbStats {
    pub page_count: u64,
    pub link_count: u64,
    pub chunk_count: u64,
}

/// A chunk of page content with its embedding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkRecord {
    pub group: String,
    /// Record link to the parent page.
    pub page: RecordId,
    pub chunk_index: i32,
    pub chunk_text: String,
    pub heading_path: Vec<String>,
    pub embedding: Vec<f32>,
    pub model: String,
    pub dimensions: i32,
    pub created_at: DateTime<Utc>,
}

/// Result of a vector KNN search.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorResult {
    pub page_url: String,
    pub page_title: Option<String>,
    pub chunk_text: String,
    pub heading_path: Vec<String>,
    pub score: f64,
}

/// Result of a hybrid BM25 + vector search.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridResult {
    pub page_url: String,
    pub page_title: Option<String>,
    pub chunk_text: String,
    pub heading_path: Vec<String>,
    pub chunk_index: i32,
    pub bm25_score: Option<f64>,
    pub vector_score: Option<f64>,
    #[serde(default)]
    pub expanded_text: String,
}
