use serde::{Deserialize, Serialize};

/// Viewport configuration for headless browser
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Viewport {
    pub width: u32,
    pub height: u32,
}

impl Default for Viewport {
    fn default() -> Self {
        Self {
            width: 1920,
            height: 1080,
        }
    }
}

/// Browser context maintaining rendering state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserContext {
    pub viewport: Viewport,
    pub user_agent: String,
    pub is_headless: bool,
}

impl Default for BrowserContext {
    fn default() -> Self {
        Self {
            viewport: Viewport::default(),
            user_agent: "VibeEye/0.1.0 (Headless; Servo 0.1.0)".to_string(),
            is_headless: true,
        }
    }
}

/// Navigation state tracking current URL and history
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NavigationState {
    pub current_url: Option<String>,
    pub history_stack: Vec<String>,
    pub pending_url: Option<String>,
}

/// Content extraction format options
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ContentFormat {
    Markdown,
    Html,
    Text,
}

/// Captured content buffer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderedBuffer {
    pub url: String,
    pub html_content: Option<String>,
    pub markdown_content: Option<String>,
    pub text_content: Option<String>,
    pub title: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}
