//! Minimal robots.txt parser.
//!
//! Handles `User-agent: *`, `Disallow:`, and `Allow:` directives.
//! Falls back to "allow all" if the fetch fails.

use std::time::Duration;

/// Parsed robots.txt rules.
#[derive(Debug, Default, Clone)]
pub struct RobotsTxt {
    rules: Vec<Rule>,
}

#[derive(Debug, Clone)]
struct Rule {
    path: String,
    allowed: bool,
}

impl RobotsTxt {
    /// Fetch and parse robots.txt for the given origin.
    pub async fn fetch(origin: &str) -> Self {
        let robots_url = format!("{origin}/robots.txt");
        let client = match reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
        {
            Ok(c) => c,
            Err(_) => return Self::default(),
        };

        match client.get(&robots_url).send().await {
            Ok(resp) if resp.status().is_success() => match resp.text().await {
                Ok(body) => Self::parse(&body),
                Err(_) => Self::default(),
            },
            _ => Self::default(),
        }
    }

    /// Parse robots.txt text.
    pub fn parse(text: &str) -> Self {
        let mut rules = Vec::new();
        let mut in_wildcard_block = false;

        for line in text.lines() {
            if let Some(rule) = parse_line(line, &mut in_wildcard_block) {
                rules.push(rule);
            }
        }

        Self { rules }
    }

    /// Return true if the given path is allowed.
    pub fn is_allowed(&self, path: &str) -> bool {
        if self.rules.is_empty() {
            return true;
        }

        let mut allowed = true;
        let mut longest_match = 0;

        for rule in &self.rules {
            if path.starts_with(&rule.path) && rule.path.len() >= longest_match {
                longest_match = rule.path.len();
                allowed = rule.allowed;
            }
        }

        allowed
    }
}

fn parse_line(line: &str, in_wildcard_block: &mut bool) -> Option<Rule> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return None;
    }

    let lower = line.to_ascii_lowercase();
    if lower.starts_with("user-agent:") {
        let ua = line[11..].trim();
        *in_wildcard_block = ua == "*";
        return None;
    }

    if !*in_wildcard_block {
        return None;
    }

    if lower.starts_with("disallow:") {
        let path = line[9..].trim().to_string();
        return Some(Rule {
            path,
            allowed: false,
        });
    }

    if lower.starts_with("allow:") {
        let path = line[6..].trim().to_string();
        return Some(Rule {
            path,
            allowed: true,
        });
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_robots_allow_all_when_empty() {
        let robots = RobotsTxt::default();
        assert!(robots.is_allowed("/page"));
    }

    #[test]
    fn test_robots_disallow() {
        let robots = RobotsTxt::parse(
            "User-agent: *\nDisallow: /private/\nDisallow: /admin\n",
        );
        assert!(!robots.is_allowed("/private/page"));
        assert!(!robots.is_allowed("/admin"));
        assert!(robots.is_allowed("/public"));
    }

    #[test]
    fn test_robots_allow_override() {
        let robots = RobotsTxt::parse(
            "User-agent: *\nDisallow: /\nAllow: /public/\n",
        );
        assert!(!robots.is_allowed("/secret"));
        assert!(robots.is_allowed("/public/page"));
    }

    #[test]
    fn test_robots_only_wildcard_block() {
        let robots = RobotsTxt::parse(
            "User-agent: BotA\nDisallow: /\nUser-agent: *\nDisallow: /admin\n",
        );
        assert!(!robots.is_allowed("/admin"));
        assert!(robots.is_allowed("/page"));
    }
}
