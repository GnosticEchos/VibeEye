# Spec: SurrealDB Integration — Phase 1
**Status**: RATIFIED  
**Mission**: 003-surrealdb-integration  
**Date**: 2026-05-27  
**Brainstorm**: [brainstorm.md](./brainstorm.md)

---

## 1. Objective

Add an optional SurrealDB backend to VibeEye for persistent, queryable crawl storage. Phase 1 focuses exclusively on **full-text search (BM25)** and **graph link topology**. Embeddings, chunking, and vector search are deferred to Phase 2.

Users opt-in by building with `--features surrealdb` and using `-o embed` on crawl.

---

## 2. Functional Requirements

### 2.1 Database Storage
- **FR-01**: Embedded SurrealDB (SurrealKV) at `~/.local/share/vibeeye/db.surrealkv`
- **FR-02**: Crawl results stored as `page` records with URL, title, content, depth, format, timestamp
- **FR-03**: Discovered links stored as graph edges (`discovered` relation) with optional anchor text
- **FR-04**: Duplicate URL detection via unique index; re-crawling same URL updates the existing record

### 2.2 Full-Text Search
- **FR-05**: BM25 index on `page.content` using `doc_analyzer` (blank, class, camel tokenizers + lowercase, snowball filters)
- **FR-06**: `vibe-eye query "<text>"` performs BM25 search and returns ranked results
- **FR-07**: Query results include: URL, title, snippet, BM25 score

### 2.3 CLI Interface
- **FR-08**: `vibe-eye crawl <URL> -o embed` stores crawl in SurrealDB
- **FR-09**: `vibe-eye query "<text>" [--limit N]` searches stored pages
- **FR-10**: `vibe-eye db reset` wipes all crawled data (confirmation required)
- **FR-11**: File output (`-o ./dir`) continues to work unchanged when `surrealdb` feature is enabled

### 2.4 Group Isolation
- **FR-12**: All crawls stored in a single SurrealDB **database** (`vibeeye/crawl`) with a `group` field for logical isolation
- **FR-13**: Default group name derived from start URL domain (`surrealdb.com` → `surrealdb_com`), sanitized to valid SurrealDB identifier
- **FR-14**: `-g, --group <NAME>` overrides default group on both `crawl` and `query`
- **FR-15**: Query without `--group` searches **all groups** (cross-domain search)
- **FR-16**: `vibe-eye db list` shows all existing groups (distinct `group` values from `page` table)

### 2.5 Configuration
- **FR-17**: TOML config section `[surrealdb]` with `endpoint`, `namespace` fields (database is runtime-derived)
- **FR-18**: Config file at `~/.config/vibe-eye/crawl.toml` (existing path)
- **FR-19**: Default endpoint: `surrealkv://~/.local/share/vibeeye/db`

---

## 3. Non-Functional Requirements

- **NFR-01**: Binary size increase ≤ 30MB with `surrealdb` feature
- **NFR-02**: RAM: minimal overhead (BM25 index is lightweight)
- **NFR-03**: No external services or network calls required
- **NFR-04**: Feature-gated: `cargo build` without `surrealdb` produces identical binary to pre-integration
- **NFR-05**: All code passes `cargo clippy --all-targets --all-features -- -D warnings`
- **NFR-06**: CRAP threshold ≤ 30 for new functions
- **NFR-07**: Test coverage for DB operations using `kv-mem` backend

---

## 4. Constraints

- **C-01**: No embedding code in Phase 1
- **C-02**: No migration system; schema bootstraps idempotently via `DEFINE IF NOT EXISTS`
- **C-03**: No chunking; entire page stored as single record
- **C-04**: Feature flag `surrealdb` gates all new code paths

---

## 5. SurrealDB Schema

```surrealql
-- Table: page (crawled documents)
DEFINE TABLE page TYPE NORMAL SCHEMAFULL PERMISSIONS NONE;

DEFINE FIELD group ON page TYPE string ASSERT $value != NONE PERMISSIONS FULL;
DEFINE FIELD url ON page TYPE string ASSERT $value != NONE PERMISSIONS FULL;
DEFINE FIELD title ON page TYPE string PERMISSIONS FULL;
DEFINE FIELD content ON page TYPE string PERMISSIONS FULL;
DEFINE FIELD depth ON page TYPE int PERMISSIONS FULL;
DEFINE FIELD format ON page TYPE string PERMISSIONS FULL;
DEFINE FIELD crawled_at ON page TYPE datetime DEFAULT time::now() PERMISSIONS FULL;

-- Composite unique: same URL can exist in different groups
DEFINE INDEX idx_url_group ON page FIELDS url, group UNIQUE;

-- Fast group filtering and deletion
DEFINE INDEX idx_page_group ON page FIELDS group;

-- Analyzer for documentation text
DEFINE ANALYZER doc_analyzer
    TOKENIZERS BLANK,CLASS,CAMEL
    FILTERS LOWERCASE,SNOWBALL(ENGLISH);

-- BM25 full-text index on page content (does not include group filter; group applied in WHERE)
DEFINE INDEX idx_content ON page
    FIELDS content
    FULLTEXT ANALYZER doc_analyzer BM25(1.2,0.75);

-- Table: discovered (graph edges for link topology)
DEFINE TABLE discovered TYPE RELATION IN page OUT page SCHEMAFULL PERMISSIONS NONE;

DEFINE FIELD group ON discovered TYPE string ASSERT $value != NONE PERMISSIONS FULL;
DEFINE FIELD anchor_text ON discovered TYPE none | string PERMISSIONS FULL;
DEFINE FIELD discovered_at ON discovered TYPE datetime DEFAULT time::now() PERMISSIONS FULL;
```

---

## 6. Module Layout

```
crates/
├── vibeeye-core/
│   └── src/
│       └── output/           (NEW)
│           └── mod.rs        — Output trait: FileOutput, SurrealOutput
│
├── vibeeye-app/
│   └── src/
│       ├── db/               (NEW, gated behind #[cfg(feature = "surrealdb")])
│       │   ├── mod.rs        — Public API: init, store_page, store_links, query, reset
│       │   ├── schema.rs     — SurrealQL schema strings, bootstrap function
│       │   ├── models.rs     — Rust structs: PageRecord, LinkRecord, QueryResult
│       │   └── client.rs     — SurrealDB connection wrapper
│       ├── config/
│       │   └── crawl.rs      — Add SurrealDB config section
│       └── crawl/
│           └── mod.rs          — Wire SurrealOutput into crawl pipeline
│
├── vibeeye-cli/
│   └── src/
│       ├── cli.rs            — Add `query` subcommand, `-o embed` option
│       └── commands.rs         — Dispatch query command, handle --output=embed
│
└── vibeeye-mcp/
    └── (no changes in Phase 1)
```

---

## 7. Data Flow

### Crawl → SurrealDB
```
1. User: vibe-eye crawl https://example.com -o embed
2. CLI: Parse args, load config, build CrawlOptions
3. App: crawl::run(options) iterates BFS
4. App: For each PageCapture:
   a. Convert to PageRecord
   b. Upsert into SurrealDB (ON DUPLICATE KEY UPDATE via INSERT or UPDATE)
   c. For each discovered link:
      i. Ensure target page exists (placeholder or full)
      ii. RELATE current_page -> discovered -> target_page
5. App: Commit transaction, close DB connection
```

### Query Flow
```
1. User: vibe-eye query "HNSW indexing" --limit 10
2. CLI: Parse args
3. App: db::query(q, limit) executes:
   SELECT url, title, content, search::score(1) AS score
   FROM page WHERE content @@ $query
   ORDER BY score DESC LIMIT $limit
4. App: Format results (pretty table or JSON)
5. CLI: Print results
```

---

## 8. Rust Types

```rust
// crates/vibeeye-app/src/db/models.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct PageRecord {
    pub url: String,
    pub title: String,
    pub content: String,
    pub depth: i32,
    pub format: String,
    pub crawled_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QueryResult {
    pub url: String,
    pub title: String,
    pub snippet: String,
    pub score: f64,
}
```

### Group Name Derivation

```rust
pub fn derive_group(url: &str, override_name: Option<&str>) -> String {
    if let Some(name) = override_name {
        return sanitize_identifier(name);
    }
    let domain = extract_domain(url);  // "surrealdb.com"
    sanitize_identifier(&domain)       // "surrealdb_com"
}

fn sanitize_identifier(name: &str) -> String {
    let mut s = name
        .to_lowercase()
        .replace(|c: char| !c.is_alphanumeric() && c != '_', "_")
        .replace("__", "_")
        .trim_matches('_')
        .to_string();
    // SurrealDB identifiers max length is generous; truncate to 64 for safety
    s.truncate(64);
    if s.is_empty() { s = "default".to_string(); }
    s
}
```

### Query Patterns

```surrealql
-- Single-group search
SELECT url, title, content, search::score(1) AS score
FROM page
WHERE group = 'surrealdb_com' AND content @@ $query
ORDER BY score DESC LIMIT $limit;

-- Cross-group search (default when no --group provided)
SELECT url, title, content, search::score(1) AS score
FROM page
WHERE content @@ $query
ORDER BY score DESC LIMIT $limit;

-- Multi-group search (future: --group flag with comma list)
SELECT url, title, content, search::score(1) AS score
FROM page
WHERE group IN ('surrealdb_com', 'rust_lang_org') AND content @@ $query
ORDER BY score DESC LIMIT $limit;

-- Group-scoped graph traversal
SELECT ->discovered->page.url AS linked_url
FROM page
WHERE group = 'surrealdb_com' AND url = 'https://surrealdb.com/docs';

-- Fast group deletion (for db reset)
DELETE page WHERE group = 'surrealdb_com';
DELETE discovered WHERE group = 'surrealdb_com';
```

---

## 9. CLI Specification

### `vibe-eye crawl`
```
OPTIONS:
  -o, --output <PATH>     Output directory (default) or "embed" for SurrealDB
  -g, --group <NAME>      Crawl group name [default: derived from URL domain]

EXAMPLES:
  vibe-eye crawl https://example.com -o ./out              # flat files
  vibe-eye crawl https://surrealdb.com/docs -o embed        # group = surrealdb_com
  vibe-eye crawl https://surrealdb.com/docs -o embed -g docs-v2
```

### `vibe-eye query`
```
USAGE:
  vibe-eye query [OPTIONS] <QUERY>

ARGS:
  <QUERY>  Search query text

OPTIONS:
  -g, --group <NAME>  Search only this group [default: all groups]
  -l, --limit <N>     Maximum results [default: 10]
  -f, --format <FMT>  Output format: table, json [default: table]

EXAMPLES:
  vibe-eye query "HNSW indexing"                    # search all groups
  vibe-eye query "HNSW indexing" --group surrealdb_com  # search one group
  vibe-eye query "transactions" --limit 20 --format json
```

### `vibe-eye db`
```
USAGE:
  vibe-eye db <COMMAND>

COMMANDS:
  list     Show all crawl groups
  reset    Wipe a specific group's data (requires confirmation)
  status   Show database stats for a group (record counts, index status)

OPTIONS:
  -g, --group <NAME>  Target group [required for reset, optional for status]

EXAMPLES:
  vibe-eye db list
  vibe-eye db status --group surrealdb_com
  vibe-eye db reset --group surrealdb_com
```

---

## 10. Feature Flag Strategy

```toml
# vibeeye-app/Cargo.toml
[features]
default = []
surrealdb = ["dep:surrealdb"]

[dependencies]
surrealdb = { workspace = true, optional = true, features = ["kv-surrealkv"] }
```

```toml
# vibeeye-cli/Cargo.toml
[features]
default = []
surrealdb = ["vibeeye-app/surrealdb"]
```

Code gating:
```rust
#[cfg(feature = "surrealdb")]
mod db;
```

---

## 11. Dependencies

### New Workspace Dependency
```toml
# Cargo.toml [workspace.dependencies]
surrealdb = { version = "3.1", default-features = false, features = ["kv-surrealkv"] }
```

### Existing Dependencies (No Changes)
- `tokio` ✅ (already has full features including `tokio/time`)
- `serde` ✅
- `chrono` ✅
- `dirs` ✅
- `toml` ✅

---

## 12. Testing Strategy

| Component | Test Type | Backend |
|-----------|-----------|---------|
| `db::schema::bootstrap` | Unit | `kv-mem` |
| `db::client::connect` | Unit | `kv-mem` |
| `db::store_page` + `query` | Integration | `kv-mem` |
| Upsert (duplicate URL) | Integration | `kv-mem` |
| Graph edge creation | Integration | `kv-mem` |
| CLI `query` dispatch | Unit (mock db) | N/A |

All DB tests use `Surreal::new::<Mem>(()).await` for speed and isolation.

---

## 13. Success Criteria

- [ ] `cargo build --workspace` succeeds (without `surrealdb` feature)
- [ ] `cargo build --workspace --features surrealdb` succeeds
- [ ] `vibe-eye crawl https://example.com -o embed` stores pages and links
- [ ] `vibe-eye query "example"` returns ranked results with scores
- [ ] Re-crawling same URL updates existing record, not duplicates
- [ ] `vibe-eye db reset` clears all data
- [ ] File output (`-o ./dir`) still works with `surrealdb` feature enabled
- [ ] All tests pass with `cargo test --features surrealdb`
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` is clean
- [ ] CRAP scores for new functions ≤ 30

---

## 14. Deferred to Phase 2

- Embeddings and HNSW vector index
- Chunking strategy
- Hybrid search (BM25 + KNN)
- Candle server auto-start
- `dimension_metadata` tracking table
- MCP tool exposure for query
- Progress reporting during embedding
