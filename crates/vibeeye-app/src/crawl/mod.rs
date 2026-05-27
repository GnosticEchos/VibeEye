//! BFS web crawler built on VibeEye's Servo browser engine.
//!
//! Reuses `navigate_and_capture` for page fetching and the extraction
//! pipeline for content distillation.

use crate::extraction;
use crate::browser::BrowserSession;
use crate::tools::common::PageCapture;
use crate::Result;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::fs;
use tokio::sync::Semaphore;
use tracing::{info, warn};
use url::Url;
use vibeeye_core::ContentFormat;

mod links;
pub mod robots;
pub mod sitemap;

pub use links::{extract_links, is_same_origin, normalize_url};

/// Maximum discovered URLs before we stop enqueuing new ones.
const MAX_DISCOVERED_URLS: usize = 5000;

/// Options that drive a crawl job.
#[derive(Debug, Clone)]
pub struct CrawlOptions {
    pub url: String,
    pub max_depth: u32,
    pub max_pages: usize,
    pub format: ContentFormat,
    pub respect_robots: bool,
    pub requests_per_second: f64,
    pub concurrency: usize,
    pub same_origin: bool,
    pub timeout_secs: u64,
    pub use_sitemap: bool,
    pub output_dir: Option<PathBuf>,
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

    let mut session = BrowserSession::new()
        .map_err(|e| crate::AppError::Browser(e.to_string()))?;

    while let Some((url, depth)) = queue.pop_front() {
        if results.len() >= opts.max_pages {
            break;
        }
        if !should_crawl_page(depth, &url, &robots, &opts) {
            continue;
        }

        let permit = acquire_permit(semaphore.clone()).await?;
        apply_rate_limit(&url, &opts, &mut host_last_request).await;

        let result =
            crawl_one_page(&url, depth, &base_url, &opts, &mut visited, &mut queue, &mut session).await;
        drop(permit);
        results.push(result);
        info!(
            url = %url,
            depth = depth,
            queue_size = queue.len(),
            completed = results.len(),
            "crawled page"
        );
    }

    emit_results(&results, &opts).await?;

    let _ = session.close().await;

    info!(
        total = results.len(),
        successful = results.iter().filter(|r| r.error.is_none()).count(),
        errors = results.iter().filter(|r| r.error.is_some()).count(),
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
            if !visited.contains(&normalized) && visited.len() < opts.max_pages {
                visited.insert(normalized.clone());
                queue.push_back((normalized, 0));
            }
        }
    }

    let seed = normalize_url(&opts.url);
    if !visited.contains(&seed) {
        visited.insert(seed.clone());
        queue.push_back((seed, 0));
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

fn should_crawl_page(depth: u32, url: &str, robots: &robots::RobotsTxt, opts: &CrawlOptions) -> bool {
    if depth > opts.max_depth {
        return false;
    }
    !robots_blocks(robots, url)
}

fn robots_blocks(robots: &robots::RobotsTxt, url: &str) -> bool {
    let Ok(parsed) = Url::parse(url) else { return false };
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
        async {
            session
                .navigate(url)
                .await
                .map_err(|e| crate::AppError::Navigation(e.to_string()))?;
            let html = session
                .get_html()
                .await
                .map_err(|e| crate::AppError::Browser(e.to_string()))?;
            let title = crate::extraction::extract_title(&html);
            let current_url = session.current_url().unwrap_or(url).to_string();
            Ok(PageCapture { url: current_url, html, title })
        },
    )
    .await;

    match result {
        Ok(Ok(c)) => Ok(c),
        Ok(Err(e)) => Err(e),
        Err(_) => Err(crate::AppError::Navigation("timeout".into())),
    }
}

fn error_result(url: &str, depth: u32, format: &ContentFormat, err: &crate::AppError) -> CrawlResult {
    CrawlResult {
        url: url.to_string(),
        depth,
        content: String::new(),
        format: format_name(format),
        title: None,
        links_found: 0,
        error: Some(err.to_string()),
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
    for link in extract_links(html, base_url) {
        if opts.same_origin && !is_same_origin(&link, base_url) {
            continue;
        }
        let normalized = normalize_url(&link);
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

    enqueue_discovered(&capture.html, depth, base_url, opts, visited, queue);
    extract_and_build(url, depth, capture, opts.format)
}

fn extract_and_build(
    url: &str,
    depth: u32,
    capture: PageCapture,
    format: ContentFormat,
) -> CrawlResult {
    let links_found =
        extract_links(&capture.html, &Url::parse(url).unwrap_or_else(|_| Url::parse("http://localhost").unwrap())).len();

    match extraction::extract(&capture.html, format) {
        Ok(content) => CrawlResult {
            url: url.to_string(),
            depth,
            content,
            format: format_name(&format),
            title: capture.title,
            links_found,
            error: None,
        },
        Err(e) => CrawlResult {
            url: url.to_string(),
            depth,
            content: String::new(),
            format: format_name(&format),
            title: capture.title,
            links_found,
            error: Some(e.to_string()),
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

fn file_extension(fmt: &ContentFormat) -> &'static str {
    match fmt {
        ContentFormat::Markdown => "md",
        ContentFormat::Html => "html",
        ContentFormat::Text => "txt",
    }
}

async fn emit_results(results: &[CrawlResult], opts: &CrawlOptions) -> Result<()> {
    if let Some(dir) = &opts.output_dir {
        write_to_directory(dir, results, file_extension(&opts.format)).await
    } else {
        for result in results {
            println!("{}", serde_json::to_string(result).unwrap_or_default());
        }
        Ok(())
    }
}

async fn write_to_directory(
    dir: &PathBuf,
    results: &[CrawlResult],
    ext: &str,
) -> Result<()> {
    fs::create_dir_all(dir).await.map_err(|e| {
        crate::AppError::InvalidInput(format!("failed to create output directory: {e}"))
    })?;

    let mut manifest: Vec<serde_json::Value> = Vec::new();

    for (idx, result) in results.iter().enumerate() {
        let filename = format!("{:04}.{ext}", idx + 1);
        let filepath = dir.join(&filename);
        fs::write(&filepath, &result.content)
            .await
            .map_err(|e| crate::AppError::InvalidInput(format!("failed to write file: {e}")))?;
        manifest.push(build_manifest_entry(result, &filename));
    }

    write_manifest(dir, &manifest).await
}

fn build_manifest_entry(result: &CrawlResult, filename: &str) -> serde_json::Value {
    serde_json::json!({
        "url": result.url,
        "file": filename,
        "depth": result.depth,
        "title": result.title,
        "links_found": result.links_found,
        "error": result.error,
    })
}

async fn write_manifest(dir: &std::path::Path, manifest: &[serde_json::Value]) -> Result<()> {
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

    #[test]
    fn test_format_name() {
        assert_eq!(format_name(&ContentFormat::Markdown), "markdown");
        assert_eq!(format_name(&ContentFormat::Html), "html");
        assert_eq!(format_name(&ContentFormat::Text), "text");
    }
}
