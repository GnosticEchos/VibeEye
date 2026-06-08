//! BFS web crawler built on VibeEye's Servo browser engine.
//!
//! Reuses `navigate_and_capture` for page fetching and the extraction
//! pipeline for content distillation.

use crate::Result;
use crate::browser::BrowserSession;
use crate::extraction;
use crate::tools::common::PageCapture;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::fs;
use tokio::sync::Semaphore;
use tracing::{debug, info, trace, warn};
use url::Url;
use vibeeye_core::ContentFormat;

mod links;
pub mod output;
pub mod robots;
pub mod sitemap;
pub mod validator;

pub use links::{extract_links, is_same_origin, normalize_url};

/// Maximum discovered URLs before we stop enqueuing new ones.
const MAX_DISCOVERED_URLS: usize = 5000;

/// Options that drive a crawl job.
#[derive(Debug, Clone)]
pub struct CrawlOptions {
    pub url: String,
    pub seed_urls: Vec<String>,
    pub max_depth: u32,
    pub max_pages: usize,
    pub format: ContentFormat,
    pub respect_robots: bool,
    pub requests_per_second: f64,
    pub concurrency: usize,
    pub same_origin: bool,
    pub timeout_secs: u64,
    pub use_sitemap: bool,
    pub settle_ms: u64,
    pub outputs: Vec<std::sync::Arc<dyn output::CrawlOutput>>,
}

/// Result emitted for each successfully crawled page.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlResult {
    pub url: String,
    pub depth: u32,
    pub content: String,
    pub format: String,
    pub title: Option<String>,
    pub links_found: usize,
    pub error: Option<String>,
    pub http_status: Option<u16>,
    pub local_storage: Option<HashMap<String, String>>,
    pub meta: Option<serde_json::Value>,
}

/// Run a BFS crawl starting from `opts.url`.
///
/// Results are either streamed as JSON Lines to stdout (when
/// `output_dir` is `None`) or written as individual files into
/// `output_dir`.
pub async fn run(opts: CrawlOptions) -> Result<()> {
    let base_url = Url::parse(&opts.url)
        .map_err(|e| crate::AppError::InvalidInput(format!("invalid seed URL: {e}")))?;

    let origin = format!(
        "{}://{}",
        base_url.scheme(),
        base_url.host_str().unwrap_or("")
    );

    let (mut queue, mut visited) = build_queue(&opts, &origin).await;
    let robots = load_robots(&opts, &origin).await;
    let semaphore = Arc::new(Semaphore::new(opts.concurrency.max(1)));
    let mut host_last_request: HashMap<String, Instant> = HashMap::new();
    let mut results: Vec<CrawlResult> = Vec::new();
    let mut total_crawled = 0usize;
    let mut total_successful = 0usize;
    let mut total_errors = 0usize;

    let mut session = BrowserSession::new().map_err(|e| crate::AppError::Browser(e.to_string()))?;

    const EMIT_BATCH_SIZE: usize = 50;
    let emit_semaphore = Arc::new(Semaphore::new(2));
    let mut emit_handles: Vec<tokio::task::JoinHandle<()>> = Vec::new();

    while let Some((url, depth)) = queue.pop_front() {
        if opts.max_pages > 0 && total_crawled >= opts.max_pages {
            info!(
                "reached max_pages limit ({}) — stopping crawl",
                opts.max_pages
            );
            break;
        }
        if !should_crawl_page(depth, &url, &robots, &opts) {
            continue;
        }

        let permit = acquire_permit(semaphore.clone()).await?;
        apply_rate_limit(&url, &opts, &mut host_last_request).await;

        let result = crawl_one_page(
            &url,
            depth,
            &base_url,
            &opts,
            &mut visited,
            &mut queue,
            &mut session,
        )
        .await;
        drop(permit);
        total_crawled += 1;
        if result.error.is_some() {
            total_errors += 1;
        } else {
            total_successful += 1;
        }
        results.push(result);
        info!(
            url = %url,
            depth = depth,
            queue_size = queue.len(),
            completed = total_crawled,
            "crawled page"
        );

        if results.len() >= EMIT_BATCH_SIZE {
            let batch = std::mem::take(&mut results);
            let opts = opts.clone();
            let sem = emit_semaphore.clone();
            let handle = tokio::spawn(async move {
                let _permit = sem.acquire().await;
                if let Err(e) = emit_results(&batch, &opts).await {
                    tracing::error!(error = %e, "background emit failed");
                }
            });
            emit_handles.push(handle);
        }
    }

    // Final batch
    if !results.is_empty() {
        emit_results(&results, &opts).await?;
    }

    // Wait for all background emit tasks to finish
    for handle in emit_handles {
        if let Err(e) = handle.await {
            tracing::error!(error = %e, "emit task panicked");
        }
    }

    let _ = session.close().await;

    info!(
        total = total_crawled,
        successful = total_successful,
        errors = total_errors,
        "crawl complete"
    );

    Ok(())
}

async fn build_queue(
    opts: &CrawlOptions,
    origin: &str,
) -> (VecDeque<(String, u32)>, HashSet<String>) {
    let mut queue = VecDeque::new();
    let mut visited = HashSet::new();

    if opts.use_sitemap {
        for url in sitemap::fetch_sitemap(origin).await {
            let normalized = normalize_url(&url);
            if !visited.contains(&normalized)
                && (opts.max_pages == 0 || visited.len() < opts.max_pages)
            {
                visited.insert(normalized.clone());
                queue.push_back((normalized, 0));
            }
        }
    }

    // Use curated seed URLs when provided, otherwise fall back to single url
    let seeds: Vec<String> = if opts.seed_urls.is_empty() {
        vec![opts.url.clone()]
    } else {
        opts.seed_urls.clone()
    };

    for url in &seeds {
        let seed = normalize_url(url);
        if !visited.contains(&seed) {
            visited.insert(seed.clone());
            queue.push_back((seed, 0));
        }
    }

    (queue, visited)
}

async fn load_robots(opts: &CrawlOptions, origin: &str) -> robots::RobotsTxt {
    if opts.respect_robots {
        robots::RobotsTxt::fetch(origin).await
    } else {
        robots::RobotsTxt::default()
    }
}

fn should_crawl_page(
    depth: u32,
    url: &str,
    robots: &robots::RobotsTxt,
    opts: &CrawlOptions,
) -> bool {
    if depth > opts.max_depth {
        return false;
    }
    !robots_blocks(robots, url)
}

fn robots_blocks(robots: &robots::RobotsTxt, url: &str) -> bool {
    let Ok(parsed) = Url::parse(url) else {
        return false;
    };
    let allowed = robots.is_allowed(parsed.path());
    if !allowed {
        tracing::debug!(%url, "blocked by robots.txt");
    }
    !allowed
}

async fn acquire_permit(semaphore: Arc<Semaphore>) -> Result<tokio::sync::OwnedSemaphorePermit> {
    match semaphore.acquire_owned().await {
        Ok(p) => Ok(p),
        Err(_) => {
            warn!("semaphore closed unexpectedly, stopping crawl");
            Err(crate::AppError::InvalidInput("semaphore closed".into()))
        }
    }
}

async fn apply_rate_limit(
    url: &str,
    opts: &CrawlOptions,
    host_last_request: &mut HashMap<String, Instant>,
) {
    let Ok(parsed) = Url::parse(url) else { return };
    let host = parsed.host_str().unwrap_or("").to_string();
    let min_interval = Duration::from_secs_f64(1.0 / opts.requests_per_second.max(0.01));
    if let Some(last) = host_last_request.get(&host) {
        let elapsed = last.elapsed();
        if elapsed < min_interval {
            tokio::time::sleep(min_interval - elapsed).await;
        }
    }
    host_last_request.insert(host, Instant::now());
}

async fn fetch_with_session(
    url: &str,
    session: &mut BrowserSession,
    opts: &CrawlOptions,
) -> Result<PageCapture> {
    let result = tokio::time::timeout(
        Duration::from_secs(opts.timeout_secs),
        do_fetch(url, session, opts.settle_ms),
    )
    .await;

    match result {
        Ok(Ok(c)) => Ok(c),
        Ok(Err(e)) => Err(e),
        Err(_) => Err(crate::AppError::Navigation("timeout".into())),
    }
}

async fn do_fetch(url: &str, session: &mut BrowserSession, settle_ms: u64) -> Result<PageCapture> {
    session
        .navigate(url)
        .await
        .map_err(|e| crate::AppError::Navigation(e.to_string()))?;

    // Check HTTP status via PerformanceNavigationTiming
    let status_str = session
        .eval_js(
            r#"
            (() => {
                const nav = performance.getEntriesByType('navigation')[0];
                if (nav && nav.responseStatus) return String(nav.responseStatus);
                return "200";
            })()
            "#,
        )
        .await
        .unwrap_or_else(|_| "200".to_string());

    let status_num: u16 = status_str.parse().unwrap_or(200);
    if status_num >= 400 {
        return Err(crate::AppError::Navigation(format!("HTTP {status_num}")));
    }

    let mut html = session
        .get_html()
        .await
        .map_err(|e| crate::AppError::Browser(e.to_string()))?;

    if html.to_lowercase().contains("<script") {
        html = settle_and_recapture(session, settle_ms).await?;
    }

    // Capture localStorage snapshot
    let local_storage = session
        .eval_js(
            r#"
            (() => {
                const items = {};
                for (let i = 0; i < localStorage.length; i++) {
                    const key = localStorage.key(i);
                    items[key] = localStorage.getItem(key);
                }
                return JSON.stringify(items);
            })()
            "#,
        )
        .await
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok());

    let title = crate::extraction::extract_title(&html);
    let current_url = session.current_url().unwrap_or(url).to_string();
    Ok(PageCapture {
        url: current_url,
        html,
        title,
        http_status: Some(status_num),
        local_storage,
    })
}

async fn settle_and_recapture(session: &mut BrowserSession, settle_ms: u64) -> Result<String> {
    debug!("SPA detected, running settle loop");
    let max_iterations = 3;
    let sleep_per_iteration = Duration::from_millis(settle_ms.max(1) / max_iterations);

    for i in 0..max_iterations {
        let before = session
            .eval_js("document.body ? document.body.scrollHeight : 0")
            .await
            .unwrap_or_else(|_| "0".to_string());
        session
            .eval_js("window.scrollTo(0, document.body.scrollHeight)")
            .await
            .ok();
        tokio::time::sleep(sleep_per_iteration).await;
        let after = session
            .eval_js("document.body ? document.body.scrollHeight : 0")
            .await
            .unwrap_or_else(|_| "0".to_string());

        if before == after {
            trace!(iteration = i, "DOM stable after settle");
            break;
        }
    }

    session
        .get_html()
        .await
        .map_err(|e| crate::AppError::Browser(e.to_string()))
}

fn error_result(
    url: &str,
    depth: u32,
    format: &ContentFormat,
    err: &crate::AppError,
) -> CrawlResult {
    CrawlResult {
        url: url.to_string(),
        depth,
        content: String::new(),
        format: format_name(format),
        title: None,
        links_found: 0,
        error: Some(err.to_string()),
        http_status: None,
        local_storage: None,
        meta: None,
    }
}

fn enqueue_discovered(
    html: &str,
    depth: u32,
    base_url: &Url,
    opts: &CrawlOptions,
    visited: &mut HashSet<String>,
    queue: &mut VecDeque<(String, u32)>,
) {
    if depth >= opts.max_depth {
        return;
    }
    let links = extract_links(html, base_url);
    enqueue_discovered_links(&links, depth, base_url, opts, visited, queue);
}

fn enqueue_discovered_links(
    links: &[String],
    depth: u32,
    base_url: &Url,
    opts: &CrawlOptions,
    visited: &mut HashSet<String>,
    queue: &mut VecDeque<(String, u32)>,
) {
    if depth >= opts.max_depth {
        return;
    }
    for link in links {
        if opts.same_origin && !is_same_origin(link, base_url) {
            continue;
        }
        let normalized = normalize_url(link);
        if !visited.contains(&normalized) && visited.len() < MAX_DISCOVERED_URLS {
            visited.insert(normalized.clone());
            queue.push_back((normalized, depth + 1));
        }
    }
}

async fn crawl_one_page(
    url: &str,
    depth: u32,
    base_url: &Url,
    opts: &CrawlOptions,
    visited: &mut HashSet<String>,
    queue: &mut VecDeque<(String, u32)>,
    session: &mut BrowserSession,
) -> CrawlResult {
    let capture = match fetch_with_session(url, session, opts).await {
        Ok(c) => c,
        Err(err) => return error_result(url, depth, &opts.format, &err),
    };

    // Run page validation before any further processing
    let validator = validator::PageValidator::default();
    if let Err(reason) = validator.validate(&capture) {
        return CrawlResult {
            url: url.to_string(),
            depth,
            content: String::new(),
            format: format_name(&opts.format),
            title: capture.title,
            links_found: 0,
            error: Some(reason),
            http_status: capture.http_status,
            local_storage: capture.local_storage,
            meta: None,
        };
    }

    // Compare raw HTML links vs live DOM links to decide if page is SPA-rendered
    let raw_links = extract_links(&capture.html, base_url);
    let dom_links = session.get_dom_links().await.unwrap_or_default();
    let use_dom = dom_links.len() > raw_links.len().saturating_mul(2) && dom_links.len() > 5;

    if use_dom {
        debug!(%url, raw = raw_links.len(), dom = dom_links.len(), "using rendered DOM links");
        enqueue_discovered_links(&dom_links, depth, base_url, opts, visited, queue);
    } else {
        enqueue_discovered(&capture.html, depth, base_url, opts, visited, queue);
    }

    let meta = extract_structured_meta(session).await.ok();
    extract_and_build(url, depth, capture, opts.format, meta)
}

pub(crate) fn extract_and_build(
    url: &str,
    depth: u32,
    capture: PageCapture,
    format: ContentFormat,
    meta: Option<serde_json::Value>,
) -> CrawlResult {
    let links_found = extract_links(
        &capture.html,
        &Url::parse(url).unwrap_or_else(|_| Url::parse("http://localhost").unwrap()),
    )
    .len();

    match extraction::extract(&capture.html, format) {
        Ok(content) => CrawlResult {
            url: url.to_string(),
            depth,
            content,
            format: format_name(&format),
            title: capture.title,
            links_found,
            error: None,
            http_status: capture.http_status,
            local_storage: capture.local_storage,
            meta,
        },
        Err(e) => CrawlResult {
            url: url.to_string(),
            depth,
            content: String::new(),
            format: format_name(&format),
            title: capture.title,
            links_found,
            error: Some(e.to_string()),
            http_status: capture.http_status,
            local_storage: capture.local_storage,
            meta,
        },
    }
}

fn format_name(fmt: &ContentFormat) -> String {
    match fmt {
        ContentFormat::Markdown => "markdown".to_string(),
        ContentFormat::Html => "html".to_string(),
        ContentFormat::Text => "text".to_string(),
    }
}

#[cfg(test)]
fn file_extension(fmt: &ContentFormat) -> &'static str {
    match fmt {
        ContentFormat::Markdown => "md",
        ContentFormat::Html => "html",
        ContentFormat::Text => "txt",
    }
}

async fn extract_structured_meta(session: &BrowserSession) -> Result<serde_json::Value> {
    let json_ld = session
        .eval_js(
            r#"
            JSON.stringify(
                Array.from(document.querySelectorAll('script[type="application/ld+json"]'))
                    .map(s => { try { return JSON.parse(s.innerText); } catch(e) { return null; }})
                    .filter(x => x !== null)
            )
        "#,
        )
        .await
        .unwrap_or_else(|_| "[]".to_string());

    let og = session
        .eval_js(
            r#"
            const props = {};
            document.querySelectorAll('meta[property^="og:"]').forEach(m => {
                props[m.getAttribute('property')] = m.getAttribute('content');
            });
            JSON.stringify(props);
        "#,
        )
        .await
        .unwrap_or_else(|_| "{}".to_string());

    Ok(serde_json::json!({
        "json_ld": serde_json::from_str::<serde_json::Value>(&json_ld).unwrap_or(serde_json::Value::Null),
        "open_graph": serde_json::from_str::<serde_json::Value>(&og).unwrap_or(serde_json::Value::Null),
    }))
}

async fn emit_results(results: &[CrawlResult], opts: &CrawlOptions) -> Result<()> {
    for output in &opts.outputs {
        output.emit_results(results).await?;
    }
    Ok(())
}

pub fn build_manifest_entry(result: &CrawlResult, filename: &str) -> serde_json::Value {
    serde_json::json!({
        "url": result.url,
        "file": filename,
        "depth": result.depth,
        "title": result.title,
        "links_found": result.links_found,
        "error": result.error,
    })
}

pub async fn write_manifest(dir: &std::path::Path, manifest: &[serde_json::Value]) -> Result<()> {
    let path = dir.join("manifest.json");
    let json = serde_json::to_string_pretty(manifest)
        .map_err(|e| crate::AppError::InvalidInput(format!("failed to serialize manifest: {e}")))?;
    fs::write(&path, json)
        .await
        .map_err(|e| crate::AppError::InvalidInput(format!("failed to write manifest: {e}")))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;

    #[test]
    fn test_format_name() {
        assert_eq!(format_name(&ContentFormat::Markdown), "markdown");
        assert_eq!(format_name(&ContentFormat::Html), "html");
        assert_eq!(format_name(&ContentFormat::Text), "text");
    }

    #[test]
    fn test_file_extension() {
        assert_eq!(file_extension(&ContentFormat::Markdown), "md");
        assert_eq!(file_extension(&ContentFormat::Html), "html");
        assert_eq!(file_extension(&ContentFormat::Text), "txt");
    }

    #[test]
    fn test_should_crawl_page_depth_limit() {
        let robots = robots::RobotsTxt::default();
        let opts = CrawlOptions {
            url: "https://example.com".to_string(),
            seed_urls: vec![],
            max_depth: 2,
            max_pages: 10,
            format: ContentFormat::Markdown,
            respect_robots: false,
            requests_per_second: 100.0,
            concurrency: 1,
            same_origin: true,
            timeout_secs: 5,
            use_sitemap: false,
            settle_ms: 2000,
            outputs: vec![std::sync::Arc::new(output::StdoutOutput)],
        };
        assert!(should_crawl_page(0, "https://example.com/", &robots, &opts));
        assert!(should_crawl_page(2, "https://example.com/", &robots, &opts));
        assert!(!should_crawl_page(
            3,
            "https://example.com/",
            &robots,
            &opts
        ));
    }

    #[test]
    fn test_should_crawl_page_respects_robots() {
        let robots = robots::RobotsTxt::parse("User-agent: *\nDisallow: /private/\n");
        let opts = CrawlOptions {
            url: "https://example.com".to_string(),
            seed_urls: vec![],
            max_depth: 2,
            max_pages: 10,
            format: ContentFormat::Markdown,
            respect_robots: true,
            requests_per_second: 100.0,
            concurrency: 1,
            same_origin: true,
            timeout_secs: 5,
            use_sitemap: false,
            settle_ms: 2000,
            outputs: vec![std::sync::Arc::new(output::StdoutOutput)],
        };
        assert!(should_crawl_page(
            0,
            "https://example.com/public",
            &robots,
            &opts
        ));
        assert!(!should_crawl_page(
            0,
            "https://example.com/private/page",
            &robots,
            &opts
        ));
    }

    #[test]
    fn test_enqueue_discovered_adds_links() {
        let html = r#"
            <html><body>
            <a href="/page1">Page 1</a>
            <a href="/page2">Page 2</a>
            <a href="https://other.com/page">External</a>
            </body></html>
        "#;
        let base_url = Url::parse("https://example.com").unwrap();
        let opts = CrawlOptions {
            url: "https://example.com".to_string(),
            seed_urls: vec![],
            max_depth: 2,
            max_pages: 10,
            format: ContentFormat::Markdown,
            respect_robots: false,
            requests_per_second: 100.0,
            concurrency: 1,
            same_origin: true,
            timeout_secs: 5,
            use_sitemap: false,
            settle_ms: 2000,
            outputs: vec![std::sync::Arc::new(output::StdoutOutput)],
        };
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        visited.insert("https://example.com/".to_string());

        enqueue_discovered(html, 0, &base_url, &opts, &mut visited, &mut queue);

        // Two internal links should be enqueued
        assert_eq!(queue.len(), 2);
        let urls: Vec<_> = queue.iter().map(|(u, _)| u.as_str()).collect();
        assert!(urls.contains(&"https://example.com/page1"));
        assert!(urls.contains(&"https://example.com/page2"));
        // External link should be skipped due to same_origin=true
        assert!(!urls.contains(&"https://other.com/page"));
    }

    #[test]
    fn test_enqueue_discovered_allows_external_when_same_origin_false() {
        let html = r#"
            <html><body>
            <a href="https://other.com/page">External</a>
            </body></html>
        "#;
        let base_url = Url::parse("https://example.com").unwrap();
        let opts = CrawlOptions {
            url: "https://example.com".to_string(),
            seed_urls: vec![],
            max_depth: 2,
            max_pages: 10,
            format: ContentFormat::Markdown,
            respect_robots: false,
            requests_per_second: 100.0,
            concurrency: 1,
            same_origin: false,
            timeout_secs: 5,
            use_sitemap: false,
            settle_ms: 2000,
            outputs: vec![std::sync::Arc::new(output::StdoutOutput)],
        };
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        visited.insert("https://example.com/".to_string());

        enqueue_discovered(html, 0, &base_url, &opts, &mut visited, &mut queue);

        assert_eq!(queue.len(), 1);
        assert_eq!(queue[0].0, "https://other.com/page");
    }

    #[test]
    fn test_enqueue_discovered_respects_max_depth() {
        let html = r#"<html><body><a href="/page1">Page 1</a></body></html>"#;
        let base_url = Url::parse("https://example.com").unwrap();
        let opts = CrawlOptions {
            url: "https://example.com".to_string(),
            seed_urls: vec![],
            max_depth: 2,
            max_pages: 10,
            format: ContentFormat::Markdown,
            respect_robots: false,
            requests_per_second: 100.0,
            concurrency: 1,
            same_origin: true,
            timeout_secs: 5,
            use_sitemap: false,
            settle_ms: 2000,
            outputs: vec![std::sync::Arc::new(output::StdoutOutput)],
        };
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        visited.insert("https://example.com/".to_string());

        // At depth == max_depth, no new links should be enqueued
        enqueue_discovered(html, 2, &base_url, &opts, &mut visited, &mut queue);
        assert!(queue.is_empty());
    }

    #[test]
    fn test_enqueue_discovered_deduplicates() {
        let html = r#"
            <html><body>
            <a href="/page1">Page 1</a>
            <a href="/page1">Page 1 again</a>
            </body></html>
        "#;
        let base_url = Url::parse("https://example.com").unwrap();
        let opts = CrawlOptions {
            url: "https://example.com".to_string(),
            seed_urls: vec![],
            max_depth: 2,
            max_pages: 10,
            format: ContentFormat::Markdown,
            respect_robots: false,
            requests_per_second: 100.0,
            concurrency: 1,
            same_origin: true,
            timeout_secs: 5,
            use_sitemap: false,
            settle_ms: 2000,
            outputs: vec![std::sync::Arc::new(output::StdoutOutput)],
        };
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        visited.insert("https://example.com/".to_string());

        enqueue_discovered(html, 0, &base_url, &opts, &mut visited, &mut queue);

        // Same link twice should only enqueue once
        assert_eq!(queue.len(), 1);
    }

    #[test]
    fn test_error_result_format() {
        let err = crate::AppError::Navigation("timeout".to_string());
        let result = error_result("https://example.com/", 1, &ContentFormat::Markdown, &err);
        assert_eq!(result.url, "https://example.com/");
        assert_eq!(result.depth, 1);
        assert_eq!(result.format, "markdown");
        assert!(result.error.is_some());
        assert!(result.content.is_empty());
        assert_eq!(result.links_found, 0);
    }

    use crate::crawl::output::CrawlOutput;

    #[tokio::test]
    async fn test_write_to_directory_creates_files() {
        let temp_dir = tempfile::tempdir().unwrap();
        let output_dir = temp_dir.path().join("output");

        let results = vec![
            CrawlResult {
                url: "https://example.com/".to_string(),
                depth: 0,
                content: "page 1 content".to_string(),
                format: "md".to_string(),
                title: Some("Page 1".to_string()),
                links_found: 2,
                error: None,
                http_status: None,
                local_storage: None,
                meta: None,
            },
            CrawlResult {
                url: "https://example.com/page2".to_string(),
                depth: 1,
                content: "page 2 content".to_string(),
                format: "md".to_string(),
                title: Some("Page 2".to_string()),
                links_found: 0,
                error: None,
                http_status: None,
                local_storage: None,
                meta: None,
            },
        ];

        let output = output::DirectoryOutput::new(output_dir.clone(), "md");
        output.emit_results(&results).await.unwrap();

        assert!(output_dir.exists());
        assert!(output_dir.join("0001.md").exists());
        assert!(output_dir.join("0002.md").exists());
        assert!(output_dir.join("manifest.json").exists());

        let content = std::fs::read_to_string(output_dir.join("0001.md")).unwrap();
        assert_eq!(content, "page 1 content");

        let manifest = std::fs::read_to_string(output_dir.join("manifest.json")).unwrap();
        let manifest_json: serde_json::Value = serde_json::from_str(&manifest).unwrap();
        let arr = manifest_json.as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0]["url"], "https://example.com/");
        assert_eq!(arr[1]["url"], "https://example.com/page2");
    }

    #[test]
    fn test_build_queue_without_sitemap() {
        let opts = CrawlOptions {
            url: "https://example.com".to_string(),
            seed_urls: vec![],
            max_depth: 2,
            max_pages: 10,
            format: ContentFormat::Markdown,
            respect_robots: false,
            requests_per_second: 100.0,
            concurrency: 1,
            same_origin: true,
            timeout_secs: 5,
            use_sitemap: false,
            settle_ms: 2000,
            outputs: vec![std::sync::Arc::new(output::StdoutOutput)],
        };

        let rt = tokio::runtime::Runtime::new().unwrap();
        let (queue, visited) =
            rt.block_on(async { build_queue(&opts, "https://example.com").await });

        assert_eq!(queue.len(), 1);
        assert_eq!(queue[0].0, "https://example.com/");
        assert_eq!(queue[0].1, 0);
        assert!(visited.contains("https://example.com/"));
    }

    #[test]
    fn test_extract_and_build_success() {
        let capture = PageCapture {
            url: "https://example.com/".to_string(),
            html: "<html><head><title>Test Page</title></head><body>Hello</body></html>"
                .to_string(),
            title: Some("Test Page".to_string()),
            http_status: None,
            local_storage: None,
        };

        let result = extract_and_build(
            "https://example.com/",
            0,
            capture,
            ContentFormat::Markdown,
            None,
        );
        assert_eq!(result.url, "https://example.com/");
        assert_eq!(result.depth, 0);
        assert_eq!(result.title, Some("Test Page".to_string()));
        assert!(result.error.is_none());
        assert_eq!(result.format, "markdown");
    }
}
