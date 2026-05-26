//! SonarDiscovery trait for reflective capability discovery

use async_trait::async_trait;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::json;

/// Static metadata for capability discovery (no instantiation required)
pub trait CapabilityProvider: Send + Sync {
    /// Tool name for registration
    fn name() -> &'static str;

    /// Tool description
    fn description() -> &'static str;

    /// JSON schema for tool input
    fn input_schema() -> serde_json::Value;

    /// JSON schema for tool output
    fn output_schema() -> serde_json::Value;
}

/// Dynamic trait for commands/tools that support reflective discovery (runtime)
pub trait SonarDiscovery: Send + Sync {
    /// Returns the command name
    fn command_name(&self) -> &str;

    /// Returns the description
    fn description(&self) -> &str;

    /// Returns full capability metadata as JSON
    fn capability_metadata(&self) -> serde_json::Value {
        json!({
            "name": self.command_name(),
            "description": self.description(),
        })
    }
}

/// Async tool execution trait for MCP/CLI parity
#[async_trait]
pub trait Tool: SonarDiscovery {
    /// Input type for this tool
    type Input: DeserializeOwned + Send;
    /// Output type for this tool
    type Output: Serialize + Send;

    /// Execute the tool with given input
    async fn execute(&self, input: Self::Input) -> crate::Result<Self::Output>;
}

/// Metadata for a discovered capability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityMetadata {
    pub name: String,
    pub description: String,
    pub arguments: Vec<ArgumentMetadata>,
}

/// Metadata for a single argument
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArgumentMetadata {
    pub name: String,
    pub description: String,
    pub required: bool,
    pub arg_type: String,
}

/// Convert a type to JSON schema for tool definitions
pub fn type_to_schema<T: schemars::JsonSchema>() -> serde_json::Value {
    serde_json::to_value(schemars::schema_for!(T)).unwrap()
}
