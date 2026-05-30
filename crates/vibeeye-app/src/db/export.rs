//! Export crawl data to SurQL data-only format.

use anyhow::Result;
use std::io::Write;

use super::DbClient;

/// Exports a group's data as `CREATE` statements (no schema).
pub async fn export_group(db: &DbClient, group: &str, writer: &mut dyn Write) -> Result<()> {
    writeln!(writer, "-- Export for group: {group}")?;
    export_pages(db, group, writer).await?;
    export_links(db, group, writer).await?;
    export_chunks(db, group, writer).await?;
    Ok(())
}

async fn export_pages(db: &DbClient, group: &str, writer: &mut dyn Write) -> Result<()> {
    let pages: Vec<serde_json::Value> = db
        .query("SELECT * FROM page WHERE group = $group")
        .bind(("group", group.to_string()))
        .await?
        .take(0)?;

    writeln!(writer, "-- Pages: {}", pages.len())?;
    for page in pages {
        let content = escape_string(page["content"].as_str().unwrap_or(""));
        let title = escape_string(page["title"].as_str().unwrap_or(""));
        let url = escape_string(page["url"].as_str().unwrap_or(""));
        let depth = page["depth"].as_i64().unwrap_or(0);
        let format = page["format"].as_str().unwrap_or("markdown");
        let crawled_at = page["crawled_at"].as_str().unwrap_or("");

        writeln!(
            writer,
            "CREATE page SET group = '{group}', url = '{url}', title = '{title}', \
             content = '{content}', depth = {depth}, format = '{format}', \
             crawled_at = '{crawled_at}'",
        )?;
    }
    Ok(())
}

async fn export_links(db: &DbClient, group: &str, writer: &mut dyn Write) -> Result<()> {
    let links: Vec<serde_json::Value> = db
        .query("SELECT * FROM discovered WHERE group = $group")
        .bind(("group", group.to_string()))
        .await?
        .take(0)?;

    writeln!(writer, "-- Discovered links: {}", links.len())?;
    for link in links {
        let from_url = escape_string(link["in"]["url"].as_str().unwrap_or(""));
        let to_url = escape_string(link["out"]["url"].as_str().unwrap_or(""));
        let anchor = escape_string(link["anchor_text"].as_str().unwrap_or(""));
        let discovered_at = link["discovered_at"].as_str().unwrap_or("");

        writeln!(
            writer,
            "RELATE (SELECT id FROM page WHERE url = '{from_url}') ->discovered-> \
             (SELECT id FROM page WHERE url = '{to_url}') \
             SET group = '{group}', anchor_text = '{anchor}', discovered_at = '{discovered_at}'",
        )?;
    }
    Ok(())
}

async fn export_chunks(db: &DbClient, group: &str, writer: &mut dyn Write) -> Result<()> {
    let chunks: Vec<serde_json::Value> = db
        .query("SELECT group, page_url, chunk_index, chunk_text, heading_path, model, dimensions, created_at FROM chunk WHERE group = $group")
        .bind(("group", group.to_string()))
        .await?
        .take(0)?;

    writeln!(writer, "-- Chunks: {}", chunks.len())?;
    for chunk in chunks {
        let page_url = escape_string(chunk["page_url"].as_str().unwrap_or(""));
        let chunk_index = chunk["chunk_index"].as_i64().unwrap_or(0);
        let chunk_text = escape_string(chunk["chunk_text"].as_str().unwrap_or(""));
        let heading_path = serde_json::to_string(&chunk["heading_path"]).unwrap_or_default();
        let model = escape_string(chunk["model"].as_str().unwrap_or(""));
        let dimensions = chunk["dimensions"].as_i64().unwrap_or(0);
        let created_at = chunk["created_at"].as_str().unwrap_or("");

        writeln!(
            writer,
            "CREATE chunk SET group = '{group}', page_url = '{page_url}', chunk_index = {chunk_index}, \
             chunk_text = '{chunk_text}', heading_path = {heading_path}, model = '{model}', \
             dimensions = {dimensions}, created_at = '{created_at}'",
        )?;
    }
    Ok(())
}

fn escape_string(s: &str) -> String {
    s.replace("'", "\\'")
}
