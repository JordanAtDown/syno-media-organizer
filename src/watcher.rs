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
/// Only files whose `mtime` is newer than the previous scan are processed,
/// which prevents re-processing files in `move_files = false` (copy) mode.
///
/// On the very first scan `last_scan = UNIX_EPOCH`, so all pre-existing files
/// are processed — useful for catching up after the daemon was stopped.
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
    let mut last_scan = SystemTime::UNIX_EPOCH;

    info!(
        interval_secs = config.poll_interval_secs,
        folders = config.folders.len(),
        "watcher running (polling mode)"
    );

    while !shutdown.load(Ordering::SeqCst) {
        let scan_start = SystemTime::now();

        for folder in &config.folders {
            match scan_folder(folder, last_scan, dry_run) {
                Ok(0) => {}
                Ok(n) => info!(path = %folder.input.display(), files = n, "processed files"),
                Err(e) => error!(path = %folder.input.display(), error = %e, "scan error"),
            }
        }

        last_scan = scan_start;

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

/// Scan one input folder and process every file newer than `since`.
///
/// Returns the number of files successfully processed.
fn scan_folder(
    folder: &crate::config::FolderConfig,
    since: SystemTime,
    dry_run: bool,
) -> Result<usize, WatcherError> {
    let max_depth = if folder.recursive { usize::MAX } else { 1 };
    let mut processed = 0;

    for entry in WalkDir::new(&folder.input)
        .max_depth(max_depth)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let mtime = entry
            .metadata()
            .ok()
            .and_then(|m| m.modified().ok())
            .unwrap_or(SystemTime::UNIX_EPOCH);

        if mtime <= since {
            continue;
        }

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
