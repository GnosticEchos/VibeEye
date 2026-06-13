//! Common tool execution helpers

use crate::browser::BrowserSession;
use crate::{Error, Result};
use std::collections::HashMap;
use std::time::Duration;

/// Result of navigating to a page and capturing basic info
#[derive(Debug, Clone)]
pub struct PageCapture {
    pub url: String,
    pub html: String,
    pub title: Option<String>,
    pub http_status: Option<u16>,
    pub local_storage: Option<HashMap<String, String>>,
}

/// Use a lightweight HTTP GET to follow redirects and discover the final URL.
/// This compensates for Servo headless mode not updating `window.location.href`
/// after server-side redirects, which breaks relative link resolution.
pub(crate) async fn resolve_redirect_url(url: &str) -> Option<String> {
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::limited(10))
        .timeout(Duration::from_secs(5))
        .build()
        .ok()?;
    let response = client.get(url).send().await.ok()?;
    let final_url = response.url().to_string();
    let original = url::Url::parse(url).ok()?;
    let resolved = url::Url::parse(&final_url).ok()?;
    if resolved != original {
        return Some(final_url);
    }
    None
}

/// Navigate to URL and capture page data
pub async fn navigate_and_capture(url: &str) -> Result<PageCapture> {
    let resolved_url = resolve_redirect_url(url)
        .await
        .unwrap_or_else(|| url.to_string());

    let mut session = BrowserSession::new().map_err(|e| Error::Browser(e.to_string()))?;

    session
        .navigate(&resolved_url)
        .await
        .map_err(|e| Error::Navigation(e.to_string()))?;

    let mut html = session
        .get_html()
        .await
        .map_err(|e| Error::Browser(e.to_string()))?;

    if html.to_lowercase().contains("<script") {
        html = session
            .settle_and_get_html(2000)
            .await
            .map_err(|e| Error::Browser(e.to_string()))?;
    }

    let title = crate::extraction::extract_title(&html);

    let url = session.current_url().unwrap_or(&resolved_url).to_string();

    session.close().await.ok();

    Ok(PageCapture {
        url,
        html,
        title,
        http_status: None,
        local_storage: None,
    })
}
