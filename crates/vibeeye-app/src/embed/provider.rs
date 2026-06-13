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
    retry_delay: fn(usize) -> std::time::Duration,
}

impl Clone for EmbeddingProvider {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            endpoints: self.endpoints.clone(),
            model: self.model.clone(),
            api_key: self.api_key.clone(),
            round_robin: AtomicUsize::new(self.round_robin.load(Ordering::Relaxed)),
            retry_delay: self.retry_delay,
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

fn default_retry_delay(attempt: usize) -> std::time::Duration {
    std::time::Duration::from_secs(5 * (1 << (attempt - 1)))
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
            retry_delay: default_retry_delay,
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
                let delay = (self.retry_delay)(attempt);
                if delay > std::time::Duration::ZERO {
                    tracing::warn!(attempt, ?delay, "embedding batch retry");
                    tokio::time::sleep(delay).await;
                }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::{TcpListener, TcpStream};
    use tokio::task::JoinHandle;

    fn test_provider(endpoints: Vec<String>, api_key: Option<&str>) -> EmbeddingProvider {
        EmbeddingProvider {
            client: reqwest::Client::builder().no_proxy().build().unwrap(),
            endpoints,
            model: "test-model".to_string(),
            api_key: api_key.map(str::to_string),
            round_robin: AtomicUsize::new(0),
            retry_delay: |_| Duration::ZERO,
        }
    }

    async fn read_request(stream: &mut TcpStream) -> String {
        let mut buf = Vec::new();
        let mut chunk = [0; 1024];

        loop {
            let n = stream.read(&mut chunk).await.unwrap();
            if n == 0 {
                break;
            }
            buf.extend_from_slice(&chunk[..n]);

            if let Some(header_end) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                let header_end = header_end + 4;
                let headers = String::from_utf8_lossy(&buf[..header_end]);
                let content_length = headers
                    .lines()
                    .find_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        if name.eq_ignore_ascii_case("content-length") {
                            value.trim().parse::<usize>().ok()
                        } else {
                            None
                        }
                    })
                    .unwrap_or(0);

                if buf.len() >= header_end + content_length {
                    break;
                }
            }
        }

        String::from_utf8_lossy(&buf).into_owned()
    }

    async fn write_response(stream: &mut TcpStream, status: &str, body: &str) {
        let body_bytes = body.as_bytes();
        let response = format!(
            "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
            body_bytes.len()
        );
        stream.write_all(response.as_bytes()).await.unwrap();
        stream.write_all(body_bytes).await.unwrap();
        stream.flush().await.unwrap();
    }

    async fn spawn_success_server(body: &'static str) -> (String, JoinHandle<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let handle = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let request = read_request(&mut stream).await;
            write_response(&mut stream, "200 OK", body).await;
            request
        });

        (format!("http://{addr}"), handle)
    }

    async fn spawn_status_server(
        status: &'static str,
        body: &'static str,
        attempts: usize,
    ) -> (String, JoinHandle<Vec<String>>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let handle = tokio::spawn(async move {
            let mut requests = Vec::new();
            for _ in 0..attempts {
                let (mut stream, _) = listener.accept().await.unwrap();
                requests.push(read_request(&mut stream).await);
                write_response(&mut stream, status, body).await;
            }
            requests
        });

        (format!("http://{addr}"), handle)
    }

    #[tokio::test]
    async fn embed_batch_empty_returns_empty() {
        let provider = test_provider(vec!["http://127.0.0.1:9".to_string()], None);

        let embeddings = provider.embed_batch(&[]).await.unwrap();

        assert!(embeddings.is_empty());
    }

    #[tokio::test]
    async fn embed_batch_success_maps_response_and_sends_auth() {
        let (endpoint, handle) =
            spawn_success_server(r#"{"data":[{"embedding":[1.0,2.0]},{"embedding":[3.0,4.0]}]}"#)
                .await;
        let provider = test_provider(vec![endpoint], Some("secret"));

        let embeddings = provider
            .embed_batch(&["hello".to_string(), "world".to_string()])
            .await
            .unwrap();
        let request = handle.await.unwrap();

        assert_eq!(embeddings, vec![vec![1.0, 2.0], vec![3.0, 4.0]]);
        let request_lower = request.to_ascii_lowercase();
        assert!(request.contains("POST / HTTP/1.1"));
        assert!(
            request_lower.contains("authorization: bearer secret"),
            "missing auth header in request:\n{request}"
        );
        assert!(request.contains(r#""model":"test-model""#));
        assert!(request.contains(r#""input":["hello","world"]"#));
    }

    #[tokio::test]
    async fn embed_batch_retries_non_success_and_preserves_body() {
        let (endpoint, handle) =
            spawn_status_server("500 Internal Server Error", "bad gateway body", 4).await;
        let provider = test_provider(vec![endpoint], None);

        let err = provider
            .embed_batch(&["hello".to_string()])
            .await
            .unwrap_err();
        let requests = handle.await.unwrap();

        assert_eq!(requests.len(), 4);
        assert!(
            err.to_string()
                .contains("embedding request failed: 500 Internal Server Error")
        );
        assert!(err.to_string().contains("bad gateway body"));
    }
}
