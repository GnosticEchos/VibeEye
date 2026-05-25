use std::sync::atomic::{AtomicBool, Ordering};
use std::rc::Rc;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Duration;

/// Global guard: once Servo initialisation fails we skip trying again.
/// Protected by a mutex so parallel tests only attempt init once.
static SERVO_POSSIBLE: Mutex<bool> = Mutex::new(true);

use servo::{
    EventLoopWaker, Preferences, Servo, ServoBuilder, WebView, WebViewBuilder,
    WebViewDelegate,
};
use servo::{RenderingContext, SoftwareRenderingContext};
use tokio::sync::oneshot;
use tracing::{debug, error, info, warn};
use url::Url;

use vibeeye_core::{Viewport, VibeError};
use crate::{AppError, Result};

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
    Shutdown,
}

/// Headless browser engine backed by Servo.
///
/// Owns a dedicated thread that houses the `Servo` instance and its
/// `SoftwareRenderingContext`. All interaction happens through async
/// methods that bridge to the thread via channels.
pub struct ServoEngine {
    cmd_tx: mpsc::Sender<EngineCommand>,
    _thread: thread::JoinHandle<()>,
}

impl ServoEngine {
    /// Start a new headless Servo engine with the given viewport size.
    pub fn new(viewport: Viewport) -> Result<Self> {
        let mut possible = SERVO_POSSIBLE.lock().unwrap();
        if !*possible {
            return Err(AppError::Core(VibeError::Engine(
                "Servo previously failed to initialise".to_string(),
            )));
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
            .map_err(|e| VibeError::Engine(format!("failed to spawn engine thread: {e}")))?;

        match ready_rx.recv_timeout(Duration::from_secs(5)) {
            Ok(Ok(())) => Ok(ServoEngine { cmd_tx, _thread: thread }),
            Ok(Err(e)) => {
                *possible = false;
                Err(AppError::Core(VibeError::Engine(format!("engine init failed: {e}"))))
            }
            Err(_) => {
                *possible = false;
                Err(AppError::Core(VibeError::Engine(
                    "engine init timed out (no Mesa / display available?)".to_string(),
                )))
            }
        }
    }

    /// Navigate to `url`, wait for load to complete, and return the final URL.
    pub async fn navigate(&self, url: &str) -> Result<String> {
        let (tx, rx) = oneshot::channel();
        self.cmd_tx
            .send(EngineCommand::Navigate {
                url: url.to_string(),
                respond: tx,
            })
            .map_err(|e| VibeError::Engine(format!("send navigate: {e}")))?;
        rx.await
            .map_err(|e| VibeError::Engine(format!("recv navigate: {e}")))?
    }

    /// Get the current page's raw HTML.
    pub async fn get_html(&self) -> Result<String> {
        let (tx, rx) = oneshot::channel();
        self.cmd_tx
            .send(EngineCommand::GetHtml { respond: tx })
            .map_err(|e| VibeError::Engine(format!("send get_html: {e}")))?;
        rx.await
            .map_err(|e| VibeError::Engine(format!("recv get_html: {e}")))?
    }

    /// Get the current page's visible text.
    pub async fn get_text(&self) -> Result<String> {
        let (tx, rx) = oneshot::channel();
        self.cmd_tx
            .send(EngineCommand::GetText { respond: tx })
            .map_err(|e| VibeError::Engine(format!("send get_text: {e}")))?;
        rx.await
            .map_err(|e| VibeError::Engine(format!("recv get_text: {e}")))?
    }

    /// Gracefully shut down the engine thread.
    pub fn shutdown(&self) {
        let _ = self.cmd_tx.send(EngineCommand::Shutdown);
    }
}

impl Drop for ServoEngine {
    fn drop(&mut self) {
        self.shutdown();
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
            .map_err(|e| VibeError::Engine(format!("SoftwareRenderingContext: {e:?}")))?,
    );

    rendering_context
        .make_current()
        .map_err(|e| VibeError::Engine(format!("make_current: {e:?}")))?;

    let preferences = Preferences {
        network_http_proxy_uri: String::new(),
        network_https_proxy_uri: String::new(),
        ..Preferences::default()
    };

    let user_event_triggered = Arc::new(AtomicBool::new(false));
    let waker = Box::new(EventLoopWakerImpl(user_event_triggered));

    let servo = ServoBuilder::default()
        .preferences(preferences)
        .event_loop_waker(waker)
        .build();

    info!("Servo engine initialized");

    if ready.send(Ok(())).is_err() {
        return Err(AppError::Core(VibeError::Engine("parent dropped before ready".to_string())));
    }

    let mut active_webview: Option<WebView> = None;

    loop {
        servo.spin_event_loop();

        while let Ok(cmd) = cmd_rx.try_recv() {
            match cmd {
                EngineCommand::Navigate { url, respond } => {
                    let result = navigate_cmd(&servo, rendering_context.clone(), &mut active_webview, &url);
                    let _ = respond.send(result);
                }
                EngineCommand::GetHtml { respond } => {
                    let result = get_html_cmd(&servo, &active_webview);
                    let _ = respond.send(result);
                }
                EngineCommand::GetText { respond } => {
                    let result = get_text_cmd(&servo, &active_webview);
                    let _ = respond.send(result);
                }
                EngineCommand::Shutdown => {
                    debug!("Servo engine received shutdown");
                    drop(active_webview);
                    return Ok(());
                }
            }
        }

        std::thread::yield_now();
    }
}

fn navigate_cmd(
    servo: &Servo,
    rendering_context: Rc<dyn RenderingContext>,
    active_webview: &mut Option<WebView>,
    url: &str,
) -> Result<String> {
    let parsed = Url::parse(url)
        .map_err(|e| VibeError::Navigation(format!("invalid URL: {e}")))?;

    let delegate = Rc::new(HeadlessWebViewDelegate);

    let webview = WebViewBuilder::new(servo, rendering_context.clone())
        .url(parsed.clone())
        .delegate(delegate)
        .build();

    // Wait for load to complete
    while webview.load_status() != servo::LoadStatus::Complete {
        servo.spin_event_loop();
        std::thread::yield_now();
    }

    *active_webview = Some(webview);
    Ok(url.to_string())
}

fn get_html_cmd(servo: &Servo, active_webview: &Option<WebView>) -> Result<String> {
    let webview = active_webview
        .as_ref()
        .ok_or_else(|| VibeError::Engine("no active webview".to_string()))?;

    let result = crate::browser::navigation::extract_html(servo, webview)?;
    Ok(result)
}

fn get_text_cmd(servo: &Servo, active_webview: &Option<WebView>) -> Result<String> {
    let webview = active_webview
        .as_ref()
        .ok_or_else(|| VibeError::Engine("no active webview".to_string()))?;

    let result = crate::browser::navigation::extract_text(servo, webview)?;
    Ok(result)
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
