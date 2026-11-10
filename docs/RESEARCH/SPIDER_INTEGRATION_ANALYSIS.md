# Spider Integration Analysis

## Executive Summary

**Recommendation: Use Spider for crawling, keep VibeEye for browser features**

Spider is a mature, feature-rich crawler with 100+ feature flags. VibeEye's unique value is the Servo browser engine integration. We should leverage Spider for what it does best (crawling) and focus VibeEye on what it does best (Servo-based extraction).

---

## Spider Capabilities

### Architecture
- **Streaming HTML parsing** via `lol_html` (handles multi-GB documents)
- **Feature-flag driven** - 100+ optional features for fine-grained control
- **Multi-backend**: HTTP, Chrome CDP, WebDriver (including Servo!)
- **Cloud-ready**: Spider Cloud API for anti-bot bypass and proxies

### Link Discovery Features
| Feature | Spider | VibeEye |
|---------|--------|---------|
| `<a href>` extraction | ✅ | ✅ |
| `<link>` extraction | ✅ | ❌ |
| `<area>` extraction | ✅ | ❌ |
| `<iframe>` extraction | ✅ | ❌ |
| `<meta refresh>` | ✅ | ❌ |
| SSG manifest parsing | ✅ (Next.js, Nuxt, Gatsby) | ❌ |
| Sitemap.xml parsing | ✅ | ✅ (pre-seed only) |
| Robots.txt handling | ✅ | ✅ |

### Performance Features
- **io_uring** support (Linux async I/O)
- **Zero-copy parsing**
- **NUMA-aware** thread pinning
- **mmap-backed bloom filter** for URL deduplication
- **Adaptive concurrency** (AIMD-based)
- **Rate limiting** per-domain
- **Request coalescing** (dedup in-flight requests)

### Browser Backends
| Backend | Use Case |
|---------|----------|
| HTTP (reqwest) | Fast static crawling |
| Chrome CDP | JavaScript rendering, anti-bot |
| WebDriver | Browser automation |
| **Servo** (via `servo` feature!) | Memory-safe browser engine |

### Integration Points
```rust
// Spider already supports Servo as a backend!
spider = { version = "2", features = ["servo"] }
```

---

## Integration Options

### Option 1: Replace VibeEye Crawler with Spider (Recommended)

**What changes:**
- Use `spider` crate for crawling and link discovery
- Keep VibeEye's Servo integration for extraction
- Remove VibeEye's crawl implementation entirely

**Code flow:**
```
Spider.crawl() → discover URLs → VibeEye.extract(url) → Servo rendering → SurrealDB
```

**Pros:**
- Instant access to all Spider features (SSG, sitemap, etc.)
- Battle-tested crawler with 100+ features
- Active maintenance and community
- Servo backend already supported in Spider
- Removes ~1500 lines of crawl code from VibeEye

**Cons:**
- Additional dependency
- Need to adapt Spider's page model to VibeEye's extraction
- Two separate crawl paths (Spider for discovery, VibeEye for extraction)

**Implementation:**
```rust
// crates/vibeeye-app/src/crawl/spider_backend.rs
use spider::website::Website;
use spider::page::Page;

pub async fn crawl_with_spider(opts: CrawlOptions) -> Result<Vec<CrawlResult>> {
    let mut website = Website::new(&opts.url)
        .with_depth(opts.max_depth.unwrap_or(25))
        .with_limit(opts.concurrency)
        .with_delay(opts.delay_ms)
        .with_respect_robots_txt(opts.respect_robots)
        .build()
        .unwrap();
    
    // Use Spider for discovery
    website.crawl().await;
    
    // Use VibeEye for extraction with Servo
    let results = Vec::new();
    for url in website.get_links() {
        let extracted = vibeeye_extract(&url, &opts).await?;
        results.push(extracted);
    }
    
    Ok(results)
}
```

### Option 2: Spider as Optional Backend

**What changes:**
- Add Spider as optional dependency with feature flag
- Default to VibeEye crawler, opt-in to Spider for heavy workloads

**Cargo.toml:**
```toml
[features]
default = ["surrealdb"]
spider-backend = ["dep:spider"]
```

**Pros:**
- Backwards compatible
- Users can choose their backend
- Gradual migration path

**Cons:**
- Maintaining two crawl implementations
- Feature complexity
- Doesn't solve the core issue (hard-coded limits)

### Option 3: Hybrid - Spider for Discovery Only

**What changes:**
- Use Spider for URL discovery phase
- Use VibeEye's Servo for actual page rendering

**Code flow:**
```
Spider.discover_links() → URL queue → VibeEye.render_with_servo() → Extract → Store
```

**Pros:**
- Best of both worlds
- Spider's superior link discovery
- VibeEye's Servo-based extraction

**Cons:**
- More complex integration
- Two passes over the site (discovery + extraction)
- Need to coordinate state between systems

---

## Spider Features Worth Adopting

If we don't integrate Spider, these features should be added to VibeEye:

### High Priority
1. **Remove hard-coded limits** - Spider has no URL cap
2. **SSG manifest parsing** - Next.js, Nuxt, Gatsby route discovery
3. **Additional link elements** - `<link>`, `<area>`, `<iframe>`
4. **Streaming HTML parser** - `lol_html` instead of `scraper`

### Medium Priority
5. **Case-insensitive URL dedup** - `CaseInsensitiveString`
6. **Bloom filter dedup** - mmap-backed for large crawls
7. **Adaptive concurrency** - Auto-tune based on response times
8. **Request coalescing** - Dedup concurrent requests to same URL

### Low Priority
9. **Anti-bot detection** - 25+ patterns
10. **Chrome CDP support** - Already have Servo
11. **WARC output** - Web archive format
12. **Spider Cloud API** - Managed crawling service

---

## Recommended Path Forward

### Phase 1: Quick Fix (Immediate)
Remove hard-coded `MAX_DISCOVERED_URLS` limit in VibeEye:
```rust
// Before
const MAX_DISCOVERED_URLS: usize = 5000;

// After
pub struct CrawlOptions {
    pub max_discovered_urls: Option<usize>, // None = unlimited
}
```

### Phase 2: Spider Integration (Short-term)
Add Spider as optional crawl backend:
```toml
[dependencies]
spider = { version = "2", optional = true, features = ["basic"] }

[features]
spider-backend = ["dep:spider"]
```

### Phase 3: Native Servo Backend for Spider (Long-term)
Spider already has a `servo` feature flag. Contribute to Spider's Servo integration to make it production-ready, benefiting both projects.

---

## Dependency Comparison

| Dependency | VibeEye | Spider |
|------------|---------|--------|
| Browser engine | Servo 0.7.0 | Chrome CDP / Servo |
| HTML parser | scraper | lol_html (streaming) |
| URL handling | url | url + case_insensitive_string |
| Async runtime | tokio | tokio |
| HTTP client | (via Servo) | reqwest |
| Deduplication | HashSet | bloom filter / HashSet |
| Feature flags | 3 | 100+ |

---

## Code Impact

### Lines of Code
| Component | VibeEye | Spider |
|-----------|---------|--------|
| Crawl orchestration | ~500 LOC | Mature implementation |
| Link extraction | ~150 LOC | ~1000 LOC (streaming) |
| Robots.txt | ~100 LOC | ~200 LOC |
| Sitemap | ~200 LOC | ~300 LOC |
| URL normalization | ~100 LOC | ~500 LOC |

### If We Integrate Spider
- **Remove**: ~1000 LOC from VibeEye crawl implementation
- **Add**: ~200 LOC for Spider integration layer
- **Net**: -800 LOC, +all Spider features

---

## Decision Matrix

| Criteria | Keep VibeEye Crawler | Integrate Spider |
|----------|---------------------|------------------|
| Development effort | Medium (fix limits + add features) | Low (use existing) |
| Feature completeness | Low (missing SSG, etc.) | High (100+ features) |
| Maintenance burden | High (own implementation) | Low (upstream) |
| Servo integration | Already have | Need to configure |
| Flexibility | Full control | Constrained by Spider API |
| Performance | Good | Excellent (io_uring, etc.) |
| Binary size | Smaller | Larger |

---

## Final Recommendation

**Use Spider for crawling, VibeEye for extraction:**

1. Add `spider` as dependency with `basic` features
2. Replace VibeEye's crawl implementation with Spider
3. Keep VibeEye's Servo integration for content extraction
4. Configure Spider to use Servo backend for JS-heavy sites
5. Focus VibeEye development on extraction quality, embeddings, search

This gives us the best of both worlds:
- Spider's battle-tested crawling with 100+ features
- VibeEye's unique Servo-based extraction
- Reduced maintenance burden
- Faster feature development
