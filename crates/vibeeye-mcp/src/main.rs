mod handler;
mod tools;

use handler::VibeEyeMcpHandler;
use rust_mcp_sdk::{
    McpServer, ServerDetails, StdioTransport, ToMcpServerHandler, TransportOptions,
    error::SdkResult,
    mcp_server::{McpServerOptions, ServerRuntime, server_runtime},
    schema::{Implementation, ServerCapabilities, ServerCapabilitiesTools},
};
use std::sync::Arc;
use tracing::info;

#[tokio::main]
async fn main() -> SdkResult<()> {
    tracing_subscriber::fmt::init();
    info!("VibeEye MCP Server starting...");

    let server_details = ServerDetails {
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
        instructions: None,
        meta: None,
    };

    let transport = StdioTransport::new(TransportOptions::default())?;

    #[cfg(feature = "surrealdb")]
    let handler = {
        let db_url = vibeeye_app::config::resolve_db_url();
        let client = vibeeye_app::db::DbClient::connect(&db_url)
            .await
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        client
            .use_ns_db("vibeeye", "crawl")
            .await
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        VibeEyeMcpHandler::new(client)
    };
    #[cfg(not(feature = "surrealdb"))]
    let handler = VibeEyeMcpHandler::new();

    let server: Arc<ServerRuntime> = server_runtime::create_server(McpServerOptions {
        server_details,
        transport,
        handler: handler.to_mcp_server_handler(),
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
