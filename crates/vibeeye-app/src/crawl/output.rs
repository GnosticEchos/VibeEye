use crate::Result;
use crate::crawl::CrawlResult;
use std::path::PathBuf;
use tokio::io::AsyncWriteExt;

/// Abstract output sink for crawl results.
#[async_trait::async_trait]
pub trait CrawlOutput: Send + Sync + std::fmt::Debug {
    async fn emit_results(&self, results: &[CrawlResult]) -> Result<()>;
}

/// Append URLs (one per line) to a text file.
#[derive(Debug, Clone)]
pub struct UrlListOutput {
    path: PathBuf,
}

impl UrlListOutput {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

#[async_trait::async_trait]
impl CrawlOutput for UrlListOutput {
    async fn emit_results(&self, results: &[CrawlResult]) -> Result<()> {
        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .await
            .map_err(|e| {
                crate::AppError::InvalidInput(format!("failed to open URL list file: {e}"))
            })?;

        for result in results {
            if result.error.is_none() {
                let line = format!("{}\n", result.url);
                file.write_all(line.as_bytes()).await.map_err(|e| {
                    crate::AppError::InvalidInput(format!("failed to write URL: {e}"))
                })?;
            }
        }
        Ok(())
    }
}

/// Print results as JSON Lines to stdout.
#[derive(Debug)]
pub struct StdoutOutput;

#[async_trait::async_trait]
impl CrawlOutput for StdoutOutput {
    async fn emit_results(&self, results: &[CrawlResult]) -> Result<()> {
        for result in results {
            println!("{}", serde_json::to_string(result).unwrap_or_default());
        }
        Ok(())
    }
}

/// Write results to a directory with a manifest.json.
#[derive(Debug, Clone)]
pub struct DirectoryOutput {
    dir: PathBuf,
    ext: String,
}

impl DirectoryOutput {
    pub fn new(dir: PathBuf, ext: impl Into<String>) -> Self {
        Self {
            dir,
            ext: ext.into(),
        }
    }
}

#[async_trait::async_trait]
impl CrawlOutput for DirectoryOutput {
    async fn emit_results(&self, results: &[CrawlResult]) -> Result<()> {
        tokio::fs::create_dir_all(&self.dir).await.map_err(|e| {
            crate::AppError::InvalidInput(format!("failed to create output directory: {e}"))
        })?;

        let mut manifest: Vec<serde_json::Value> = Vec::new();

        for (idx, result) in results.iter().enumerate() {
            let filename = format!("{:04}.{}", idx + 1, self.ext);
            let filepath = self.dir.join(&filename);
            tokio::fs::write(&filepath, &result.content)
                .await
                .map_err(|e| crate::AppError::InvalidInput(format!("failed to write file: {e}")))?;
            manifest.push(crate::crawl::build_manifest_entry(result, &filename));
        }

        crate::crawl::write_manifest(&self.dir, &manifest).await
    }
}
