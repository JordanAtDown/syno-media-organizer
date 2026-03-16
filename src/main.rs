use syno_media_organizer::{config, watcher};

use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use tracing::info;

#[derive(Parser, Debug)]
#[command(name = "syno-media-organizer")]
#[command(about = "Automatically organize photos and videos by EXIF date")]
#[command(version)]
struct Cli {
    /// Path to the configuration file
    #[arg(short, long, default_value = "/etc/syno-media-organizer/config.toml")]
    config: PathBuf,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,

    /// Dry run: show what would be done without moving files
    #[arg(long)]
    dry_run: bool,

    /// Log format: text (default) or json
    #[arg(long, default_value = "text")]
    log_format: String,
}

fn init_tracing(verbose: bool, format: &str) {
    let level = if verbose { "debug" } else { "info" };
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(level));

    if format == "json" {
        tracing_subscriber::fmt()
            .json()
            .with_env_filter(env_filter)
            .init();
    } else {
        tracing_subscriber::fmt().with_env_filter(env_filter).init();
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    init_tracing(cli.verbose, &cli.log_format);

    info!(
        version = env!("CARGO_PKG_VERSION"),
        config = %cli.config.display(),
        dry_run = cli.dry_run,
        "syno-media-organizer starting"
    );

    let cfg = config::load(&cli.config)?;

    let cache_path = cli
        .config
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."))
        .join("no_date_cache.json");

    watcher::run(cfg, cli.dry_run, cache_path)?;

    Ok(())
}
