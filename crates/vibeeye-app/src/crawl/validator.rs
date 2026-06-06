//! Page validation layer — filters out error pages, soft-404s, and low-quality content.

use crate::tools::common::PageCapture;

/// Configurable rules for rejecting pages before persistence.
#[derive(Debug, Clone)]
pub struct PageValidator {
    /// Reject pages with HTTP status >= 400.
    pub reject_4xx: bool,
    /// Reject pages with HTTP status >= 500.
    pub reject_5xx: bool,
    /// Title substrings that indicate a soft-404 (200 OK but not found).
    pub soft_404_patterns: Vec<String>,
    /// Minimum content length (in chars) after extraction.
    pub min_content_length: usize,
    /// Reject pages with `<meta name="robots" content="noindex">`.
    pub check_robots_noindex: bool,
}

impl Default for PageValidator {
    fn default() -> Self {
        Self {
            reject_4xx: true,
            reject_5xx: true,
            soft_404_patterns: vec![
                "page not found".to_string(),
                "file not found".to_string(),
                "not found".to_string(),
                "no such".to_string(),
                "404".to_string(),
            ],
            min_content_length: 50,
            check_robots_noindex: true,
        }
    }
}

impl PageValidator {
    /// Validate a captured page. Returns `Ok(())` if the page passes all checks,
    /// or `Err(reason)` describing the first failure.
    pub fn validate(&self, capture: &PageCapture) -> Result<(), String> {
        // 1. HTTP status code check
        if let Some(status) = capture.http_status {
            if self.reject_4xx && (400..500).contains(&status) {
                return Err(format!("HTTP {status}"));
            }
            if self.reject_5xx && status >= 500 {
                return Err(format!("HTTP {status}"));
            }
        }

        // 2. Soft-404 detection via title
        if let Some(ref title) = capture.title {
            let lower = title.to_lowercase();
            for pat in &self.soft_404_patterns {
                if lower.contains(pat) {
                    return Err(format!("soft 404 (title contains \"{pat}\")"));
                }
            }
        }

        // 3. Content length check
        let text_len = capture.html.len();
        if text_len < self.min_content_length {
            return Err(format!(
                "content too short ({text_len} chars, min {})",
                self.min_content_length
            ));
        }

        // 4. Robots noindex check
        if self.check_robots_noindex {
            let lower = capture.html.to_lowercase();
            if lower.contains("noindex") {
                // Narrow check: look for meta robots tag
                if lower.contains("<meta") && lower.contains("robots") {
                    return Err("robots noindex".to_string());
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_capture(title: &str, html: &str, status: Option<u16>) -> PageCapture {
        PageCapture {
            url: "https://example.com".to_string(),
            html: html.to_string(),
            title: Some(title.to_string()),
            http_status: status,
            local_storage: None,
        }
    }

    #[test]
    fn test_rejects_404_status() {
        let v = PageValidator::default();
        let cap = make_capture("OK", "<html><body>content</body></html>", Some(404));
        assert_eq!(v.validate(&cap), Err("HTTP 404".to_string()));
    }

    #[test]
    fn test_rejects_500_status() {
        let v = PageValidator::default();
        let cap = make_capture("OK", "<html><body>content</body></html>", Some(500));
        assert_eq!(v.validate(&cap), Err("HTTP 500".to_string()));
    }

    #[test]
    fn test_accepts_200_status() {
        let v = PageValidator::default();
        let cap = make_capture(
            "OK",
            "<html><body>this is a real page with plenty of content here that exceeds fifty characters</body></html>",
            Some(200),
        );
        assert!(v.validate(&cap).is_ok());
    }

    #[test]
    fn test_rejects_soft_404_title() {
        let v = PageValidator::default();
        let cap = make_capture(
            "Page not found · GitHub Pages",
            "<html><body>content</body></html>",
            Some(200),
        );
        assert_eq!(
            v.validate(&cap),
            Err("soft 404 (title contains \"page not found\")".to_string())
        );
    }

    #[test]
    fn test_rejects_short_content() {
        let v = PageValidator::default();
        let cap = make_capture("OK", "hi", Some(200));
        assert_eq!(
            v.validate(&cap),
            Err("content too short (2 chars, min 50)".to_string())
        );
    }

    #[test]
    fn test_rejects_noindex() {
        let v = PageValidator::default();
        let cap = make_capture(
            "OK",
            r#"<html><head><meta name="robots" content="noindex"></head><body>content</body></html>"#,
            Some(200),
        );
        assert_eq!(v.validate(&cap), Err("robots noindex".to_string()));
    }

    #[test]
    fn test_accepts_valid_page() {
        let v = PageValidator::default();
        let cap = make_capture(
            "Good Page",
            "<html><body>this is a real page with plenty of content here</body></html>",
            Some(200),
        );
        assert!(v.validate(&cap).is_ok());
    }
}
