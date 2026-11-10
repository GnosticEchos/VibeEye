# Crawl Improvements Plan

## Problem Summary

1. **Hard-coded `MAX_DISCOVERED_URLS = 5000`** stops URL discovery prematurely
2. **`--max-pages 0` for unlimited** is unintuitive (should be omitted or -1)
3. **Missing URL discovery features** that Spider has:
   - SSG/SPA manifest parsing (Next.js, Gatsby, Nuxt)
   - Additional link element types (`<link>`, `<area>`, `<iframe>`, etc.)
   - Sitemap.xml integration during crawl (not just pre-seed)

---

## Phase 1: Remove Hard-Coded Limits

### 1.1 Make `MAX_DISCOVERED_URLS` configurable

**Current:** `const MAX_DISCOVERED_URLS: usize = 5000;`

**Proposed:**
```rust
pub struct CrawlOptions {
    // ... existing fields ...
    
    /// Maximum URLs to discover before stopping (0 = unlimited, default: 0)
    pub max_discovered_urls: usize,
}
```

**TOML config:**
```toml
[global]
max_discovered_urls = 0  # unlimited by default
```

**CLI flag:**
```
--max-discovered-urls <N>  Maximum URLs to discover (0 = unlimited)
```

### 1.2 Fix `max_pages` semantics

**Current:** `0` means unlimited (confusing)

**Proposed:** 
- `None` or omitted means unlimited
- `Some(n)` means limit to n pages
- CLI: omit flag for unlimited, `--max-pages 100` for limit

```rust
pub struct CrawlOptions {
    /// Maximum pages to crawl (None = unlimited)
    pub max_pages: Option<usize>,
}
```

---

## Phase 2: Enhanced Link Discovery

### 2.1 Add additional link element types

**Current:** Only `<a href>` elements

**Proposed:** Extract from additional elements:

| Element | Attribute | Use Case |
|---------|-----------|----------|
| `<a href>` | href | Primary navigation |
| `<link href>` | href | Stylesheets, canonical, alternate, prev/next |
| `<area href>` | href | Image map links |
| `<iframe src>` | src | Embedded content |
| `<meta refresh>` | content | Meta refresh redirects |

**Implementation:**
```rust
/// Link extraction configuration
#[derive(Debug, Clone)]
pub struct LinkExtractionConfig {
    /// Extract from <a href> elements (default: true)
    pub extract_anchor_links: bool,
    /// Extract from <link href> elements (default: false)
    pub extract_link_elements: bool,
    /// Extract from <area href> elements (default: false)
    pub extract_area_links: bool,
    /// Extract from <iframe src> elements (default: false)
    pub extract_iframe_sources: bool,
    /// Follow meta refresh redirects (default: true)
    pub follow_meta_refresh: bool,
}
```

### 2.2 SSG/SPA Manifest Discovery

Spider detects Single-Site-Generator build manifests to discover routes that aren't in HTML.

**Supported frameworks:**

| Framework | Manifest | Detection Pattern |
|-----------|----------|-------------------|
| Next.js | `__NEXT_DATA__` | `<script id="__NEXT_DATA__">` |
| Nuxt | `__NUXT_DATA__` | `<script id="__NUXT_DATA__">` |
| Gatsby | Chunk mapping | `window.___chunkMapping` |
| React SSR | Embedded data | `window.__REACT_DEVTOOLS_GLOBAL_HOOK__` |

**Implementation approach:**

```rust
/// SSG manifest discovery configuration
#[derive(Debug, Clone)]
pub struct SsgDiscoveryConfig {
    /// Enable SSG manifest parsing (default: false)
    pub enabled: bool,
    /// Frameworks to detect (None = all)
    pub frameworks: Option<Vec<SsgFramework>>,
}

pub enum SsgFramework {
    NextJs,
    Nuxt,
    Gatsby,
    ReactSsr,
}
```

**Detection flow:**
1. After page load, check for SSG-specific script tags
2. Parse JSON manifest
3. Extract dynamic routes from `pages`, `dynamicRoutes`, `staticPaths`
4. Add discovered URLs to queue

### 2.3 Enhanced Sitemap Integration

**Current:** Pre-seed queue from sitemap.xml (optional)

**Proposed:** 
- Parse sitemap.xml during crawl to discover new URLs
- Support sitemap index files (sitemap of sitemaps)
- Parse `lastmod`, `changefreq`, `priority` for smarter crawling

```rust
pub struct SitemapConfig {
    /// Enable sitemap parsing (default: true when --sitemap flag used)
    pub enabled: bool,
    /// Crawl sitemaps discovered during crawl (not just pre-seed)
    pub discover_sitemaps: bool,
    /// Respect lastmod timestamps (skip if unchanged)
    pub respect_lastmod: bool,
    /// Minimum priority to include (0.0 - 1.0)
    pub min_priority: Option<f32>,
}
```

---

## Phase 3: URL Normalization Enhancements

### 3.1 Case-insensitive URL comparison

**Current:** Case-sensitive URL deduplication

**Proposed:** Optional case-insensitive normalization

```rust
pub struct UrlNormalizationConfig {
    /// Lowercase scheme and host (default: true)
    pub lowercase_host: bool,
    /// Lowercase path (default: false)
    pub lowercase_path: bool,
    /// Sort query parameters (default: false)
    pub sort_query_params: bool,
    /// Remove trailing slash (default: false)
    pub remove_trailing_slash: bool,
}
```

### 3.2 Query parameter handling

**Current:** Query params preserved as-is

**Proposed:** Configurable query parameter handling

```rust
pub struct QueryParamConfig {
    /// Remove these query params (e.g., tracking params)
    pub remove_params: Vec<String>,
    /// Keep only these query params (whitelist)
    pub keep_params: Option<Vec<String>>,
    /// Sort query parameters for deduplication
    pub sort_params: bool,
}
```

---

## Phase 4: Smart Defaults

### 4.1 Recommended defaults

| Setting | Current | Proposed |
|---------|---------|----------|
| `max_discovered_urls` | 5000 (hard-coded) | 0 (unlimited) |
| `max_pages` | 0 = unlimited | None = unlimited |
| `max_depth` | 2 (hard-coded) | None = unlimited |
| `extract_link_elements` | false | false (opt-in) |
| `ssg_discovery` | false | false (opt-in) |
| `sitemap` | false | true (auto-detect) |

### 4.2 TOML configuration example

```toml
[global]
# Limits
max_discovered_urls = 0      # unlimited
max_pages = 0                # unlimited  
max_depth = 10               # reasonable default

# Link discovery
extract_anchor_links = true
extract_link_elements = false
extract_area_links = false
extract_iframe_sources = false

# SSG/SPA discovery
ssg_discovery = true
ssg_frameworks = ["nextjs", "nuxt"]

# Sitemap
sitemap = true
discover_sitemaps = true

# URL normalization
lowercase_host = true
sort_query_params = false

# Domain-specific overrides
[domain."docs.example.com"]
max_depth = 15
extract_link_elements = true
```

---

## Implementation Priority

1. **High priority** - Remove hard-coded limits (Phase 1)
2. **Medium priority** - Additional link elements (Phase 2.1)
3. **Medium priority** - Enhanced sitemap (Phase 2.3)
4. **Low priority** - SSG manifest discovery (Phase 2.2)
5. **Low priority** - URL normalization options (Phase 3)

---

## API Changes

### CrawlOptions additions

```rust
pub struct CrawlOptions {
    // Existing fields...
    pub url: String,
    pub max_depth: Option<u32>,         // Changed from u32 to Option<u32>
    pub max_pages: Option<usize>,       // Changed from usize to Option<usize>
    
    // New fields
    pub max_discovered_urls: Option<usize>,
    pub link_extraction: LinkExtractionConfig,
    pub ssg_discovery: SsgDiscoveryConfig,
    pub sitemap_config: SitemapConfig,
    pub url_normalization: UrlNormalizationConfig,
}
```

### CLI changes

```bash
# Current (confusing)
vibe-eye crawl https://example.com --max-pages 0 --max-depth 0

# Proposed (clear)
vibe-eye crawl https://example.com  # unlimited by default
vibe-eye crawl https://example.com --max-pages 100 --max-depth 5

# New options
vibe-eye crawl https://example.com --extract-link-elements --ssg-discovery
vibe-eye crawl https://example.com --max-discovered-urls 10000
```
