//! total-recall - Claude Sessions TUI
//!
//! "Get your ass to Claude." - Quaid, probably
//!
//! A terminal-based application for browsing and resuming Claude Code
//! conversations across all your projects.

mod app;
mod config;
mod models;
mod screens;
mod services;
mod utils;

use anyhow::Result;
use clap::Parser;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// total-recall - Claude Sessions Browser
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Enable debug logging
    #[arg(short, long)]
    debug: bool,

    /// Config file path (default: ~/.config/total-recall/config.toml)
    #[arg(short, long)]
    config: Option<String>,

    /// Claude directory path (default: ~/.claude)
    #[arg(long)]
    claude_dir: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Set up logging
    let filter = if args.debug {
        "total_recall=debug,info"
    } else {
        "total_recall=info,warn"
    };

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| filter.into()))
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr))
        .init();

    // Load configuration
    let mut config = if let Some(path) = args.config {
        config::Config::from_file(&path)?
    } else {
        config::Config::load()?
    };

    // Override claude_dir if specified
    if let Some(claude_dir) = args.claude_dir {
        config.claude.claude_dir = claude_dir;
    }

    // Run the TUI application
    let mut app = app::App::new(config).await?;
    app.run().await?;

    Ok(())
}
