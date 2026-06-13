//! Hierarchical crawl configuration backed by TOML.
//!
//! Config file: `~/.config/vibe-eye/crawl.toml` (or custom via `--config`).
//!
//! # Example
//! ```toml
//! [global]
//! max_depth = 2
//! max_pages = 100
//! format = "markdown"
//! respect_robots = false
//! requests_per_second = 2.0
//! concurrency = 4
//! same_origin = true
//! timeout = 15
//! sitemap = false
//!
//! [domain."example.com"]
//! max_depth = 3
//! respect_robots = true
//!
//! [subdomain."docs.example.com"]
//! max_depth = 5
//! format = "html"
//! sitemap = true
//! ```

use crate::Result;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

/// A profile of crawl settings — every field is optional so that
/// merge semantics are simple (more-specific overrides less-specific).
#[derive(Debug, Default, Clone, Deserialize)]
pub struct CrawlProfile {
    pub max_depth: Option<u32>,
    pub max_pages: Option<usize>,
    pub format: Option<String>,
    pub respect_robots: Option<bool>,
    pub requests_per_second: Option<f64>,
    pub concurrency: Option<usize>,
    pub same_origin: Option<bool>,
    pub timeout: Option<u64>,
    pub sitemap: Option<bool>,
    pub output: Option<String>,
    /// SurrealDB connection URL (e.g. "ws://user:pass@127.0.0.1:8000",
    /// "surrealkv:///path/to/db", "mem://"). Overrides surrealdb_path.
    pub db_url: Option<String>,
    /// SurrealDB embedded storage path (e.g. "~/.local/share/vibe-eye/db").
    /// Deprecated: use db_url instead.
    pub surrealdb_path: Option<String>,
    /// SurrealDB namespace.
    pub surrealdb_ns: Option<String>,
    /// SurrealDB database name.
    pub surrealdb_db: Option<String>,
    /// Crawl group name override (default: derived from domain).
    pub group: Option<String>,
    /// Embedding provider configuration.
    pub embeddings: Option<super::embeddings::EmbeddingConfig>,
    /// CLI display configuration (help-tree styling, etc.)
    pub cli: Option<CliConfig>,
}

/// CLI-level configuration (display, theming, etc.).
#[derive(Debug, Clone, Deserialize)]
pub struct CliConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub help_tree: Option<HelpTreeConfig>,
}

/// Help-tree theming overrides per token type.
#[derive(Debug, Clone, Deserialize)]
pub struct HelpTreeConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<TextThemeConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options: Option<TextThemeConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<TextThemeConfig>,
}

/// Single token style + color override.
#[derive(Debug, Clone, Deserialize)]
pub struct TextThemeConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

macro_rules! merge_opt {
    ($self:ident, $other:ident, $field:ident) => {
        if $other.$field.is_some() {
            $self.$field = $other.$field.clone();
        }
    };
    ($self:ident, $other:ident, $field:ident, copy) => {
        if $other.$field.is_some() {
            $self.$field = $other.$field;
        }
    };
}

impl CrawlProfile {
    /// Merge `other` into `self`, with `other` taking precedence for any
    /// field that is `Some`.
    fn merge(&mut self, other: &CrawlProfile) {
        merge_opt!(self, other, max_depth, copy);
        merge_opt!(self, other, max_pages, copy);
        merge_opt!(self, other, format);
        merge_opt!(self, other, respect_robots, copy);
        merge_opt!(self, other, requests_per_second, copy);
        merge_opt!(self, other, concurrency, copy);
        merge_opt!(self, other, same_origin, copy);
        merge_opt!(self, other, timeout, copy);
        merge_opt!(self, other, sitemap, copy);
        merge_opt!(self, other, output);
        merge_opt!(self, other, db_url);
        merge_opt!(self, other, surrealdb_path);
        merge_opt!(self, other, surrealdb_ns);
        merge_opt!(self, other, surrealdb_db);
        merge_opt!(self, other, group);
        merge_opt!(self, other, embeddings);
        merge_opt!(self, other, cli);
    }
}

/// Hierarchical crawl configuration.
#[derive(Debug, Default, Deserialize)]
pub struct CrawlConfig {
    pub global: CrawlProfile,
    #[serde(default)]
    pub domain: HashMap<String, CrawlProfile>,
    #[serde(default)]
    pub subdomain: HashMap<String, CrawlProfile>,
    /// Top-level CLI theming config (preferred over global.cli).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cli: Option<CliConfig>,
}

impl CrawlConfig {
    /// Load configuration from an explicit path, or fall back to the
    /// default location (`~/.config/vibe-eye/crawl.toml`).
    pub fn load(explicit: Option<&Path>) -> Result<Self> {
        let path = match explicit {
            Some(p) => p.to_path_buf(),
            None => {
                let config_dir = dirs::config_dir().ok_or_else(|| {
                    crate::Error::InvalidInput("no config directory found".into())
                })?;
                config_dir.join("vibe-eye").join("crawl.toml")
            }
        };

        if !path.exists() {
            return Ok(Self::default());
        }

        let content = std::fs::read_to_string(&path)
            .map_err(|e| crate::Error::InvalidInput(format!("failed to read config: {e}")))?;
        let config: CrawlConfig = toml::from_str(&content)
            .map_err(|e| crate::Error::InvalidInput(format!("invalid TOML config: {e}")))?;
        Ok(config)
    }

    /// Resolve the effective profile for a given URL.
    ///
    /// Merge order: global → matching domain → matching subdomain.
    pub fn resolve(&self, url: &str) -> Result<CrawlProfile> {
        let parsed = url::Url::parse(url)
            .map_err(|e| crate::Error::InvalidInput(format!("invalid URL: {e}")))?;
        let host = parsed.host_str().unwrap_or("");

        let mut profile = self.global.clone();

        // Apply domain-level overrides (wildcard match: example.com matches *.example.com)
        for (domain_key, domain_profile) in &self.domain {
            if match_domain(host, domain_key) {
                profile.merge(domain_profile);
            }
        }

        // Apply subdomain-level overrides (exact host match)
        if let Some(sub_profile) = self.subdomain.get(host) {
            profile.merge(sub_profile);
        }

        Ok(profile)
    }
}

/// Return true when `host` belongs to `domain`.
///
/// * `example.com` matches `example.com`, `www.example.com`, `docs.example.com`
/// * `docs.example.com` matches only `docs.example.com`
fn match_domain(host: &str, domain: &str) -> bool {
    if host == domain {
        return true;
    }
    if let Some(suffix) = host.strip_prefix('.') {
        return suffix == domain;
    }
    host.ends_with(&format!(".{domain}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_match_domain() {
        assert!(match_domain("example.com", "example.com"));
        assert!(match_domain("www.example.com", "example.com"));
        assert!(match_domain("docs.example.com", "example.com"));
        assert!(!match_domain("other.com", "example.com"));
        assert!(!match_domain("example.com", "www.example.com"));
        assert!(match_domain("docs.example.com", "docs.example.com"));
    }

    #[test]
    fn test_crawl_profile_merge() {
        let mut base = CrawlProfile {
            max_depth: Some(2),
            max_pages: Some(100),
            ..Default::default()
        };
        let override_profile = CrawlProfile {
            max_depth: Some(5),
            respect_robots: Some(true),
            ..Default::default()
        };
        base.merge(&override_profile);
        assert_eq!(base.max_depth, Some(5));
        assert_eq!(base.max_pages, Some(100));
        assert_eq!(base.respect_robots, Some(true));
    }

    #[test]
    fn test_crawl_config_resolve() {
        let mut config = CrawlConfig::default();
        config.global.max_depth = Some(2);
        config.global.max_pages = Some(50);

        config.domain.insert(
            "example.com".to_string(),
            CrawlProfile {
                max_depth: Some(3),
                ..Default::default()
            },
        );

        config.subdomain.insert(
            "docs.example.com".to_string(),
            CrawlProfile {
                max_depth: Some(5),
                sitemap: Some(true),
                ..Default::default()
            },
        );

        let r1 = config.resolve("https://unknown.com/page").unwrap();
        assert_eq!(r1.max_depth, Some(2));
        assert_eq!(r1.max_pages, Some(50));

        let r2 = config.resolve("https://www.example.com/page").unwrap();
        assert_eq!(r2.max_depth, Some(3));
        assert_eq!(r2.max_pages, Some(50));

        let r3 = config.resolve("https://docs.example.com/page").unwrap();
        assert_eq!(r3.max_depth, Some(5));
        assert_eq!(r3.sitemap, Some(true));
    }
}
