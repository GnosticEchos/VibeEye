//! Tool discovery and execution traits

use async_trait::async_trait;
use serde::{Deserialize, Serialize, de::DeserializeOwned};

/// Metadata for a discovered tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMetadata {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
    pub output_schema: serde_json::Value,
}

/// Strongly-typed tool trait. Not object-safe due to associated types.
#[async_trait]
pub trait TypedTool: Send + Sync {
    type Input: DeserializeOwned + Send;
    type Output: Serialize + Send;

    fn name() -> &'static str;
    fn description() -> &'static str;
    fn input_schema() -> serde_json::Value;
    fn output_schema() -> serde_json::Value;

    async fn execute(&self, input: Self::Input) -> crate::Result<Self::Output>;
}

/// Object-safe trait for dynamic tool execution and discovery.
#[async_trait]
pub trait Tool: Send + Sync {
    fn metadata(&self) -> ToolMetadata;
    async fn execute_json(&self, input: serde_json::Value) -> crate::Result<serde_json::Value>;
}

/// Wrapper to adapt a `TypedTool` into an object-safe `Tool`.
pub struct ToolAdapter<T: TypedTool>(pub T);

#[async_trait]
impl<T: TypedTool> Tool for ToolAdapter<T> {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            name: T::name().to_string(),
            description: T::description().to_string(),
            input_schema: T::input_schema(),
            output_schema: T::output_schema(),
        }
    }

    async fn execute_json(&self, input: serde_json::Value) -> crate::Result<serde_json::Value> {
        let typed_input: T::Input = serde_json::from_value(input)
            .map_err(|e| crate::AppError::InvalidInput(format!("Invalid tool input: {}", e)))?;

        let output = self.0.execute(typed_input).await?;

        serde_json::to_value(output).map_err(crate::AppError::Serde)
    }
}
