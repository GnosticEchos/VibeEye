//! Embedding provider configuration.
//!
//! Supports OpenAI-compatible HTTP endpoints (Ollama, OpenAI, etc.).

use serde::{Deserialize, Serialize};

/// Configuration for an external embedding provider.
#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct EmbeddingConfig {
    /// Provider type — only "openai-compatible" is supported.
    pub provider: String,
    /// HTTP endpoint URL (e.g. "http://localhost:11434/v1/embeddings").
    #[serde(default)]
    pub endpoint: String,
    /// Multiple HTTP endpoint URLs for load balancing across instances.
    pub endpoints: Option<Vec<String>>,
    /// Model name (e.g. "nomic-embed-text").
    pub model: String,
    /// Vector dimension expected from this model (optional — auto-detected from server response).
    pub dimensions: Option<usize>,
    /// Max tokens per context window.
    pub context_window: usize,
    /// Optional API key — may contain `${VAR}` env references.
    pub api_key: Option<String>,
    /// Target chunk size in tokens (default: 512).
    pub chunk_size: Option<usize>,
    /// Chunk overlap in tokens (default: 50).
    pub chunk_overlap: Option<usize>,
    /// Max concurrent embedding requests (default: 4).
    pub embed_concurrency: Option<usize>,
}

impl EmbeddingConfig {
    /// Resolve API key, interpolating `${VAR}` and `$VAR` references.
    pub fn resolved_api_key(&self) -> Option<String> {
        self.api_key.as_ref().map(|k| interpolate_env_vars(k))
    }

    /// Target chunk size with default.
    pub fn target_chunk_size(&self) -> usize {
        self.chunk_size.unwrap_or(512)
    }

    /// Chunk overlap with default.
    pub fn chunk_overlap(&self) -> usize {
        self.chunk_overlap.unwrap_or(50)
    }

    /// Embedding concurrency with default.
    pub fn embed_concurrency(&self) -> usize {
        self.embed_concurrency.unwrap_or(4).max(1)
    }

    /// Return all endpoint URLs for load balancing.
    /// If `endpoints` is set, returns those; otherwise returns a single-element
    /// vec containing `endpoint`.
    pub fn endpoints(&self) -> Vec<String> {
        if let Some(eps) = &self.endpoints {
            if !eps.is_empty() {
                return eps.clone();
            }
        }
        vec![self.endpoint.clone()]
    }
}

/// Replace `${VAR}` and `$VAR` with environment variable values.
pub fn interpolate_env_vars(value: &str) -> String {
    replace_plain_vars(&replace_braced_vars(value))
}

fn replace_braced_vars(value: &str) -> String {
    let mut result = value.to_string();
    loop {
        if let Some(start) = result.find("${") {
            if let Some(end) = result[start..].find('}') {
                let var_name = &result[start + 2..start + end];
                let full = &result[start..start + end + 1];
                let replacement = std::env::var(var_name).unwrap_or_default();
                result = result.replacen(full, &replacement, 1);
                continue;
            }
        }
        break;
    }
    result
}

fn replace_plain_vars(value: &str) -> String {
    let mut output = String::new();
    let mut chars = value.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '$' {
            let mut var_name = String::new();
            while let Some(&next) = chars.peek() {
                if next.is_alphanumeric() || next == '_' {
                    var_name.push(next);
                    chars.next();
                } else {
                    break;
                }
            }
            if var_name.is_empty() {
                output.push('$');
            } else {
                output.push_str(&std::env::var(&var_name).unwrap_or_default());
            }
        } else {
            output.push(ch);
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interpolate_env_vars_brace() {
        unsafe {
            std::env::set_var("TEST_KEY", "secret123");
        }
        assert_eq!(
            interpolate_env_vars("Bearer ${TEST_KEY}"),
            "Bearer secret123"
        );
    }

    #[test]
    fn test_interpolate_env_vars_plain() {
        unsafe {
            std::env::set_var("API_HOST", "localhost");
        }
        assert_eq!(
            interpolate_env_vars("http://$API_HOST:11434"),
            "http://localhost:11434"
        );
    }

    #[test]
    fn test_interpolate_missing_var() {
        unsafe {
            std::env::remove_var("DEFINITELY_NOT_SET_VAR_42");
        }
        assert_eq!(
            interpolate_env_vars("key=${DEFINITELY_NOT_SET_VAR_42}"),
            "key="
        );
    }
}
