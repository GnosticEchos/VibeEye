//! Browser engine abstraction
//!
//! Thin wrapper around Servo 0.1.0 for headless rendering.

use crate::Result;
use std::sync::atomic::{AtomicUsize, Ordering};
use tracing::{debug, info, trace};
use vibeeye_core::NavigationState;

#[cfg(feature = "servo")]
pub mod engine;
pub mod metrics;
#[cfg(feature = "servo")]
pub mod navigation;

#[cfg(feature = "servo")]
use engine::ServoEngine;
use metrics::{EngineMetrics, HealthThresholds};

/// Process-wide singleton engine pool.
///
/// Servo contains process-wide singletons (Opts, rustls CryptoProvider) and
/// background C++ threads that cannot be reliably torn down. Creating a second
/// engine instance causes init timeouts. We therefore keep exactly one engine
/// alive for the lifetime of the process and loan it out to sessions.
static GLOBAL_ENGINE: std::sync::OnceLock<std::sync::Mutex<Option<ServoEngine>>> =
    std::sync::OnceLock::new();

/// Process-wide metrics tracking for intelligent restart decisions
static GLOBAL_METRICS: std::sync::OnceLock<std::sync::Mutex<EngineMetrics>> =
    std::sync::OnceLock::new();

/// Total engine restarts across all sessions
static TOTAL_RESTARTS: AtomicUsize = AtomicUsize::new(0);

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
    #[allow(dead_code)]
    thresholds: HealthThresholds,
}

impl BrowserSession {
    /// Create a new browser session with default headless viewport.
    pub fn new() -> Result<Self> {
        Self::with_thresholds(HealthThresholds::default())
    }

    /// Create a browser session with custom health thresholds.
    pub fn with_thresholds(thresholds: HealthThresholds) -> Result<Self> {
        // In tests we skip real initialisation to avoid cross-test
        // pollution from Servo's process-wide singletons.
        if is_test_mode() {
            return Ok(Self {
                engine: None,
                nav_state: NavigationState::default(),
                thresholds,
            });
        }

        // Initialize metrics if needed
        let metrics = GLOBAL_METRICS.get_or_init(|| std::sync::Mutex::new(EngineMetrics::new()));

        // Check if we should restart the engine
        {
            let mut m = metrics.lock().unwrap();
            m.update_memory();
            let restart_reason = m.should_restart(&thresholds);

            if restart_reason.should_restart() {
                info!(
                    reason = restart_reason.description(),
                    health_score = m.health_score(&thresholds),
                    "Triggering intelligent engine restart"
                );

                // Drop existing engine if present
                if let Some(mutex) = GLOBAL_ENGINE.get()
                    && let Ok(mut guard) = mutex.lock()
                    && let Some(old_engine) = guard.take()
                {
                    drop(old_engine);
                }

                // Reset metrics
                *m = EngineMetrics::new();
                TOTAL_RESTARTS.fetch_add(1, Ordering::Relaxed);
            }
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
            return Err(crate::Error::Browser(
                "Browser engine unavailable (init failed or already in use)".to_string(),
            ));
        }

        Ok(Self {
            engine,
            nav_state: NavigationState::default(),
            thresholds,
        })
    }

    /// Navigate to a URL and wait for the page to finish loading.
    pub async fn navigate(&mut self, url: &str) -> Result<()> {
        self.nav_state.pending_url = Some(url.to_string());

        let start = std::time::Instant::now();

        if let Some(ref engine) = self.engine {
            let result = engine.navigate(url).await;

            // Record metrics
            if let Some(metrics) = GLOBAL_METRICS.get() {
                let mut m = metrics.lock().unwrap();
                m.update_memory();

                match &result {
                    Ok(_) => {
                        let load_time_ms = start.elapsed().as_millis() as f64;
                        m.record_success(load_time_ms);
                        m.record_webview_creation();
                    }
                    Err(_) => {
                        m.record_failure();
                    }
                }
            }

            let final_url = result.map_err(|e| crate::Error::Navigation(e.to_string()))?;
            self.nav_state.current_url = Some(final_url);
        } else if is_test_mode() {
            // Test stub: accept the URL as-is
            self.nav_state.current_url = Some(url.to_string());
        } else {
            return Err(crate::Error::Browser(
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
                .map_err(|e| crate::Error::Browser(e.to_string()))
        } else if is_test_mode() {
            // Test stub: return minimal placeholder HTML
            let url = self.current_url().unwrap_or("unknown");
            Ok(format!(
                "<html><head><title>Test</title></head><body>Navigated to: {url}</body></html>"
            ))
        } else {
            Err(crate::Error::Browser(
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
                .map_err(|e| crate::Error::Browser(e.to_string()))
        } else if is_test_mode() {
            // Test stub: return simple text
            let url = self.current_url().unwrap_or("unknown");
            Ok(format!("Navigated to: {url}"))
        } else {
            Err(crate::Error::Browser(
                "Browser engine unavailable".to_string(),
            ))
        }
    }

    /// Evaluate arbitrary JavaScript in the current page context.
    pub async fn eval_js(&self, script: &str) -> Result<String> {
        if let Some(ref engine) = self.engine {
            let result = engine
                .eval_js(script)
                .await
                .map_err(|e| crate::Error::Browser(e.to_string()));

            // Record JS evaluation
            if let Some(metrics) = GLOBAL_METRICS.get() {
                metrics.lock().unwrap().record_js_eval();
            }

            result
        } else if is_test_mode() {
            // Test stub: echo the script
            Ok(format!("// test eval: {script}"))
        } else {
            Err(crate::Error::Browser(
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
                .map_err(|e| crate::Error::Browser(e.to_string()))
        } else if is_test_mode() {
            // Test stub: return empty list
            Ok(Vec::new())
        } else {
            Err(crate::Error::Browser(
                "Browser engine unavailable".to_string(),
            ))
        }
    }

    /// Run a settle loop for SPA content, then return the final HTML.
    ///
    /// Scrolls the page and waits for DOM stability before capturing.
    pub async fn settle_and_get_html(&self, settle_ms: u64) -> Result<String> {
        debug!("SPA detected, running settle loop");
        let max_iterations = 3;
        let sleep_per_iteration =
            std::time::Duration::from_millis(settle_ms.max(1) / max_iterations);

        for i in 0..max_iterations {
            let before = self
                .eval_js("document.body ? document.body.scrollHeight : 0")
                .await
                .unwrap_or_else(|_| "0".to_string());
            self.eval_js("window.scrollTo(0, document.body.scrollHeight)")
                .await
                .ok();
            tokio::time::sleep(sleep_per_iteration).await;
            let after = self
                .eval_js("document.body ? document.body.scrollHeight : 0")
                .await
                .unwrap_or_else(|_| "0".to_string());

            if before == after {
                trace!(iteration = i, "DOM stable after settle");
                break;
            }
        }

        self.get_html()
            .await
            .map_err(|e| crate::Error::Browser(e.to_string()))
    }

    /// Close the browser session, returning the engine to the global pool.
    pub async fn close(self) -> Result<()> {
        // Engine is returned to the pool via Drop when this session is dropped.
        Ok(())
    }

    /// Get current engine metrics
    pub fn metrics() -> Option<EngineMetrics> {
        GLOBAL_METRICS.get().map(|m| m.lock().unwrap().clone())
    }

    /// Get total engine restarts
    pub fn total_restarts() -> usize {
        TOTAL_RESTARTS.load(Ordering::Relaxed)
    }

    /// Log current health status
    pub fn log_health_status() {
        if let Some(metrics) = GLOBAL_METRICS.get() {
            let m = metrics.lock().unwrap();
            let thresholds = HealthThresholds::default();
            let health_score = m.health_score(&thresholds);

            info!(
                pages_processed = m.pages_processed,
                successful = m.successful_navigations,
                failed = m.failed_navigations,
                memory_mb = m.memory_mb,
                peak_memory_mb = m.peak_memory_mb,
                health_score,
                avg_load_time_ms = m.avg_load_time_ms,
                js_evaluations = m.js_evaluations,
                webview_creations = m.webview_creations,
                total_restarts = TOTAL_RESTARTS.load(Ordering::Relaxed),
                "engine health status"
            );
        }
    }
}

impl Drop for BrowserSession {
    fn drop(&mut self) {
        if let Some(engine) = self.engine.take()
            && let Some(mutex) = GLOBAL_ENGINE.get()
            && let Ok(mut guard) = mutex.lock()
        {
            *guard = Some(engine);
        }
    }
}

impl Default for BrowserSession {
    fn default() -> Self {
        Self::new().expect("default viewport should always initialise")
    }
}
