use rust_mcp_sdk::macros::{JsonSchema, mcp_tool};
use serde::{Deserialize, Serialize};

#[mcp_tool(
    name = "browser_navigate",
    description = "Navigate to a URL and load the page"
)]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct NavigateTool {
    /// URL to navigate to
    pub url: String,
    /// Optional wait condition
    #[serde(default)]
    pub wait_until: Option<String>,
}

#[mcp_tool(
    name = "browser_snapshot",
    description = "Return the current page URL, title, and body text"
)]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct SnapshotTool {
    /// URL to capture
    pub url: String,
}

#[mcp_tool(
    name = "browser_extract",
    description = "Extract page content as Markdown, HTML, or plain text"
)]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct ExtractTool {
    /// URL to extract content from
    pub url: String,
    /// Output format: markdown, html, or text
    #[serde(default = "default_format")]
    pub format: String,
}

fn default_format() -> String {
    "markdown".to_string()
}
