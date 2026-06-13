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
    "img[src*=\"analytics\"]",
    "img[src*=\"t.co\"]",
    "img[src*=\"twitter.com\"]",
    "a[rel=\"prev\"]",
    "a[rel=\"next\"]",
    "a[href*=\"/edit/\"]",
    "a[href*=\"/issues/new\"]",
    ".pagination",
    ".page-nav",
    ".article-nav",
    ".prev-next",
    "._page-navigation",
];

/// Strip non-content markup (scripts, styles, SVGs, nav, etc.)
/// before converting to text or markdown.
pub fn clean_html(html: &str) -> String {
    let mut document = Html::parse_document(html);
    for sel_str in NOISE_SELECTORS {
        let Ok(selector) = Selector::parse(sel_str) else {
            continue;
        };
        let nodes: Vec<_> = document.select(&selector).map(|el| el.id()).collect();
        for id in nodes {
            if let Some(mut node) = document.tree.get_mut(id) {
                node.detach();
            }
        }
    }
    document.html()
}

/// Extract content from HTML string in the requested format.
pub fn extract(html: &str, format: ContentFormat) -> Result<String> {
    let cleaned = clean_html(html);
    match format {
        ContentFormat::Html => Ok(html.to_string()),
        ContentFormat::Text => Ok(strip_html(&cleaned)),
        ContentFormat::Markdown => Ok(remove_nav_and_analytics_noise(&markdown::html_to_markdown(
            &cleaned,
        )?)),
    }
}

/// Simple HTML-to-text stripper.
///
/// Used for `ContentFormat::Text` — a fast, lossy conversion that
/// removes all tags and collapses whitespace.
pub fn strip_html(html: &str) -> String {
    let text = html
        .split('<')
        .filter_map(|s| s.split('>').nth(1))
        .collect::<Vec<_>>()
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    remove_nav_and_analytics_noise(&text)
}

fn remove_nav_and_analytics_noise(text: &str) -> String {
    let mut cleaned = text.to_string();

    if let Some(idx) = cleaned.find("Edit page") {
        if let Some(prev_idx) = cleaned[idx..].find("Previous") {
            cleaned.truncate(idx + prev_idx);
        }
    }

    if let Some(helpful_idx) = cleaned.find("Was this page helpful") {
        if let Some(prev_idx) = cleaned[helpful_idx..].find("Previous") {
            cleaned.truncate(helpful_idx + prev_idx);
        }
    }

    for pattern in ["analytics.twitter.com", "t.co/1/i/adsct", "analytics."] {
        while let Some(start) = cleaned.find(pattern) {
            let end = cleaned[start..]
                .find(')')
                .map(|i| start + i)
                .unwrap_or(cleaned.len());
            cleaned.replace_range(start..=end, "");
        }
    }

    cleaned.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Extract page title from raw HTML.
///
/// Delegates to the DOM metadata extractor for robust parsing
/// (handles `<title>`, Open Graph, and JSON-LD).
pub fn extract_title(html: &str) -> Option<String> {
    dom::extract_metadata(html).ok().and_then(|meta| meta.title)
}
