//! Blocking navigation helpers that run on the Servo engine thread.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use servo::{Servo, WebView};
use tracing::{debug, trace, warn};

use crate::Result;
use vibeeye_core::VibeError;

/// Spin the event loop until the webview reports `LoadStatus::Complete`.
pub fn wait_for_load(servo: &Servo, webview: &WebView) {
    let start = Instant::now();
    while webview.load_status() != servo::LoadStatus::Complete {
        if start.elapsed() > Duration::from_secs(15) {
            warn!("page load timed out after 15s");
            break;
        }
        servo.spin_event_loop();
        std::thread::yield_now();
    }
    debug!("page load complete");
}

/// Extract raw HTML via JavaScript `document.documentElement.outerHTML`.
pub fn extract_html(servo: &Servo, webview: &WebView) -> Result<String> {
    let result_slot: Arc<
        Mutex<Option<std::result::Result<String, servo::JavaScriptEvaluationError>>>,
    > = Arc::new(Mutex::new(None));

    let slot = result_slot.clone();
    webview.evaluate_javascript("document.documentElement.outerHTML", move |result| {
        let mut guard = slot.lock().unwrap();
        *guard = Some(result.map(|v| match v {
            servo::JSValue::String(s) => s,
            other => format!("{other:?}"),
        }));
    });

    let start = Instant::now();
    while result_slot.lock().unwrap().is_none() {
        if start.elapsed() > Duration::from_secs(5) {
            return Err(crate::AppError::Core(VibeError::Extraction(
                "extract_html timed out (JS callback never fired)".to_string(),
            )));
        }
        servo.spin_event_loop();
        std::thread::yield_now();
    }

    let guard = result_slot.lock().unwrap();
    let js_result = guard
        .as_ref()
        .expect("result populated by callback")
        .as_ref()
        .map_err(|e| VibeError::Extraction(format!("JS eval error: {e:?}")))?
        .clone();

    trace!(html_len = js_result.len(), "extracted html");
    Ok(js_result)
}

/// Extract visible text via JavaScript `document.body.innerText`.
pub fn extract_text(servo: &Servo, webview: &WebView) -> Result<String> {
    let result_slot: Arc<
        Mutex<Option<std::result::Result<String, servo::JavaScriptEvaluationError>>>,
    > = Arc::new(Mutex::new(None));

    let slot = result_slot.clone();
    webview.evaluate_javascript(
        "document.body ? document.body.innerText : ''",
        move |result| {
            let mut guard = slot.lock().unwrap();
            *guard = Some(result.map(|v| match v {
                servo::JSValue::String(s) => s,
                other => format!("{other:?}"),
            }));
        },
    );

    let start = Instant::now();
    while result_slot.lock().unwrap().is_none() {
        if start.elapsed() > Duration::from_secs(5) {
            return Err(crate::AppError::Core(VibeError::Extraction(
                "extract_text timed out (JS callback never fired)".to_string(),
            )));
        }
        servo.spin_event_loop();
        std::thread::yield_now();
    }

    let guard = result_slot.lock().unwrap();
    let text = guard
        .as_ref()
        .expect("result populated by callback")
        .as_ref()
        .map_err(|e| VibeError::Extraction(format!("JS eval error: {e:?}")))?
        .clone();

    trace!(text_len = text.len(), "extracted text");
    Ok(text)
}
