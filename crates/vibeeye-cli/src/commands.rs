//! CLI command handlers

use anyhow::Result;
use serde::Serialize;

use vibeeye_app::discovery::Tool;
use vibeeye_app::tools::{
    BrowseInput, BrowseTool, ExtractInput, ExtractTool, SnapshotInput, SnapshotTool,
};

use crate::cli::Commands;

/// Run the selected command
pub async fn run(command: Commands) -> Result<()> {
    match command {
        Commands::Navigate { url } => navigate(url).await,
        Commands::Snapshot { url } => snapshot(url).await,
        Commands::Extract { url, format } => extract(url, format).await,
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
