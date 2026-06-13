use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::Duration;

/// Global guard: once Servo initialisation fails we skip trying again.
/// Protected by a mutex so parallel tests only attempt init once.
static SERVO_POSSIBLE: Mutex<bool> = Mutex::new(true);

use servo::{
    EventLoopWaker, Preferences, Servo, ServoBuilder, WebView, WebViewBuilder, WebViewDelegate,
};
use servo::{RenderingContext, SoftwareRenderingContext};
use tokio::sync::oneshot;
use tracing::{debug, error, info, trace, warn};
use url::Url;

use crate::{Error, Result};
use vibeeye_core::Viewport;

/// Commands sent from the async `BrowserSession` to the dedicated Servo thread.
pub(crate) enum EngineCommand {
    Navigate {
        url: String,
        respond: oneshot::Sender<Result<String>>,
    },
    GetHtml {
        respond: oneshot::Sender<Result<String>>,
    },
    GetText {
        respond: oneshot::Sender<Result<String>>,
    },
    EvalJs {
        script: String,
        respond: oneshot::Sender<Result<String>>,
    },
    GetDomLinks {
        respond: oneshot::Sender<Result<Vec<String>>>,
    },
    Shutdown,
}

/// Headless browser engine backed by Servo.
///
/// Owns a dedicated thread that houses the `Servo` instance and its
/// `SoftwareRenderingContext`. All interaction happens through async
/// methods that bridge to the thread via channels.
pub struct ServoEngine {
    cmd_tx: mpsc::Sender<EngineCommand>,
    thread: Option<thread::JoinHandle<()>>,
}

impl ServoEngine {
    /// Start a new headless Servo engine with the given viewport size.
    pub fn new(viewport: Viewport) -> Result<Self> {
        let mut possible = SERVO_POSSIBLE.lock().unwrap();
        if !*possible {
            return Err(Error::Browser(
                "Servo previously failed to initialise".to_string(),
            ));
        }

        let (cmd_tx, cmd_rx) = mpsc::channel::<EngineCommand>();
        let (ready_tx, ready_rx) = mpsc::channel::<std::result::Result<(), String>>();

        let thread = thread::Builder::new()
            .name("servo-engine".to_string())
            .spawn(move || {
                if let Err(e) = run_engine(viewport, cmd_rx, ready_tx) {
                    error!(error = %e, "Servo engine thread failed");
                }
            })
            .map_err(|e| Error::Browser(format!("failed to spawn engine thread: {e}")))?;

        await_ready(ready_rx, &mut possible).map(|()| ServoEngine {
            cmd_tx,
            thread: Some(thread),
        })
    }

    /// Navigate to `url`, wait for load to complete, and return the final URL.
    pub async fn navigate(&self, url: &str) -> Result<String> {
        let (tx, rx) = oneshot::channel();
        self.cmd_tx
            .send(EngineCommand::Navigate {
                url: url.to_string(),
                respond: tx,
            })
            .map_err(|e| Error::Browser(format!("send navigate: {e}")))?;
        rx.await
            .map_err(|e| Error::Browser(format!("recv navigate: {e}")))?
    }

    /// Get the current page's raw HTML.
    pub async fn get_html(&self) -> Result<String> {
        let (tx, rx) = oneshot::channel();
        self.cmd_tx
            .send(EngineCommand::GetHtml { respond: tx })
            .map_err(|e| Error::Browser(format!("send get_html: {e}")))?;
        rx.await
            .map_err(|e| Error::Browser(format!("recv get_html: {e}")))?
    }

    /// Get the current page's visible text.
    pub async fn get_text(&self) -> Result<String> {
        let (tx, rx) = oneshot::channel();
        self.cmd_tx
            .send(EngineCommand::GetText { respond: tx })
            .map_err(|e| Error::Browser(format!("send get_text: {e}")))?;
        rx.await
            .map_err(|e| Error::Browser(format!("recv get_text: {e}")))?
    }

    /// Evaluate arbitrary JavaScript in the current page context.
    pub async fn eval_js(&self, script: &str) -> Result<String> {
        let (tx, rx) = oneshot::channel();
        self.cmd_tx
            .send(EngineCommand::EvalJs {
                script: script.to_string(),
                respond: tx,
            })
            .map_err(|e| Error::Browser(format!("send eval_js: {e}")))?;
        rx.await
            .map_err(|e| Error::Browser(format!("recv eval_js: {e}")))?
    }

    /// Get all link URLs from the live DOM after JavaScript execution.
    pub async fn get_dom_links(&self) -> Result<Vec<String>> {
        let (tx, rx) = oneshot::channel();
        self.cmd_tx
            .send(EngineCommand::GetDomLinks { respond: tx })
            .map_err(|e| Error::Browser(format!("send get_dom_links: {e}")))?;
        rx.await
            .map_err(|e| Error::Browser(format!("recv get_dom_links: {e}")))?
    }

    /// Gracefully shut down the engine thread and wait for it to finish.
    pub fn shutdown(&mut self) {
        let _ = self.cmd_tx.send(EngineCommand::Shutdown);
        if let Some(thread) = self.thread.take() {
            // Poll with a 10 s timeout so we don't hang forever if Servo's
            // C++ background threads deadlock during teardown.
            let start = std::time::Instant::now();
            while !thread.is_finished() {
                if start.elapsed() > Duration::from_secs(10) {
                    warn!("Servo engine shutdown timed out (thread still running)");
                    return;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            let _ = thread.join();
            debug!("Servo engine shutdown complete");
        }
    }
}

impl Drop for ServoEngine {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// Block until the engine thread signals it is ready or times out.
fn await_ready(
    ready_rx: mpsc::Receiver<std::result::Result<(), String>>,
    possible: &mut bool,
) -> Result<()> {
    match ready_rx.recv_timeout(Duration::from_secs(5)) {
        Ok(Ok(())) => Ok(()),
        Ok(Err(e)) => {
            *possible = false;
            Err(Error::Browser(format!("engine init failed: {e}")))
        }
        Err(_) => {
            *possible = false;
            Err(Error::Browser(
                "engine init timed out (no Mesa / display available?)".to_string(),
            ))
        }
    }
}

/// Runs on the dedicated "servo-engine" thread.
fn run_engine(
    viewport: Viewport,
    cmd_rx: mpsc::Receiver<EngineCommand>,
    ready: mpsc::Sender<std::result::Result<(), String>>,
) -> Result<()> {
    let size = dpi::PhysicalSize {
        width: viewport.width,
        height: viewport.height,
    };

    let rendering_context: Rc<dyn RenderingContext> = Rc::new(
        SoftwareRenderingContext::new(size)
            .map_err(|e| Error::Browser(format!("SoftwareRenderingContext: {e:?}")))?,
    );

    rendering_context
        .make_current()
        .map_err(|e| Error::Browser(format!("make_current: {e:?}")))?;

    let devtools_enabled = std::env::var("VIBEYE_DEVTOOLS").is_ok();
    let preferences = Preferences {
        network_http_proxy_uri: String::new(),
        network_https_proxy_uri: String::new(),
        devtools_server_enabled: devtools_enabled,
        devtools_server_listen_address: if devtools_enabled {
            "127.0.0.1:0".to_string()
        } else {
            String::new()
        },
        ..Preferences::default()
    };

    // Servo's networking stack uses rustls for HTTPS.  rustls 0.23+
    // requires an explicit process-level CryptoProvider installation.
    // We use `ring` and ignore the error if already installed.
    let _ = rustls::crypto::ring::default_provider().install_default();

    let user_event_triggered = Arc::new(AtomicBool::new(false));
    let waker = Box::new(EventLoopWakerImpl(user_event_triggered));

    let servo = ServoBuilder::default()
        .preferences(preferences)
        .event_loop_waker(waker)
        .build();

    info!("Servo engine initialized");

    if ready.send(Ok(())).is_err() {
        return Err(Error::Browser("parent dropped before ready".to_string()));
    }

    let mut active_webview: Option<WebView> = None;

    loop {
        servo.spin_event_loop();

        while let Ok(cmd) = cmd_rx.try_recv() {
            if !handle_command(&servo, cmd, &mut active_webview, &rendering_context) {
                return Ok(());
            }
        }

        std::thread::yield_now();
    }
}

/// Dispatch a single engine command.
/// Returns `true` to keep the loop running, `false` if the engine should shut down.
fn handle_command(
    servo: &Servo,
    cmd: EngineCommand,
    active_webview: &mut Option<WebView>,
    rendering_context: &Rc<dyn RenderingContext>,
) -> bool {
    match cmd {
        EngineCommand::Navigate { url, respond } => {
            let result = navigate_cmd(servo, rendering_context.clone(), active_webview, &url);
            let _ = respond.send(result);
            true
        }
        EngineCommand::GetHtml { respond } => {
            let result = get_html_cmd(servo, active_webview);
            let _ = respond.send(result);
            true
        }
        EngineCommand::GetText { respond } => {
            let result = get_text_cmd(servo, active_webview);
            let _ = respond.send(result);
            true
        }
        EngineCommand::EvalJs { script, respond } => {
            let result = eval_js_cmd(servo, active_webview, &script);
            let _ = respond.send(result);
            true
        }
        EngineCommand::GetDomLinks { respond } => {
            let result = get_dom_links_cmd(servo, active_webview);
            let _ = respond.send(result);
            true
        }
        EngineCommand::Shutdown => {
            debug!("Servo engine received shutdown");
            drop(active_webview.take());
            // Give Servo time to tear down internal C++ threads
            // (ResourceManager, FetchThread, SpiderMonkey, …).
            for _ in 0..200 {
                servo.spin_event_loop();
                std::thread::sleep(Duration::from_millis(10));
            }
            false
        }
    }
}

fn navigate_cmd(
    servo: &Servo,
    rendering_context: Rc<dyn RenderingContext>,
    active_webview: &mut Option<WebView>,
    url: &str,
) -> Result<String> {
    let parsed = Url::parse(url).map_err(|e| Error::Navigation(format!("invalid URL: {e}")))?;

    // Drop the old webview before creating a new one so Servo can
    // reclaim DOM, JS heap, and rendering resources. Without this,
    // long crawls leak gigabytes of memory and eventually get OOM-killed.
    if let Some(old) = active_webview.take() {
        drop(old);
        for _ in 0..10 {
            servo.spin_event_loop();
            std::thread::yield_now();
        }
    }

    let delegate = Rc::new(HeadlessWebViewDelegate);

    let webview = WebViewBuilder::new(servo, rendering_context.clone())
        .url(parsed.clone())
        .delegate(delegate)
        .build();

    // Wait for load to complete, but cap at 15 s so sites with
    // never-ending ads / trackers / analytics don’t hang forever.
    let start = std::time::Instant::now();
    let timeout = Duration::from_secs(15);
    while webview.load_status() != servo::LoadStatus::Complete {
        if start.elapsed() > timeout {
            warn!(%url, "page load timed out, proceeding with partial content");
            break;
        }
        servo.spin_event_loop();
        std::thread::yield_now();
    }

    // Extra spins after load completes to let the script thread
    // finish stylesheet / render-blocking bookkeeping before
    // we run JS that may touch the DOM. Helps avoid upstream
    // Servo assertion failures in decrement_*_count.
    for _ in 0..20 {
        servo.spin_event_loop();
        std::thread::yield_now();
    }

    let final_url = webview
        .url()
        .map(|u| u.to_string())
        .unwrap_or_else(|| url.to_string());

    *active_webview = Some(webview);
    Ok(final_url)
}

fn get_html_cmd(servo: &Servo, active_webview: &Option<WebView>) -> Result<String> {
    let webview = active_webview
        .as_ref()
        .ok_or_else(|| Error::Browser("no active webview".to_string()))?;

    let result = crate::browser::navigation::extract_html(servo, webview)?;
    Ok(result)
}

fn get_text_cmd(servo: &Servo, active_webview: &Option<WebView>) -> Result<String> {
    let webview = active_webview
        .as_ref()
        .ok_or_else(|| Error::Browser("no active webview".to_string()))?;

    let result = crate::browser::navigation::extract_text(servo, webview)?;
    Ok(result)
}

fn eval_js_cmd(servo: &Servo, active_webview: &Option<WebView>, script: &str) -> Result<String> {
    let webview = active_webview
        .as_ref()
        .ok_or_else(|| Error::Browser("no active webview".to_string()))?;

    let js_result = crate::browser::navigation::eval_js_with_timeout(
        servo,
        webview,
        script,
        std::time::Duration::from_secs(5),
        "eval_js_cmd",
    )?;

    trace!(
        script_len = script.len(),
        result_len = js_result.len(),
        "eval_js done"
    );
    Ok(js_result)
}

fn get_dom_links_cmd(servo: &Servo, active_webview: &Option<WebView>) -> Result<Vec<String>> {
    let script = r#"
        Array.from(document.querySelectorAll('a[href]'))
            .map(a => a.href)
            .filter(h => h && !h.startsWith('javascript:') && !h.startsWith('#'))
    "#;

    let result = eval_js_cmd(servo, active_webview, script)?;

    // The result is a JS array serialized by SpiderMonkey.
    // In practice servo::JSValue::to_string() on arrays gives something like
    // "https://a.com,https://b.com" or "a,b" — handle both.
    let links: Vec<String> = result
        .trim_matches(['[', ']'])
        .split(',')
        .map(|s| s.trim().trim_matches('"').to_string())
        .filter(|s| !s.is_empty() && s.starts_with("http"))
        .collect();

    trace!(link_count = links.len(), "extracted dom links");
    Ok(links)
}

// ------------------------------------------------------------------
//  Event loop waker
// ------------------------------------------------------------------

#[derive(Clone)]
struct EventLoopWakerImpl(Arc<AtomicBool>);

impl EventLoopWaker for EventLoopWakerImpl {
    fn clone_box(&self) -> Box<dyn EventLoopWaker> {
        Box::new(self.clone())
    }

    fn wake(&self) {
        self.0.store(true, Ordering::Relaxed);
    }
}

// ------------------------------------------------------------------
//  WebView delegate (headless — all no-ops)
// ------------------------------------------------------------------

#[derive(Default)]
struct HeadlessWebViewDelegate;

impl WebViewDelegate for HeadlessWebViewDelegate {
    fn notify_url_changed(&self, _webview: WebView, _url: Url) {
        debug!("URL changed");
    }

    fn notify_load_status_changed(&self, _webview: WebView, status: servo::LoadStatus) {
        debug!(?status, "load status changed");
    }

    fn notify_new_frame_ready(&self, _webview: WebView) {
        debug!("new frame ready");
    }

    fn notify_crashed(&self, _webview: WebView, _reason: String, _backtrace: Option<String>) {
        warn!("webview crashed: {_reason}");
    }
}
