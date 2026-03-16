use crate::config::Config;
use crate::error::{ProcessorError, WatcherError};
use crate::no_date_cache::NoDateCache;
use crate::processor;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::{debug, error, info, warn};
use walkdir::WalkDir;

/// Start watching all configured folders and process incoming files.
///
/// Scans each input folder every `config.poll_interval_secs` seconds.
/// All files present in the input folder are processed on each scan; since files
/// are always moved to the output, they cannot reappear on the next cycle.
///
/// `cache_path` is the path to the persistent no-date cache JSON file (typically
/// next to `config.toml`). Files without capture date metadata are recorded there
/// and silently skipped on subsequent scans unless their mtime changes or the TTL expires.
///
/// Blocks until the `shutdown` flag is set or SIGTERM / SIGINT is received.
pub fn run(config: Config, dry_run: bool, cache_path: PathBuf) -> Result<(), WatcherError> {
    run_with_shutdown(config, dry_run, None, cache_path)
}

/// Like `run`, but accepts an externally-controlled shutdown flag.
/// Useful for tests and for callers that manage their own signal handling.
pub fn run_with_shutdown(
    config: Config,
    dry_run: bool,
    external_shutdown: Option<Arc<AtomicBool>>,
    cache_path: PathBuf,
) -> Result<(), WatcherError> {
    let shutdown = external_shutdown.unwrap_or_else(|| Arc::new(AtomicBool::new(false)));
    let shutdown_ctrlc = Arc::clone(&shutdown);

    let _ = ctrlc::set_handler(move || {
        info!("shutdown signal received, stopping watcher");
        shutdown_ctrlc.store(true, Ordering::SeqCst);
    });

    let interval = Duration::from_secs(config.poll_interval_secs);

    let mut cache = NoDateCache::load(
        cache_path,
        config.no_date_cache_enabled,
        config.no_date_cache_ttl_days,
    );

    info!(
        interval_secs = config.poll_interval_secs,
        folders = config.folders.len(),
        "watcher running (polling mode)"
    );

    while !shutdown.load(Ordering::SeqCst) {
        for folder in &config.folders {
            match scan_folder(folder, dry_run, &mut cache) {
                Ok(0) => {}
                Ok(n) => info!(path = %folder.input.display(), files = n, "processed files"),
                Err(e) => error!(path = %folder.input.display(), error = %e, "scan error"),
            }
        }

        cache.save_if_dirty();

        // Sleep in 1-second slices so we react to shutdown within ~1 s
        // without blocking for the full interval.
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
fn scan_folder(
    folder: &crate::config::FolderConfig,
    dry_run: bool,
    cache: &mut NoDateCache,
) -> Result<usize, WatcherError> {
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
        let path = entry.path().to_path_buf();
        let mtime_secs = entry
            .metadata()
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);

        if cache.should_skip(&path, mtime_secs) {
            debug!(path = %path.display(), "skipping: no capture date (cached)");
            continue;
        }

        debug!(path = %path.display(), "found new file");

        match processor::process_file(&path, folder, dry_run) {
            Ok(()) => {
                cache.remove(&path);
                processed += 1;
            }
            Err(ProcessorError::CaptureDataNotFound) => {
                warn!(path = %path.display(), "skipping: capture date not found");
                cache.insert(path, mtime_secs);
            }
            Err(ProcessorError::ExtensionNotAllowed(ext)) => {
                debug!(path = %path.display(), ext, "skipping: extension not allowed");
            }
            Err(e) => {
                error!(path = %path.display(), error = %e, "failed to process file");
            }
        }
    }

    Ok(processed)
}
