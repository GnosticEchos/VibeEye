//! Browser engine health metrics and intelligent restart logic
//!
//! Collects memory, performance, and resource metrics to detect
//! Servo/SpiderMonkey pressure before borrow hazards cause crashes.

use std::time::{Duration, Instant};
use sysinfo::{Process, System};
use tracing::debug;

/// Thresholds for triggering engine restart
#[derive(Debug, Clone)]
pub struct HealthThresholds {
    /// Maximum memory usage in MB before restart
    pub max_memory_mb: u64,
    /// Maximum number of pages processed before restart
    pub max_pages: usize,
    /// Maximum engine uptime before restart (prevents accumulation)
    pub max_uptime: Duration,
    /// Maximum consecutive navigation failures before restart
    pub max_consecutive_failures: usize,
}

impl Default for HealthThresholds {
    fn default() -> Self {
        Self {
            max_memory_mb: 2048,                   // 2GB limit
            max_pages: 300,                        // Restart after 300 pages
            max_uptime: Duration::from_secs(1800), // 30 minutes
            max_consecutive_failures: 5,           // 5 consecutive failures
        }
    }
}

/// Metrics collected from the browser engine
#[derive(Debug, Clone)]
pub struct EngineMetrics {
    /// Number of pages processed by current engine instance
    pub pages_processed: usize,
    /// Number of successful navigations
    pub successful_navigations: usize,
    /// Number of failed navigations
    pub failed_navigations: usize,
    /// Consecutive navigation failures
    pub consecutive_failures: usize,
    /// Engine start time
    pub start_time: Instant,
    /// Last successful navigation time
    pub last_success: Option<Instant>,
    /// Current memory usage in MB
    pub memory_mb: u64,
    /// Peak memory usage in MB
    pub peak_memory_mb: u64,
    /// Average page load time in milliseconds
    pub avg_load_time_ms: f64,
    /// Total JS evaluations performed
    pub js_evaluations: usize,
    /// Total WebView creations (pipeline switches)
    pub webview_creations: usize,
}

impl Default for EngineMetrics {
    fn default() -> Self {
        Self {
            pages_processed: 0,
            successful_navigations: 0,
            failed_navigations: 0,
            consecutive_failures: 0,
            start_time: Instant::now(),
            last_success: None,
            memory_mb: 0,
            peak_memory_mb: 0,
            avg_load_time_ms: 0.0,
            js_evaluations: 0,
            webview_creations: 0,
        }
    }
}

impl EngineMetrics {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a successful navigation
    pub fn record_success(&mut self, load_time_ms: f64) {
        self.pages_processed += 1;
        self.successful_navigations += 1;
        self.consecutive_failures = 0;
        self.last_success = Some(Instant::now());

        // Update rolling average load time
        if self.avg_load_time_ms == 0.0 {
            self.avg_load_time_ms = load_time_ms;
        } else {
            self.avg_load_time_ms = (self.avg_load_time_ms * 0.9) + (load_time_ms * 0.1);
        }
    }

    /// Record a failed navigation
    pub fn record_failure(&mut self) {
        self.pages_processed += 1;
        self.failed_navigations += 1;
        self.consecutive_failures += 1;
    }

    /// Record a JS evaluation
    pub fn record_js_eval(&mut self) {
        self.js_evaluations += 1;
    }

    /// Record a WebView creation (pipeline switch)
    pub fn record_webview_creation(&mut self) {
        self.webview_creations += 1;
    }

    /// Update memory usage from system
    pub fn update_memory(&mut self) {
        if let Some(memory) = get_process_memory_mb() {
            self.memory_mb = memory;
            if memory > self.peak_memory_mb {
                self.peak_memory_mb = memory;
            }
        }
    }

    /// Check if engine should restart based on health thresholds
    pub fn should_restart(&self, thresholds: &HealthThresholds) -> RestartReason {
        // Check memory pressure
        if self.memory_mb >= thresholds.max_memory_mb {
            return RestartReason::MemoryPressure {
                current_mb: self.memory_mb,
                threshold_mb: thresholds.max_memory_mb,
            };
        }

        // Check page count
        if self.pages_processed >= thresholds.max_pages {
            return RestartReason::PageLimit {
                current: self.pages_processed,
                limit: thresholds.max_pages,
            };
        }

        // Check uptime
        let uptime = self.start_time.elapsed();
        if uptime >= thresholds.max_uptime {
            return RestartReason::UptimeLimit {
                elapsed: uptime,
                limit: thresholds.max_uptime,
            };
        }

        // Check consecutive failures
        if self.consecutive_failures >= thresholds.max_consecutive_failures {
            return RestartReason::ConsecutiveFailures {
                count: self.consecutive_failures,
                threshold: thresholds.max_consecutive_failures,
            };
        }

        // Check for memory growth trend (early warning)
        if self.peak_memory_mb > 0 && self.memory_mb > self.peak_memory_mb * 90 / 100 {
            debug!(
                current_mb = self.memory_mb,
                peak_mb = self.peak_memory_mb,
                "approaching peak memory usage"
            );
        }

        RestartReason::None
    }

    /// Get a health score (0-100, higher is healthier)
    pub fn health_score(&self, thresholds: &HealthThresholds) -> u8 {
        let mut score = 100u8;

        // Deduct for memory usage
        let memory_pct = (self.memory_mb * 100) / thresholds.max_memory_mb.max(1);
        if memory_pct > 80 {
            score = score.saturating_sub((memory_pct - 80) as u8);
        }

        // Deduct for page count
        let page_pct = (self.pages_processed * 100) / thresholds.max_pages.max(1);
        if page_pct > 70 {
            score = score.saturating_sub((page_pct - 70) as u8 / 2);
        }

        // Deduct for failures
        if self.pages_processed > 0 {
            let failure_rate = (self.failed_navigations * 100) / self.pages_processed.max(1);
            if failure_rate > 5 {
                score = score.saturating_sub(failure_rate as u8);
            }
        }

        // Deduct for uptime
        let uptime_pct =
            (self.start_time.elapsed().as_secs() * 100) / thresholds.max_uptime.as_secs().max(1);
        if uptime_pct > 80 {
            score = score.saturating_sub((uptime_pct - 80) as u8 / 2);
        }

        score
    }
}

/// Reason for engine restart
#[derive(Debug, Clone)]
pub enum RestartReason {
    None,
    MemoryPressure { current_mb: u64, threshold_mb: u64 },
    PageLimit { current: usize, limit: usize },
    UptimeLimit { elapsed: Duration, limit: Duration },
    ConsecutiveFailures { count: usize, threshold: usize },
}

impl RestartReason {
    pub fn should_restart(&self) -> bool {
        !matches!(self, RestartReason::None)
    }

    pub fn description(&self) -> String {
        match self {
            RestartReason::None => "No restart needed".to_string(),
            RestartReason::MemoryPressure {
                current_mb,
                threshold_mb,
            } => {
                format!(
                    "Memory pressure: {}MB / {}MB threshold",
                    current_mb, threshold_mb
                )
            }
            RestartReason::PageLimit { current, limit } => {
                format!("Page limit reached: {} / {} pages", current, limit)
            }
            RestartReason::UptimeLimit { elapsed, limit } => {
                format!("Uptime limit reached: {:?} / {:?}", elapsed, limit)
            }
            RestartReason::ConsecutiveFailures { count, threshold } => {
                format!("Consecutive failures: {} / {} threshold", count, threshold)
            }
        }
    }
}

/// Get current process memory usage in MB
pub fn get_process_memory_mb() -> Option<u64> {
    let mut sys = System::new();
    sys.refresh_processes_specifics(
        sysinfo::ProcessesToUpdate::Some(&[sysinfo::get_current_pid().ok()?]),
        true,
        sysinfo::ProcessRefreshKind::everything(),
    );

    let pid = sysinfo::get_current_pid().ok()?;
    let process: &Process = sys.process(pid)?;

    // Convert bytes to MB
    Some(process.memory() / 1024 / 1024)
}

/// Get available system memory in MB
pub fn get_available_memory_mb() -> Option<u64> {
    let mut sys = System::new();
    sys.refresh_memory_specifics(sysinfo::MemoryRefreshKind::everything());

    Some(sys.available_memory() / 1024 / 1024)
}

/// Get total system memory in MB
pub fn get_total_memory_mb() -> Option<u64> {
    let mut sys = System::new();
    sys.refresh_memory_specifics(sysinfo::MemoryRefreshKind::everything());

    Some(sys.total_memory() / 1024 / 1024)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_record_success() {
        let mut metrics = EngineMetrics::new();
        metrics.record_success(100.0);
        assert_eq!(metrics.pages_processed, 1);
        assert_eq!(metrics.successful_navigations, 1);
        assert_eq!(metrics.consecutive_failures, 0);
    }

    #[test]
    fn test_metrics_record_failure() {
        let mut metrics = EngineMetrics::new();
        metrics.record_failure();
        metrics.record_failure();
        assert_eq!(metrics.failed_navigations, 2);
        assert_eq!(metrics.consecutive_failures, 2);
    }

    #[test]
    fn test_should_restart_memory() {
        let mut metrics = EngineMetrics::new();
        let thresholds = HealthThresholds {
            max_memory_mb: 100,
            ..Default::default()
        };

        // Manually set memory to avoid calling update_memory() which uses sysinfo
        metrics.memory_mb = 150;
        let reason = metrics.should_restart(&thresholds);
        assert!(reason.should_restart());
    }

    #[test]
    fn test_should_restart_pages() {
        let metrics = EngineMetrics {
            pages_processed: 500,
            ..EngineMetrics::new()
        };
        let thresholds = HealthThresholds {
            max_pages: 300,
            ..Default::default()
        };

        let reason = metrics.should_restart(&thresholds);
        assert!(reason.should_restart());
    }

    #[test]
    fn test_health_score() {
        let mut metrics = EngineMetrics::new();
        metrics.pages_processed = 50;
        metrics.successful_navigations = 48;
        metrics.failed_navigations = 2;
        metrics.memory_mb = 500;

        let thresholds = HealthThresholds::default();

        let score = metrics.health_score(&thresholds);
        assert!(score > 80, "Health score should be > 80, got {}", score);
    }

    // Note: get_process_memory_mb tests skipped due to Servo's large TLS
    // causing stack overflow in test runner
}
