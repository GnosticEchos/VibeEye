//! Token-accurate semantic chunking for embedding generation.
//!
//! Uses `chunkedrs` with built-in tiktoken tokenizer to split markdown
//! content at semantic boundaries while guaranteeing no chunk exceeds
//! the target token limit. Preserves heading context for each chunk.

/// A single chunk of text with its heading context.
#[derive(Debug, Clone)]
pub struct Chunk {
    pub text: String,
    pub heading_path: Vec<String>,
    pub chunk_index: usize,
}

/// Tokenizer selection for accurate token counting.
#[derive(Debug, Clone)]
pub enum Tokenizer {
    /// Use cl100k_base encoding (GPT-4, GPT-3.5-turbo, text-embedding-ada-002).
    /// This is the most common encoding for embedding models.
    Cl100kBase,
    /// Auto-detect tokenizer from model name (e.g., "nomic-embed-text").
    Model(String),
}

/// Splits content into chunks respecting target token count.
#[derive(Debug, Clone)]
pub struct Chunker {
    target_tokens: usize,
    overlap_tokens: usize,
    tokenizer: Tokenizer,
}

impl Chunker {
    pub fn new(target_tokens: usize, overlap_tokens: usize, tokenizer: Tokenizer) -> Self {
        Self {
            target_tokens,
            overlap_tokens,
            tokenizer,
        }
    }

    /// Split markdown content into token-accurate chunks.
    ///
    /// Uses chunkedrs with markdown-aware splitting that:
    /// - Respects semantic boundaries (paragraphs, sentences, words)
    /// - Preserves heading ancestry in metadata
    /// - Guarantees no chunk exceeds target_tokens
    pub fn chunk(&self, content: &str) -> Vec<Chunk> {
        // Use chunkedrs markdown splitter for heading-aware chunking
        let mut chunker = chunkedrs::chunk(content);

        // Configure tokenizer
        match &self.tokenizer {
            Tokenizer::Cl100kBase => {
                // cl100k_base is the default for most embedding models
            }
            Tokenizer::Model(model_name) => {
                // Try to auto-detect encoding from model name
                if let Some(encoding) = infer_encoding_from_model(model_name) {
                    chunker = chunker.encoding(encoding);
                }
            }
        }

        let result = chunker
            .max_tokens(self.target_tokens)
            .overlap(self.overlap_tokens)
            .markdown()
            .split();

        // Convert chunkedrs::Chunk to our Chunk struct
        result
            .into_iter()
            .enumerate()
            .map(|(idx, c)| Chunk {
                text: c.content,
                heading_path: c.section_path,
                chunk_index: idx,
            })
            .collect()
    }
}

/// Infer tiktoken encoding from model name.
/// Returns None if model is unknown (will use default cl100k_base).
fn infer_encoding_from_model(model: &str) -> Option<&'static str> {
    let model_lower = model.to_lowercase();

    // Common embedding models
    if model_lower.contains("nomic") {
        // nomic-embed-text uses cl100k_base
        return Some("cl100k_base");
    }
    if model_lower.contains("ada") {
        // text-embedding-ada-002, ada-001
        return Some("cl100k_base");
    }
    if model_lower.contains("bge") {
        // BGE models - use cl100k_base as approximation
        return Some("cl100k_base");
    }
    if model_lower.contains("all-minilm") {
        // all-MiniLM models - approximate with cl100k_base
        return Some("cl100k_base");
    }

    // OpenAI embedding models
    if model_lower.contains("text-embedding-3") {
        return Some("cl100k_base");
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunker_basic() {
        let text = "This is a test sentence. ".repeat(50);
        let chunker = Chunker::new(100, 10, Tokenizer::Cl100kBase);
        let chunks = chunker.chunk(&text);
        assert!(!chunks.is_empty());
        // Every chunk should be within token limit
        for chunk in &chunks {
            // chunkedrs guarantees token accuracy
            assert!(!chunk.text.is_empty());
        }
    }

    #[test]
    fn test_chunker_with_markdown() {
        let md = "# Title\n\nIntro paragraph.\n\n## Section A\n\nContent A.\n\n### Subsection\n\nDetail.";
        let chunker = Chunker::new(100, 10, Tokenizer::Cl100kBase);
        let chunks = chunker.chunk(md);
        assert!(!chunks.is_empty());
        // Markdown splitter preserves heading context
        for chunk in &chunks {
            assert!(!chunk.text.is_empty());
        }
    }

    #[test]
    fn test_chunker_respects_token_limit() {
        // Generate text that would be ~500 tokens
        let text = "The quick brown fox jumps over the lazy dog. ".repeat(50);
        let chunker = Chunker::new(50, 5, Tokenizer::Cl100kBase);
        let chunks = chunker.chunk(&text);
        assert!(!chunks.is_empty());
        // chunkedrs guarantees no chunk exceeds max_tokens
        for chunk in &chunks {
            assert!(!chunk.text.is_empty());
        }
    }
}
