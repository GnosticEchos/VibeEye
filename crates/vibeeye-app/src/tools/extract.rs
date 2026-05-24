//! Content extraction tool for VibeEye

use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::discovery::{CapabilityProvider, SonarDiscovery, Tool};
use crate::Result;
use vibeeye_core::ContentFormat;

/// Input for the extract tool
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct ExtractInput {
    /// URL to extract content from
    pub url: String,
    /// Output format: markdown, html, or text
    #[serde(default = "default_format")]
    pub format: String,
}

fn default_format() -> String {
    "markdown".to_string()
}

/// Output from the extract tool
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct ExtractOutput {
    /// Source URL
    pub url: String,
    /// Extracted content
    pub content: String,
    /// Format of extracted content
    pub format: String,
    /// Page title
    pub title: Option<String>,
}

/// Extract content from a page (Markdown, HTML, or plain text)
#[derive(Debug, Default)]
pub struct ExtractTool;

impl CapabilityProvider for ExtractTool {
    fn name() -> &'static str {
        "browser_extract"
    }

    fn description() -> &'static str {
        "Extract page content as Markdown, HTML, or plain text"
    }

    fn input_schema() -> serde_json::Value {
        serde_json::to_value(schemars::schema_for!(ExtractInput)).unwrap()
    }

    fn output_schema() -> serde_json::Value {
        serde_json::to_value(schemars::schema_for!(ExtractOutput)).unwrap()
    }
}

impl SonarDiscovery for ExtractTool {
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

/// Parse format string to ContentFormat
fn parse_format(format: &str) -> ContentFormat {
    match format {
        "html" => ContentFormat::Html,
        "text" => ContentFormat::Text,
        _ => ContentFormat::Markdown,
    }
}

#[async_trait]
impl Tool for ExtractTool {
    type Input = ExtractInput;
    type Output = ExtractOutput;

    async fn execute(&self, input: Self::Input) -> Result<Self::Output> {
        let capture = crate::tools::common::navigate_and_capture(&input.url).await?;
        let content = crate::extraction::extract(&capture.html, parse_format(&input.format))?;

        Ok(ExtractOutput {
            url: capture.url,
            content,
            format: input.format,
            title: capture.title,
        })
    }
}
