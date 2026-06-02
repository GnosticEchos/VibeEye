//! Browser engine abstraction
//!
//! Thin wrapper around Servo 0.1.0 for headless rendering.

use crate::Result;
use vibeeye_core::NavigationState;

pub mod engine;
pub mod navigation;

use engine::ServoEngine;

/// Process-wide singleton engine pool.
///
/// Servo contains process-wide singletons (Opts, rustls CryptoProvider) and
/// background C++ threads that cannot be reliably torn down. Creating a second
/// engine instance causes init timeouts. We therefore keep exactly one engine
/// alive for the lifetime of the process and loan it out to sessions.
static GLOBAL_ENGINE: std::sync::OnceLock<std::sync::Mutex<Option<ServoEngine>>> =
    std::sync::OnceLock::new();

/// Returns true when running inside `cargo test` or when the
/// `VIBEYE_TEST_STUB` environment variable is set.
///
/// Integration tests in `tests/` compile the library without `cfg(test)`,
/// so we also check the env var to allow them to opt into stub mode.
fn is_test_mode() -> bool {
    cfg!(test) || std::env::var("VIBEYE_TEST_STUB").is_ok()
}

/// Browser session handle
///
/// Sessions borrow the process-wide `ServoEngine` singleton. When a session
/// is dropped the engine is returned to the pool so the next call can reuse it.
pub struct BrowserSession {
    engine: Option<ServoEngine>,
    nav_state: NavigationState,
}

impl BrowserSession {
    /// Create a new browser session with default headless viewport.
    pub fn new() -> Result<Self> {
        // In tests we skip real initialisation to avoid cross-test
        // pollution from Servo's process-wide singletons.
        if is_test_mode() {
            return Ok(Self {
                engine: None,
                nav_state: NavigationState::default(),
            });
        }

        let engine = {
            let mutex = GLOBAL_ENGINE.get_or_init(|| {
                match ServoEngine::new(vibeeye_core::Viewport::default()) {
                    Ok(engine) => {
                        tracing::debug!("Servo engine initialised");
                        std::sync::Mutex::new(Some(engine))
                    }
                    Err(e) => {
                        tracing::error!("Servo engine init failed: {e}");
                        std::sync::Mutex::new(None)
                    }
                }
            });
            mutex.lock().unwrap().take()
        };

        if engine.is_none() && !is_test_mode() {
            return Err(crate::AppError::Browser(
                "Browser engine unavailable (init failed or already in use)".to_string(),
            ));
        }

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
        } else if is_test_mode() {
            // Test stub: accept the URL as-is
            self.nav_state.current_url = Some(url.to_string());
        } else {
            return Err(crate::AppError::Browser(
                "Browser engine unavailable".to_string(),
            ));
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
        } else if is_test_mode() {
            // Test stub: return minimal placeholder HTML
            let url = self.current_url().unwrap_or("unknown");
            Ok(format!(
                "<html><head><title>Test</title></head><body>Navigated to: {url}</body></html>"
            ))
        } else {
            Err(crate::AppError::Browser(
                "Browser engine unavailable".to_string(),
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
        } else if is_test_mode() {
            // Test stub: return simple text
            let url = self.current_url().unwrap_or("unknown");
            Ok(format!("Navigated to: {url}"))
        } else {
            Err(crate::AppError::Browser(
                "Browser engine unavailable".to_string(),
            ))
        }
    }

    /// Evaluate arbitrary JavaScript in the current page context.
    pub async fn eval_js(&self, script: &str) -> Result<String> {
        if let Some(ref engine) = self.engine {
            engine
                .eval_js(script)
                .await
                .map_err(|e| crate::AppError::Browser(e.to_string()))
        } else if is_test_mode() {
            // Test stub: echo the script
            Ok(format!("// test eval: {script}"))
        } else {
            Err(crate::AppError::Browser(
                "Browser engine unavailable".to_string(),
            ))
        }
    }

    /// Get all link URLs from the live DOM (post-JavaScript execution).
    pub async fn get_dom_links(&self) -> Result<Vec<String>> {
        if let Some(ref engine) = self.engine {
            engine
                .get_dom_links()
                .await
                .map_err(|e| crate::AppError::Browser(e.to_string()))
        } else if is_test_mode() {
            // Test stub: return empty list
            Ok(Vec::new())
        } else {
            Err(crate::AppError::Browser(
                "Browser engine unavailable".to_string(),
            ))
        }
    }

    /// Close the browser session, returning the engine to the global pool.
    pub async fn close(self) -> Result<()> {
        // Engine is returned to the pool via Drop when this session is dropped.
        Ok(())
    }
}

impl Drop for BrowserSession {
    fn drop(&mut self) {
        if let Some(engine) = self.engine.take() {
            if let Some(mutex) = GLOBAL_ENGINE.get() {
                if let Ok(mut guard) = mutex.lock() {
                    *guard = Some(engine);
                }
            }
        }
    }
}

impl Default for BrowserSession {
    fn default() -> Self {
        Self::new().expect("default viewport should always initialise")
    }
}
