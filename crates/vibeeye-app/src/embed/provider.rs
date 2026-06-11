//! OpenAI-compatible embedding provider.

use anyhow::Result;
use reqwest;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicUsize, Ordering};

/// HTTP client for an OpenAI-compatible embedding endpoint.
/// Supports round-robin load balancing across multiple endpoints.
#[derive(Debug)]
pub struct EmbeddingProvider {
    client: reqwest::Client,
    endpoints: Vec<String>,
    model: String,
    api_key: Option<String>,
    round_robin: AtomicUsize,
}

impl Clone for EmbeddingProvider {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            endpoints: self.endpoints.clone(),
            model: self.model.clone(),
            api_key: self.api_key.clone(),
            round_robin: AtomicUsize::new(self.round_robin.load(Ordering::Relaxed)),
        }
    }
}

#[derive(Debug, Serialize)]
struct EmbeddingRequest {
    model: String,
    input: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct EmbeddingResponse {
    data: Vec<EmbeddingData>,
}

#[derive(Debug, Deserialize)]
struct EmbeddingData {
    embedding: Vec<f32>,
}

impl EmbeddingProvider {
    pub fn new(config: &crate::config::embeddings::EmbeddingConfig) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .build()?;

        Ok(Self {
            client,
            endpoints: config.endpoints(),
            model: config.model.clone(),
            api_key: config.resolved_api_key(),
            round_robin: AtomicUsize::new(0),
        })
    }

    /// Pick the next endpoint via atomic round-robin.
    fn next_endpoint(&self) -> &str {
        let idx = self.round_robin.fetch_add(1, Ordering::Relaxed);
        &self.endpoints[idx % self.endpoints.len()]
    }

    /// Embed a batch of texts in a single HTTP request.
    /// Retries with exponential backoff on transient failures.
    pub async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        let req = EmbeddingRequest {
            model: self.model.clone(),
            input: texts.to_vec(),
        };

        let mut last_err = None;
        for attempt in 0..=3 {
            if attempt > 0 {
                let delay = std::time::Duration::from_secs(5 * (1 << (attempt - 1)));
                eprintln!(
                    "WARN: embedding batch retry {}/3 after {:?}",
                    attempt, delay
                );
                tokio::time::sleep(delay).await;
            }

            let endpoint = self.next_endpoint();
            let mut builder = self.client.post(endpoint).json(&req);
            if let Some(key) = &self.api_key {
                builder = builder.header("Authorization", format!("Bearer {}", key));
            }

            match builder.send().await {
                Ok(resp) => {
                    let status = resp.status();
                    if !status.is_success() {
                        let body = resp.text().await.unwrap_or_default();
                        last_err = Some(anyhow::anyhow!(
                            "embedding request failed: {} — {}",
                            status,
                            body
                        ));
                        continue;
                    }
                    match resp.json::<EmbeddingResponse>().await {
                        Ok(data) => {
                            let embeddings: Vec<Vec<f32>> =
                                data.data.into_iter().map(|d| d.embedding).collect();
                            return Ok(embeddings);
                        }
                        Err(e) => {
                            last_err =
                                Some(anyhow::anyhow!("failed to parse embedding response: {}", e));
                            continue;
                        }
                    }
                }
                Err(e) => {
                    last_err = Some(anyhow::anyhow!("error sending request: {}", e));
                    continue;
                }
            }
        }

        Err(last_err.unwrap_or_else(|| anyhow::anyhow!("embedding batch failed after retries")))
    }

    /// Embed a single text.
    pub async fn embed_single(&self, text: &str) -> Result<Vec<f32>> {
        let mut batch = self.embed_batch(&[text.to_string()]).await?;
        batch
            .pop()
            .ok_or_else(|| anyhow::anyhow!("empty embedding response"))
    }
}
