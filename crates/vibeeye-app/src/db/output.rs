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
        let links: Vec<LinkRecord> = to_ids
            .iter()
            .zip(anchor_texts.iter())
            .map(|(to_id, anchor_text)| LinkRecord {
                group: self.group.clone(),
                from_page: from_id.clone(),
                to_page: to_id.clone(),
                anchor_text: anchor_text.clone(),
                discovered_at: Utc::now(),
            })
            .collect();
        self.client.insert_discovered(&links).await
    }
}

#[async_trait::async_trait]
impl crate::crawl::output::CrawlOutput for SurrealOutput {
    async fn emit_results(&self, results: &[CrawlResult]) -> crate::Result<()> {
        let mut page_ids: HashMap<String, RecordId> = HashMap::new();
        for result in results {
            if result.error.is_some() {
                continue;
            }
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
        let provider = std::sync::Arc::new(crate::embed::EmbeddingProvider::new(config)?);
        let chunker = std::sync::Arc::new(crate::chunk::Chunker::new(
            config.target_chunk_size(),
            config.chunk_overlap(),
            crate::chunk::Tokenizer::CharHeuristic,
        ));

        let eligible: Vec<_> = results
            .iter()
            .filter(|r| {
                r.error.is_none() && !r.content.trim().is_empty() && page_ids.contains_key(&r.url)
            })
            .collect();

        if eligible.is_empty() {
            return Ok(());
        }

        // Probe dimension with first page before spawning parallel tasks.
        // This ensures the HNSW index exists before any chunks are inserted,
        // preventing a race where ensure_embeddings_index deletes chunks.
        let known_dim = {
            let first = &eligible[0];
            let chunks = chunker.chunk(&first.content);
            if !chunks.is_empty() {
                let texts: Vec<String> = chunks.iter().map(|c| c.text.clone()).collect();
                if let Ok(embeddings) = provider.embed_batch(&texts).await {
                    if let Some(first_emb) = embeddings.first() {
                        let d = first_emb.len();
                        self.client.ensure_embeddings_index(d).await?;
                        Some(d)
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        };

        let embed_concurrency = config.embed_concurrency();
        let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(embed_concurrency));
        let monitor = std::sync::Arc::new(EmbedMonitor::new(embed_concurrency));
        let detected_dimension = std::sync::Arc::new(tokio::sync::Mutex::new(known_dim));
        let total_inserted = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));

        let progress = crate::progress::ProgressReporter::new(eligible.len() as u64, "Embedding");
        let progress = std::sync::Arc::new(std::sync::Mutex::new(progress));

        let mut tasks = tokio::task::JoinSet::new();

        for result in eligible {
            let page_id = page_ids.get(&result.url).unwrap().clone();
            let result = result.clone();
            let client = self.client.clone();
            let group = self.group.clone();
            let config = config.clone();
            let provider = provider.clone();
            let chunker = chunker.clone();
            let semaphore = semaphore.clone();
            let monitor = monitor.clone();
            let detected_dimension = detected_dimension.clone();
            let total_inserted = total_inserted.clone();
            let progress = progress.clone();

            tasks.spawn(async move {
                let _permit = semaphore.acquire().await;
                let ctx = PageEmbedCtx {
                    client: &client,
                    group: &group,
                    result: &result,
                    page_id: &page_id,
                    chunker: &chunker,
                    provider: &provider,
                    detected_dimension: &detected_dimension,
                    config: &config,
                    monitor: &monitor,
                };
                let count = Self::process_page_owned(&ctx).await;
                total_inserted.fetch_add(count, std::sync::atomic::Ordering::Relaxed);
                progress.lock().unwrap().inc(1);
                count
            });
        }

        let mut last_report = std::time::Instant::now();
        while let Some(task_result) = tasks.join_next().await {
            if let Err(e) = task_result {
                tracing::warn!(error = %e, "embedding task panicked");
            }
            if last_report.elapsed().as_secs() >= 30 {
                monitor.report();
                last_report = std::time::Instant::now();
            }
        }

        progress.lock().unwrap().finish();
        monitor.report();
        println!(
            "Chunks inserted: {}",
            total_inserted.load(std::sync::atomic::Ordering::Relaxed)
        );
        Ok(())
    }

    async fn process_page_owned(ctx: &PageEmbedCtx<'_, '_>) -> usize {
        let page_start = std::time::Instant::now();

        let t0 = std::time::Instant::now();
        let _ = ctx.client.delete_chunks_for_page(ctx.page_id).await;
        let delete_ms = t0.elapsed().as_millis();

        let mut chunks = ctx.chunker.chunk(&ctx.result.content);
        if chunks.is_empty() {
            tracing::info!(url = %ctx.result.url, delete_ms, "page has no chunks");
            return 0;
        }

        // Cap chunks per page to avoid overwhelming the embedding server
        // and SurrealDB on massive pages (e.g., huge rustdoc tables).
        const MAX_CHUNKS_PER_PAGE: usize = 1000;
        if chunks.len() > MAX_CHUNKS_PER_PAGE {
            tracing::warn!(
                url = %ctx.result.url,
                chunks = chunks.len(),
                max = MAX_CHUNKS_PER_PAGE,
                "truncating chunks for page"
            );
            chunks.truncate(MAX_CHUNKS_PER_PAGE);
        }

        let texts: Vec<String> = chunks.iter().map(|c| c.text.clone()).collect();

        let t0 = std::time::Instant::now();
        let embeddings = match ctx.provider.embed_batch(&texts).await {
            Ok(e) => {
                ctx.monitor.record_success(t0.elapsed());
                e
            }
            Err(e) => {
                ctx.monitor.record_error();
                tracing::warn!(url = %ctx.result.url, error = %e, "embedding request failed");
                return 0;
            }
        };
        let embed_ms = t0.elapsed().as_millis();

        let dim = {
            let mut dim_guard = ctx.detected_dimension.lock().await;
            if dim_guard.is_none() {
                if let Some(first) = embeddings.first() {
                    let d = first.len();
                    *dim_guard = Some(d);
                    drop(dim_guard);
                    let _ = ctx.client.ensure_embeddings_index(d).await;
                    tracing::info!(dimension = d, "auto-detected embedding dimension");
                    d as i32
                } else {
                    0
                }
            } else {
                dim_guard.unwrap_or(0) as i32
            }
        };

        let records: Vec<crate::db::models::ChunkRecord> = chunks
            .into_iter()
            .zip(embeddings)
            .enumerate()
            .map(|(idx, (chunk, embedding))| crate::db::models::ChunkRecord {
                group: ctx.group.to_string(),
                page: ctx.page_id.clone(),
                chunk_index: idx as i32,
                chunk_text: chunk.text,
                heading_path: chunk.heading_path,
                embedding,
                model: ctx.config.model.clone(),
                dimensions: dim,
                created_at: chrono::Utc::now(),
            })
            .collect();

        let count = records.len();
        let t0 = std::time::Instant::now();
        let result = match ctx.client.insert_chunks(&records).await {
            Ok(()) => count,
            Err(e) => {
                eprintln!(
                    "WARN: failed to insert chunks for {}: {}",
                    ctx.result.url, e
                );
                0
            }
        };
        let insert_ms = t0.elapsed().as_millis();
        let total_ms = page_start.elapsed().as_millis();

        tracing::info!(
            url = %ctx.result.url,
            chunks = count,
            delete_ms,
            embed_ms,
            insert_ms,
            total_ms,
            "page embedding complete"
        );

        result
    }
}

#[cfg(feature = "embeddings")]
struct PageEmbedCtx<'a, 'b> {
    client: &'a DbClient,
    group: &'a str,
    result: &'a CrawlResult,
    page_id: &'a RecordId,
    chunker: &'a crate::chunk::Chunker,
    provider: &'a crate::embed::EmbeddingProvider,
    detected_dimension: &'a tokio::sync::Mutex<Option<usize>>,
    config: &'a crate::config::embeddings::EmbeddingConfig,
    monitor: &'b EmbedMonitor,
}

#[cfg(feature = "embeddings")]
struct EmbedMonitor {
    concurrency: usize,
    successes: std::sync::atomic::AtomicU64,
    errors: std::sync::atomic::AtomicU64,
    total_latency_ms: std::sync::atomic::AtomicU64,
    max_latency_ms: std::sync::atomic::AtomicU64,
}

#[cfg(feature = "embeddings")]
impl EmbedMonitor {
    fn new(concurrency: usize) -> Self {
        Self {
            concurrency,
            successes: std::sync::atomic::AtomicU64::new(0),
            errors: std::sync::atomic::AtomicU64::new(0),
            total_latency_ms: std::sync::atomic::AtomicU64::new(0),
            max_latency_ms: std::sync::atomic::AtomicU64::new(0),
        }
    }

    fn record_success(&self, elapsed: std::time::Duration) {
        let ms = elapsed.as_millis() as u64;
        self.successes
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.total_latency_ms
            .fetch_add(ms, std::sync::atomic::Ordering::Relaxed);
        let mut current = self
            .max_latency_ms
            .load(std::sync::atomic::Ordering::Relaxed);
        while ms > current {
            match self.max_latency_ms.compare_exchange_weak(
                current,
                ms,
                std::sync::atomic::Ordering::Relaxed,
                std::sync::atomic::Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(v) => current = v,
            }
        }
    }

    fn record_error(&self) {
        self.errors
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    fn report(&self) {
        let successes = self.successes.load(std::sync::atomic::Ordering::Relaxed);
        let errors = self.errors.load(std::sync::atomic::Ordering::Relaxed);
        let total = successes + errors;
        if total == 0 {
            return;
        }
        let avg_ms = self
            .total_latency_ms
            .load(std::sync::atomic::Ordering::Relaxed)
            / successes.max(1);
        let max_ms = self
            .max_latency_ms
            .load(std::sync::atomic::Ordering::Relaxed);
        let error_rate = (errors as f64 / total as f64) * 100.0;

        let health = if error_rate > 10.0 {
            "⚠️  HIGH ERRORS"
        } else if avg_ms > 5000 {
            "⚠️  SLOW"
        } else if avg_ms > 2000 {
            "⚡ MODERATE"
        } else {
            "✅ HEALTHY"
        };

        tracing::info!(
            health = health,
            concurrency = self.concurrency,
            requests = total,
            errors = errors,
            error_rate_pct = format!("{:.1}", error_rate),
            avg_latency_ms = avg_ms,
            max_latency_ms = max_ms,
            "embedding server status"
        );
    }
}
