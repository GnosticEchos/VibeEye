//! Snapshot tool for VibeEye

use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::Result;
use crate::discovery::{CapabilityProvider, SonarDiscovery, Tool};

/// Input for the snapshot tool
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct SnapshotInput {
    /// URL to capture (if not already navigated)
    pub url: String,
}

/// Output from the snapshot tool
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct SnapshotOutput {
    /// Page URL
    pub url: String,
    /// Page title
    pub title: Option<String>,
    /// Page body text
    pub body_text: String,
    /// Raw HTML content
    pub html: String,
}

/// Capture a page snapshot (URL, title, body text, HTML)
#[derive(Debug, Default)]
pub struct SnapshotTool;

impl CapabilityProvider for SnapshotTool {
    fn name() -> &'static str {
        "browser_snapshot"
    }

    fn description() -> &'static str {
        "Return the current page URL, title, and body text"
    }

    fn input_schema() -> serde_json::Value {
        serde_json::to_value(schemars::schema_for!(SnapshotInput)).unwrap()
    }

    fn output_schema() -> serde_json::Value {
        serde_json::to_value(schemars::schema_for!(SnapshotOutput)).unwrap()
    }
}

impl SonarDiscovery for SnapshotTool {
    fn command_name(&self) -> &str {
        <Self as CapabilityProvider>::name()
    }

    fn description(&self) -> &str {
        <Self as CapabilityProvider>::description()
    }

    fn capability_metadata(&self) -> serde_json::Value {
        json!({
            "name": <Self as CapabilityProvider>::name(),
            "description": <Self as CapabilityProvider>::description(),
            "inputSchema": <Self as CapabilityProvider>::input_schema(),
            "outputSchema": <Self as CapabilityProvider>::output_schema(),
        })
    }
}

#[async_trait]
impl Tool for SnapshotTool {
    type Input = SnapshotInput;
    type Output = SnapshotOutput;

    async fn execute(&self, input: Self::Input) -> Result<Self::Output> {
        let capture = crate::tools::common::navigate_and_capture(&input.url).await?;

        let body_text = crate::extraction::strip_html(&capture.html);

        Ok(SnapshotOutput {
            url: capture.url,
            title: capture.title,
            body_text,
            html: capture.html,
        })
    }
}
