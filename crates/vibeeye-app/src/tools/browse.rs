//! Browse/Navigate tool for VibeEye

use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::Result;
use crate::discovery::TypedTool;

/// Input for the browse tool
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct BrowseInput {
    /// URL to navigate to
    pub url: String,
    /// Optional wait condition (stub for WP05)
    #[serde(default)]
    pub wait_until: Option<String>,
}

/// Output from the browse tool
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct BrowseOutput {
    /// Whether navigation succeeded
    pub success: bool,
    /// Current URL after navigation
    pub current_url: String,
    /// Page title
    pub title: Option<String>,
}

/// Navigate to a URL
#[derive(Debug, Default)]
pub struct BrowseTool;

#[async_trait]
impl TypedTool for BrowseTool {
    type Input = BrowseInput;
    type Output = BrowseOutput;

    fn name() -> &'static str {
        "browser_navigate"
    }

    fn description() -> &'static str {
        "Navigate to a URL and load the page"
    }

    fn input_schema() -> serde_json::Value {
        serde_json::to_value(schemars::schema_for!(BrowseInput)).unwrap()
    }

    fn output_schema() -> serde_json::Value {
        serde_json::to_value(schemars::schema_for!(BrowseOutput)).unwrap()
    }

    async fn execute(&self, input: Self::Input) -> Result<Self::Output> {
        let capture = crate::tools::common::navigate_and_capture(&input.url).await?;

        Ok(BrowseOutput {
            success: true,
            current_url: capture.url,
            title: capture.title,
        })
    }
}
