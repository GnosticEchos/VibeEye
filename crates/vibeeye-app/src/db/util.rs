//! Group name derivation and identifier sanitization.

/// Derive a group name from a URL, optionally overridden by user input.
pub fn derive_group(url: &str, override_name: Option<&str>) -> String {
    if let Some(name) = override_name {
        return sanitize_identifier(name);
    }
    let domain = extract_domain(url);
    sanitize_identifier(&domain)
}

/// Extract the host component from a URL.
fn extract_domain(url: &str) -> String {
    url::Url::parse(url)
        .ok()
        .and_then(|u| u.host_str().map(String::from))
        .unwrap_or_else(|| "unknown".to_string())
}

/// Sanitize a string into a valid SurrealDB identifier.
///
/// - Lowercases
/// - Replaces non-alphanumeric chars (except `_`) with `_`
/// - Collapses consecutive `_`
/// - Trims leading/trailing `_`
/// - Truncates to 64 chars
/// - Falls back to `"default"` if empty
pub fn sanitize_identifier(name: &str) -> String {
    let mut s = name
        .to_lowercase()
        .replace(|c: char| !c.is_alphanumeric() && c != '_', "_")
        .replace("__", "_")
        .trim_matches('_')
        .to_string();
    s.truncate(64);
    if s.is_empty() {
        s = "default".to_string();
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_group_from_url() {
        assert_eq!(
            derive_group("https://surrealdb.com/docs", None),
            "surrealdb_com"
        );
        assert_eq!(derive_group("https://docs.rs/tokio/1.0", None), "docs_rs");
    }

    #[test]
    fn test_derive_group_override() {
        assert_eq!(
            derive_group("https://surrealdb.com/docs", Some("docs-v2")),
            "docs_v2"
        );
    }

    #[test]
    fn test_sanitize_identifier() {
        assert_eq!(sanitize_identifier("Hello World"), "hello_world");
        assert_eq!(sanitize_identifier("a--b__c"), "a_b_c");
        assert_eq!(sanitize_identifier("___test___"), "test");
        assert_eq!(sanitize_identifier(""), "default");
    }
}
