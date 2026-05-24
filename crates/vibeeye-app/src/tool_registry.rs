//! Tool registry for managing available tools

use crate::discovery::SonarDiscovery;
use std::collections::HashMap;

/// Registry of all available tools
pub struct ToolRegistry {
    tools: HashMap<String, Box<dyn SonarDiscovery + Send + Sync>>,
}

impl Default for ToolRegistry {
    fn default() -> Self {
        let mut registry = Self {
            tools: HashMap::new(),
        };
        // Register built-in tools
        registry.register(Box::new(crate::tools::BrowseTool));
        registry.register(Box::new(crate::tools::SnapshotTool));
        registry.register(Box::new(crate::tools::ExtractTool));
        registry
    }
}

impl ToolRegistry {
    /// Create a new registry with default tools
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a tool
    pub fn register(&mut self, tool: Box<dyn SonarDiscovery + Send + Sync>) {
        let name = tool.command_name().to_string();
        self.tools.insert(name, tool);
    }

    /// Get all tool metadata for Sonar discovery
    pub fn discover_all(&self) -> Vec<serde_json::Value> {
        self.tools
            .values()
            .map(|t| t.capability_metadata())
            .collect()
    }

    /// Get metadata for a specific tool
    pub fn get_metadata(&self, name: &str) -> Option<serde_json::Value> {
        self.tools.get(name).map(|t| t.capability_metadata())
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
