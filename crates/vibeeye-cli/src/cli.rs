//! CLI argument parsing for vibe-eye

use clap::{Parser, Subcommand};

/// VibeEye - Headless browser for agentic content extraction
#[derive(Parser, Debug)]
#[command(name = "vibe-eye")]
#[command(about = "VibeEye - Headless browser for agentic content extraction")]
#[command(version)]
pub struct Cli {
    #[command(flatten)]
    pub help_tree: crate::help_tree::HelpTreeArgs,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Navigate to a URL
    Navigate {
        /// URL to navigate to
        url: String,
    },

    /// Capture a page snapshot (URL, title, body, HTML)
    Snapshot {
        /// URL to capture
        url: String,
    },

    /// Extract page content as Markdown, HTML, or text
    Extract {
        /// URL to extract content from
        url: String,

        /// Output format: markdown, html, or text
        #[arg(short, long, default_value = "markdown")]
        format: String,
    },
}
