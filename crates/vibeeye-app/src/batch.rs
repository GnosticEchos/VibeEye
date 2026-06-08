//! Batch fetch tool: process a list of URLs without link discovery or BFS.

use crate::Result;
use crate::browser::BrowserSession;
use crate::crawl::output::CrawlOutput;
use crate::crawl::{CrawlResult, extract_and_build};
use crate::tools::common::PageCapture;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;
use tracing::{info, warn};
use vibeeye_core::ContentFormat;

/// Options that drive a batch fetch job.
#[derive(Debug, Clone)]
pub struct BatchOptions {
    pub urls: Vec<String>,
    pub format: ContentFormat,
    pub timeout_secs: u64,
    pub settle_ms: u64,
    pub concurrency: usize,
    pub outputs: Vec<Arc<dyn CrawlOutput>>,
}

/// Fetch and process every URL in `opts.urls` without discovering or following links.
///
/// Each URL is fetched with the same concurrency and timeout semantics as the crawler,
/// but there is no BFS queue, no depth limit, and no `same_origin` filtering.
pub async fn run(opts: BatchOptions) -> Result<()> {
    let semaphore = Arc::new(Semaphore::new(opts.concurrency.max(1)));
    let mut results: Vec<CrawlResult> = Vec::new();
    let mut total_success = 0usize;
    let mut total_error = 0usize;

    let mut session = BrowserSession::new().map_err(|e| crate::AppError::Browser(e.to_string()))?;

    for (idx, url) in opts.urls.iter().enumerate() {
        let permit = match semaphore.clone().acquire_owned().await {
            Ok(p) => p,
            Err(_) => {
                warn!("semaphore closed unexpectedly, stopping batch");
                break;
            }
        };

        let result = fetch_one(url, &mut session, &opts).await;
        drop(permit);

        if result.error.is_some() {
            total_error += 1;
        } else {
            total_success += 1;
        }

        results.push(result);

        if results.len() >= 50 {
            emit_results(&results, &opts.outputs).await?;
            results.clear();
        }

        info!(
            progress = idx + 1,
            total = opts.urls.len(),
            total_success,
            total_error,
            "batch progress"
        );
    }

    if !results.is_empty() {
        emit_results(&results, &opts.outputs).await?;
    }

    info!(
        total_success,
        total_error,
        total = opts.urls.len(),
        "batch complete"
    );
    Ok(())
}

async fn fetch_one(url: &str, session: &mut BrowserSession, opts: &BatchOptions) -> CrawlResult {
    let capture = match tokio::time::timeout(
        Duration::from_secs(opts.timeout_secs),
        do_fetch(url, session, opts.settle_ms),
    )
    .await
    {
        Ok(Ok(c)) => c,
        Ok(Err(e)) => {
            return error_result(url, &opts.format, &e);
        }
        Err(_) => {
            return error_result(
                url,
                &opts.format,
                &crate::AppError::Navigation("timeout".into()),
            );
        }
    };

    extract_and_build(url, 0, capture, opts.format, None)
}

async fn do_fetch(url: &str, session: &mut BrowserSession, settle_ms: u64) -> Result<PageCapture> {
    session
        .navigate(url)
        .await
        .map_err(|e| crate::AppError::Navigation(e.to_string()))?;

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

    let title = crate::extraction::extract_title(&html);
    let current_url = session.current_url().unwrap_or(url).to_string();

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
        .and_then(|json_str| serde_json::from_str(&json_str).ok());

    Ok(PageCapture {
        url: current_url,
        html,
        title,
        http_status: Some(status_num),
        local_storage,
    })
}

async fn settle_and_recapture(session: &mut BrowserSession, settle_ms: u64) -> Result<String> {
    let max_iterations = 3;
    let sleep_per_iteration = Duration::from_millis(settle_ms.max(1) / max_iterations);

    for _ in 0..max_iterations {
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
            break;
        }
    }

    session
        .get_html()
        .await
        .map_err(|e| crate::AppError::Browser(e.to_string()))
}

fn error_result(url: &str, format: &ContentFormat, err: &crate::AppError) -> CrawlResult {
    let format_str = match format {
        ContentFormat::Markdown => "markdown",
        ContentFormat::Html => "html",
        ContentFormat::Text => "text",
    };
    CrawlResult {
        url: url.to_string(),
        depth: 0,
        content: String::new(),
        format: format_str.to_string(),
        title: None,
        links_found: 0,
        error: Some(err.to_string()),
        http_status: None,
        local_storage: None,
        meta: None,
    }
}

async fn emit_results(results: &[CrawlResult], outputs: &[Arc<dyn CrawlOutput>]) -> Result<()> {
    for output in outputs {
        output.emit_results(results).await?;
    }
    Ok(())
}
