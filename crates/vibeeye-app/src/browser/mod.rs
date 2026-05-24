//! Browser engine abstraction
//!
//! Thin wrapper around Servo 0.1.0 for headless rendering.
//! Actual Servo integration happens in WP05.

use crate::Result;
use vibeeye_core::{BrowserContext, NavigationState};

/// Browser session handle
#[derive(Debug)]
pub struct BrowserSession {
    _context: BrowserContext,
    nav_state: NavigationState,
}

impl BrowserSession {
    /// Create a new browser session with default headless context
    pub fn new() -> Self {
        Self {
            _context: BrowserContext::default(),
            nav_state: NavigationState::default(),
        }
    }

    /// Navigate to a URL
    pub async fn navigate(&mut self, url: &str) -> Result<()> {
        self.nav_state.pending_url = Some(url.to_string());
        // Stub: actual Servo integration in WP05
        self.nav_state.current_url = Some(url.to_string());
        self.nav_state.history_stack.push(url.to_string());
        self.nav_state.pending_url = None;
        Ok(())
    }

    /// Get current URL
    pub fn current_url(&self) -> Option<&str> {
        self.nav_state.current_url.as_deref()
    }

    /// Get page content as raw HTML
    pub async fn get_html(&self) -> Result<String> {
        // Stub: returns placeholder until WP05
        Ok(format!(
            "<html><head><title>VibeEye</title></head><body>Navigated to: {}</body></html>",
            self.current_url().unwrap_or("unknown")
        ))
    }

    /// Get page content as text
    pub async fn get_text(&self) -> Result<String> {
        let html = self.get_html().await?;
        // Simple HTML strip for stub
        Ok(html.replace(['<', '>', '/'], " "))
    }

    /// Close the browser session
    pub async fn close(self) -> Result<()> {
        Ok(())
    }
}

impl Default for BrowserSession {
    fn default() -> Self {
        Self::new()
    }
}
