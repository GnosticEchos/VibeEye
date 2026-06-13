use async_trait::async_trait;
use rust_mcp_sdk::{
    McpServer,
    mcp_server::ServerHandler,
    schema::{
        CallToolRequestParams, CallToolResult, ListToolsResult, PaginatedRequestParams, RpcError,
        TextContent, schema_utils::CallToolError,
    },
};
use std::sync::Arc;
use vibeeye_app::ToolRegistry;

#[cfg(feature = "surrealdb")]
use crate::tools::{
    CrawlTool, DbExportTool, DbImportTool, DbListTool, DbQueryTool, DbResetAllTool, DbResetTool,
    DbStatusTool,
};

#[cfg(feature = "embeddings")]
use crate::tools::{DbHybridTool, DbVectorTool};

#[cfg(feature = "surrealdb")]
#[derive(Debug, Clone)]
pub struct VibeEyeMcpHandler {
    db: vibeeye_app::db::DbClient,
}

#[cfg(not(feature = "surrealdb"))]
#[derive(Debug, Clone)]
pub struct VibeEyeMcpHandler;

#[cfg(feature = "surrealdb")]
impl VibeEyeMcpHandler {
    pub fn new(db: vibeeye_app::db::DbClient) -> Self {
        Self { db }
    }
}

#[cfg(not(feature = "surrealdb"))]
impl VibeEyeMcpHandler {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl ServerHandler for VibeEyeMcpHandler {
    async fn handle_list_tools_request(
        &self,
        _request: Option<PaginatedRequestParams>,
        _runtime: Arc<dyn McpServer>,
    ) -> std::result::Result<ListToolsResult, RpcError> {
        let registry = ToolRegistry::new();
        #[allow(unused_mut)]
        let mut tools: Vec<rust_mcp_sdk::schema::Tool> = Vec::new();

        for meta in registry.discover_all() {
            let tool_json = serde_json::json!({
                "name": meta.name,
                "description": meta.description,
                "inputSchema": meta.input_schema,
            });
            let mcp_tool: rust_mcp_sdk::schema::Tool = serde_json::from_value(tool_json)
                .map_err(|e| RpcError::internal_error().with_message(e.to_string()))?;
            tools.push(mcp_tool);
        }

        #[cfg(feature = "surrealdb")]
        {
            tools.push(DbQueryTool::tool());
            tools.push(DbListTool::tool());
            tools.push(DbStatusTool::tool());
            tools.push(DbExportTool::tool());
            tools.push(DbImportTool::tool());
            tools.push(DbResetTool::tool());
            tools.push(DbResetAllTool::tool());
            tools.push(CrawlTool::tool());
        }

        #[cfg(feature = "embeddings")]
        {
            tools.push(DbVectorTool::tool());
            tools.push(DbHybridTool::tool());
        }

        Ok(ListToolsResult {
            meta: None,
            next_cursor: None,
            tools,
        })
    }

    async fn handle_call_tool_request(
        &self,
        params: CallToolRequestParams,
        _runtime: Arc<dyn McpServer>,
    ) -> std::result::Result<CallToolResult, CallToolError> {
        let arguments = params.arguments.unwrap_or_default();
        let name = params.name.as_str();
        if name.starts_with("browser_") {
            dispatch_browser(name, arguments).await
        } else if name.starts_with("db_") {
            dispatch_db(self, name, arguments).await
        } else if name == "crawl" {
            #[cfg(feature = "surrealdb")]
            return self.call_crawl(arguments).await;
            #[cfg(not(feature = "surrealdb"))]
            return Err(CallToolError::unknown_tool(params.name));
        } else {
            Err(CallToolError::unknown_tool(params.name))
        }
    }
}

async fn dispatch_browser(
    name: &str,
    arguments: serde_json::Map<String, serde_json::Value>,
) -> std::result::Result<CallToolResult, CallToolError> {
    let registry = ToolRegistry::new();
    let input_value = serde_json::Value::Object(arguments);
    let output_value = registry
        .execute(name, input_value)
        .await
        .map_err(|e| CallToolError::from_message(e.to_string()))?;
    tool_result(&output_value)
}

#[cfg(feature = "surrealdb")]
async fn dispatch_db(
    handler: &VibeEyeMcpHandler,
    name: &str,
    arguments: serde_json::Map<String, serde_json::Value>,
) -> std::result::Result<CallToolResult, CallToolError> {
    match name {
        "db_query" => handler.call_db_query(arguments).await,
        "db_list" => handler.call_db_list(arguments).await,
        "db_status" => handler.call_db_status(arguments).await,
        #[cfg(feature = "embeddings")]
        "db_vector" => handler.call_db_vector(arguments).await,
        #[cfg(feature = "embeddings")]
        "db_hybrid" => handler.call_db_hybrid(arguments).await,
        "db_export" => handler.call_db_export(arguments).await,
        "db_import" => handler.call_db_import(arguments).await,
        "db_reset" => handler.call_db_reset(arguments).await,
        "db_reset_all" => handler.call_db_reset_all(arguments).await,
        _ => Err(CallToolError::unknown_tool(name.to_string())),
    }
}

#[cfg(not(feature = "surrealdb"))]
async fn dispatch_db(
    _handler: &VibeEyeMcpHandler,
    name: &str,
    _arguments: serde_json::Map<String, serde_json::Value>,
) -> std::result::Result<CallToolResult, CallToolError> {
    Err(CallToolError::unknown_tool(name.to_string()))
}

// ── SurrealDB tools ────────────────────────────────────────────────────────

#[cfg(feature = "surrealdb")]
impl VibeEyeMcpHandler {
    async fn call_db_query(
        &self,
        arguments: serde_json::Map<String, serde_json::Value>,
    ) -> std::result::Result<CallToolResult, CallToolError> {
        let tool: DbQueryTool = serde_json::from_value(serde_json::Value::Object(arguments))
            .map_err(|e| CallToolError::from_message(e.to_string()))?;
        let results = self
            .db
            .bm25_search(tool.group.as_deref(), &tool.query, tool.limit as usize)
            .await
            .map_err(|e| CallToolError::from_message(e.to_string()))?;
        tool_result(&results)
    }

    async fn call_db_list(
        &self,
        _arguments: serde_json::Map<String, serde_json::Value>,
    ) -> std::result::Result<CallToolResult, CallToolError> {
        let groups = self
            .db
            .list_groups()
            .await
            .map_err(|e| CallToolError::from_message(e.to_string()))?;
        tool_result(&groups)
    }

    async fn call_db_status(
        &self,
        arguments: serde_json::Map<String, serde_json::Value>,
    ) -> std::result::Result<CallToolResult, CallToolError> {
        let tool: DbStatusTool = serde_json::from_value(serde_json::Value::Object(arguments))
            .map_err(|e| CallToolError::from_message(e.to_string()))?;
        let stats = self
            .db
            .group_stats(&tool.group)
            .await
            .map_err(|e| CallToolError::from_message(e.to_string()))?;
        tool_result(&stats)
    }

    async fn call_db_export(
        &self,
        arguments: serde_json::Map<String, serde_json::Value>,
    ) -> std::result::Result<CallToolResult, CallToolError> {
        let tool: DbExportTool = serde_json::from_value(serde_json::Value::Object(arguments))
            .map_err(|e| CallToolError::from_message(e.to_string()))?;
        let mut file = std::fs::File::create(&tool.target_path)
            .map_err(|e| CallToolError::from_message(e.to_string()))?;
        vibeeye_app::db::export::export_group(&self.db, &tool.group, &mut file)
            .await
            .map_err(|e| CallToolError::from_message(e.to_string()))?;
        tool_result(&serde_json::json!({"status": "exported", "path": tool.target_path}))
    }

    async fn call_db_import(
        &self,
        arguments: serde_json::Map<String, serde_json::Value>,
    ) -> std::result::Result<CallToolResult, CallToolError> {
        let tool: DbImportTool = serde_json::from_value(serde_json::Value::Object(arguments))
            .map_err(|e| CallToolError::from_message(e.to_string()))?;
        let path = std::path::Path::new(&tool.source_path);
        let source = if path.extension().and_then(|e| e.to_str()) == Some("surql") {
            vibeeye_app::db::import::ImportSource::SurqlFile(path)
        } else {
            vibeeye_app::db::import::ImportSource::OutputDirectory(path)
        };
        vibeeye_app::db::import::import(&self.db, &tool.group, source)
            .await
            .map_err(|e| CallToolError::from_message(e.to_string()))?;
        tool_result(&serde_json::json!({"status": "imported", "path": tool.source_path}))
    }

    async fn call_crawl(
        &self,
        arguments: serde_json::Map<String, serde_json::Value>,
    ) -> std::result::Result<CallToolResult, CallToolError> {
        let tool: CrawlTool = serde_json::from_value(serde_json::Value::Object(arguments))
            .map_err(|e| CallToolError::from_message(e.to_string()))?;

        let group = vibeeye_app::db::util::derive_group(&tool.url, tool.group.as_deref());
        let cli = format!(
            "vibe-eye crawl '{}' --group {}{}",
            tool.url,
            group,
            if tool.embed { " --embed" } else { "" }
        );

        tool_result(&serde_json::json!({
            "status": "needs_cli",
            "reason": "Crawls are long-running operations that can tie up MCP resources and block the agent session. Run this in your terminal for full control, progress output, and proper process lifecycle management.",
            "suggested_cli": cli,
            "url": tool.url,
            "group": group,
            "max_depth": tool.max_depth,
            "max_pages": tool.max_pages,
            "embed": tool.embed,
        }))
    }

    async fn call_db_reset(
        &self,
        arguments: serde_json::Map<String, serde_json::Value>,
    ) -> std::result::Result<CallToolResult, CallToolError> {
        let tool: DbResetTool = serde_json::from_value(serde_json::Value::Object(arguments))
            .map_err(|e| CallToolError::from_message(e.to_string()))?;
        self.db
            .reset_group(&tool.group)
            .await
            .map_err(|e| CallToolError::from_message(e.to_string()))?;
        tool_result(&serde_json::json!({"status": "reset", "group": tool.group}))
    }

    async fn call_db_reset_all(
        &self,
        _arguments: serde_json::Map<String, serde_json::Value>,
    ) -> std::result::Result<CallToolResult, CallToolError> {
        self.db
            .reset_all()
            .await
            .map_err(|e| CallToolError::from_message(e.to_string()))?;
        tool_result(&serde_json::json!({"status": "reset all"}))
    }
}

// ── Embedding tools ────────────────────────────────────────────────────────

#[cfg(feature = "embeddings")]
impl VibeEyeMcpHandler {
    async fn call_db_vector(
        &self,
        arguments: serde_json::Map<String, serde_json::Value>,
    ) -> std::result::Result<CallToolResult, CallToolError> {
        let tool: DbVectorTool = serde_json::from_value(serde_json::Value::Object(arguments))
            .map_err(|e| CallToolError::from_message(e.to_string()))?;
        let provider = load_embedding_provider()
            .await
            .map_err(|e| CallToolError::from_message(e.to_string()))?;
        let embedding = provider
            .embed_single(&tool.query)
            .await
            .map_err(|e| CallToolError::from_message(e.to_string()))?;
        let results = self
            .db
            .knn_search(tool.group.as_deref(), &embedding, tool.limit as usize)
            .await
            .map_err(|e| CallToolError::from_message(e.to_string()))?;
        tool_result(&results)
    }

    async fn call_db_hybrid(
        &self,
        arguments: serde_json::Map<String, serde_json::Value>,
    ) -> std::result::Result<CallToolResult, CallToolError> {
        let tool: DbHybridTool = serde_json::from_value(serde_json::Value::Object(arguments))
            .map_err(|e| CallToolError::from_message(e.to_string()))?;
        let provider = load_embedding_provider()
            .await
            .map_err(|e| CallToolError::from_message(e.to_string()))?;
        let embedding = provider
            .embed_single(&tool.query)
            .await
            .map_err(|e| CallToolError::from_message(e.to_string()))?;
        let results = self
            .db
            .hybrid_search(
                tool.group.as_deref(),
                &tool.query,
                &embedding,
                tool.bm25_limit as usize,
                tool.limit as usize,
            )
            .await
            .map_err(|e| CallToolError::from_message(e.to_string()))?;
        tool_result(&results)
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

#[cfg(feature = "embeddings")]
async fn load_embedding_provider() -> Result<vibeeye_app::embed::EmbeddingProvider, CallToolError> {
    let config = vibeeye_app::config::embeddings::load_embedding_config()
        .map_err(|e| CallToolError::from_message(e.to_string()))?;
    vibeeye_app::embed::EmbeddingProvider::new(&config)
        .map_err(|e| CallToolError::from_message(e.to_string()))
}

fn tool_result<T: serde::Serialize>(
    output: &T,
) -> std::result::Result<CallToolResult, CallToolError> {
    let text =
        serde_json::to_string(output).map_err(|e| CallToolError::from_message(e.to_string()))?;
    Ok(CallToolResult::text_content(vec![TextContent::from(text)]))
}
