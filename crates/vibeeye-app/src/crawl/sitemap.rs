//! Sitemap.xml fetcher and parser.

use quick_xml::Reader;
use quick_xml::events::Event;
use std::time::Duration;

/// Fetch and parse a sitemap.xml, returning the list of `<loc>` URLs.
/// Uses the provided base_url to construct the sitemap URL.
pub async fn fetch_sitemap(base_url: &str) -> Vec<String> {
    // Construct sitemap URL from the base URL (e.g., https://example.com/docs → https://example.com/docs/sitemap.xml)
    let sitemap_url = format!("{}/sitemap.xml", base_url.trim_end_matches('/'));
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            tracing::debug!(error = %e, "failed to create HTTP client for sitemap");
            return Vec::new();
        }
    };

    let body = match client.get(&sitemap_url).send().await {
        Ok(resp) if resp.status().is_success() => match resp.text().await {
            Ok(t) => t,
            Err(e) => {
                tracing::debug!(error = %e, sitemap_url, "failed to read sitemap response");
                return Vec::new();
            }
        },
        Ok(resp) => {
            tracing::debug!(
                status = resp.status().as_u16(),
                sitemap_url,
                "sitemap not found"
            );
            return Vec::new();
        }
        Err(e) => {
            tracing::debug!(error = %e, sitemap_url, "failed to fetch sitemap");
            return Vec::new();
        }
    };

    parse_sitemap(&body)
}

/// Parse sitemap XML text, extracting all `<loc>` values.
fn parse_sitemap(xml: &str) -> Vec<String> {
    let mut reader = Reader::from_str(xml);
    let mut urls = Vec::new();
    let mut buf = Vec::new();
    let mut in_loc = false;

    while let Ok(event) = reader.read_event_into(&mut buf) {
        if handle_event(event, &mut in_loc, &mut urls) {
            break;
        }
        buf.clear();
    }

    urls
}

fn handle_event(event: Event, in_loc: &mut bool, urls: &mut Vec<String>) -> bool {
    if let Event::Eof = event {
        return true;
    }

    if is_start_tag(&event, "loc") {
        *in_loc = true;
    } else if is_end_tag(&event, "loc") {
        *in_loc = false;
    } else if let Event::Text(e) = event {
        try_extract_url(&e, *in_loc, urls);
    }

    false
}

fn try_extract_url(text: &str, in_loc: bool, urls: &mut Vec<String>) {
    if !in_loc {
        return;
    }
    let url = text.trim().to_string();
    if !url.is_empty() {
        urls.push(url);
    }
}

fn is_start_tag(event: &Event, name: &str) -> bool {
    matches!(event, Event::Start(e) if e.name().0 == name)
}

fn is_end_tag(event: &Event, name: &str) -> bool {
    matches!(event, Event::End(e) if e.name().0 == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_sitemap() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
  <url>
    <loc>https://example.com/page1</loc>
    <lastmod>2024-01-01</lastmod>
  </url>
  <url>
    <loc>https://example.com/page2</loc>
  </url>
</urlset>"#;

        let urls = parse_sitemap(xml);
        assert_eq!(urls.len(), 2);
        assert_eq!(urls[0], "https://example.com/page1");
        assert_eq!(urls[1], "https://example.com/page2");
    }

    #[test]
    fn test_parse_sitemap_empty() {
        let urls = parse_sitemap("<urlset></urlset>");
        assert!(urls.is_empty());
    }
}
