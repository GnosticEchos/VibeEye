//! Structured DOM metadata and table extraction.
//!
//! Uses `html-to-markdown-rs` to parse HTML once and pull out semantic
//! structures (title, tables, headings hierarchy) without re-parsing.

use crate::{AppError, Result};
use vibeeye_core::VibeError;

/// Metadata extracted from an HTML document.
#[derive(Debug, Default, Clone)]
pub struct PageMetadata {
    pub title: Option<String>,
    pub description: Option<String>,
    pub canonical_url: Option<String>,
    pub og_image: Option<String>,
}

/// Extract metadata from raw HTML.
pub fn extract_metadata(html: &str) -> Result<PageMetadata> {
    let result = html_to_markdown_rs::convert(html, None)
        .map_err(|e| AppError::Core(VibeError::Extraction(format!("metadata extraction: {e}"))))?;

    let meta = result.metadata.document;
    Ok(PageMetadata {
        title: meta.title.filter(|s: &String| !s.is_empty()),
        description: meta.description.filter(|s: &String| !s.is_empty()),
        canonical_url: meta.canonical_url.filter(|s: &String| !s.is_empty()),
        og_image: meta.open_graph.get("og:image").cloned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_title_extraction() {
        let html = "<html><head><title>Hello World</title></head><body></body></html>";
        let meta = extract_metadata(html).unwrap();
        assert_eq!(meta.title, Some("Hello World".to_string()));
    }

    #[test]
    fn test_empty_html() {
        let html = "";
        let meta = extract_metadata(html).unwrap();
        assert_eq!(meta.title, None);
    }
}
