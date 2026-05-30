//! OpenAI-compatible embedding provider.

use anyhow::Result;
use reqwest;
use serde::{Deserialize, Serialize};

/// HTTP client for an OpenAI-compatible embedding endpoint.
#[derive(Debug, Clone)]
pub struct EmbeddingProvider {
    client: reqwest::Client,
    endpoint: String,
    model: String,
    api_key: Option<String>,
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
            .timeout(std::time::Duration::from_secs(120))
            .build()?;

        Ok(Self {
            client,
            endpoint: config.endpoint.clone(),
            model: config.model.clone(),
            api_key: config.resolved_api_key(),
        })
    }

    /// Embed a batch of texts in a single HTTP request.
    pub async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        let req = EmbeddingRequest {
            model: self.model.clone(),
            input: texts.to_vec(),
        };

        let mut builder = self.client.post(&self.endpoint).json(&req);
        if let Some(key) = &self.api_key {
            builder = builder.header("Authorization", format!("Bearer {}", key));
        }

        let resp = builder.send().await?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "embedding request failed: {} — {}",
                status,
                body
            ));
        }

        let data: EmbeddingResponse = resp.json().await?;
        let embeddings: Vec<Vec<f32>> = data.data.into_iter().map(|d| d.embedding).collect();
        Ok(embeddings)
    }

    /// Embed a single text.
    pub async fn embed_single(&self, text: &str) -> Result<Vec<f32>> {
        let mut batch = self.embed_batch(&[text.to_string()]).await?;
        batch
            .pop()
            .ok_or_else(|| anyhow::anyhow!("empty embedding response"))
    }
}
