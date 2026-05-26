//! High-fidelity Markdown distillation from rendered HTML.
//!
//! Uses `html-to-markdown-rs` (astral-tl parser) for robust heading/link/list/table/code
//! conversion. Replaces the primitive `strip_html` stub from pre-WP06.

use crate::{AppError, Result};
use vibeeye_core::VibeError;

/// Convert raw HTML into clean Markdown.
///
/// Handles headings, paragraphs, links, lists, tables, code blocks,
/// blockquotes and inline emphasis.
pub fn html_to_markdown(html: &str) -> Result<String> {
    let result = html_to_markdown_rs::convert(html, None)
        .map_err(|e| AppError::Core(VibeError::Extraction(format!("markdown conversion: {e}"))))?;

    Ok(result.content.unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heading_and_paragraph() {
        let html = "<h1>Hello</h1><p>World</p>";
        let md = html_to_markdown(html).unwrap();
        assert!(md.contains("# Hello"));
        assert!(md.contains("World"));
    }

    #[test]
    fn test_link() {
        let html = r#"<a href="https://example.com">click me</a>"#;
        let md = html_to_markdown(html).unwrap();
        assert!(md.contains("[click me](https://example.com)"));
    }

    #[test]
    fn test_list() {
        let html = "<ul><li>one</li><li>two</li></ul>";
        let md = html_to_markdown(html).unwrap();
        assert!(md.contains("- one"));
        assert!(md.contains("- two"));
    }

    #[test]
    fn test_code_block() {
        let html = "<pre><code>fn main() {}</code></pre>";
        let md = html_to_markdown(html).unwrap();
        assert!(md.contains("```"));
        assert!(md.contains("fn main()"));
    }
}
