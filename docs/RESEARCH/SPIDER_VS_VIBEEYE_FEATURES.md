# Spider vs VibeEye Feature Comparison

## Spider's 100+ Feature Flags

### Core Crawl Features
| Feature | Description |
|---------|-------------|
| `basic` | HTTP crawling with cookies, encoding, rate limiting, caching |
| `sync` | Tokio sync primitives |
| `fs` | Filesystem operations |
| `time` | Time-based features |
| `cmd` | Process spawning |
| `control` | Flow control |

### Browser Backends
| Feature | Description |
|---------|-------------|
| `chrome` | Headless Chrome via CDP |
| `chrome_headed` | Visible Chrome window |
| `chrome_stealth` | Anti-fingerprinting |
| `chrome_screenshot` | Page screenshots |
| `chrome_intercept` | Network blocking |
| `chrome_cpu` | GPU disabled |
| `chrome_headless_new` | Uses `--headless=new` |
| `chrome_simd` | SIMD acceleration |
| `chrome_remote_cache` | Remote cache via Spider Cloud |
| `chrome_tls_connection` | TLS connection reuse |
| `servo` | **Servo browser engine** (via WebDriver) |
| `webdriver` | Generic WebDriver support |
| `webdriver_chrome` | Chrome via WebDriver |
| `webdriver_firefox` | Firefox via WebDriver |
| `webdriver_edge` | Edge via WebDriver |
| `webdriver_stealth` | Stealth WebDriver |
| `webdriver_screenshot` | WebDriver screenshots |
| `smart` | Auto HTTP → Chrome upgrade |

### Performance Features
| Feature | Description |
|---------|-------------|
| `io_uring` | Linux async I/O |
| `tcp_fastopen` | TCP Fast Open |
| `splice` | Zero-copy splice |
| `numa` | NUMA-aware thread pinning |
| `zero_copy` | Zero-copy parsing |
| `simd` | SIMD JSON parsing |
| `bloom` | mmap-backed bloom filter for URL dedup |
| `hedge` | Hedged requests for slow crawls |
| `adaptive_concurrency` | AIMD-based auto-tuning |
| `priority_frontier` | Prioritized URL queue |
| `rate_limit` | Per-domain rate limiting |
| `request_coalesce` | Dedup concurrent in-flight requests |
| `auto_throttle` | Adaptive request timing |
| `etag_cache` | ETag-based caching |

### Caching Features
| Feature | Description |
|---------|-------------|
| `cache` | Disk-backed HTTP cache |
| `cache_mem` | In-memory HTTP cache |
| `cache_request` | Request-level caching |
| `cache_chrome_hybrid` | Chrome + HTTP hybrid cache |
| `cache_chrome_hybrid_mem` | In-memory hybrid cache |
| `cache_openai` | OpenAI response cache |
| `cache_gemini` | Gemini response cache |

### Networking Features
| Feature | Description |
|---------|-------------|
| `socks` | SOCKS proxy support |
| `dns_cache` | DNS caching |
| `reqwest_rustls_tls` | Rustls TLS |
| `reqwest_native_tls` | Native TLS |
| `reqwest_hickory_dns` | Hickory DNS |
| `wreq` | Custom HTTP client |
| `headers` | HTTP header handling |
| `remote_addr` | Remote address extraction |
| `real_browser` | Browser-like request patterns |
| `spoof` | User agent spoofing |

### Content Processing
| Feature | Description |
|---------|-------------|
| `encoding` | Character encoding detection |
| `regex` | Regex-based filtering |
| `glob` | Glob pattern matching |
| `sitemap` | XML sitemap parsing |
| `full_resources` | Download all resources |
| `serde` | Serialization support |
| `flexbuffers` | FlexBuffers serialization |
| `llm_json` | LLM JSON extraction |

### Storage Features
| Feature | Description |
|---------|-------------|
| `disk` | SQLite-based persistence |
| `disk_native_tls` | SQLite with native TLS |
| `disk_aws` | SQLite with AWS LC |
| `string_interner_buffer_backend` | String interning (buffer) |
| `string_interner_string_backend` | String interning (string) |
| `string_interner_bucket_backend` | String interning (bucket) |

### LLM/AI Features
| Feature | Description |
|---------|-------------|
| `openai` | OpenAI integration |
| `openai_slim_fit` | Lightweight OpenAI |
| `gemini` | Google Gemini integration |
| `agent` | Spider Agent framework |
| `agent_openai` | Agent with OpenAI |
| `agent_chrome` | Agent with Chrome |
| `agent_webdriver` | Agent with WebDriver |
| `agent_skills` | Agent skills system |
| `agent_fs` | Agent filesystem access |
| `agent_search_*` | Agent search integrations |
| `agent_full` | Full agent capabilities |

### Search Features
| Feature | Description |
|---------|-------------|
| `search` | Search integration base |
| `search_serper` | Serper API |
| `search_brave` | Brave Search API |
| `search_bing` | Bing Search API |
| `search_tavily` | Tavily Search API |

### Ad Blocking
| Feature | Description |
|---------|-------------|
| `adblock` | Ad blocking |
| `adblock_easylist` | EasyList filter lists |
| `firewall` | Network firewall |

### Cloud Features
| Feature | Description |
|---------|-------------|
| `spider_cloud` | Spider Cloud API |
| `parallel_backends` | Multiple backends racing |
| `parallel_backends_full` | All backends enabled |
| `decentralized` | Decentralized crawling |
| `decentralized_headers` | Decentralized headers |

### Scheduling
| Feature | Description |
|---------|-------------|
| `cron` | Cron-based scheduling |
| `balance` | Load balancing |

### Monitoring
| Feature | Description |
|---------|-------------|
| `tracing` | Tracing/logging |
| `page_error_status_details` | Detailed error reporting |
| `extra_information` | Extra page metadata |
| `warc` | WARC archive output |

### TLS Variants
| Feature | Description |
|---------|-------------|
| `reqwest_native_tls` | Native TLS backend |
| `reqwest_native_tls_alpn` | Native TLS with ALPN |
| `reqwest_native_tls_vendored` | Vendored native TLS |
| `reqwest_native_tls_manual_roots` | Manual root certificates |
| `reqwest_native_tls_native_roots` | Native root certificates |
| `reqwest_native_tls_webpki_roots` | WebPKI root certificates |
| `reqwest_rustls_tls` | Rustls backend |

### WebDriver Variants
| Feature | Description |
|---------|-------------|
| `webdriver` | Base WebDriver support |
| `webdriver_headed` | Visible browser |
| `webdriver_stealth` | Stealth mode |
| `webdriver_chrome` | Chrome browser |
| `webdriver_firefox` | Firefox browser |
| `webdriver_edge` | Edge browser |
| `webdriver_screenshot` | Screenshots |

### Chrome Variants
| Feature | Description |
|---------|-------------|
| `chrome` | Base Chrome support |
| `chrome_headed` | Visible Chrome |
| `chrome_cpu` | CPU-only mode |
| `chrome_stealth` | Stealth mode |
| `chrome_screenshot` | Screenshots |
| `chrome_intercept` | Network interception |
| `chrome_headless_new` | New headless mode |
| `chrome_simd` | SIMD acceleration |
| `chrome_store_page` | Page storage |
| `chrome_remote_cache` | Remote caching |
| `chrome_remote_cache_disk` | Disk caching |
| `chrome_remote_cache_mem` | Memory caching |
| `chrome_tls_connection` | TLS connection reuse |
| `chrome_serde_stacker` | Serde stacker |

### Agent Variants
| Feature | Description |
|---------|-------------|
| `agent` | Base agent |
| `agent_openai` | OpenAI agent |
| `agent_chrome` | Chrome agent |
| `agent_webdriver` | WebDriver agent |
| `agent_skills` | Skills system |
| `agent_skills_s3` | S3 skills |
| `agent_fs` | Filesystem access |
| `agent_search_serper` | Serper search |
| `agent_search_brave` | Brave search |
| `agent_search_bing` | Bing search |
| `agent_search_tavily` | Tavily search |
| `agent_full` | Full capabilities |

---

## VibeEye Crawl Command Functionality

### Data Structures

**CrawlResult** (the contract between crawl and persistence):
```rust
pub struct CrawlResult {
    pub url: String,
    pub depth: u32,
    pub content: String,           // Distilled markdown/html/text
    pub format: String,
    pub title: Option<String>,
    pub links_found: usize,
    pub error: Option<String>,
    pub http_status: Option<u16>,
    pub local_storage: Option<HashMap<String, String>>,
    pub meta: Option<serde_json::Value>,  // JSON-LD + Open Graph
}
```

### Crawl Pipeline

```
1. Queue Initialization
   ├── Sitemap pre-seed (optional)
   ├── Seed URLs
   └── visited: HashSet<String> for dedup

2. BFS Loop
   ├── Pop URL from queue
   ├── Check max_pages, robots.txt
   ├── Acquire semaphore (concurrency)
   ├── Apply rate limiting
   └── crawl_one_page()

3. crawl_one_page()
   ├── fetch_with_session() → PageCapture
   │   └── BrowserSession::navigate() with Servo
   ├── Page validation (4xx, 5xx, soft 404, noindex)
   ├── Link extraction
   │   ├── Raw HTML links (scraper crate)
   │   ├── Live DOM links (via JS eval)
   │   └── SPA detection (DOM > 2x raw)
   ├── Structured metadata extraction
   │   ├── JSON-LD scripts
   │   └── Open Graph meta tags
   └── Content extraction (markdown/html/text)

4. Output Emission (batch of 50)
   └── CrawlOutput::emit_results(&[CrawlResult])
```

### CrawlOutput Implementations

| Output | Description |
|--------|-------------|
| `StdoutOutput` | JSON Lines to stdout |
| `UrlListOutput` | URLs to text file |
| `DirectoryOutput` | Files + manifest.json |
| `SurrealOutput` | **SurrealDB persistence + embeddings** |

### SurrealDB Schema

**Table: `page`**
```sql
CREATE page {
    group: string,
    url: string,
    title: string,
    content: string,        -- BM25 indexed
    depth: int,
    format: string,
    crawled_at: datetime,
    meta: option<object>    -- JSON-LD + Open Graph
}
```

**Table: `discovered`** (relation)
```sql
CREATE discovered {
    group: string,
    from_page: record<page>,
    to_page: record<page>,
    anchor_text: option<string>
}
```

**Table: `chunk`** (embeddings)
```sql
CREATE chunk {
    group: string,
    page: record<page>,
    chunk_index: int,
    chunk_text: string,
    heading_path: array<string>,
    embedding: array<float>,  -- HNSW indexed
    model: string,
    dimensions: int
}
```

### Embedding Pipeline

```
SurrealOutput::emit_results()
    │
    ├── emit_page() → insert into page table
    │
    └── embed_and_index() [if embed_config set]
        │
        ├── Chunker::chunk()
        │   └── Split by headings → paragraphs → sentences → words
        │
        ├── probe_dimension()
        │   └── Embed first 50 chunks, detect dimension
        │   └── ensure_hnsw_index() → create vector index
        │
        ├── embed_batches()
        │   ├── Batch 100 chunks per request
        │   ├── Concurrent embedding (default 4)
        │   └── HTTP to Ollama/OpenAI/etc.
        │
        └── insert_chunk_records()
            ├── Delete old chunks for page
            └── Insert new chunks (max 500 per batch)
```

### Search Implementations

| Search | Query | Index |
|--------|-------|-------|
| BM25 | `content @@ $query` | Full-text analyzer |
| Vector | `vector::similarity::cosine()` | HNSW |
| Hybrid | BM25 pre-filter + Vector rerank | Both |

---

## Critical Coupling Points

### If We Replace Crawl Implementation, We Must Preserve:

1. **CrawlResult structure** — Used by all CrawlOutput implementations
2. **CrawlOutput trait** — Embedding generation happens inside SurrealOutput
3. **Markdown content** — Chunker splits by headings
4. **Link discovery** — `discovered` table requires anchor text
5. **Group concept** — All records scoped by group

### What Would NOT Break:
- Embedding generation (self-contained in SurrealOutput)
- DB schema (independent of crawl)
- Search queries (only depend on page/chunk tables)

### What WOULD Break:
- Markdown quality → chunking quality degrades
- Missing `meta` field → structured metadata lost
- Link discovery changes → graph edges incomplete

---

## Integration Strategy

### Option A: Spider for Discovery, VibeEye for Extraction

```
Spider.crawl() → discover URLs
      ↓
VibeEye.render_with_servo() → extract content
      ↓
SurrealOutput.emit_results() → persist + embed
```

**Pros:**
- Spider's superior URL discovery
- VibeEye's Servo-based extraction
- Embedding pipeline preserved

**Cons:**
- Two passes over site (discovery + extraction)
- Need to coordinate state

### Option B: Spider as URL Provider

```
Spider.get_links() → URL list
      ↓
VibeEye batch crawl → CrawlResult batch
      ↓
SurrealOutput.emit_results() → persist + embed
```

**Pros:**
- Single pass extraction
- Minimal changes to VibeEye
- Embedding pipeline untouched

**Cons:**
- Still running VibeEye's BFS logic
- Not leveraging Spider's rendering

### Option C: Spider with Custom Page Handler

```rust
// Implement Spider's PageHandler trait
impl PageHandler for VibeEyeHandler {
    async fn handle_page(&self, page: &Page) {
        // Use Servo for extraction
        let content = self.vibeeye.extract(&page.url).await;
        
        // Emit to SurrealOutput
        self.output.emit_results(&[content]).await;
    }
}
```

**Pros:**
- Leverages Spider's full pipeline
- Custom extraction with Servo
- Maintains embedding pipeline

**Cons:**
- Need to implement Spider traits
- More complex integration

---

## Recommended Path

**Phase 1:** Fix VibeEye's hard-coded limits (immediate)
**Phase 2:** Add Spider as URL discovery enhancement
**Phase 3:** Integrate Spider's Servo backend for full replacement

This preserves the embedding pipeline while gaining Spider's URL discovery.
