//! CLI command handlers

use anyhow::Result;
use serde::Serialize;
use std::io::Write;
use std::path::PathBuf;

use vibeeye_app::config::CrawlConfig;
use vibeeye_app::crawl::{self, CrawlOptions};
use vibeeye_app::discovery::Tool;
use vibeeye_app::tools::{
    BrowseInput, BrowseTool, ExtractInput, ExtractTool, SnapshotInput, SnapshotTool,
};
use vibeeye_core::ContentFormat;

use crate::cli::Commands;
#[cfg(feature = "surrealdb")]
use crate::cli::{DbCommands, OutputFormat};
#[cfg(feature = "surrealdb")]
use crate::format::format_value;

/// Return the SurrealDB connection URL.
#[cfg(feature = "surrealdb")]
fn db_url() -> String {
    vibeeye_app::config::resolve_db_url()
}

/// Run the selected command
pub async fn run(command: Commands) -> Result<()> {
    match command {
        Commands::Navigate { url } => navigate(url).await,
        Commands::Snapshot { url } => snapshot(url).await,
        Commands::Extract { url, format } => extract(url, format).await,
        other => handle_complex_command(other).await,
    }
}

async fn handle_complex_command(command: Commands) -> Result<()> {
    match command {
        Commands::Crawl {
            url,
            config,
            max_depth,
            max_pages,
            format,
            output,
            respect_robots,
            requests_per_second,
            concurrency,
            same_origin,
            timeout,
            sitemap,
            #[cfg(feature = "surrealdb")]
            surrealdb,
            #[cfg(feature = "embeddings")]
            embed,
        } => {
            crawl_command(
                url,
                config,
                max_depth,
                max_pages,
                format,
                output,
                respect_robots,
                requests_per_second,
                concurrency,
                same_origin,
                timeout,
                sitemap,
                #[cfg(feature = "surrealdb")]
                surrealdb,
                #[cfg(feature = "embeddings")]
                embed,
            )
            .await
        }
        #[cfg(feature = "surrealdb")]
        Commands::Db { command } => db_command(command).await,
        #[cfg(feature = "embeddings")]
        Commands::Import { source, group } => import_command(source, group).await,
        #[cfg(feature = "embeddings")]
        Commands::Export { target, group } => export_command(target, group).await,
        _ => unreachable!("simple commands should be handled in run()"),
    }
}

async fn navigate(url: String) -> Result<()> {
    tracing::debug!(%url, "navigate command");
    let tool = BrowseTool;
    let input = BrowseInput {
        url,
        wait_until: None,
    };
    let output = Tool::execute(&tool, input).await?;
    tracing::debug!(title = ?output.title, "navigate complete");
    print_json(&output)
}

async fn snapshot(url: String) -> Result<()> {
    tracing::debug!(%url, "snapshot command");
    let tool = SnapshotTool;
    let input = SnapshotInput { url };
    let output = Tool::execute(&tool, input).await?;
    tracing::debug!(title = ?output.title, html_len = output.html.len(), "snapshot complete");
    print_json(&output)
}

async fn extract(url: String, format: String) -> Result<()> {
    tracing::debug!(%url, %format, "extract command");
    let tool = ExtractTool;
    let input = ExtractInput { url, format };
    let output = Tool::execute(&tool, input).await?;
    tracing::debug!(content_len = output.content.len(), "extract complete");
    print_json(&output)
}

fn print_json<T: Serialize>(value: &T) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    // Servo embeds SpiderMonkey, whose global mutex destructor segfaults
    // during normal process teardown.  std::process::exit runs atexit
    // handlers so we use libc::_exit which bypasses them entirely.
    std::io::stdout().flush().unwrap();
    std::io::stderr().flush().unwrap();
    unsafe { libc::_exit(0) };
}

#[allow(clippy::too_many_arguments)]
async fn crawl_command(
    url: String,
    config_path: Option<PathBuf>,
    max_depth: Option<u32>,
    max_pages: Option<usize>,
    format: Option<String>,
    output: Option<PathBuf>,
    respect_robots: Option<bool>,
    requests_per_second: Option<f64>,
    concurrency: Option<usize>,
    same_origin: Option<bool>,
    timeout: Option<u64>,
    sitemap: Option<bool>,
    #[cfg(feature = "surrealdb")] surrealdb: bool,
    #[cfg(feature = "embeddings")] embed: bool,
) -> Result<()> {
    tracing::debug!(%url, "crawl command");

    let (_config, profile) = load_profile(config_path.as_deref(), &url)?;
    let content_format = resolve_content_format(format.as_deref(), profile.format.as_deref());

    #[cfg(feature = "surrealdb")]
    let surreal_output = setup_surreal_output(surrealdb, &profile, &url).await?;

    let opts = build_crawl_options(
        url,
        max_depth,
        max_pages,
        content_format,
        respect_robots,
        requests_per_second,
        concurrency,
        same_origin,
        timeout,
        sitemap,
        output,
        profile,
        #[cfg(feature = "surrealdb")]
        surreal_output,
        #[cfg(feature = "embeddings")]
        embed,
    );

    crawl::run(opts).await?;
    // Servo embeds SpiderMonkey, whose global mutex destructor segfaults
    // during normal process teardown.  std::process::exit runs atexit
    // handlers so we use libc::_exit which bypasses them entirely.
    std::io::stdout().flush().unwrap();
    std::io::stderr().flush().unwrap();
    unsafe { libc::_exit(0) };
}

fn load_profile(
    config_path: Option<&std::path::Path>,
    url: &str,
) -> Result<(CrawlConfig, vibeeye_app::config::CrawlProfile)> {
    let config = CrawlConfig::load(config_path)?;
    let profile = config.resolve(url)?;
    Ok((config, profile))
}

fn resolve_content_format(format: Option<&str>, profile_format: Option<&str>) -> ContentFormat {
    let format_str = format.or(profile_format).unwrap_or("markdown");
    match format_str {
        "html" => ContentFormat::Html,
        "text" => ContentFormat::Text,
        _ => ContentFormat::Markdown,
    }
}

#[allow(clippy::too_many_arguments)]
fn build_crawl_options(
    url: String,
    max_depth: Option<u32>,
    max_pages: Option<usize>,
    content_format: ContentFormat,
    respect_robots: Option<bool>,
    requests_per_second: Option<f64>,
    concurrency: Option<usize>,
    same_origin: Option<bool>,
    timeout: Option<u64>,
    sitemap: Option<bool>,
    output: Option<PathBuf>,
    profile: vibeeye_app::config::CrawlProfile,
    #[cfg(feature = "surrealdb")] surreal_output: Option<vibeeye_app::db::SurrealOutput>,
    #[cfg(feature = "embeddings")] embed: bool,
) -> CrawlOptions {
    let outputs = build_outputs(
        content_format,
        output,
        &profile,
        #[cfg(feature = "surrealdb")]
        surreal_output,
        #[cfg(feature = "embeddings")]
        embed,
    );

    let effective_max_pages = max_pages.or(profile.max_pages).unwrap_or(100);
    if max_pages.is_none() && profile.max_pages.is_none() {
        eprintln!(
            "⚠️  Using default max-pages=100. Use --max-pages 0 for unlimited, or set max_pages in ~/.config/vibe-eye/crawl.toml"
        );
    }

    CrawlOptions {
        url,
        max_depth: max_depth.or(profile.max_depth).unwrap_or(2),
        max_pages: effective_max_pages,
        format: content_format,
        respect_robots: respect_robots.or(profile.respect_robots).unwrap_or(false),
        requests_per_second: requests_per_second
            .or(profile.requests_per_second)
            .unwrap_or(2.0),
        concurrency: concurrency.or(profile.concurrency).unwrap_or(4),
        same_origin: same_origin.or(profile.same_origin).unwrap_or(true),
        timeout_secs: timeout.or(profile.timeout).unwrap_or(15),
        use_sitemap: sitemap.or(profile.sitemap).unwrap_or(false),
        settle_ms: 2000,
        outputs,
    }
}

fn build_outputs(
    content_format: ContentFormat,
    output: Option<PathBuf>,
    profile: &vibeeye_app::config::CrawlProfile,
    #[cfg(feature = "surrealdb")] surreal_output: Option<vibeeye_app::db::SurrealOutput>,
    #[cfg(feature = "embeddings")] embed: bool,
) -> Vec<std::sync::Arc<dyn vibeeye_app::crawl::output::CrawlOutput>> {
    use std::sync::Arc;
    use vibeeye_app::crawl::output::{CrawlOutput, DirectoryOutput, StdoutOutput};

    let mut outputs: Vec<Arc<dyn CrawlOutput>> = Vec::new();

    #[cfg(feature = "surrealdb")]
    if let Some(mut surreal) = surreal_output {
        #[cfg(feature = "embeddings")]
        if embed {
            surreal.embed_config = profile.embeddings.clone();
        }
        outputs.push(Arc::new(surreal));
    }

    let dir = output.or_else(|| profile.output.clone().map(PathBuf::from));
    if let Some(dir) = dir {
        let ext = match content_format {
            ContentFormat::Markdown => "md",
            ContentFormat::Html => "html",
            ContentFormat::Text => "txt",
        };
        outputs.push(Arc::new(DirectoryOutput::new(dir, ext)));
    }

    if outputs.is_empty() {
        outputs.push(Arc::new(StdoutOutput));
    }

    outputs
}

#[cfg(feature = "surrealdb")]
async fn setup_surreal_output(
    surrealdb: bool,
    profile: &vibeeye_app::config::CrawlProfile,
    url: &str,
) -> Result<Option<vibeeye_app::db::SurrealOutput>> {
    if !surrealdb {
        return Ok(None);
    }
    let client = vibeeye_app::db::DbClient::connect(&db_url()).await?;
    client
        .use_ns_db(
            profile.surrealdb_ns.as_deref().unwrap_or("vibeeye"),
            profile.surrealdb_db.as_deref().unwrap_or("crawl"),
        )
        .await?;
    client.bootstrap().await?;
    Ok(Some(vibeeye_app::db::SurrealOutput::new(
        client,
        url,
        profile.group.as_deref(),
    )))
}

#[cfg(feature = "embeddings")]
async fn import_command(source: PathBuf, group: String) -> Result<()> {
    use vibeeye_app::db::DbClient;

    let client = DbClient::connect(&db_url()).await?;
    client.use_ns_db("vibeeye", "crawl").await?;

    let source = if source.extension().and_then(|e| e.to_str()) == Some("surql") {
        vibeeye_app::db::import::ImportSource::SurqlFile(&source)
    } else if source.join("manifest.json").exists() {
        vibeeye_app::db::import::ImportSource::OutputDirectory(&source)
    } else {
        vibeeye_app::db::import::ImportSource::TextDirectory(&source)
    };

    vibeeye_app::db::import::import(&client, &group, source).await?;
    println!("Imported into group: {}", group);
    Ok(())
}

#[cfg(feature = "embeddings")]
async fn export_command(target: PathBuf, group: Option<String>) -> Result<()> {
    use vibeeye_app::db::DbClient;

    let client = DbClient::connect(&db_url()).await?;
    client.use_ns_db("vibeeye", "crawl").await?;

    let mut file = std::fs::File::create(&target)?;

    if let Some(group) = group {
        vibeeye_app::db::export::export_group(&client, &group, &mut file).await?;
    } else {
        let groups = client.list_groups().await?;
        for group in groups {
            vibeeye_app::db::export::export_group(&client, &group, &mut file).await?;
        }
    }
    println!("Exported to: {}", target.display());
    Ok(())
}

#[cfg(feature = "embeddings")]
async fn load_embedding_config() -> Result<vibeeye_app::config::embeddings::EmbeddingConfig> {
    let config_path = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("vibe-eye")
        .join("crawl.toml");
    let config = if config_path.exists() {
        vibeeye_app::config::CrawlConfig::load(Some(&config_path))?
    } else {
        vibeeye_app::config::CrawlConfig::default()
    };
    let profile = config.global;
    profile.embeddings.ok_or_else(|| {
        anyhow::anyhow!(
            "no [embeddings] section found in config. Add one to ~/.config/vibe-eye/crawl.toml"
        )
    })
}

#[cfg(feature = "surrealdb")]
async fn db_command(command: DbCommands) -> Result<()> {
    use vibeeye_app::db::DbClient;

    let client = DbClient::connect(&db_url()).await?;
    client.use_ns_db("vibeeye", "crawl").await?;

    match command {
        DbCommands::List => db_list(&client).await,
        DbCommands::Status { group } => db_status(&client, &group).await,
        DbCommands::Query {
            query,
            group,
            limit,
            format,
        } => db_query(&client, query, group, limit, format).await,
        #[cfg(feature = "embeddings")]
        DbCommands::Vector {
            query,
            group,
            limit,
            format,
        } => db_vector(&client, query, group, limit, format).await,
        #[cfg(feature = "embeddings")]
        DbCommands::Hybrid {
            query,
            group,
            limit,
            bm25_limit,
            format,
        } => db_hybrid(&client, query, group, limit, bm25_limit, format).await,
        DbCommands::Reset { group } => db_reset(&client, &group).await,
        DbCommands::ResetAll => db_reset_all(&client).await,
    }
}

#[cfg(feature = "surrealdb")]
async fn db_list(client: &vibeeye_app::db::DbClient) -> Result<()> {
    let groups = client.list_groups().await?;
    println!(
        "{}",
        format_value(&serde_json::to_value(&groups)?, OutputFormat::Json)
    );
    Ok(())
}

#[cfg(feature = "surrealdb")]
async fn db_status(client: &vibeeye_app::db::DbClient, group: &str) -> Result<()> {
    let stats = client.group_stats(group).await?;
    println!(
        "{}",
        format_value(&serde_json::to_value(&stats)?, OutputFormat::Json)
    );
    Ok(())
}

#[cfg(feature = "surrealdb")]
async fn db_query(
    client: &vibeeye_app::db::DbClient,
    query: String,
    group: Option<String>,
    limit: usize,
    format: OutputFormat,
) -> Result<()> {
    let results = client.bm25_search(group.as_deref(), &query, limit).await?;
    println!("{}", format_value(&serde_json::to_value(&results)?, format));
    Ok(())
}

#[cfg(all(feature = "surrealdb", feature = "embeddings"))]
async fn db_vector(
    client: &vibeeye_app::db::DbClient,
    query: String,
    group: Option<String>,
    limit: usize,
    format: OutputFormat,
) -> Result<()> {
    let config = load_embedding_config().await?;
    let provider = vibeeye_app::embed::EmbeddingProvider::new(&config)?;
    let embedding = provider.embed_single(&query).await?;
    let results = client
        .knn_search(group.as_deref(), &embedding, limit)
        .await?;
    println!("{}", format_value(&serde_json::to_value(&results)?, format));
    Ok(())
}

#[cfg(all(feature = "surrealdb", feature = "embeddings"))]
async fn db_hybrid(
    client: &vibeeye_app::db::DbClient,
    query: String,
    group: Option<String>,
    limit: usize,
    bm25_limit: usize,
    format: OutputFormat,
) -> Result<()> {
    let config = load_embedding_config().await?;
    let provider = vibeeye_app::embed::EmbeddingProvider::new(&config)?;
    let embedding = provider.embed_single(&query).await?;
    let results = client
        .hybrid_search(group.as_deref(), &query, &embedding, bm25_limit, limit)
        .await?;
    println!("{}", format_value(&serde_json::to_value(&results)?, format));
    Ok(())
}

#[cfg(feature = "surrealdb")]
async fn db_reset(client: &vibeeye_app::db::DbClient, group: &str) -> Result<()> {
    client.reset_group(group).await?;
    println!("Reset group: {}", group);
    Ok(())
}

#[cfg(feature = "surrealdb")]
async fn db_reset_all(client: &vibeeye_app::db::DbClient) -> Result<()> {
    client.reset_all().await?;
    println!("Reset all groups");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use vibeeye_app::config::CrawlProfile;
    use vibeeye_core::ContentFormat;

    // ── build_outputs tests ─────────────────────────────────────────────

    fn call_build_outputs(
        format: ContentFormat,
        output: Option<PathBuf>,
        profile: &CrawlProfile,
    ) -> Vec<std::sync::Arc<dyn vibeeye_app::crawl::output::CrawlOutput>> {
        #[cfg(feature = "surrealdb")]
        return build_outputs(format, output, profile, None, false);
        #[cfg(not(feature = "surrealdb"))]
        return build_outputs(format, output, profile);
    }

    #[test]
    fn test_build_outputs_stdout_fallback() {
        let outputs = call_build_outputs(ContentFormat::Markdown, None, &CrawlProfile::default());
        assert_eq!(outputs.len(), 1);
    }

    #[test]
    fn test_build_outputs_directory() {
        let outputs = call_build_outputs(
            ContentFormat::Html,
            Some(PathBuf::from("/tmp/out")),
            &CrawlProfile::default(),
        );
        assert_eq!(outputs.len(), 1);
    }

    #[test]
    fn test_build_outputs_profile_dir_fallback() {
        let profile = CrawlProfile {
            output: Some("/tmp/profile_out".to_string()),
            ..Default::default()
        };
        let outputs = call_build_outputs(ContentFormat::Text, None, &profile);
        assert_eq!(outputs.len(), 1);
    }

    // ── DB command tests ────────────────────────────────────────────────

    #[cfg(feature = "surrealdb")]
    mod db_tests {
        use super::*;
        use vibeeye_app::db::DbClient;

        async fn setup_db() -> Result<DbClient> {
            let db = DbClient::connect_mem().await?;
            db.use_ns_db("vibeeye", "crawl").await?;
            db.bootstrap().await?;
            Ok(db)
        }

        #[tokio::test]
        async fn test_db_list_empty() -> Result<()> {
            let client = setup_db().await?;
            db_list(&client).await?;
            Ok(())
        }

        #[tokio::test]
        async fn test_db_status_empty() -> Result<()> {
            let client = setup_db().await?;
            db_status(&client, "test-group").await?;
            Ok(())
        }

        #[tokio::test]
        async fn test_db_reset_nonexistent() -> Result<()> {
            let client = setup_db().await?;
            db_reset(&client, "nonexistent").await?;
            Ok(())
        }

        #[tokio::test]
        async fn test_db_reset_all_empty() -> Result<()> {
            let client = setup_db().await?;
            db_reset_all(&client).await?;
            Ok(())
        }

        #[tokio::test]
        async fn test_export_command_single_group() -> Result<()> {
            let client = setup_db().await?;
            let mut buf = Vec::new();
            vibeeye_app::db::export::export_group(&client, "test", &mut buf).await?;
            let output = String::from_utf8(buf).unwrap();
            assert!(output.contains("Export for group: test"));
            Ok(())
        }
    }
}
