//! Import crawl data from various formats.

use anyhow::Result;
use std::path::Path;

use super::DbClient;

/// Supported import sources.
pub enum ImportSource<'a> {
    /// Data-only SurQL file (from export).
    SurqlFile(&'a Path),
    /// Output directory with manifest.json + *.md files.
    OutputDirectory(&'a Path),
    /// Flat directory of .md/.txt files.
    TextDirectory(&'a Path),
}

/// Import data into SurrealDB.
pub async fn import(db: &DbClient, group: &str, source: ImportSource<'_>) -> Result<()> {
    match source {
        ImportSource::SurqlFile(path) => import_surql(db, group, path).await,
        ImportSource::OutputDirectory(path) => import_output_dir(db, group, path).await,
        ImportSource::TextDirectory(path) => import_text_dir(db, group, path).await,
    }
}

async fn import_surql(_db: &DbClient, _group: &str, _path: &Path) -> Result<()> {
    let content = tokio::fs::read_to_string(_path).await?;
    let response = _db.query(&content).await?;
    // Validate every statement in the response; statement-level errors
    // (e.g. parse failures) are returned here, unlike the top-level Future.
    response.check()?;
    Ok(())
}

fn detect_format(filename: &str) -> &'static str {
    if filename.ends_with(".md") {
        "markdown"
    } else {
        "text"
    }
}

fn build_page_record(
    group: &str,
    url: String,
    title: String,
    content: String,
    depth: i32,
    format: &str,
) -> crate::db::models::PageRecord {
    crate::db::models::PageRecord {
        id: None,
        group: group.to_string(),
        url,
        title,
        content,
        depth,
        format: format.to_string(),
        crawled_at: chrono::Utc::now(),
        meta: None,
    }
}

async fn process_manifest_entry(
    db: &DbClient,
    group: &str,
    base_path: &Path,
    entry: serde_json::Value,
) -> Result<()> {
    let filename = entry["file"].as_str().unwrap_or_default();
    let filepath = base_path.join(filename);
    if !filepath.exists() {
        return Ok(());
    }

    let content = tokio::fs::read_to_string(&filepath).await?;
    let url = entry["url"].as_str().unwrap_or(filename).to_string();
    let title = entry["title"].as_str().unwrap_or("").to_string();
    let depth = entry["depth"].as_i64().unwrap_or(0) as i32;
    let format = detect_format(filename);

    let record = build_page_record(group, url, title, content, depth, format);
    db.insert_page(&record).await?;
    Ok(())
}

async fn import_output_dir(db: &DbClient, group: &str, path: &Path) -> Result<()> {
    let manifest_path = path.join("manifest.json");
    if !manifest_path.exists() {
        return Err(anyhow::anyhow!(
            "manifest.json not found in {}",
            path.display()
        ));
    }

    let manifest_raw = tokio::fs::read_to_string(&manifest_path).await?;
    let manifest: Vec<serde_json::Value> = serde_json::from_str(&manifest_raw)?;

    for entry in manifest {
        process_manifest_entry(db, group, path, entry).await?;
    }

    Ok(())
}

async fn process_text_file(db: &DbClient, group: &str, path: &Path) -> Result<()> {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    if ext != "md" && ext != "txt" {
        return Ok(());
    }

    let content = tokio::fs::read_to_string(path).await?;
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unnamed");
    let format = if ext == "md" { "markdown" } else { "text" };

    let record = build_page_record(
        group,
        format!("file://{}", path.display()),
        stem.to_string(),
        content,
        0,
        format,
    );
    db.insert_page(&record).await?;
    Ok(())
}

async fn import_text_dir(db: &DbClient, group: &str, path: &Path) -> Result<()> {
    let mut entries = tokio::fs::read_dir(path).await?;
    while let Some(entry) = entries.next_entry().await? {
        process_text_file(db, group, &entry.path()).await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[tokio::test]
    async fn test_import_surql_single_page() -> Result<()> {
        let db = DbClient::connect_mem().await?;
        db.use_ns_db("test", "test").await?;
        db.bootstrap().await?;

        let mut file = tempfile::NamedTempFile::with_suffix(".surql")?;
        writeln!(
            file,
            "CREATE page SET `group` = 'test_group', url = 'https://example.com/', \
             title = 'Test', content = 'Hello world', depth = 0, format = 'text', \
             crawled_at = d'2026-05-30T00:00:00Z';"
        )?;
        file.flush()?;

        import_surql(&db, "test_group", file.path()).await?;

        let stats = db.group_stats("test_group").await?;
        assert_eq!(stats.page_count, 1, "imported page should exist");
        Ok(())
    }

    #[tokio::test]
    async fn test_import_surql_rejects_invalid_syntax() -> Result<()> {
        let db = DbClient::connect_mem().await?;
        db.use_ns_db("test", "test").await?;
        db.bootstrap().await?;

        let mut file = tempfile::NamedTempFile::with_suffix(".surql")?;
        writeln!(file, "INVALID_STATEMENT foo;")?;
        file.flush()?;

        let result = import_surql(&db, "test_group", file.path()).await;
        assert!(result.is_err(), "invalid SurQL should be rejected");
        Ok(())
    }
}
