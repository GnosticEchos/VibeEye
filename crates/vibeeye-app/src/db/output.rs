use anyhow::Result;
use chrono::Utc;
use std::collections::HashMap;
use surrealdb::types::RecordId;

use crate::crawl::CrawlResult;
use crate::db::client::DbClient;
use crate::db::models::{LinkRecord, PageRecord};
use crate::db::util::derive_group;

/// Persists crawl results into SurrealDB.
#[derive(Debug, Clone)]
pub struct SurrealOutput {
    pub client: DbClient,
    pub group: String,
    #[cfg(feature = "embeddings")]
    pub embed_config: Option<crate::config::embeddings::EmbeddingConfig>,
}

impl SurrealOutput {
    /// Create a new SurrealOutput with a client and explicit or derived group name.
    pub fn new(client: DbClient, start_url: &str, group_override: Option<&str>) -> Self {
        let group = derive_group(start_url, group_override);
        Self {
            client,
            group,
            #[cfg(feature = "embeddings")]
            embed_config: None,
        }
    }

    /// Persist a single crawl result as a page record.
    /// Returns the RecordId of the upserted page.
    pub async fn emit_page(&self, result: &CrawlResult) -> Result<RecordId> {
        let record = PageRecord {
            id: None,
            group: self.group.clone(),
            url: result.url.clone(),
            title: result.title.clone().unwrap_or_default(),
            content: result.content.clone(),
            depth: result.depth as i32,
            format: result.format.clone(),
            crawled_at: Utc::now(),
            meta: result.meta.clone(),
        };
        self.client.insert_page(&record).await
    }

    /// Persist discovered links between pages.
    pub async fn emit_links(
        &self,
        from_id: RecordId,
        to_ids: &[RecordId],
        anchor_texts: &[Option<String>],
    ) -> Result<()> {
        for (to_id, anchor_text) in to_ids.iter().zip(anchor_texts.iter()) {
            let link = LinkRecord {
                group: self.group.clone(),
                from_page: from_id.clone(),
                to_page: to_id.clone(),
                anchor_text: anchor_text.clone(),
                discovered_at: Utc::now(),
            };
            self.client.insert_discovered(&link).await?;
        }
        Ok(())
    }
}

#[async_trait::async_trait]
impl crate::crawl::output::CrawlOutput for SurrealOutput {
    async fn emit_results(&self, results: &[CrawlResult]) -> crate::Result<()> {
        let mut page_ids: HashMap<String, RecordId> = HashMap::new();
        for result in results {
            match self.emit_page(result).await {
                Ok(id) => {
                    page_ids.insert(result.url.clone(), id);
                }
                Err(e) => {
                    tracing::warn!(error = %e, url = %result.url, "failed to persist page to SurrealDB");
                }
            }
        }

        #[cfg(feature = "embeddings")]
        if let Some(config) = &self.embed_config {
            if let Err(e) = self.embed_and_index(results, &page_ids, config).await {
                eprintln!("ERROR: embedding post-processing failed: {}", e);
            }
        }

        Ok(())
    }
}

#[cfg(feature = "embeddings")]
impl SurrealOutput {
    async fn embed_and_index(
        &self,
        results: &[CrawlResult],
        page_ids: &HashMap<String, RecordId>,
        config: &crate::config::embeddings::EmbeddingConfig,
    ) -> anyhow::Result<()> {
        let provider = crate::embed::EmbeddingProvider::new(config)?;
        let chunker = crate::chunk::Chunker::new(
            config.target_chunk_size(),
            config.chunk_overlap(),
            crate::chunk::Tokenizer::CharHeuristic,
        );

        let eligible: Vec<_> = results
            .iter()
            .filter(|r| {
                r.error.is_none() && !r.content.trim().is_empty() && page_ids.contains_key(&r.url)
            })
            .collect();

        if eligible.is_empty() {
            return Ok(());
        }

        let progress = crate::progress::ProgressReporter::new(eligible.len() as u64, "Embedding");
        let mut detected_dimension: Option<usize> = None;
        let mut total_inserted = 0usize;

        for result in eligible {
            let page_id = page_ids.get(&result.url).unwrap().clone();
            let count = self
                .process_page(
                    result,
                    &page_id,
                    &chunker,
                    &provider,
                    &mut detected_dimension,
                    config,
                )
                .await;
            total_inserted += count;
            progress.inc(1);
        }

        progress.finish();
        println!("Chunks inserted: {}", total_inserted);
        Ok(())
    }

    async fn process_page(
        &self,
        result: &CrawlResult,
        page_id: &RecordId,
        chunker: &crate::chunk::Chunker,
        provider: &crate::embed::EmbeddingProvider,
        detected_dimension: &mut Option<usize>,
        config: &crate::config::embeddings::EmbeddingConfig,
    ) -> usize {
        let _ = self.client.delete_chunks_for_page(page_id).await;

        let (chunks, embeddings, dim) = match self
            .embed_page_chunks(result, chunker, provider, detected_dimension)
            .await
        {
            Some(v) => v,
            None => return 0,
        };

        let records = self.build_chunk_records(page_id, chunks, embeddings, dim, config);
        self.insert_records(&result.url, records).await
    }

    async fn embed_page_chunks(
        &self,
        result: &CrawlResult,
        chunker: &crate::chunk::Chunker,
        provider: &crate::embed::EmbeddingProvider,
        detected_dimension: &mut Option<usize>,
    ) -> Option<(Vec<crate::chunk::Chunk>, Vec<Vec<f32>>, i32)> {
        let chunks = chunker.chunk(&result.content);
        if chunks.is_empty() {
            return None;
        }

        let texts: Vec<String> = chunks.iter().map(|c| c.text.clone()).collect();
        let embeddings = provider.embed_batch(&texts).await.ok()?;

        let dim = self
            .ensure_dimension(&embeddings, detected_dimension)
            .await?;
        Some((chunks, embeddings, dim))
    }

    async fn ensure_dimension(
        &self,
        embeddings: &[Vec<f32>],
        detected_dimension: &mut Option<usize>,
    ) -> Option<i32> {
        if detected_dimension.is_none() {
            let dim = embeddings.first()?.len();
            *detected_dimension = Some(dim);
            self.client.ensure_embeddings_index(dim).await.ok()?;
            tracing::info!(
                dimension = dim,
                "auto-detected embedding dimension from server"
            );
        }
        Some(detected_dimension.unwrap_or(0) as i32)
    }

    async fn insert_records(
        &self,
        url: &str,
        records: Vec<crate::db::models::ChunkRecord>,
    ) -> usize {
        let count = records.len();
        match self.client.insert_chunks(&records).await {
            Ok(()) => count,
            Err(e) => {
                eprintln!("WARN: failed to insert chunks for {}: {}", url, e);
                0
            }
        }
    }

    fn build_chunk_records(
        &self,
        page_id: &RecordId,
        chunks: Vec<crate::chunk::Chunk>,
        embeddings: Vec<Vec<f32>>,
        dim: i32,
        config: &crate::config::embeddings::EmbeddingConfig,
    ) -> Vec<crate::db::models::ChunkRecord> {
        chunks
            .into_iter()
            .zip(embeddings)
            .enumerate()
            .map(|(idx, (chunk, embedding))| crate::db::models::ChunkRecord {
                group: self.group.clone(),
                page: page_id.clone(),
                chunk_index: idx as i32,
                chunk_text: chunk.text,
                heading_path: chunk.heading_path,
                embedding,
                model: config.model.clone(),
                dimensions: dim,
                created_at: chrono::Utc::now(),
            })
            .collect()
    }
}
