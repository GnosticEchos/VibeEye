//! WP07: End-to-end tool execution tests.
//!
//! Verifies that the shared tools (Browse, Snapshot, Extract) execute
//! correctly through the stub backend (no Servo required).

use vibeeye_app::{
    BrowseInput, BrowseTool, ExtractInput, ExtractTool, SnapshotInput, SnapshotTool, Tool,
};

#[tokio::test]
async fn test_browse_navigate_roundtrip() {
    let tool = BrowseTool;
    let input = BrowseInput {
        url: "https://example.com".to_string(),
        wait_until: None,
    };
    let output = tool.execute(input).await.unwrap();

    assert_eq!(output.current_url, "https://example.com");
    assert!(output.title.is_some());
    assert!(!output.title.as_ref().unwrap().is_empty());
}

#[tokio::test]
async fn test_snapshot_returns_html_and_text() {
    let tool = SnapshotTool;
    let input = SnapshotInput {
        url: "https://example.com/snapshot".to_string(),
    };
    let output = tool.execute(input).await.unwrap();

    assert_eq!(output.url, "https://example.com/snapshot");
    assert!(output.title.is_some());
    assert!(!output.html.is_empty());
    assert!(!output.body_text.is_empty());
}

#[tokio::test]
async fn test_extract_markdown() {
    let tool = ExtractTool;
    let input = ExtractInput {
        url: "https://example.com/extract".to_string(),
        format: "markdown".to_string(),
    };
    let output = tool.execute(input).await.unwrap();

    assert_eq!(output.url, "https://example.com/extract");
    assert!(!output.content.is_empty());
    assert_eq!(output.format, "markdown");
}

#[tokio::test]
async fn test_extract_html_passthrough() {
    let tool = ExtractTool;
    let input = ExtractInput {
        url: "https://example.com/extract".to_string(),
        format: "html".to_string(),
    };
    let output = tool.execute(input).await.unwrap();

    assert!(!output.content.is_empty());
    assert_eq!(output.format, "html");
}

#[tokio::test]
async fn test_extract_text() {
    let tool = ExtractTool;
    let input = ExtractInput {
        url: "https://example.com/extract".to_string(),
        format: "text".to_string(),
    };
    let output = tool.execute(input).await.unwrap();

    assert!(!output.content.is_empty());
    assert_eq!(output.format, "text");
}
