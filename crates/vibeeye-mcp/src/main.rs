mod handler;
mod tools;

use handler::VibeEyeMcpHandler;
use rust_mcp_sdk::{
    error::SdkResult,
    mcp_server::{server_runtime, McpServerOptions, ServerRuntime},
    schema::{
        Implementation, InitializeResult, ProtocolVersion, ServerCapabilities,
        ServerCapabilitiesTools,
    },
    McpServer, StdioTransport, ToMcpServerHandler, TransportOptions,
};
use std::sync::Arc;
use tracing::info;

#[tokio::main]
async fn main() -> SdkResult<()> {
    tracing_subscriber::fmt::init();
    info!("VibeEye MCP Server starting...");

    let server_details = InitializeResult {
        server_info: Implementation {
            name: "vibeeye-mcp".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            title: Some("VibeEye MCP Server".into()),
            description: Some("Thin MCP interface over vibeeye-app tools".into()),
            icons: vec![],
            website_url: None,
        },
        capabilities: ServerCapabilities {
            tools: Some(ServerCapabilitiesTools { list_changed: None }),
            ..Default::default()
        },
        meta: None,
        instructions: None,
        protocol_version: ProtocolVersion::V2025_11_25.into(),
    };

    let transport = StdioTransport::new(TransportOptions::default())?;
    let handler = VibeEyeMcpHandler {};

    let server: Arc<ServerRuntime> = server_runtime::create_server(McpServerOptions {
        server_details,
        transport,
        handler: handler.to_mcp_server_handler(),
        task_store: None,
        client_task_store: None,
        message_observer: None,
    });

    if let Err(start_error) = server.start().await {
        eprintln!(
            "{}",
            start_error
                .rpc_error_message()
                .unwrap_or(&start_error.to_string())
        );
    }
    Ok(())
}
