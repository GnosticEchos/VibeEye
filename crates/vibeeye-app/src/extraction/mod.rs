//! Content extraction from rendered DOM
//!
//! Markdown distillation and raw HTML extraction.
//! Full implementation in WP06.

use crate::Result;
use scraper::{Html, Selector};
use vibeeye_core::ContentFormat;

pub mod dom;
pub mod markdown;

/// Elements that are never page content — remove before extraction.
const NOISE_SELECTORS: &[&str] = &[
    "script",
    "style",
    "svg",
    "nav",
    "header",
    "footer",
    "aside",
    "noscript",
    "iframe",
    "canvas",
    "template",
    "img[src^=\"data:image/svg+xml\"]",
];

/// Strip non-content markup (scripts, styles, SVGs, nav, etc.)
/// before converting to text or markdown.
pub fn clean_html(html: &str) -> String {
    let mut document = Html::parse_document(html);
    for sel_str in NOISE_SELECTORS {
        let Ok(selector) = Selector::parse(sel_str) else {
            continue;
        };
        let ids: Vec<_> = document.select(&selector).map(|el| el.id()).collect();
        for id in ids {
            if let Some(mut node) = document.tree.get_mut(id) {
                node.detach();
            }
        }
    }
    document.html()
}

/// Extract content from HTML string in the requested format.
pub fn extract(html: &str, format: ContentFormat) -> Result<String> {
    match format {
        ContentFormat::Html => Ok(html.to_string()),
        ContentFormat::Text => Ok(strip_html(&clean_html(html))),
        ContentFormat::Markdown => markdown::html_to_markdown(&clean_html(html)),
    }
}

/// Simple HTML-to-text stripper.
///
/// Used for `ContentFormat::Text` — a fast, lossy conversion that
/// removes all tags and collapses whitespace.
pub fn strip_html(html: &str) -> String {
    html.split('<')
        .filter_map(|s| s.split('>').nth(1))
        .collect::<Vec<_>>()
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Extract page title from raw HTML.
///
/// Delegates to the DOM metadata extractor for robust parsing
/// (handles `<title>`, Open Graph, and JSON-LD).
pub fn extract_title(html: &str) -> Option<String> {
    dom::extract_metadata(html).ok().and_then(|meta| meta.title)
}
