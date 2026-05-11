mod cli;
mod error;
mod index;
mod search;
mod server;

use clap::Parser;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::cli::{Cli, run_cli};
use crate::index::manager::IndexManager;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "tantivy_cli=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let cli = Cli::parse();
    let manager = IndexManager::new(&cli.index_dir)?;

    // Try to load existing indexes eagerly (except for serve which does it lazily)
    if !matches!(cli.command, cli::Commands::Serve { .. }) {
        let _ = manager.load_all_indexes();
    }

    run_cli(cli, &manager).await?;
    Ok(())
}
