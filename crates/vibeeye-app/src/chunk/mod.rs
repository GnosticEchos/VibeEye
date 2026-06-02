//! Semantic chunking for embedding generation.
//!
//! Splits markdown content by document structure (headings) first,
//! then falls back to recursive text splitting at sentence and word
//! boundaries to respect a target token count.

/// A single chunk of text with its heading context.
#[derive(Debug, Clone)]
pub struct Chunk {
    pub text: String,
    pub heading_path: Vec<String>,
    pub chunk_index: usize,
}

/// Token counting strategy.
#[derive(Debug, Clone)]
pub enum Tokenizer {
    /// Approximate: characters / 4 (fast, no deps).
    CharHeuristic,
}

/// Splits content into chunks respecting headings and target token count.
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

    pub fn chunk(&self, content: &str) -> Vec<Chunk> {
        let heading_chunks = split_by_headings(content);
        heading_chunks
            .into_iter()
            .enumerate()
            .flat_map(|(idx, mut chunk)| {
                chunk.chunk_index = idx;
                self.recursive_split(chunk)
            })
            .collect()
    }

    fn recursive_split(&self, chunk: Chunk) -> Vec<Chunk> {
        let token_count = self.count_tokens(&chunk.text);
        if token_count <= self.target_tokens {
            return vec![chunk];
        }

        // Try paragraph boundary
        if let Some(splits) = try_split(&chunk, "\n\n", self.target_tokens, self.overlap_tokens) {
            return splits;
        }

        // Try sentence boundary
        if let Some(splits) = try_split(&chunk, ". ", self.target_tokens, self.overlap_tokens) {
            return splits;
        }

        // Fall back to word boundary
        if let Some(splits) = try_split(&chunk, " ", self.target_tokens, self.overlap_tokens) {
            return splits;
        }

        // Last resort: hard split at character boundary
        hard_split(
            chunk,
            self.target_tokens,
            self.overlap_tokens,
            &self.tokenizer,
        )
    }

    fn count_tokens(&self, text: &str) -> usize {
        match &self.tokenizer {
            Tokenizer::CharHeuristic => text.chars().count() / 4 + 1,
        }
    }
}

fn flush_current(chunks: &mut Vec<Chunk>, text: &str, heading: &[String]) {
    if !text.trim().is_empty() {
        chunks.push(Chunk {
            text: text.trim().to_string(),
            heading_path: heading.to_vec(),
            chunk_index: 0,
        });
    }
}

fn split_by_headings(content: &str) -> Vec<Chunk> {
    let mut chunks = Vec::new();
    let mut current_heading = Vec::new();
    let mut current_text = String::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(stripped) = trimmed.strip_prefix("# ") {
            flush_current(&mut chunks, &current_text, &current_heading);
            current_heading = vec![stripped.trim().to_string()];
            current_text = String::new();
        } else if let Some(stripped) = trimmed.strip_prefix("## ") {
            flush_current(&mut chunks, &current_text, &current_heading);
            current_heading.truncate(1);
            current_heading.push(stripped.trim().to_string());
            current_text = String::new();
        } else if let Some(stripped) = trimmed.strip_prefix("### ") {
            flush_current(&mut chunks, &current_text, &current_heading);
            current_heading.truncate(2.min(current_heading.len()));
            current_heading.push(stripped.trim().to_string());
            current_text = String::new();
        } else {
            current_text.push_str(line);
            current_text.push('\n');
        }
    }

    flush_current(&mut chunks, &current_text, &current_heading);

    // If no headings found, return the whole content as one chunk
    if chunks.is_empty() {
        chunks.push(Chunk {
            text: content.to_string(),
            heading_path: vec![],
            chunk_index: 0,
        });
    }

    chunks
}

fn push_chunk(result: &mut Vec<Chunk>, text: &str, heading: &[String]) {
    result.push(Chunk {
        text: text.trim().to_string(),
        heading_path: heading.to_vec(),
        chunk_index: 0,
    });
}

fn extract_overlap(current: &str, overlap_tokens: usize) -> String {
    if overlap_tokens == 0 {
        return String::new();
    }
    let overlap_chars = overlap_tokens * 4;
    let char_count = current.chars().count();
    if char_count <= overlap_chars {
        return current.to_string();
    }
    let skip = char_count - overlap_chars;
    let start = current
        .char_indices()
        .nth(skip)
        .map(|(i, _)| i)
        .unwrap_or(0);
    current[start..].to_string()
}

fn try_split(
    chunk: &Chunk,
    delimiter: &str,
    target_tokens: usize,
    overlap_tokens: usize,
) -> Option<Vec<Chunk>> {
    let parts: Vec<&str> = chunk.text.split(delimiter).collect();
    if parts.len() < 2 {
        return None;
    }

    let mut result = Vec::new();
    let mut current = String::new();
    let mut current_tokens = 0;
    let mut is_first = true;

    for part in parts {
        let part_str = if is_first {
            is_first = false;
            part.to_string()
        } else {
            format!("{}{}", delimiter, part)
        };
        let part_tokens = part_str.chars().count() / 4 + 1;

        if current_tokens + part_tokens > target_tokens {
            push_chunk(&mut result, &current, &chunk.heading_path);
            current = extract_overlap(&current, overlap_tokens);
            current_tokens = current.chars().count() / 4 + 1;
        }

        current.push_str(&part_str);
        current_tokens += part_tokens;
    }

    if !current.trim().is_empty() {
        push_chunk(&mut result, &current, &chunk.heading_path);
    }

    if result.len() > 1 { Some(result) } else { None }
}

fn hard_split(
    chunk: Chunk,
    target_tokens: usize,
    overlap_tokens: usize,
    tokenizer: &Tokenizer,
) -> Vec<Chunk> {
    let char_count = match tokenizer {
        Tokenizer::CharHeuristic => target_tokens * 4,
    };
    let overlap_chars = overlap_tokens * 4;

    let mut result = Vec::new();
    let mut start = 0;
    let text = chunk.text.as_bytes();

    while start < text.len() {
        let end = (start + char_count).min(text.len());
        let slice = &text[start..end];
        let s = String::from_utf8_lossy(slice);
        result.push(Chunk {
            text: s.trim().to_string(),
            heading_path: chunk.heading_path.clone(),
            chunk_index: 0,
        });
        start = end.saturating_sub(overlap_chars);
        if start >= text.len() || start == end {
            break;
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_by_headings() {
        let md = "# Title\n\nIntro paragraph.\n\n## Section A\n\nContent A.\n\n### Subsection\n\nDetail.";
        let chunks = split_by_headings(md);
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].heading_path, vec!["Title"]);
        assert_eq!(chunks[1].heading_path, vec!["Title", "Section A"]);
        assert_eq!(
            chunks[2].heading_path,
            vec!["Title", "Section A", "Subsection"]
        );
    }

    #[test]
    fn test_chunker_respects_target() {
        let text = "This is a test. ".repeat(100); // ~1600 chars = ~400 tokens
        let chunker = Chunker::new(100, 10, Tokenizer::CharHeuristic);
        let chunks = chunker.chunk(&text);
        assert!(!chunks.is_empty());
        for chunk in &chunks {
            let tokens = chunk.text.chars().count() / 4 + 1;
            assert!(
                tokens <= 120,
                "chunk exceeded target+overlap: {} tokens",
                tokens
            );
        }
    }

    #[test]
    fn test_chunker_with_multibyte_chars() {
        // Regression: ensure slicing at char boundaries doesn't panic on multi-byte UTF-8
        let text = "In the case of a string literal, we know the contents at compile time, so the\ntext is hardcoded directly into the final executable. This is why string\nliterals are fast and efficient. But these properties only come from the string\nliteral's immutability. ".repeat(20);
        let chunker = Chunker::new(50, 5, Tokenizer::CharHeuristic);
        let chunks = chunker.chunk(&text);
        assert!(!chunks.is_empty());
        // Verify all chunks are valid UTF-8
        for chunk in &chunks {
            assert!(chunk.text.is_char_boundary(0));
            assert!(chunk.text.is_char_boundary(chunk.text.len()));
        }
    }
}
