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

use crate::tools::{ExtractTool, NavigateTool, SnapshotTool};
use vibeeye_app::Tool;

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
        #[allow(unused_mut)]
        let mut tools = vec![
            NavigateTool::tool(),
            SnapshotTool::tool(),
            ExtractTool::tool(),
        ];

        #[cfg(feature = "surrealdb")]
        {
            tools.push(DbQueryTool::tool());
            tools.push(DbListTool::tool());
            tools.push(DbStatusTool::tool());
            tools.push(DbExportTool::tool());
            tools.push(DbImportTool::tool());
            tools.push(DbResetTool::tool());
            tools.push(DbResetAllTool::tool());
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
    match name {
        "browser_navigate" => call_navigate(arguments).await,
        "browser_snapshot" => call_snapshot(arguments).await,
        "browser_extract" => call_extract(arguments).await,
        _ => Err(CallToolError::unknown_tool(name.to_string())),
    }
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

// ── Browser tools ──────────────────────────────────────────────────────────

async fn call_navigate(
    arguments: serde_json::Map<String, serde_json::Value>,
) -> std::result::Result<CallToolResult, CallToolError> {
    let tool: NavigateTool = serde_json::from_value(serde_json::Value::Object(arguments))
        .map_err(|e| CallToolError::from_message(e.to_string()))?;
    let input = vibeeye_app::BrowseInput {
        url: tool.url,
        wait_until: tool.wait_until,
    };
    let output = vibeeye_app::BrowseTool
        .execute(input)
        .await
        .map_err(|e: vibeeye_app::AppError| CallToolError::from_message(e.to_string()))?;
    tool_result(&output)
}

async fn call_snapshot(
    arguments: serde_json::Map<String, serde_json::Value>,
) -> std::result::Result<CallToolResult, CallToolError> {
    let tool: SnapshotTool = serde_json::from_value(serde_json::Value::Object(arguments))
        .map_err(|e| CallToolError::from_message(e.to_string()))?;
    let input = vibeeye_app::SnapshotInput { url: tool.url };
    let output = vibeeye_app::SnapshotTool
        .execute(input)
        .await
        .map_err(|e: vibeeye_app::AppError| CallToolError::from_message(e.to_string()))?;
    tool_result(&output)
}

async fn call_extract(
    arguments: serde_json::Map<String, serde_json::Value>,
) -> std::result::Result<CallToolResult, CallToolError> {
    let tool: ExtractTool = serde_json::from_value(serde_json::Value::Object(arguments))
        .map_err(|e| CallToolError::from_message(e.to_string()))?;
    let input = vibeeye_app::ExtractInput {
        url: tool.url,
        format: tool.format,
    };
    let output = vibeeye_app::ExtractTool
        .execute(input)
        .await
        .map_err(|e: vibeeye_app::AppError| CallToolError::from_message(e.to_string()))?;
    tool_result(&output)
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

        let outputs = self.crawl_outputs(&tool).await?;
        let opts = vibeeye_app::crawl::CrawlOptions {
            url: tool.url.clone(),
            max_depth: tool.max_depth,
            max_pages: tool.max_pages as usize,
            format: vibeeye_core::ContentFormat::Markdown,
            respect_robots: true,
            requests_per_second: 2.0,
            concurrency: 4,
            same_origin: true,
            timeout_secs: 30,
            use_sitemap: true,
            settle_ms: 2000,
            outputs,
        };

        vibeeye_app::crawl::run(opts)
            .await
            .map_err(|e| CallToolError::from_message(e.to_string()))?;

        let group = vibeeye_app::db::util::derive_group(&tool.url, tool.group.as_deref());
        tool_result(&serde_json::json!({
            "status": "crawl_completed",
            "url": tool.url,
            "group": group,
            "max_depth": tool.max_depth,
            "max_pages": tool.max_pages,
            "note": format!("Query results with: db_query <query> --group {}", group),
        }))
    }

    async fn crawl_outputs(
        &self,
        tool: &CrawlTool,
    ) -> std::result::Result<
        Vec<std::sync::Arc<dyn vibeeye_app::crawl::output::CrawlOutput>>,
        CallToolError,
    > {
        if !tool.surrealdb {
            return Ok(vec![]);
        }
        let mut output = vibeeye_app::db::output::SurrealOutput::new(
            self.db.clone(),
            &tool.url,
            tool.group.as_deref(),
        );
        #[cfg(feature = "embeddings")]
        if tool.embed {
            let config = load_embedding_config()
                .await
                .map_err(|e| CallToolError::from_message(e.to_string()))?;
            output.embed_config = Some(config);
        }
        Ok(vec![std::sync::Arc::new(output)])
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
    let config = load_embedding_config()
        .await
        .map_err(|e| CallToolError::from_message(e.to_string()))?;
    vibeeye_app::embed::EmbeddingProvider::new(&config)
        .map_err(|e| CallToolError::from_message(e.to_string()))
}

#[cfg(feature = "embeddings")]
async fn load_embedding_config()
-> Result<vibeeye_app::config::embeddings::EmbeddingConfig, anyhow::Error> {
    let config_path = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("vibe-eye")
        .join("crawl.toml");
    let config = if config_path.exists() {
        vibeeye_app::config::CrawlConfig::load(Some(&config_path))?
    } else {
        vibeeye_app::config::CrawlConfig::default()
    };
    config.global.embeddings.ok_or_else(|| {
        anyhow::anyhow!(
            "no [embeddings] section found in config. Add one to ~/.config/vibe-eye/crawl.toml"
        )
    })
}

fn tool_result<T: serde::Serialize>(
    output: &T,
) -> std::result::Result<CallToolResult, CallToolError> {
    let text =
        serde_json::to_string(output).map_err(|e| CallToolError::from_message(e.to_string()))?;
    Ok(CallToolResult::text_content(vec![TextContent::from(text)]))
}
