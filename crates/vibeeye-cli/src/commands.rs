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
    let tool = BrowseTool;
    let input = BrowseInput { url, wait_until: None };
    let output = Tool::execute(&tool, input).await?;
    print_json(&output)
}

async fn snapshot(url: String) -> Result<()> {
    let tool = SnapshotTool;
    let input = SnapshotInput { url };
    let output = Tool::execute(&tool, input).await?;
    print_json(&output)
}

async fn extract(url: String, format: String) -> Result<()> {
    let tool = ExtractTool;
    let input = ExtractInput { url, format };
    let output = Tool::execute(&tool, input).await?;
    print_json(&output)
}

fn print_json<T: Serialize>(value: &T) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}
