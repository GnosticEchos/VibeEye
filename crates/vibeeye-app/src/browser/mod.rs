//! Browser engine abstraction
//!
//! Thin wrapper around Servo 0.1.0 for headless rendering.

use crate::Result;
use vibeeye_core::NavigationState;

pub mod engine;
pub mod navigation;

use engine::ServoEngine;

/// Browser session handle
///
/// Each session owns a dedicated `ServoEngine` thread when the engine is
/// available. If Servo cannot be initialised (e.g. missing Mesa drivers
/// or running in a test environment) the session transparently falls back
/// to a stub backend. Sessions are intentionally short-lived: one session
/// per tool invocation.
pub struct BrowserSession {
    engine: Option<ServoEngine>,
    nav_state: NavigationState,
}

impl BrowserSession {
    /// Create a new browser session with default headless viewport.
    pub fn new() -> Result<Self> {
        // In tests we always use the stub backend.  Servo contains
        // process-wide singletons (Opts, rustls CryptoProvider) and
        // background threads that cannot be reliably torn down between
        // tests, so attempting real initialisation causes hangs and
        // cross-test pollution.
        #[cfg(test)]
        let engine = None;

        #[cfg(not(test))]
        let engine = match ServoEngine::new(vibeeye_core::Viewport::default()) {
            Ok(engine) => {
                tracing::debug!("Servo engine initialised");
                Some(engine)
            }
            Err(e) => {
                tracing::warn!("Servo engine unavailable, using stub backend: {e}");
                None
            }
        };

        Ok(Self {
            engine,
            nav_state: NavigationState::default(),
        })
    }

    /// Navigate to a URL and wait for the page to finish loading.
    pub async fn navigate(&mut self, url: &str) -> Result<()> {
        self.nav_state.pending_url = Some(url.to_string());

        if let Some(ref engine) = self.engine {
            let final_url = engine
                .navigate(url)
                .await
                .map_err(|e| crate::AppError::Navigation(e.to_string()))?;
            self.nav_state.current_url = Some(final_url);
        } else {
            // Stub: no real navigation, just record the URL
            self.nav_state.current_url = Some(url.to_string());
            self.nav_state.history_stack.push(url.to_string());
        }

        self.nav_state.pending_url = None;
        Ok(())
    }

    /// Get the current URL, if any.
    pub fn current_url(&self) -> Option<&str> {
        self.nav_state.current_url.as_deref()
    }

    /// Get page content as raw HTML.
    pub async fn get_html(&self) -> Result<String> {
        if let Some(ref engine) = self.engine {
            engine
                .get_html()
                .await
                .map_err(|e| crate::AppError::Browser(e.to_string()))
        } else {
            // Stub: returns placeholder HTML
            Ok(format!(
                "<html><head><title>VibeEye</title></head><body>Navigated to: {}</body></html>",
                self.current_url().unwrap_or("unknown")
            ))
        }
    }

    /// Get page content as visible text.
    pub async fn get_text(&self) -> Result<String> {
        if let Some(ref engine) = self.engine {
            engine
                .get_text()
                .await
                .map_err(|e| crate::AppError::Browser(e.to_string()))
        } else {
            // Stub: simple HTML strip
            let html = self.get_html().await?;
            Ok(html.replace(['<', '>', '/'], " "))
        }
    }

    /// Close the browser session, shutting down the engine thread.
    pub async fn close(self) -> Result<()> {
        if let Some(mut engine) = self.engine {
            engine.shutdown();
        }
        Ok(())
    }
}

impl Default for BrowserSession {
    fn default() -> Self {
        Self::new().expect("default viewport should always initialise")
    }
}
