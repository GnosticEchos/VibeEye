use anyhow::Result;
use clap::Parser;

mod cli;
mod commands;
mod format;
mod help_tree;

use cli::Cli;

fn main() -> Result<()> {
    #[cfg(feature = "embeddings")]
    let _ = dotenvy::dotenv();

    let args: Vec<String> = std::env::args().collect();
    let invocation =
        help_tree::parse_help_tree_invocation(&args[1..]).map_err(|e| anyhow::anyhow!(e))?;
    if let Some(mut invocation) = invocation {
        let config = vibeeye_app::config::CrawlConfig::load(None).ok();
        let help_tree = config
            .as_ref()
            .and_then(|c| c.cli.as_ref())
            .or_else(|| config.as_ref().and_then(|c| c.global.cli.as_ref()))
            .and_then(|cli| cli.help_tree.clone());
        let theme = help_tree
            .as_ref()
            .map(|ht| {
                help_tree::theme::theme_from_config(
                    Some(ht),
                    &help_tree::theme::HelpTreeTheme::default(),
                )
            })
            .unwrap_or_default();
        invocation.opts.theme = theme;
        help_tree::run_for_path::<Cli>(invocation.opts, &invocation.path)
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;
        return Ok(());
    }

    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async_main())
}

async fn async_main() -> Result<()> {
    let cli = Cli::parse();

    if cli.verbose {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .init();
    }

    if let Some(command) = cli.command {
        commands::run(command).await?;
    } else {
        println!("VibeEye - Headless browser for agentic content extraction");
        println!("Run with --help for usage information");
    }

    Ok(())
}
