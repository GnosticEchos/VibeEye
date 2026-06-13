//! Blocking navigation helpers that run on the Servo engine thread.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use servo::{Servo, WebView};
use tracing::{debug, trace, warn};

use crate::{Error, Result};

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

/// Evaluate a JavaScript snippet on the Servo engine thread and wait for the
/// callback, with a configurable timeout.  DRY helper used by `extract_html`,
/// `extract_text`, and the engine's `eval_js_cmd`.
pub fn eval_js_with_timeout(
    servo: &Servo,
    webview: &WebView,
    script: &str,
    timeout: Duration,
    context: &str,
) -> Result<String> {
    let result_slot: Arc<
        Mutex<Option<std::result::Result<String, servo::JavaScriptEvaluationError>>>,
    > = Arc::new(Mutex::new(None));

    let slot = result_slot.clone();
    webview.evaluate_javascript(script, move |result| {
        let mut guard = slot.lock().unwrap();
        *guard = Some(result.map(|v| match v {
            servo::JSValue::String(s) => s,
            other => format!("{other:?}"),
        }));
    });

    let start = Instant::now();
    while result_slot.lock().unwrap().is_none() {
        if start.elapsed() > timeout {
            return Err(crate::Error::Extraction(format!(
                "{context} timed out (JS callback never fired)"
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
        .map_err(|e| Error::Extraction(format!("JS eval error: {e:?}")))?
        .clone();

    Ok(js_result)
}

/// Extract raw HTML via JavaScript `document.documentElement.outerHTML`.
pub fn extract_html(servo: &Servo, webview: &WebView) -> Result<String> {
    let js_result = eval_js_with_timeout(
        servo,
        webview,
        "document.documentElement.outerHTML",
        Duration::from_secs(5),
        "extract_html",
    )?;
    trace!(html_len = js_result.len(), "extracted html");
    Ok(js_result)
}

/// Extract visible text via JavaScript `document.body.innerText`.
pub fn extract_text(servo: &Servo, webview: &WebView) -> Result<String> {
    let text = eval_js_with_timeout(
        servo,
        webview,
        "document.body ? document.body.innerText : ''",
        Duration::from_secs(5),
        "extract_text",
    )?;
    trace!(text_len = text.len(), "extracted text");
    Ok(text)
}
