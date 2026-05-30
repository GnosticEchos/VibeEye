//! Link extraction and URL normalization for the BFS crawler.

use scraper::{Html, Selector};
use url::Url;

/// Extract all valid `<a href="...">` links from HTML, resolving relative
/// URLs against the base and filtering out non-HTTP schemes.
pub fn extract_links(html: &str, base_url: &Url) -> Vec<String> {
    let document = Html::parse_document(html);
    let selector = Selector::parse("a[href]").expect("static selector is valid");

    let mut links = Vec::new();
    for element in document.select(&selector) {
        if let Some(href) = element.value().attr("href") {
            if let Some(absolute) = resolve_link(href, base_url) {
                links.push(absolute);
            }
        }
    }
    links
}

/// Resolve a potentially-relative link against a base URL.
///
/// Returns `None` for fragments, `javascript:`, `mailto:`, `tel:`,
/// and other non-HTTP(S) schemes.
fn resolve_link(raw: &str, base: &Url) -> Option<String> {
    if is_non_navigable(raw) {
        return None;
    }

    let resolved = base.join(raw).ok()?;
    if !is_http_scheme(resolved.scheme()) {
        return None;
    }

    Some(normalize_url(resolved.as_ref()))
}

fn is_non_navigable(raw: &str) -> bool {
    if raw.starts_with('#') || raw.is_empty() {
        return true;
    }
    let lower = raw.to_ascii_lowercase();
    lower.starts_with("javascript:")
        || lower.starts_with("mailto:")
        || lower.starts_with("tel:")
        || lower.starts_with("data:")
}

fn is_http_scheme(scheme: &str) -> bool {
    matches!(scheme.to_ascii_lowercase().as_str(), "http" | "https")
}

/// Normalize a URL for deduplication:
/// - strip fragment
/// - collapse duplicate slashes in path
/// - remove default port
/// - lowercase scheme and host
pub fn normalize_url(url: &str) -> String {
    let Ok(parsed) = Url::parse(url) else {
        return url.to_string();
    };

    let scheme = parsed.scheme().to_ascii_lowercase();
    let host = parsed.host_str().unwrap_or("").to_ascii_lowercase();

    let port = parsed.port_or_known_default();
    let port_str = match port {
        Some(80) if scheme == "http" => String::new(),
        Some(443) if scheme == "https" => String::new(),
        Some(p) => format!(":{p}"),
        None => String::new(),
    };

    let path = parsed.path();
    let path = path.replace("//", "/");

    let query = parsed.query();
    let query_str = query.map_or(String::new(), |q| format!("?{q}"));

    format!("{scheme}://{host}{port_str}{path}{query_str}")
}

/// Return true when `url` shares the same origin (scheme + host) as `origin_url`.
pub fn is_same_origin(url: &str, origin_url: &Url) -> bool {
    let Ok(parsed) = Url::parse(url) else {
        return false;
    };
    let scheme = parsed.scheme().to_ascii_lowercase();
    let host = parsed.host_str().unwrap_or("").to_ascii_lowercase();

    let origin_scheme = origin_url.scheme().to_ascii_lowercase();
    let origin_host = origin_url.host_str().unwrap_or("").to_ascii_lowercase();

    scheme == origin_scheme && host == origin_host
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_url_strips_fragment() {
        assert_eq!(
            normalize_url("https://example.com/page#section"),
            "https://example.com/page"
        );
    }

    #[test]
    fn test_normalize_url_collapses_slashes() {
        assert_eq!(
            normalize_url("https://example.com//foo//bar"),
            "https://example.com/foo/bar"
        );
    }

    #[test]
    fn test_normalize_url_lowercases_host() {
        assert_eq!(
            normalize_url("HTTPS://Example.COM/Page"),
            "https://example.com/Page"
        );
    }

    #[test]
    fn test_normalize_url_keeps_query() {
        assert_eq!(
            normalize_url("https://example.com/search?q=rust&lang=en"),
            "https://example.com/search?q=rust&lang=en"
        );
    }

    #[test]
    fn test_is_same_origin() {
        let origin = Url::parse("https://example.com").unwrap();
        assert!(is_same_origin("https://example.com/page", &origin));
        assert!(is_same_origin("https://example.com:443/page", &origin));
        assert!(!is_same_origin("https://other.com/page", &origin));
        assert!(!is_same_origin("http://example.com/page", &origin));
    }

    #[test]
    fn test_extract_links_basic() {
        let html = "
            <a href=\"/page1\">Page 1</a>
            <a href=\"https://example.com/page2\">Page 2</a>
            <a href=\"//other.com/page3\">Page 3</a>
            <a href=\"#fragment\">Skip</a>
            <a href=\"mailto:test@example.com\">Skip</a>
            <a href=\"javascript:void(0)\">Skip</a>
        ";
        let base = Url::parse("https://example.com").unwrap();
        let links = extract_links(html, &base);

        assert!(links.contains(&"https://example.com/page1".to_string()));
        assert!(links.contains(&"https://example.com/page2".to_string()));
        // protocol-relative //other.com should resolve to https://other.com
        assert!(links.contains(&"https://other.com/page3".to_string()));
        assert_eq!(links.len(), 3);
    }
}
