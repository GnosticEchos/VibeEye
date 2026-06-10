# VibeEye

Headless browser for agentic content extraction. Built on [Servo](https://servo.org/) for real browser rendering, with vector storage via SurrealDB and semantic search via embeddings.

## Features

- **Real browser engine** — Servo renders JavaScript, handles SPAs (crates.io, docs.rs), and settles dynamic content
- **Structured extraction** — Markdown, HTML, or plain text with automatic JSON-LD / Open Graph metadata capture
- **Instant page cleanup** — `extract` strips bloat (nav, ads, scripts) so you can paste clean content into your LLM chat
- **SurrealDB persistence** — Store pages and chunks with BM25 full-text + HNSW vector indexes
- **Semantic search** — BM25 keyword search, k-NN vector search, and hybrid ranking
- **Batch processing** — Crawl a list of URLs in parallel with shared DB output
- **MCP server** — Expose crawl and query tools to MCP-compatible agents
- **Multi-endpoint embedding** — Round-robin load balancing across multiple embedding servers

## Quick Start

### Prerequisites

- Rust 1.86+
- SurrealDB (standalone server, or use embedded SurrealKV)
- An embedding server (Ollama, llama.cpp, or FLM)

### Build

```bash
cargo build --release --features "surrealdb embeddings"
```

### Extract a page

The fastest way to get clean content from any URL — no database needed. Strips navigation, ads, and scripts, leaving readable Markdown you can paste straight into an LLM chat.

```bash
# Clean Markdown (default)
./target/release/vibe-eye extract https://doc.rust-lang.org/book/ch04-01-what-is-ownership.html

# Plain text
./target/release/vibe-eye extract https://example.com/article --format text

# Raw HTML
./target/release/vibe-eye extract https://example.com/article --format html
```

### Configure

Copy the example config and edit:

```bash
mkdir -p ~/.config/vibe-eye
cp crawl.example.toml ~/.config/vibe-eye/crawl.toml
```

At minimum, set your SurrealDB URL and embedding endpoint:

```toml
[global]
db_url = "ws://user:pass@127.0.0.1:8099"

[global.embeddings]
provider = "openai-compatible"
endpoints = ["http://localhost:11434/v1/embeddings"]
model = "nomic-embed-text"
embed_concurrency = 4
```

### Crawl a site

```bash
# Crawl into SurrealDB
./target/release/vibe-eye crawl https://doc.rust-lang.org/book --surrealdb

# Crawl + embed chunks for semantic search
./target/release/vibe-eye crawl https://doc.rust-lang.org/book --surrealdb --embed

# Batch crawl from a URL list
./target/release/vibe-eye batch urls.txt --surrealdb --group docs --embed
```

### Search

```bash
# Full-text (BM25)
./target/release/vibe-eye db query "rust ownership" --group docs

# Vector (k-NN) — requires --embed
./target/release/vibe-eye db vector "rust ownership" --group docs

# Hybrid — best of both
./target/release/vibe-eye db hybrid "rust ownership" --group docs
```

## CLI Commands

```
vibe-eye extract <URL>     Clean single-page extraction (Markdown / text / HTML)
vibe-eye crawl <URL>       BFS crawl from seed URL
vibe-eye batch <FILE>      Batch crawl from URL list (one per line)
vibe-eye db list           List crawl groups
vibe-eye db status <GROUP> Page / chunk counts for a group
vibe-eye db query <Q>      BM25 keyword search
vibe-eye db vector <Q>     Vector similarity search
vibe-eye db hybrid <Q>     Hybrid BM25 + vector search
vibe-eye db reset <GROUP>  Delete a group
vibe-eye db reset-all      Delete everything
```

## Architecture

See [`ARCHITECTURE.md`](ARCHITECTURE.md) for system diagrams, crawl pipeline flow, and MCP tool contracts.

## Workspace

| Crate | Purpose |
|-------|---------|
| `vibeeye-core` | Shared types (`ContentFormat`, `CrawlResult`) |
| `vibeeye-app` | Browser session, crawl engine, DB client, embeddings |
| `vibeeye-cli` | `vibe-eye` binary |
| `vibeeye-mcp` | MCP server tools |

## License

MIT OR Apache-2.0
