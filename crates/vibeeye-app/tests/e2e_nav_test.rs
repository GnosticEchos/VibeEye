//! WP07: End-to-end tool execution tests.
//!
//! Verifies that the shared tools (Browse, Snapshot, Extract) execute
//! correctly through the stub backend (no Servo required).

use vibeeye_app::{
    BrowseInput, BrowseTool, ExtractInput, ExtractTool, SnapshotInput, SnapshotTool, TypedTool,
};

fn setup_test_env() {
    // SAFETY: set_var in tests is safe because no other threads read
    // this env var concurrently during test execution.
    unsafe { std::env::set_var("VIBEYE_TEST_STUB", "1") };
}

#[tokio::test]
async fn test_browse_navigate_roundtrip() {
    setup_test_env();
    let tool = BrowseTool;
    let input = BrowseInput {
        url: "https://example.com".to_string(),
        wait_until: None,
    };
    let output = TypedTool::execute(&tool, input).await.unwrap();

    assert_eq!(output.current_url, "https://example.com");
    assert!(output.title.is_some());
    assert!(!output.title.as_ref().unwrap().is_empty());
}

#[tokio::test]
async fn test_snapshot_returns_html_and_text() {
    setup_test_env();
    let tool = SnapshotTool;
    let input = SnapshotInput {
        url: "https://example.com/snapshot".to_string(),
    };
    let output = TypedTool::execute(&tool, input).await.unwrap();

    assert_eq!(output.url, "https://example.com/snapshot");
    assert!(output.title.is_some());
    assert!(!output.html.is_empty());
    assert!(!output.body_text.is_empty());
}

#[tokio::test]
async fn test_extract_markdown() {
    setup_test_env();
    let tool = ExtractTool;
    let input = ExtractInput {
        url: "https://example.com/extract".to_string(),
        format: "markdown".to_string(),
    };
    let output = TypedTool::execute(&tool, input).await.unwrap();

    assert_eq!(output.url, "https://example.com/extract");
    assert!(!output.content.is_empty());
    assert_eq!(output.format, "markdown");
}

#[tokio::test]
async fn test_extract_html_passthrough() {
    setup_test_env();
    let tool = ExtractTool;
    let input = ExtractInput {
        url: "https://example.com/extract".to_string(),
        format: "html".to_string(),
    };
    let output = TypedTool::execute(&tool, input).await.unwrap();

    assert!(!output.content.is_empty());
    assert_eq!(output.format, "html");
}

#[tokio::test]
async fn test_extract_text() {
    setup_test_env();
    let tool = ExtractTool;
    let input = ExtractInput {
        url: "https://example.com/extract".to_string(),
        format: "text".to_string(),
    };
    let output = TypedTool::execute(&tool, input).await.unwrap();

    assert!(!output.content.is_empty());
    assert_eq!(output.format, "text");
}
