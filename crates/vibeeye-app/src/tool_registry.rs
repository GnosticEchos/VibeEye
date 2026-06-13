//! Tool registry for managing and executing available tools

use crate::discovery::{Tool, ToolAdapter, ToolMetadata, TypedTool};
use std::collections::HashMap;

/// Registry of all available tools
pub struct ToolRegistry {
    tools: HashMap<String, Box<dyn Tool + Send + Sync>>,
}

impl Default for ToolRegistry {
    fn default() -> Self {
        let mut registry = Self {
            tools: HashMap::new(),
        };
        // Register built-in tools
        registry.register_tool(crate::tools::BrowseTool);
        registry.register_tool(crate::tools::SnapshotTool);
        registry.register_tool(crate::tools::ExtractTool);
        registry
    }
}

impl ToolRegistry {
    /// Create a new registry with default tools
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a strongly-typed tool
    pub fn register_tool<T: TypedTool + 'static>(&mut self, tool: T) {
        let name = T::name().to_string();
        self.tools.insert(name, Box::new(ToolAdapter(tool)));
    }

    /// Register an already-wrapped dynamic tool
    pub fn register(&mut self, tool: Box<dyn Tool + Send + Sync>) {
        let name = tool.metadata().name.clone();
        self.tools.insert(name, tool);
    }

    /// Get all tool metadata for discovery
    pub fn discover_all(&self) -> Vec<ToolMetadata> {
        self.tools.values().map(|t| t.metadata()).collect()
    }

    /// Get metadata for a specific tool
    pub fn get_metadata(&self, name: &str) -> Option<ToolMetadata> {
        self.tools.get(name).map(|t| t.metadata())
    }

    /// Execute a tool by name with JSON input
    pub async fn execute(
        &self,
        name: &str,
        input: serde_json::Value,
    ) -> crate::Result<serde_json::Value> {
        let tool = self
            .tools
            .get(name)
            .ok_or_else(|| crate::Error::InvalidInput(format!("Unknown tool: {}", name)))?;
        tool.execute_json(input).await
    }

    /// List all registered tool names
    pub fn list_tools(&self) -> Vec<String> {
        self.tools.keys().cloned().collect()
    }

    /// Check if a tool is registered
    pub fn has_tool(&self, name: &str) -> bool {
        self.tools.contains_key(name)
    }
}
