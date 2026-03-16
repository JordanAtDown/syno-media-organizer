use crate::config::Config;
use crate::error::WatcherError;
use crate::processor;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tracing::{debug, error, info};
use walkdir::WalkDir;

/// Start watching all configured folders and process incoming files.
///
/// Scans each input folder every `config.poll_interval_secs` seconds.
/// All files present in the input folder are processed on each scan; since files
/// are always moved to the output, they cannot reappear on the next cycle.
///
/// Blocks until the `shutdown` flag is set or SIGTERM / SIGINT is received.
pub fn run(config: Config, dry_run: bool) -> Result<(), WatcherError> {
    run_with_shutdown(config, dry_run, None)
}

/// Like `run`, but accepts an externally-controlled shutdown flag.
/// Useful for tests and for callers that manage their own signal handling.
pub fn run_with_shutdown(
    config: Config,
    dry_run: bool,
    external_shutdown: Option<Arc<AtomicBool>>,
) -> Result<(), WatcherError> {
    let shutdown = external_shutdown.unwrap_or_else(|| Arc::new(AtomicBool::new(false)));
    let shutdown_ctrlc = Arc::clone(&shutdown);

    let _ = ctrlc::set_handler(move || {
        info!("shutdown signal received, stopping watcher");
        shutdown_ctrlc.store(true, Ordering::SeqCst);
    });

    let interval = Duration::from_secs(config.poll_interval_secs);

    info!(
        interval_secs = config.poll_interval_secs,
        folders = config.folders.len(),
        "watcher running (polling mode)"
    );

    while !shutdown.load(Ordering::SeqCst) {
        for folder in &config.folders {
            match scan_folder(folder, dry_run) {
                Ok(0) => {}
                Ok(n) => info!(path = %folder.input.display(), files = n, "processed files"),
                Err(e) => error!(path = %folder.input.display(), error = %e, "scan error"),
            }
        }

        // Sleep in 1-second slices so we react to shutdown within ~1 s
        // without blocking for the full interval.
        // Uses thread::sleep (CLOCK_MONOTONIC via OS scheduler) — no Instant needed.
        let wake_at = SystemTime::now() + interval;
        while SystemTime::now() < wake_at {
            if shutdown.load(Ordering::SeqCst) {
                break;
            }
            std::thread::sleep(Duration::from_secs(1));
        }
    }

    info!("watcher stopped");
    Ok(())
}

/// Scan one input folder and process all files found.
///
/// Returns the number of files successfully processed.
fn scan_folder(folder: &crate::config::FolderConfig, dry_run: bool) -> Result<usize, WatcherError> {
    let max_depth = if folder.recursive { usize::MAX } else { 1 };
    let mut processed = 0;

    for entry in WalkDir::new(&folder.input)
        .max_depth(max_depth)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| {
            !e.path().components().any(|c| {
                folder
                    .excluded_dirs
                    .iter()
                    .any(|ex| ex.as_str() == c.as_os_str())
            })
        })
    {
        debug!(path = %entry.path().display(), "found new file");

        match processor::process_file(entry.path(), folder, dry_run) {
            Ok(()) => processed += 1,
            Err(crate::error::ProcessorError::ExtensionNotAllowed(ext)) => {
                debug!(path = %entry.path().display(), ext, "skipping: extension not allowed");
            }
            Err(e) => {
                error!(path = %entry.path().display(), error = %e, "failed to process file");
            }
        }
    }

    Ok(processed)
}
