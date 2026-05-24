//! Content extraction from rendered DOM
//!
//! Markdown distillation and raw HTML extraction.
//! Full implementation in WP06.

use crate::Result;
use vibeeye_core::ContentFormat;

/// Extract content from HTML string
pub fn extract(html: &str, format: ContentFormat) -> Result<String> {
    match format {
        ContentFormat::Html => Ok(html.to_string()),
        ContentFormat::Text => Ok(strip_html(html)),
        ContentFormat::Markdown => Ok(html_to_markdown_stub(html)),
    }
}

/// Simple HTML-to-text stripper
pub fn strip_html(html: &str) -> String {
    html.split('<')
        .filter_map(|s| s.split('>').nth(1))
        .collect::<Vec<_>>()
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Stub markdown converter (full implementation in WP06)
fn html_to_markdown_stub(html: &str) -> String {
    // Simple conversion: just return text for now
    strip_html(html)
}

/// Extract page metadata (title, etc.)
pub fn extract_title(html: &str) -> Option<String> {
    html.find("<title>").and_then(|start| {
        html[start + 7..]
            .find("</title>")
            .map(|end| html[start + 7..start + 7 + end].to_string())
    })
}
