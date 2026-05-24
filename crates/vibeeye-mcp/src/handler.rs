use async_trait::async_trait;
use rust_mcp_sdk::{
    mcp_server::ServerHandler,
    schema::{
        schema_utils::CallToolError, CallToolRequestParams, CallToolResult, ListToolsResult,
        PaginatedRequestParams, RpcError, TextContent,
    },
    McpServer,
};
use std::sync::Arc;

use crate::tools::{ExtractTool, NavigateTool, SnapshotTool};
use vibeeye_app::Tool;

pub struct VibeEyeMcpHandler;

#[async_trait]
impl ServerHandler for VibeEyeMcpHandler {
    async fn handle_list_tools_request(
        &self,
        _request: Option<PaginatedRequestParams>,
        _runtime: Arc<dyn McpServer>,
    ) -> std::result::Result<ListToolsResult, RpcError> {
        Ok(ListToolsResult {
            meta: None,
            next_cursor: None,
            tools: vec![
                NavigateTool::tool(),
                SnapshotTool::tool(),
                ExtractTool::tool(),
            ],
        })
    }

    async fn handle_call_tool_request(
        &self,
        params: CallToolRequestParams,
        _runtime: Arc<dyn McpServer>,
    ) -> std::result::Result<CallToolResult, CallToolError> {
        let arguments = params.arguments.unwrap_or_default();
        match params.name.as_str() {
            "browser_navigate" => call_navigate(arguments).await,
            "browser_snapshot" => call_snapshot(arguments).await,
            "browser_extract" => call_extract(arguments).await,
            _ => Err(CallToolError::unknown_tool(params.name)),
        }
    }
}

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

fn tool_result<T: serde::Serialize>(
    output: &T,
) -> std::result::Result<CallToolResult, CallToolError> {
    let text =
        serde_json::to_string(output).map_err(|e| CallToolError::from_message(e.to_string()))?;
    Ok(CallToolResult::text_content(vec![TextContent::from(text)]))
}
