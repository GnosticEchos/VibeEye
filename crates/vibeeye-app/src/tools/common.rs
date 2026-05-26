//! Common tool execution helpers

use crate::browser::BrowserSession;
use crate::{AppError, Result};

/// Result of navigating to a page and capturing basic info
#[derive(Debug)]
pub struct PageCapture {
    pub url: String,
    pub html: String,
    pub title: Option<String>,
}

/// Navigate to URL and capture page data
pub async fn navigate_and_capture(url: &str) -> Result<PageCapture> {
    let mut session = BrowserSession::new().map_err(|e| AppError::Browser(e.to_string()))?;

    session
        .navigate(url)
        .await
        .map_err(|e| AppError::Navigation(e.to_string()))?;

    let html = session
        .get_html()
        .await
        .map_err(|e| AppError::Browser(e.to_string()))?;

    let title = crate::extraction::extract_title(&html);

    let url = session.current_url().unwrap_or(url).to_string();

    session.close().await.ok();

    Ok(PageCapture { url, html, title })
}
