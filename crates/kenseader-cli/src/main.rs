use std::sync::Arc;

use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use kenseader_core::{AppConfig, storage::Database};

mod commands;

#[derive(Parser)]
#[command(name = "kenseader")]
#[command(author, version, about = "A high-performance terminal RSS reader")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Subscribe to an RSS feed (shorthand for `subscribe`)
    #[arg(short = 's', long = "subscribe")]
    subscribe_url: Option<String>,

    /// Name for the subscription (used with -s)
    #[arg(short = 'n', long = "name")]
    subscribe_name: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the TUI
    Run,
    /// Subscribe to an RSS feed
    Subscribe {
        /// RSS feed URL (supports rsshub:// protocol)
        #[arg(short = 's', long)]
        url: String,
        /// Local name for the subscription
        #[arg(short = 'n', long)]
        name: String,
    },
    /// Unsubscribe from a feed
    Unsubscribe {
        /// Name of the subscription to remove
        name: String,
    },
    /// List all subscriptions
    List,
    /// Refresh all feeds
    Refresh,
    /// Clean up old articles
    Cleanup,
    /// Background daemon for automatic feed refresh and summarization
    Daemon {
        #[command(subcommand)]
        action: DaemonAction,
    },
}

#[derive(Subcommand)]
enum DaemonAction {
    /// Start the background daemon
    Start,
    /// Stop the background daemon
    Stop,
    /// Check daemon status
    Status,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer().with_target(false))
        .init();

    let cli = Cli::parse();

    // Load configuration
    let config = Arc::new(AppConfig::load()?);

    // Initialize database
    let db = Arc::new(Database::new(&config).await?);

    // Handle shorthand subscription (-s -n flags)
    if let (Some(url), Some(name)) = (cli.subscribe_url, cli.subscribe_name) {
        return commands::subscribe::run(&db, &config, &url, &name).await;
    }

    // Handle commands
    match cli.command {
        Some(Commands::Run) | None => {
            // TUI uses daemon client, no direct database access
            commands::run::run(config).await
        }
        Some(Commands::Subscribe { url, name }) => {
            commands::subscribe::run(&db, &config, &url, &name).await
        }
        Some(Commands::Unsubscribe { name }) => {
            commands::unsubscribe::run(&db, &name).await
        }
        Some(Commands::List) => {
            commands::list::run(&db).await
        }
        Some(Commands::Refresh) => {
            commands::refresh::run(&db, &config).await
        }
        Some(Commands::Cleanup) => {
            commands::cleanup::run(&db, &config).await
        }
        Some(Commands::Daemon { action }) => {
            match action {
                DaemonAction::Start => commands::daemon::start(db, config).await,
                DaemonAction::Stop => commands::daemon::stop().await,
                DaemonAction::Status => commands::daemon::status().await,
            }
        }
    }
}
