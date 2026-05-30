use crate::Result;
use crate::crawl::CrawlResult;
use std::path::PathBuf;

/// Abstract output sink for crawl results.
#[async_trait::async_trait]
pub trait CrawlOutput: Send + Sync + std::fmt::Debug {
    async fn emit_results(&self, results: &[CrawlResult]) -> Result<()>;
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
