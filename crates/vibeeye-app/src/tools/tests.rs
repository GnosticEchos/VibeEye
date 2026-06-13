//! Tests for tool implementations

use crate::discovery::TypedTool;
use crate::tools::*;

#[tokio::test]
async fn test_browse_tool_execute() {
    let tool = BrowseTool;
    let input = BrowseInput {
        url: "https://example.com".to_string(),
        wait_until: None,
    };

    let result = TypedTool::execute(&tool, input).await;
    assert!(result.is_ok());

    let output = result.unwrap();
    assert!(output.success);
    assert_eq!(output.current_url, "https://example.com");
}

#[tokio::test]
async fn test_snapshot_tool_execute() {
    let tool = SnapshotTool;
    let input = SnapshotInput {
        url: "https://example.com".to_string(),
    };

    let result = TypedTool::execute(&tool, input).await;
    assert!(result.is_ok());

    let output = result.unwrap();
    assert_eq!(output.url, "https://example.com");
    assert!(!output.html.is_empty());
}

#[tokio::test]
async fn test_extract_tool_markdown() {
    let tool = ExtractTool;
    let input = ExtractInput {
        url: "https://example.com".to_string(),
        format: "markdown".to_string(),
    };

    let result = TypedTool::execute(&tool, input).await;
    assert!(result.is_ok());

    let output = result.unwrap();
    assert_eq!(output.url, "https://example.com");
    assert_eq!(output.format, "markdown");
}

#[tokio::test]
async fn test_extract_tool_html() {
    let tool = ExtractTool;
    let input = ExtractInput {
        url: "https://example.com".to_string(),
        format: "html".to_string(),
    };

    let result = TypedTool::execute(&tool, input).await;
    assert!(result.is_ok());

    let output = result.unwrap();
    assert!(output.content.contains("<html>"));
}

#[tokio::test]
async fn test_extract_tool_text() {
    let tool = ExtractTool;
    let input = ExtractInput {
        url: "https://example.com".to_string(),
        format: "text".to_string(),
    };

    let result = TypedTool::execute(&tool, input).await;
    assert!(result.is_ok());

    let output = result.unwrap();
    assert!(!output.content.is_empty());
}
