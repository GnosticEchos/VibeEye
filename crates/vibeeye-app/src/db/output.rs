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

        // Phase 1: Chunk all pages into a flat list.
        #[derive(Clone)]
        struct ChunkEntry {
            page_id: RecordId,
            chunk_index: i32,
            chunk_text: String,
            heading_path: Vec<String>,
        }

        let mut all_entries: Vec<ChunkEntry> = Vec::new();
        for result in eligible {
            let page_id = page_ids.get(&result.url).unwrap().clone();
            let mut chunks = chunker.chunk(&result.content);
            if chunks.is_empty() {
                continue;
            }
            const MAX_CHUNKS_PER_PAGE: usize = 1000;
            if chunks.len() > MAX_CHUNKS_PER_PAGE {
                tracing::warn!(
                    url = %result.url,
                    chunks = chunks.len(),
                    max = MAX_CHUNKS_PER_PAGE,
                    "truncating chunks for page"
                );
                chunks.truncate(MAX_CHUNKS_PER_PAGE);
            }
            for (idx, chunk) in chunks.into_iter().enumerate() {
                all_entries.push(ChunkEntry {
                    page_id: page_id.clone(),
                    chunk_index: idx as i32,
                    chunk_text: chunk.text,
                    heading_path: chunk.heading_path,
                });
            }
        }

        if all_entries.is_empty() {
            return Ok(());
        }

        // Phase 2: Probe dimension with a small initial batch.
        let mut known_dim: Option<usize> = None;
        {
            let probe_size = all_entries.len().min(50);
            let probe_texts: Vec<String> = all_entries[..probe_size]
                .iter()
                .map(|e| e.chunk_text.clone())
                .collect();
            if let Ok(embeddings) = provider.embed_batch(&probe_texts).await {
                if let Some(first_emb) = embeddings.first() {
                    let d = first_emb.len();
                    self.client.ensure_embeddings_index(d).await?;
                    known_dim = Some(d);
                    tracing::info!(dimension = d, "auto-detected embedding dimension");
                }
            }
        }

        let total_chunks = all_entries.len();
        let embed_concurrency = config.embed_concurrency();
        let monitor = std::sync::Arc::new(EmbedMonitor::new(embed_concurrency));
        let total_inserted = std::sync::atomic::AtomicUsize::new(0);
        let progress = crate::progress::ProgressReporter::new(total_chunks as u64, "Embedding");
        let progress = std::sync::Arc::new(std::sync::Mutex::new(progress));

        // Phase 3: Embed in large batches (200 chunks per request).
        const EMBED_BATCH_SIZE: usize = 100;
        let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(embed_concurrency));
        let mut tasks = tokio::task::JoinSet::new();

        let batches: Vec<Vec<ChunkEntry>> = all_entries
            .chunks(EMBED_BATCH_SIZE)
            .map(|chunk| chunk.to_vec())
            .collect();
        eprintln!(
            "DEBUG: {} entries -> {} batches",
            all_entries.len(),
            batches.len()
        );

        let group = self.group.clone();
        let model = config.model.clone();

        for (batch_idx, batch) in batches.into_iter().enumerate() {
            let provider = provider.clone();
            let monitor = monitor.clone();
            let progress = progress.clone();
            let semaphore = semaphore.clone();
            let group = group.clone();
            let model = model.clone();

            tasks.spawn(async move {
                let _permit = semaphore.acquire().await;
                let texts: Vec<String> = batch.iter().map(|e| e.chunk_text.clone()).collect();
                let start = std::time::Instant::now();
                let embeddings = match provider.embed_batch(&texts).await {
                    Ok(e) => {
                        monitor.record_success(start.elapsed());
                        e
                    }
                    Err(e) => {
                        monitor.record_error();
                        eprintln!("WARN: embedding batch {} failed: {}", batch_idx, e);
                        return Vec::new();
                    }
                };

                let dim = known_dim
                    .unwrap_or_else(|| embeddings.first().map(|e| e.len()).unwrap_or(0))
                    as i32;

                let records: Vec<crate::db::models::ChunkRecord> = batch
                    .into_iter()
                    .zip(embeddings)
                    .map(|(entry, embedding)| crate::db::models::ChunkRecord {
                        group: group.clone(),
                        page: entry.page_id,
                        chunk_index: entry.chunk_index,
                        chunk_text: entry.chunk_text,
                        heading_path: entry.heading_path,
                        embedding,
                        model: model.clone(),
                        dimensions: dim,
                        created_at: chrono::Utc::now(),
                    })
                    .collect();

                progress.lock().unwrap().inc(records.len() as u64);
                eprintln!(
                    "DEBUG: batch {} produced {} records",
                    batch_idx,
                    records.len()
                );
                records
            });
        }

        // Phase 4: Collect all chunk records and insert per-page.
        let mut all_records: Vec<crate::db::models::ChunkRecord> = Vec::new();
        let mut last_report = std::time::Instant::now();
        let mut task_count = 0;
        while let Some(task_result) = tasks.join_next().await {
            task_count += 1;
            match task_result {
                Ok(records) => {
                    eprintln!(
                        "DEBUG: join_next returned {} records (task {})",
                        records.len(),
                        task_count
                    );
                    all_records.extend(records);
                }
                Err(e) => {
                    eprintln!("WARN: embedding task {} panicked: {}", task_count, e);
                }
            }
            if last_report.elapsed().as_secs() >= 30 {
                monitor.report();
                last_report = std::time::Instant::now();
            }
        }
        eprintln!(
            "DEBUG: collected {} records from {} tasks",
            all_records.len(),
            task_count
        );

        #[allow(clippy::mutable_key_type)]
        let mut by_page: HashMap<RecordId, Vec<crate::db::models::ChunkRecord>> = HashMap::new();
        for record in all_records {
            by_page.entry(record.page.clone()).or_default().push(record);
        }
        eprintln!("DEBUG: grouped into {} pages", by_page.len());

        let mut failed_pages = 0;
        for (page_id, records) in by_page {
            let _ = self.client.delete_chunks_for_page(&page_id).await;
            let count = records.len();
            eprintln!("DEBUG: inserting {} chunks for page {:?}", count, page_id);
            match self.client.insert_chunks(&records).await {
                Ok(()) => {
                    total_inserted.fetch_add(count, std::sync::atomic::Ordering::Relaxed);
                    eprintln!("DEBUG: inserted {} chunks OK", count);
                }
                Err(e) => {
                    failed_pages += 1;
                    eprintln!(
                        "WARN: failed to insert {} chunks for page {:?}: {}",
                        count, page_id, e
                    );
                }
            }
        }
        if failed_pages > 0 {
            eprintln!("WARN: {} pages failed chunk insertion", failed_pages);
        }

        progress.lock().unwrap().finish();
        monitor.report();
        println!(
            "Chunks inserted: {}",
            total_inserted.load(std::sync::atomic::Ordering::Relaxed)
        );
        Ok(())
    }
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
