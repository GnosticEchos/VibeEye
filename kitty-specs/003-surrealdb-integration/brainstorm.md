# Brainstorm: SurrealDB Integration for VibeEye

**Mission**: 003-surrealdb-integration  
**Status**: RATIFIED  
**Date**: 2026-05-27

---

## 1. Problem Statement

VibeEye currently crawls web pages and writes flat files (`0001.md`, `manifest.json`) to disk. This is stateless, unsearchable, and loses structural relationships between pages. We need a persistent, queryable backend that supports:

- **Full-text search** (BM25) over crawled content
- **Graph traversal** of discovered link topology
- **Semantic search** (vector similarity) for finding conceptually related content
- **Hybrid queries** combining all three modes

SurrealDB is a multi-model database (document + graph + vector) that provides all three capabilities natively with an embedded Rust SDK. This makes it an ideal fit.

---

## 2. Design Philosophy

### Lean by Default, Rich by Opt-in
- **Phase 1** requires only SurrealDB (no ML, no embeddings). This keeps binary size and hardware requirements low.
- **Phase 2** adds embeddings via an external, user-managed Candle server or OpenAI-compatible endpoint. Feature-gated behind `embeddings`.
- **VibeEye never bundles model weights.** Users build `candle-embed-server` separately or point to an existing Ollama/OpenAI endpoint.

### Two Providers, One Code Path
- `openai-compatible` — HTTP POST to `/v1/embeddings`. Covers Ollama, OpenAI, Azure, vLLM, TGI, SGLang, and our Candle server.
- `candle-server` — VibeEye auto-starts the binary, waits for `/health`, then uses the same `OpenAiCompatibleProvider` code path.

### Output-as-Embed
- `vibe-eye crawl https://example.com -o ./out` — file output (unchanged)
- `vibe-eye crawl https://example.com -o embed` — SurrealDB output (new)
- `vibe-eye query "how does HNSW work?"` — queries the SurrealDB store (new)

---

## 3. Architecture

```
┌─────────────────────────────────────────────┐
│           vibe-eye crawl -o embed           │
└────────────────┬────────────────────────────┘
                 │
    ┌────────────┴────────────┐
    │                         │
┌───▼────┐              ┌─────▼─────┐
│  BFS   │              │  Output   │
│ Crawl  │              │  Router   │
└───┬────┘              └─────┬─────┘
    │                         │
    │    ┌────────────────────┼────────────────────┐
    │    │                    │                    │
    │ ┌──▼───┐          ┌────▼────┐         ┌────▼────┐
    │ │ File │          │ Surreal │         │ Surreal │
    │ │ Dir  │          │  Page   │         │ Chunk   │
    │ │(old) │          │(graph)  │         │(embed)  │
    │ └──────┘          └─────────┘         └─────────┘
    │                          │                 │
    │                          │        ┌────────▼────────┐
    │                          │        │ Embedding       │
    │                          │        │ Provider        │
    │                          │        │ (OpenAI-compat) │
    │                          │        └────────┬────────┘
    │                          │                 │
    │                          │     ┌─────────┴─────────┐
    │                          │     │                   │
    │                          │  ┌──▼───┐         ┌────▼────┐
    │                          │  │Remote│         │ Candle  │
    │                          │  │HTTP  │         │ Server  │
    │                          │  │      │         │(auto-start)
    │                          │  └──────┘         └─────────┘
    │                          │
    │                   ┌──────▼──────┐
    │                   │ SurrealDB   │
    │                   │ Embedded    │
    │                   │ (SurrealKV) │
    │                   └─────────────┘
    │
    │              ┌──────────────────────────┐
    └─────────────►│ Full-Text Index (BM25)   │
                   │ Graph Links (RELATE)     │
                   │ Vector Index (HNSW) ─────┘ (embed feature only)
                   └──────────────────────────┘
```

---

## 4. Phase 1: Full-Text + Graph (No Embeddings)

### 4.1 Goals
- Store crawled pages in embedded SurrealDB (SurrealKV)
- Index content with BM25 full-text search
- Track link topology as graph edges (`RELATE page -> discovered -> page`)
- New CLI: `vibe-eye crawl -o embed` and `vibe-eye query "..."`
- Zero ML dependencies, runs on minimal hardware

### 4.2 SurrealDB Schema

```surrealql
-- Table: page
DEFINE TABLE page TYPE NORMAL SCHEMAFULL PERMISSIONS NONE;

DEFINE FIELD url ON page TYPE string ASSERT $value != NONE PERMISSIONS FULL;
DEFINE FIELD title ON page TYPE string PERMISSIONS FULL;
DEFINE FIELD content ON page TYPE string PERMISSIONS FULL;
DEFINE FIELD depth ON page TYPE int PERMISSIONS FULL;
DEFINE FIELD format ON page TYPE string PERMISSIONS FULL;
DEFINE FIELD crawled_at ON page TYPE datetime DEFAULT time::now() PERMISSIONS FULL;

-- Index: URL lookup (unique prevents duplicate crawls of same URL)
DEFINE INDEX idx_url ON page FIELDS url UNIQUE;

-- Analyzer: blank splits on whitespace, class on Unicode class changes,
-- camel on camelCase, lowercase normalizes, snowball stems English
DEFINE ANALYZER doc_analyzer
    TOKENIZERS BLANK,CLASS,CAMEL
    FILTERS LOWERCASE,SNOWBALL(ENGLISH);

-- BM25 full-text index on rendered markdown
DEFINE INDEX idx_content ON page
    FIELDS content
    FULLTEXT ANALYZER doc_analyzer BM25(1.2,0.75);

-- Table: discovered (graph edges for link topology)
DEFINE TABLE discovered TYPE RELATION IN page OUT page SCHEMAFULL PERMISSIONS NONE;

DEFINE FIELD anchor_text ON discovered TYPE none | string PERMISSIONS FULL;
DEFINE FIELD discovered_at ON discovered TYPE datetime DEFAULT time::now() PERMISSIONS FULL;
```

### 4.3 CLI Surface

```bash
# Crawl into SurrealDB
vibe-eye crawl https://surrealdb.com/docs -o embed

# Full-text query
vibe-eye query "HNSW indexing"
# → returns top pages by BM25 score with highlights

# Graph query (pages within N hops)
vibe-eye query "transactions" --depth 2
# → BM25 search + traverse discovered links up to 2 hops

# Config in ~/.config/vibe-eye/crawl.toml
[surrealdb]
endpoint = "surrealkv://~/.local/share/vibeeye/db"
namespace = "vibeeye"
database = "crawl"
```

### 4.4 Chunking Strategy (Phase 1)
No chunking yet — entire page content stored as a single `page` record. BM25 operates on full content. Chunking comes in Phase 2 for vector alignment.

### 4.5 Feature Flag
```toml
# vibeeye-app/Cargo.toml
[features]
default = []
surrealdb = ["dep:surrealdb"]
```

---

## 5. Phase 2: Embeddings (Opt-in)

### 5.1 Goals
- Chunk pages at `context_window` tokens (respecting headings)
- Embed chunks via configured provider
- Store chunks with HNSW vector index
- Hybrid queries: BM25 pre-filter → KNN vector rerank

### 5.2 Schema Additions (Dimension-Segregated Tables)

When embedding model dimension changes, a new table is created. Old table persists for backward compatibility.

```surrealql
-- Dimension tracking table
DEFINE TABLE dimension_metadata TYPE NORMAL SCHEMAFULL PERMISSIONS NONE;

DEFINE FIELD component ON dimension_metadata TYPE string PERMISSIONS FULL;
DEFINE FIELD dimension ON dimension_metadata TYPE number PERMISSIONS FULL;
DEFINE FIELD embedding_model ON dimension_metadata TYPE none | string PERMISSIONS FULL;
DEFINE FIELD model_endpoint ON dimension_metadata TYPE none | string PERMISSIONS FULL;
DEFINE FIELD model_provider ON dimension_metadata TYPE none | string PERMISSIONS FULL;
DEFINE FIELD operation ON dimension_metadata TYPE string PERMISSIONS FULL;
DEFINE FIELD table_name ON dimension_metadata TYPE string PERMISSIONS FULL;
DEFINE FIELD created_at ON dimension_metadata TYPE string DEFAULT time::now() PERMISSIONS FULL;

DEFINE INDEX idx_dimension_metadata_component ON dimension_metadata FIELDS component;
DEFINE INDEX idx_dimension_metadata_dimension ON dimension_metadata FIELDS dimension;

-- Chunk table for 768-dim model (example)
DEFINE TABLE chunk_768 TYPE NORMAL SCHEMAFULL PERMISSIONS NONE;

DEFINE FIELD source ON chunk_768 TYPE record<page> PERMISSIONS FULL;
DEFINE FIELD heading_path ON chunk_768 TYPE array<string> PERMISSIONS FULL;
DEFINE FIELD heading_path.* ON chunk_768 TYPE string PERMISSIONS FULL;
DEFINE FIELD content ON chunk_768 TYPE string PERMISSIONS FULL;
DEFINE FIELD chunk_index ON chunk_768 TYPE int PERMISSIONS FULL;
DEFINE FIELD embedding ON chunk_768 TYPE array<float> PERMISSIONS FULL;
DEFINE FIELD embedding.* ON chunk_768 TYPE float PERMISSIONS FULL;
DEFINE FIELD created_at ON chunk_768 TYPE datetime DEFAULT time::now() PERMISSIONS FULL;

-- HNSW vector index (parameters tuned for 768-dim)
DEFINE INDEX hnsw_idx_chunk_768 ON chunk_768
    FIELDS embedding
    HNSW DIMENSION 768 DIST COSINE TYPE F32 EFC 150 M 12 M0 24;

-- Index for chunk-to-page lookups
DEFINE INDEX idx_chunk_source ON chunk_768 FIELDS source;
```

Table name is constructed dynamically: `format!("chunk_{}", dimensions)` → `chunk_768`, `chunk_1024`, etc.

### 5.3 Config

```toml
[embeddings]
provider = "candle-server"  # or "openai-compatible"

[candle-server]
binary_path = "/home/user/.cargo/bin/candle-embed-server"
model = "BAAI/bge-base-en-v1.5"
port = 0              # 0 = random free port
dimensions = 768
context_window = 8192
args = ["--quantize", "q4_0"]

[openai-compatible]
endpoint = "http://localhost:11434/v1/embeddings"
model = "nomic-embed-text"
dimensions = 768
context_window = 8192
api_key = "${OLLAMA_API_KEY}"   # optional, env interpolation
```

### 5.4 Auto-Start Flow (Candle Server)

```
Before crawl:
  1. Spawn candle-embed-server with random port
  2. Poll /health until ready (timeout 60s)
  3. Use OpenAiCompatibleProvider against localhost:{port}

After crawl:
  4. SIGTERM candle-embed-server
  5. Wait for graceful exit
```

### 5.5 Chunking Strategy

```
Page markdown → Split by H2/H3 headings →
Paragraph groups → Token-count split at context_window →
Store each chunk with heading_path breadcrumbs
```

Query-time expansion: retrieve top-k chunks, then expand ±1 chunk for context window assembly.

### 5.6 Hybrid Query Example

```surrealql
-- Step 1: BM25 pre-filter to narrow candidate set
LET $text_candidates = SELECT id FROM page
    WHERE content @@ 'HNSW vector indexing'
    ORDER BY search::score(1) DESC
    LIMIT 50;

-- Step 2: KNN vector search on chunks from candidate pages
LET $query_embedding = $embedding_provider.embed("HNSW vector indexing");

SELECT
    c.content,
    c.heading_path,
    c.source.title AS page_title,
    c.source.url AS page_url,
    vector::distance::knn() AS distance
FROM chunk_768 AS c
WHERE c.source IN $text_candidates
    AND c.embedding <|10|> $query_embedding
ORDER BY distance
LIMIT 10;
```

### 5.7 Feature Flag
```toml
# vibeeye-app/Cargo.toml
[features]
default = []
surrealdb = ["dep:surrealdb"]
embeddings = ["surrealdb", "dep:reqwest"]  # etc.
```

---

## 6. Non-Functional Requirements

| ID | Requirement | Phase |
|----|-------------|-------|
| NFR-01 | Binary size: +~5MB with `surrealdb` feature, +~0MB additional for `embeddings` (HTTP only) | Both |
| NFR-02 | RAM: BM25 index minimal; HNSW vectors in memory for embedding queries | 2 |
| NFR-03 | No external services required for Phase 1 | 1 |
| NFR-04 | Candle binary is user-managed; VibeEye only orchestrates | 2 |
| NFR-05 | All code passes `cargo clippy --all-targets --all-features -- -D warnings` | Both |
| NFR-06 | CRAP threshold ≤ 30 for new functions | Both |

---

## 7. Open Questions

1. **Chunking token estimation** — Use approximate char count, or pull in `tokenizers`/`tiktoken-rs` for accuracy?
2. **Schema migrations** — Phase 1: idempotent `DEFINE IF NOT EXISTS` bootstrap, no migration system. Re-crawl if schema changes. Phase 2: adopt lightweight versioning if indexes grow large.
3. **Query result format** — JSON Lines? Pretty-printed markdown? MCP-compatible?
4. **Concurrent crawls** — Same SurrealDB instance, separate databases per crawl?
5. **Progress reporting** — Embedding 500 chunks = 500 HTTP requests. Batch and show progress bar?

---

## 8. Known Issues (Separate Tracking)

See `/home/james/Projects/VibeEye/known_issues.md`:
- **Missing Crawl Pages — SurrealDB Docs**: BFS crawl missed many sub-pages on `surrealdb.com/docs` despite `extract` working. Needs investigation (SPA routing, link extraction, robots.txt, etc.).

---

## 9. Next Steps

1. ✅ Ratify this brainstorm
2. Draft detailed Phase 1 spec (schema, CLI args, module layout, SurrealDB SDK integration)
3. Identify work packages and task breakdown
4. Begin implementation
