//! CLI command handlers

use anyhow::Result;
use serde::Serialize;
use std::path::PathBuf;

use vibeeye_app::config::CrawlConfig;
use vibeeye_app::crawl::{self, CrawlOptions};
use vibeeye_app::discovery::Tool;
use vibeeye_app::tools::{
    BrowseInput, BrowseTool, ExtractInput, ExtractTool, SnapshotInput, SnapshotTool,
};
use vibeeye_core::ContentFormat;

use crate::cli::Commands;

/// Run the selected command
pub async fn run(command: Commands) -> Result<()> {
    match command {
        Commands::Navigate { url } => navigate(url).await,
        Commands::Snapshot { url } => snapshot(url).await,
        Commands::Extract { url, format } => extract(url, format).await,
        Commands::Crawl {
            url,
            config,
            max_depth,
            max_pages,
            format,
            output,
            respect_robots,
            requests_per_second,
            concurrency,
            same_origin,
            timeout,
            sitemap,
        } => {
            crawl_command(
                url,
                config,
                max_depth,
                max_pages,
                format,
                output,
                respect_robots,
                requests_per_second,
                concurrency,
                same_origin,
                timeout,
                sitemap,
            )
            .await
        }
    }
}

async fn navigate(url: String) -> Result<()> {
    tracing::debug!(%url, "navigate command");
    let tool = BrowseTool;
    let input = BrowseInput {
        url,
        wait_until: None,
    };
    let output = Tool::execute(&tool, input).await?;
    tracing::debug!(title = ?output.title, "navigate complete");
    print_json(&output)
}

async fn snapshot(url: String) -> Result<()> {
    tracing::debug!(%url, "snapshot command");
    let tool = SnapshotTool;
    let input = SnapshotInput { url };
    let output = Tool::execute(&tool, input).await?;
    tracing::debug!(title = ?output.title, html_len = output.html.len(), "snapshot complete");
    print_json(&output)
}

async fn extract(url: String, format: String) -> Result<()> {
    tracing::debug!(%url, %format, "extract command");
    let tool = ExtractTool;
    let input = ExtractInput { url, format };
    let output = Tool::execute(&tool, input).await?;
    tracing::debug!(content_len = output.content.len(), "extract complete");
    print_json(&output)
}

fn print_json<T: Serialize>(value: &T) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    // Servo embeds SpiderMonkey, whose global mutex destructor segfaults
    // during normal process teardown.  Bypass all destructors and exit
    // cleanly — this is standard practice for SpiderMonkey embedders.
    std::process::exit(0);
}

#[allow(clippy::too_many_arguments)]
async fn crawl_command(
    url: String,
    config_path: Option<PathBuf>,
    max_depth: Option<u32>,
    max_pages: Option<usize>,
    format: Option<String>,
    output: Option<PathBuf>,
    respect_robots: Option<bool>,
    requests_per_second: Option<f64>,
    concurrency: Option<usize>,
    same_origin: Option<bool>,
    timeout: Option<u64>,
    sitemap: Option<bool>,
) -> Result<()> {
    tracing::debug!(%url, "crawl command");

    let config = CrawlConfig::load(config_path.as_deref())?;
    let profile = config.resolve(&url)?;

    let format_str = format
        .as_deref()
        .or(profile.format.as_deref())
        .unwrap_or("markdown");
    let content_format = match format_str {
        "html" => ContentFormat::Html,
        "text" => ContentFormat::Text,
        _ => ContentFormat::Markdown,
    };

    let opts = CrawlOptions {
        url: url.clone(),
        max_depth: max_depth.or(profile.max_depth).unwrap_or(2),
        max_pages: max_pages.or(profile.max_pages).unwrap_or(100),
        format: content_format,
        respect_robots: respect_robots.or(profile.respect_robots).unwrap_or(false),
        requests_per_second: requests_per_second.or(profile.requests_per_second).unwrap_or(2.0),
        concurrency: concurrency.or(profile.concurrency).unwrap_or(4),
        same_origin: same_origin.or(profile.same_origin).unwrap_or(true),
        timeout_secs: timeout.or(profile.timeout).unwrap_or(15),
        use_sitemap: sitemap.or(profile.sitemap).unwrap_or(false),
        output_dir: output.or_else(|| profile.output.map(PathBuf::from)),
    };

    crawl::run(opts).await?;
    // Servo embeds SpiderMonkey, whose global mutex destructor segfaults
    // during normal process teardown.  Bypass all destructors and exit
    // cleanly — this is standard practice for SpiderMonkey embedders.
    std::process::exit(0);
}
