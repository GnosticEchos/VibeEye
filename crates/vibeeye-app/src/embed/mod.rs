//! External embedding provider client.
//!
//! Supports OpenAI-compatible HTTP endpoints (Ollama, OpenAI, vLLM, etc.).

pub mod provider;
pub use provider::EmbeddingProvider;
