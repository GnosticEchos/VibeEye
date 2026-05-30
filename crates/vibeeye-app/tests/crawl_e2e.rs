//! End-to-end crawl tests (stub backend — no Servo required).
//!
//! Verifies that `crawl::run` completes successfully, produces results,
//! and writes output files correctly. The stub backend returns HTML
//! with no links, so BFS only ever discovers the seed page.

use std::path::PathBuf;
use std::sync::Arc;
use vibeeye_app::crawl::output::{DirectoryOutput, StdoutOutput};
use vibeeye_app::crawl::{CrawlOptions, run};
use vibeeye_core::ContentFormat;

#[tokio::test]
async fn test_crawl_seed_page_completes() {
    let opts = CrawlOptions {
        url: "https://example.com".to_string(),
        max_depth: 2,
        max_pages: 10,
        format: ContentFormat::Markdown,
        respect_robots: false,
        requests_per_second: 100.0,
        concurrency: 1,
        same_origin: true,
        timeout_secs: 5,
        use_sitemap: false,
        outputs: vec![Arc::new(StdoutOutput)],
    };

    let result = run(opts).await;
    assert!(
        result.is_ok(),
        "crawl should complete without error: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn test_crawl_writes_output_directory() {
    let temp_dir = tempfile::tempdir().unwrap();
    let output_path = temp_dir.path().join("crawl-output");

    let opts = CrawlOptions {
        url: "https://example.com".to_string(),
        max_depth: 2,
        max_pages: 10,
        format: ContentFormat::Markdown,
        respect_robots: false,
        requests_per_second: 100.0,
        concurrency: 1,
        same_origin: true,
        timeout_secs: 5,
        use_sitemap: false,
        outputs: vec![Arc::new(DirectoryOutput::new(output_path.clone(), "md"))],
    };

    run(opts).await.unwrap();

    // Should create the output directory
    assert!(output_path.exists(), "output directory should be created");

    // Should write at least one page file and a manifest
    let entries: Vec<PathBuf> = std::fs::read_dir(&output_path)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();

    let manifest = entries
        .iter()
        .find(|p| p.file_name().unwrap() == "manifest.json");
    assert!(manifest.is_some(), "manifest.json should be written");

    let page_files: Vec<_> = entries
        .iter()
        .filter(|p| p.extension().is_some_and(|ext| ext == "md"))
        .collect();
    assert!(
        !page_files.is_empty(),
        "at least one .md file should be written"
    );

    // Manifest should contain the seed page
    let manifest_json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(manifest.unwrap()).unwrap()).unwrap();
    let first = manifest_json.as_array().unwrap().first().unwrap();
    assert_eq!(first["url"], "https://example.com/");
}

#[tokio::test]
async fn test_crawl_respects_max_pages() {
    let temp_dir = tempfile::tempdir().unwrap();
    let output_path = temp_dir.path().join("crawl-output");

    let opts = CrawlOptions {
        url: "https://example.com".to_string(),
        max_depth: 2,
        max_pages: 1,
        format: ContentFormat::Html,
        respect_robots: false,
        requests_per_second: 100.0,
        concurrency: 1,
        same_origin: true,
        timeout_secs: 5,
        use_sitemap: false,
        outputs: vec![Arc::new(DirectoryOutput::new(output_path.clone(), "html"))],
    };

    run(opts).await.unwrap();

    let entries: Vec<PathBuf> = std::fs::read_dir(&output_path)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();

    // Exactly 1 page file + manifest
    let page_files: Vec<_> = entries
        .iter()
        .filter(|p| p.extension().is_some_and(|ext| ext == "html"))
        .collect();
    assert_eq!(
        page_files.len(),
        1,
        "only 1 page should be written with max_pages=1"
    );
}

#[tokio::test]
async fn test_crawl_html_format_output() {
    let temp_dir = tempfile::tempdir().unwrap();
    let output_path = temp_dir.path().join("crawl-output");

    let opts = CrawlOptions {
        url: "https://example.com".to_string(),
        max_depth: 2,
        max_pages: 10,
        format: ContentFormat::Html,
        respect_robots: false,
        requests_per_second: 100.0,
        concurrency: 1,
        same_origin: true,
        timeout_secs: 5,
        use_sitemap: false,
        outputs: vec![Arc::new(DirectoryOutput::new(output_path.clone(), "html"))],
    };

    run(opts).await.unwrap();

    let entries: Vec<PathBuf> = std::fs::read_dir(&output_path)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();

    let html_files: Vec<_> = entries
        .iter()
        .filter(|p| p.extension().is_some_and(|ext| ext == "html"))
        .collect();
    assert!(
        !html_files.is_empty(),
        ".html files should be written when format=html"
    );
}

#[tokio::test]
async fn test_crawl_text_format_output() {
    let temp_dir = tempfile::tempdir().unwrap();
    let output_path = temp_dir.path().join("crawl-output");

    let opts = CrawlOptions {
        url: "https://example.com".to_string(),
        max_depth: 2,
        max_pages: 10,
        format: ContentFormat::Text,
        respect_robots: false,
        requests_per_second: 100.0,
        concurrency: 1,
        same_origin: true,
        timeout_secs: 5,
        use_sitemap: false,
        outputs: vec![Arc::new(DirectoryOutput::new(output_path.clone(), "txt"))],
    };

    run(opts).await.unwrap();

    let entries: Vec<PathBuf> = std::fs::read_dir(&output_path)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();

    let txt_files: Vec<_> = entries
        .iter()
        .filter(|p| p.extension().is_some_and(|ext| ext == "txt"))
        .collect();
    assert!(
        !txt_files.is_empty(),
        ".txt files should be written when format=text"
    );
}
