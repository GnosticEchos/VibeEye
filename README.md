<div align="center">

# VibeEye

**Headless browser for agentic content extraction — built on Servo**

Give your AI agents clean, current information from any public web page.

[![Rust](https://img.shields.io/badge/rust-1.86+-dea584.svg)](https://www.rust-lang.org)
[![Servo](https://img.shields.io/badge/servo-0.1.0-orange)](https://servo.org)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](https://github.com/vibeeye/vibeeye)
[![Built with](https://img.shields.io/badge/built%20with-SurrealDB-ff00a0)](https://surrealdb.com)

</div>

---

## Why VibeEye?

LLM agents are only as good as the information they can access. VibeEye bridges the gap between the live web and your AI agents:

- **Feed agents timely documentation** — `vibe-eye extract https://docs.example.com/api -f text` pipes clean, stripped content straight into an agent's context window
- **Build searchable knowledge bases** — crawl documentation sites, chunk pages semantically, embed them, and query with BM25 + vector hybrid search
- **No Chromium tax** — built on [Servo](https://servo.org/), the Rust-native browser engine. No Playwright, no Puppeteer, no Chrome dependency
- **Agent-native** — ships with an [MCP server](#mcp-server) that exposes every capability as tools any MCP-compatible agent host (Claude Desktop, Cursor, etc.) can call directly

### Use cases

| Scenario | Command | Why VibeEye? |
|----------|---------|-------------|
| Agent needs up-to-date docs | `extract https://docs.rs/serde/latest/serde/ -f text` | Clean text — no nav bars, no ads, no scripts |
| Build a searchable knowledge base | `crawl https://doc.rust-lang.org/book --surrealdb --embed` | Full BFS crawl → chunk → embed → hybrid search pipeline |
| Feed agent a spec from a live URL | `extract https://example.com/rfc -f markdown` | Strips noise, returns clean Markdown for context |
| Batch process many URLs | `batch urls.txt --surrealdb --group docs` | Concurrent fetch with shared DB output |
| Agent queries your knowledge base | MCP `db_hybrid` tool | BM25 pre-filter + vector rerank — best of both worlds |

---

## Features

- **Real browser engine** — Servo renders JavaScript, handles SPAs (crates.io, docs.rs), and settles dynamic content. No headless Chrome required.
- **Structured extraction** — Markdown, HTML, or plain text with automatic JSON-LD / Open Graph metadata capture.
- **BFS web crawling** — Configurable depth, page limits, rate limiting, same-origin restrictions, sitemap pre-seeding, and robots.txt compliance.
- **SPA auto-detection** — Compares raw HTML link count vs live DOM link count; if DOM has significantly more, uses rendered links for BFS discovery.
- **Semantic chunking** — Heading-aware text splitting that preserves document hierarchy (e.g., `["Title", "Section A", "Subsection"]`).
- **SurrealDB persistence** — Store pages, links, and chunks with BM25 full-text + HNSW vector indexes.
- **Hybrid search** — BM25 keyword search, k-NN vector similarity, or two-phase hybrid (BM25 pre-filter → vector rerank with context expansion).
- **Embedding-optional** — All extraction and crawling works without SurrealDB or an embedding server. Just `cargo build --release` and go.
- **Multi-endpoint embedding** — Round-robin load balancing across multiple embedding servers (Ollama, llama.cpp, FLM NPU).
- **MCP server** — Expose 13 tools to MCP-compatible agents via stdio transport.
- **Batch processing** — Fetch a list of URLs in parallel without BFS link discovery.
- **Configurable output** — Stdout (JSON Lines), directory (files + manifest), or SurrealDB — or all three simultaneously.
- **Help-tree introspection** — `--help-tree` generates a machine-readable JSON capability map for autonomous agents.

---

## Quick Start

### Prerequisites

- **Rust 1.86+** (`rustup upgrade` if needed)
- **SurrealDB** (optional) — standalone server, or use embedded SurrealKV (`surrealkv://`)
- **Embedding server** (optional) — Ollama, llama.cpp, or FLM for semantic search

> **No built-in authentication.** Only publicly accessible content is supported. Pages behind logins cannot be crawled.

### Install

```bash
git clone https://github.com/vibeeye/vibeeye
cd vibeeye

# Minimal build — no database, no embeddings
cargo build --release

# With SurrealDB support
cargo build --release --features surrealdb

# Full — SurrealDB + embeddings
cargo build --release --features "surrealdb embeddings"

# The binary is `vibe-eye`
./target/release/vibe-eye --help
```

### Try it in 30 seconds

No database, no config file — just clean content:

```bash
# Extract a documentation page as clean Markdown
./target/release/vibe-eye extract https://doc.rust-lang.org/book/ch04-01-what-is-ownership.html

# Extract as plain text (best for feeding into LLM context)
./target/release/vibe-eye extract https://surrealdb.com/docs/learn/querying/concepts-and-guides/bulk-operations-and-data-import -f text

# Snapshot (URL, title, body text, raw HTML)
./target/release/vibe-eye snapshot https://example.com

# Navigate and settle a JS-heavy SPA
./target/release/vibe-eye navigate https://crates.io/crates/serde
```

---

## CLI Reference

### Top-level commands

| Command | Description |
|---------|-------------|
| `navigate <url>` | Browse to a URL and return JSON metadata (title, etc.). Good for quick JS rendering checks. |
| `snapshot <url>` | Full page capture: URL, title, body text, raw HTML. |
| `extract <url>` | Extract clean content in your choice of format. **The workhorse for feeding agents.** |
| `batch <file>` | Fetch a list of URLs (one per line) in parallel. No BFS, no link discovery. |
| `crawl <url>` | Full BFS crawl from a seed URL. Discovers and processes linked pages. |
| `import <source>` | Import crawl data (`.surql` file, output directory, or flat text) into SurrealDB. |
| `export <target>` | Export SurrealDB group data as SurQL statements. |
| `db <subcommand>` | Database management and search. |

### Global flags

| Flag | Description |
|------|-------------|
| `-v, --verbose` | Enable DEBUG-level tracing output |
| `--help-tree` | Print a recursive, machine-readable command map (supports JSON output, depth limiting, theme configuration) |
| `-L, --tree-depth <N>` | Limit help-tree recursion depth |
| `-I, --tree-ignore <LABEL>` | Exclude subtrees from help-tree output |
| `-a, --tree-all` | Include hidden subcommands |
| `--tree-output <text|json>` | Help-tree output format |
| `--tree-style <rich>` | Tree styling mode (uses ANSI bold/italic + colour) |
| `--tree-color <auto>` | Colour mode (auto = ANSI colours only on TTY) |

### `extract` — the daily driver

```bash
vibe-eye extract <URL> [options]

Options:
  -f, --format <FORMAT>  Output format: markdown (default), html, or text
```

Extracts and cleans page content. Automatically:
- Strips navigation, ads, scripts, styles, SVGs, headers, footers
- Extracts JSON-LD + Open Graph metadata (included as YAML frontmatter)
- Converts HTML to clean Markdown (markdown format)
- Returns plain text for LLM context windows (text format)
- Preserves raw HTML structure (html format)

**Best practice for LLM agents:** use `-f text` — it produces the cleanest output with the smallest token count.

### `crawl` — full site ingestion

```bash
vibe-eye crawl <URL> [options]

Options:
  --config <FILE>             Custom TOML config path
  --max-depth <N>             Maximum BFS depth (default: 2)
  --max-pages <N>             Max pages (0 = unlimited, default: 100)
  -f, --format <FORMAT>       Output format
  -o, --output <DIR>          Directory output (files + manifest.json)
  --urls-file <FILE>          Additional seed URLs (one per line; `-` for stdin)
  --output-urls <FILE>        Append discovered URLs to a text file
  --respect-robots <bool>     Respect robots.txt (default: false)
  --requests-per-second <N>   Rate limit per host (default: 2.0)
  --concurrency <N>           Concurrent browser sessions (default: 4)
  --same-origin <bool>        Stay on same origin (default: true)
  --timeout <SECONDS>         Per-page timeout (default: 15)
  --sitemap <bool>            Pre-seed queue from sitemap.xml (default: false)
  --surrealdb                 Persist to SurrealDB
  --embed                     Generate embeddings (requires --surrealdb)
  --devtools                  Enable DevTools server (random port, set VIBEYE_DEVTOOLS=1)
```

The crawl engine:
1. **Resolves** the seed URL and builds initial queue (optionally from sitemap.xml)
2. **Loads** robots.txt if `--respect-robots`
3. **BFS loop**: pops URLs, checks depth/robots/max_pages, rate-limits per host
4. **Per page**: navigates → checks HTTP status → settles if `<script>` detected → captures HTML + localStorage → validates (rejects 4xx/5xx, soft-404s, noindex) → auto-detects SPA → extracts structured metadata (JSON-LD, Open Graph) → distills content
5. **Emits** results to all configured output sinks in batches of 50

### `batch` — targeted URL fetching

```bash
vibe-eye batch <FILE> [options]

Options:
  -f, --format <FORMAT>       Output format
  -o, --output <DIR>          Directory output
  --concurrency <N>           Max concurrent fetches (default: 4)
  --timeout <SECONDS>         Per-page timeout (default: 15)
  --settle-ms <MS>            SPA settle time in ms (default: 2000)
  --surrealdb                 Persist to SurrealDB
  --group <GROUP>             Crawl group name (required with --surrealdb)
  --embed                     Generate embeddings (requires --surrealdb)
```

Like `crawl` but no link discovery — processes exactly the URLs you provide. Use `-` for stdin to pipe URLs.

### `db` — search and manage

```bash
vibe-eye db list                                          # List all crawl groups
vibe-eye db status <group>                                # Page/chunk counts for a group
vibe-eye db query "<query>" -g <group> -l 10 -f json     # BM25 full-text search
vibe-eye db vector "<query>" -g <group> -l 10 -f json    # Vector similarity search
vibe-eye db hybrid "<query>" -g <group> -l 10 -f json    # Hybrid search (BM25 + vector)
vibe-eye db reset <group>                                 # Delete all data for a group
vibe-eye db reset-all                                     # Delete everything
```

**Output formats** (for query/vector/hybrid): `json`, `table`, `markdown`

**Search types:**

| Type | Description | Requires |
|------|-------------|----------|
| `query` | BM25 full-text with snippet highlighting | `surrealdb` |
| `vector` | Cosine similarity k-NN search | `surrealdb` + `embeddings` |
| `hybrid` | BM25 pre-filter → vector rerank → adjacent-chunk context expansion | `surrealdb` + `embeddings` |

Hybrid search is the most powerful: it first finds candidate pages via BM25 keyword search, then reranks their chunks by vector similarity, and finally expands results with adjacent chunks for richer context.

---

## MCP Server

VibeEye includes a standards-compliant MCP server (`vibeeye-mcp`) that exposes all capabilities as tools any MCP-compatible agent host can call.

### Running the MCP server

```json
{
  "mcpServers": {
    "vibeeye": {
      "command": "/path/to/vibe-eye-mcp",
      "args": []
    }
  }
}
```

Or with SurrealDB and embeddings:

```json
{
  "mcpServers": {
    "vibeeye": {
      "command": "/path/to/vibe-eye-mcp",
      "env": {
        "VIBEYE_DB_URL": "ws://user:pass@127.0.0.1:8099"
      }
    }
  }
}
```

### Available tools (13 total)

**Always available:**

| Tool | Input | Returns |
|------|-------|---------|
| `browser_navigate` | `url`, `wait_until?` | `success`, `current_url`, `title` |
| `browser_snapshot` | `url` | `url`, `title`, `body_text`, `html` |
| `browser_extract` | `url`, `format?` (default: markdown) | `url`, `content`, `format`, `title` |

**With `surrealdb` feature:**

| Tool | Input | Returns |
|------|-------|---------|
| `db_query` | `query`, `group?`, `limit?` (10) | BM25 search results |
| `db_list` | (none) | All crawl group names |
| `db_status` | `group` | Page/chunk/link counts |
| `crawl` | `url`, `max_depth?` (3), `max_pages?` (100), `group?`, `surrealdb?` (true), `embed?` (false) | Crawl summary + group name |
| `db_export` | `group`, `target_path` | Confirmation |
| `db_import` | `group`, `source_path` | Confirmation |
| `db_reset` | `group` | Confirmation |
| `db_reset_all` | (none) | Confirmation |

**With `embeddings` feature:**

| Tool | Input | Returns |
|------|-------|---------|
| `db_vector` | `query`, `group?`, `limit?` (10) | Vector similarity results |
| `db_hybrid` | `query`, `group?`, `limit?` (10), `bm25_limit?` (50) | Hybrid search results |

> **Safety:** Destructive operations (`db_reset`, `db_reset_all`) include warnings in their tool descriptions recommending CLI use instead to prevent accidental data loss during agent sessions.

---

## Configuration

### Config file location

`~/.config/vibe-eye/crawl.toml` (or custom path via `--config`)

### Merge order

```
[global] → [domain."example.com"] → [subdomain."docs.example.com"]
```

More-specific sections override less-specific ones. Every field is optional.

### Example

```toml
[global]
max_depth = 2
max_pages = 100
format = "markdown"
concurrency = 4
timeout = 15
same_origin = true
respect_robots = false
requests_per_second = 2.0

# SurrealDB (optional)
db_url = "surrealkv://~/.local/share/vibe-eye/db"
surrealdb_ns = "vibeeye"
surrealdb_db = "crawl"

[global.embeddings]
provider = "openai-compatible"
endpoint = "http://localhost:11434/v1/embeddings"
model = "nomic-embed-text"
chunk_size = 512
chunk_overlap = 50
embed_concurrency = 4

# CLI help-tree theming
[cli.help_tree.command]
style = "bold"
color = "#7ee7e6"

[cli.help_tree.description]
style = "italic"
color = "#90a2af"

# Per-domain overrides
[domain."surrealdb.com"]
max_depth = 3
respect_robots = true

[subdomain."docs.surrealdb.com"]
max_depth = 5
sitemap = true
```

### Database URL formats

| Scheme | Description |
|--------|-------------|
| `ws://user:pass@host:port` | Remote SurrealDB (WebSocket) |
| `surrealkv:///path/to/db` | Embedded SurrealKV (file-based, no server needed) |
| `mem://` | In-memory (ephemeral, for testing) |

Default: `surrealkv://~/.local/share/vibe-eye/db`

The `VIBEYE_DB_URL` environment variable overrides the config file value.

### Embedding endpoints

Supports any OpenAI-compatible `/v1/embeddings` API:

```toml
# Single endpoint
endpoint = "http://localhost:11434/v1/embeddings"

# Multi-endpoint round-robin (load balancing)
endpoints = [
    "http://localhost:7680/v1/embeddings",
    "http://localhost:7681/v1/embeddings",
]
```

Tested with: Ollama, llama.cpp, FLM (AMD NPU), OpenAI API.

### Embedding dimension handling

VibeEye auto-detects the embedding vector dimension from the first API response
and stores it alongside every chunk. If you switch embedding models (e.g., from
768-dim `nomic-embed-text` to 3584-dim `nomic-embed-code`), the dimension filter
ensures vector queries only compare against chunks with matching dimensions:

- **Auto-detection** — The first embedding batch probes the endpoint and logs the
  dimension. Set `dimensions` in config to skip the probe.
- **Dimension-scoped queries** — All `db vector` and `db hybrid` queries include a
  `dimensions = $dimension` filter, preventing `vector::similarity::cosine()` from
  crashing on mismatched vectors.
- **Coexistence** — Chunks from different embedding models can coexist in the same
  database without errors. Only chunks matching the active model's dimension are
  searched.
- **Changing models** — To switch models, just update the endpoint and model in
  your config. Old chunks remain queryable until you re-crawl with `--embed`.
  To re-embed everything, reset the group (`db reset <group>`) and re-crawl.

---

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                        CLI (vibe-eye)                    │
│  navigate │ snapshot │ extract │ batch │ crawl │ db     │
└──────────────────────┬──────────────────────────────────┘
                       │
┌──────────────────────▼──────────────────────────────────┐
│               MCP Server (vibeeye-mcp)                   │
│   13 tools exposed to AI agents via stdio transport     │
└──────────────────────┬──────────────────────────────────┘
                       │
┌──────────────────────▼──────────────────────────────────┐
│               vibeeye-app (shared library)               │
│                                                          │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌─────────┐ │
│  │ Browser  │  │  Crawl   │  │Extraction│  │    DB   │ │
│  │ Session  │  │  Engine  │  │ Pipeline │  │  Client │ │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘  └────┬────┘ │
│       │             │             │             │      │
│  ┌────▼─────┐  ┌────▼─────┐  ┌────▼─────┐  ┌────▼────┐│
│  │  Chunk   │  │  Embed   │  │  Config  │  │  Output ││
│  │          │  │ Provider  │  │  Loader  │  │  Sinks  ││
│  └──────────┘  └──────────┘  └──────────┘  └─────────┘│
└──────────────────────┬──────────────────────────────────┘
                       │
┌──────────────────────▼──────────────────────────────────┐
│              Servo Engine (dedicated thread)             │
│  ┌──────────────┐  ┌──────────────┐  ┌───────────────┐ │
│  │   Navigate   │  │    Eval JS   │  │   DevTools    │ │
│  │   + Settle   │  │  + DOM Query │  │   Server      │ │
│  └──────────────┘  └──────────────┘  └───────────────┘ │
│  Comm: mpsc + oneshot channels                           │
└──────────────────────┬──────────────────────────────────┘
                       │
┌──────────────────────▼──────────────────────────────────┐
│                   SurrealDB                              │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌─────────┐│
│  │   page   │  │discovered│  │  chunk   │  │db_meta  ││
│  │  SCHEMA  │  │ RELATION │  │ vector + │  │ key-val ││
│  │  FULL    │  │          │  │ BM25 idx │  │         ││
│  └──────────┘  └──────────┘  └──────────┘  └─────────┘│
└─────────────────────────────────────────────────────────┘
```

### Crawl pipeline

```
URL ──► Navigate ──► Load? ──► Script? ──► Settle ──► Extract ──► Persist
                │         │          │             │           │
            Timeout    Partial   No (static)   Scroll-      Stdout /
            fallback   content                 settle loop  Dir / DB
```

### SPA auto-detection

```
Raw HTML <a> count vs Live DOM <a> count

DOM > 2x raw AND DOM > 5  ──► Use rendered DOM links (SPA detected)
Otherwise                  ──► Use raw HTML links (static site)
```

### Workspace crates

| Crate | Purpose | Dependencies |
|-------|---------|-------------|
| **vibeeye-core** | Domain types (`ContentFormat`, `Viewport`, `RenderedBuffer`, `VibeError`) | serde, thiserror, chrono |
| **vibeeye-app** | Browser session, crawl engine, extraction, chunking, embeddings, SurrealDB client | Servo, surrealdb, scraper, tokio |
| **vibeeye-cli** | `vibe-eye` binary with clap CLI + help-tree introspection | clap, comfy-table, indicatif |
| **vibeeye-mcp** | MCP server exposing 13 tools via stdio transport | rust-mcp-sdk, schemars |

### Thin UI design pattern

CLI and MCP are both thin wrappers around the same shared library (`vibeeye-app`).
Every tool — `navigate`, `snapshot`, `extract`, `crawl`, `batch`, `db query` — is
implemented once in `vibeeye-app` as a struct implementing the `Tool` trait. Both
the CLI binary (`vibe-eye`) and the MCP server (`vibeeye-mcp`) import and call the
same implementation:

```
┌─────────────────────────────────────────────────────────────┐
│                     vibeeye-app (shared lib)                │
│  BrowseTool │ SnapshotTool │ ExtractTool │ crawl::run      │
│  db::DbClient │ chunk::Chunker │ embed::EmbeddingProvider  │
└────────────────────────┬────────────────────────────────────┘
                         │ imports                         
              ┌──────────┴──────────┐
              ▼                     ▼
┌──────────────────┐   ┌────────────────────────┐
│   vibeeye-cli    │   │    vibeeye-mcp          │
│  (vibe-eye bin)  │   │  (MCP stdio server)    │
│                  │   │                         │
│  clap dispatch   │   │  rust-mcp-sdk dispatch  │
│  → commands.rs   │   │  → handler.rs           │
└──────────────────┘   └────────────────────────┘
```

**Interface parity is enforced by a test.** The [`parity_test`](crates/vibeeye-cli/tests/parity_test.rs)
verifies that `vibe-eye --help-tree -f json` exposes the same tools with the same
semantics as the `ToolRegistry` that powers the MCP `tools/list` endpoint. If a new
tool is added to `vibeeye-app` without wiring it into both UIs, the test fails.

This means:
- **Users get the same capabilities** whether they run `vibe-eye extract` at a terminal
  or an AI agent calls `browser_extract` via MCP.
- **One code path to maintain** — bug fixes, improvements, and new features land in
  `vibeeye-app` and are immediately available from both interfaces.
- **Feature gates stay consistent** — the `surrealdb` and `embeddings` features gate
  the same code in CLI, MCP, and library builds.
---

## How It Works

### Browser engine

VibeEye runs a single Servo browser engine on a dedicated OS thread (`"servo-engine"`). All interactions happen via a command channel pattern:

```rust
enum EngineCommand {
    Navigate { url, respond: oneshot::Sender<Result<…>> },
    GetHtml { respond },
    EvalJs { script, respond },
    GetDomLinks { respond },
    Shutdown,
}
```

The engine thread loops: `servo.spin_event_loop()` → `try_recv(command)` → handle. Sessions are acquired from a process-wide singleton pool (`OnceLock<Mutex<Option<ServoEngine>>>`) and returned on drop.

### Content extraction

1. Noise elements removed via `scraper` CSS selectors (`script`, `style`, `nav`, `header`, `footer`, `svg`, `nav`, `.sidebar`, `#toc`, etc.)
2. HTML parsed and cleaned
3. Output format applied: Markdown (via `html-to-markdown-rs`), plain text (tag-stripped), or raw HTML
4. Structured metadata extracted: JSON-LD (`<script type="application/ld+json">`) + Open Graph meta tags

### Semantic chunking

For embedding, pages are split into chunks using heading-aware segmentation:

1. Split Markdown by `#`, `##`, `###` heading boundaries, tracking the heading path (e.g., `["Installation", "Linux", "Docker"]`)
2. Chunks exceeding the target token count are recursively split at paragraph → sentence → word → character boundaries
3. Overlap tokens preserved between adjacent chunks for context continuity

### Hybrid search

Two-phase retrieval:
1. **BM25 pre-filter** — full-text search across all chunks to find candidate pages (fast, keyword-relevant)
2. **KNN rerank** — vector similarity search only on chunks from candidate pages (precision, semantically relevant)
3. **Context expansion** — for each top result, adjacent chunks (before and after) are included for richer context

---

## Comparison to alternatives

| | VibeEye | Playwright | Headless Chrome | curl + htmlq |
|---|---|---|---|---|
| **Language** | Rust | Node.js/Python | — | Shell |
| **Browser engine** | Servo (Rust) | Chromium (Blink) | Blink | None |
| **JS rendering** | ✅ Servo + SpiderMonkey | ✅ Full Chromium | ✅ Full Chromium | ❌ |
| **SPA auto-detection** | ✅ Built-in | ❌ Manual | ❌ Manual | ❌ |
| **Crawl + extract** | ✅ Built-in | ❌ Manual setup | ❌ Manual setup | ❌ |
| **Vector storage** | ✅ SurrealDB | ❌ | ❌ | ❌ |
| **Hybrid search** | ✅ BM25 + vector | ❌ | ❌ | ❌ |
| **MCP server** | ✅ Built-in | ❌ | ❌ | ❌ |
| **No Chrome dep** | ✅ | ❌ | ❌ | ✅ |
| **No DB mode** | ✅ Standalone | ✅ Standalone | ✅ Standalone | ✅ |
| **Agent-ready** | ✅ extract + MCP | ⚠️ Requires wrapper | ⚠️ Requires wrapper | ⚠️ Requires wrapper |

---

## Known limitations

- **Authentication:** No built-in support for login forms, cookies, or session-based auth. Only public pages.
- **Servo alpha quality:** Servo 0.1.0 is pre-1.0. Some sites may render differently than Chromium. SpiderMonkey has a global mutex destructor that can segfault on normal process exit — VibeEye mitigates this with `libc::_exit(0)`.
- **SPA edge cases:** Some client-side routing patterns (Astro partial hydration, heavy WASM) may not settle properly. Known issue: SurrealDB docs crawl misses sub-pages.
- **Anti-bot protection:** Sites like GitHub may return JS-gated or rate-limited content. 0 chunks from GitHub crawls is expected.
- **Memory:** Each browser session loads a full Servo instance. OOM on long crawls is mitigated by dropping the WebView between pages.
- **Embedding concurrency:** FLM on AMD NPU is single-threaded — set `embed_concurrency = 1` to avoid lockups.

### Known issues

See [`known_issues.md`](known_issues.md) for the current list, including:
- SurrealDB docs crawl misses sub-pages (suspected: Astro SPA rendering)
- crates.io/surrealdb.com blog extraction may return incomplete content
- Link extraction from some SPA frameworks

---

## Development

### Building

```bash
# Minimal build
cargo build

# Full build with all features
cargo build --features "surrealdb embeddings"

# Release build
cargo build --release --features "surrealdb embeddings"
```

### Testing

Tests use a stub backend (no real Servo engine) activated by the `VIBEYE_TEST_STUB` environment variable:

```bash
# All tests
cargo test --features "surrealdb embeddings"

# Specific test suite
cargo test -p vibeeye-app --test crawl_e2e --features surrealdb
```

### Code quality gates

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test --features "surrealdb embeddings"
```

### Project principles

- **No Chromium** — Servo only. No Playwright, Puppeteer, or WebKit dependencies.
- **Unified Tool pattern** — Every CLI command implements the `TypedTool` trait for capability reflection and execution.
- **Headless first** — Zero X11/Wayland dependencies.
- **Thin interface** — Browser logic and interface handlers are strictly separated.

---

## License

Licensed under either of:

- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)

at your option.

---

## Project layout

```
vibeeye/
├── Cargo.toml                    # Workspace root
├── README.md                     # This file
├── ARCHITECTURE.md               # System architecture, pipeline flows, contracts
├── crawl.example.toml            # Example config with all options documented
├── known_issues.md               # Known bugs and investigation notes
├── rules.md                      # Development guidelines and principles
│
├── crates/
│   ├── vibeeye-core/             # Domain types (Viewport, ContentFormat, VibeError)
│   ├── vibeeye-app/              # Core library: browser, crawl, extraction, DB, embeddings
│   │   └── src/
│   │       ├── browser/          # Servo engine wrapper + BrowserSession
│   │       ├── crawl/            # BFS engine, link extraction, output sinks, validation
│   │       ├── config/           # Hierarchical TOML config loader
│   │       ├── db/               # SurrealDB client, migrations, CRUD, import/export
│   │       ├── extraction/       # Content cleaning and format conversion
│   │       ├── chunk/            # Heading-aware semantic chunking
│   │       ├── embed/            # OpenAI-compatible embedding provider
│   │       └── tools/            # Browse/Snapshot/Extract tool implementations
│   ├── vibeeye-cli/              # CLI binary with clap + help-tree introspection
│   └── vibeeye-mcp/              # MCP server (13 tools over stdio)
│
├── tests/                        # Integration tests
├── output_testing/               # Crawl output test data
└── surreal_server/               # SurrealDB server config
```
