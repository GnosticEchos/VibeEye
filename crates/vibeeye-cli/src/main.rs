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
    if let Some(invocation) = invocation {
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
