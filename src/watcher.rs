use crate::config::Config;
use crate::error::WatcherError;
use crate::processor;
use notify::RecursiveMode;
use notify::Watcher;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::time::Duration;
use tracing::{debug, error, info, warn};

/// Start watching all configured folders and process incoming files.
///
/// Blocks until the `shutdown` flag is set or SIGTERM / SIGINT is received.
/// Passing `None` for `shutdown` installs a `ctrlc` handler automatically.
/// Sets up a debounce of 500 ms to avoid duplicate events on large file writes.
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

    // Only install ctrlc handler when not driven by an external flag
    let _ = ctrlc::set_handler(move || {
        info!("shutdown signal received, stopping watcher");
        shutdown_ctrlc.store(true, Ordering::SeqCst);
    });

    let (tx, rx) = mpsc::channel::<Result<notify::Event, notify::Error>>();

    let mut watcher = notify::recommended_watcher(move |res| {
        let _ = tx.send(res);
    })
    .map_err(|e| WatcherError::Init(e.to_string()))?;

    // Register each input folder
    for folder in &config.folders {
        let mode = if folder.recursive {
            RecursiveMode::Recursive
        } else {
            RecursiveMode::NonRecursive
        };

        info!(path = %folder.input.display(), recursive = folder.recursive, "watching folder");
        watcher
            .watch(&folder.input, mode)
            .map_err(|e| WatcherError::Watch(e.to_string()))?;
    }

    info!("watcher running, waiting for files...");

    // Debounce: collect events for 500ms before processing
    let debounce = Duration::from_millis(500);

    while !shutdown.load(Ordering::SeqCst) {
        // Collect events with a short timeout so we can check shutdown flag
        let mut pending: Vec<PathBuf> = Vec::new();

        match rx.recv_timeout(Duration::from_millis(200)) {
            Ok(Ok(event)) => {
                for path in event.paths {
                    if path.is_file() {
                        pending.push(path);
                    }
                }
                // Drain remaining events within the debounce window
                let deadline = std::time::Instant::now() + debounce;
                while std::time::Instant::now() < deadline {
                    match rx.recv_timeout(Duration::from_millis(50)) {
                        Ok(Ok(ev)) => {
                            for p in ev.paths {
                                if p.is_file() {
                                    pending.push(p);
                                }
                            }
                        }
                        Ok(Err(e)) => warn!(error = %e, "watcher event error"),
                        Err(_) => break,
                    }
                }
            }
            Ok(Err(e)) => {
                warn!(error = %e, "watcher error");
                continue;
            }
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                error!("watcher channel disconnected");
                break;
            }
        }

        // Deduplicate paths
        pending.sort();
        pending.dedup();

        for path in pending {
            // Find the matching folder config
            let Some(folder_cfg) = config.folders.iter().find(|f| path.starts_with(&f.input))
            else {
                debug!(path = %path.display(), "no matching folder config");
                continue;
            };

            // Filter by event type — only process Create/Modify events on files
            match processor::process_file(&path, folder_cfg, dry_run) {
                Ok(()) => {}
                Err(crate::error::ProcessorError::ExtensionNotAllowed(ext)) => {
                    debug!(path = %path.display(), ext, "skipping: extension not allowed");
                }
                Err(e) => {
                    error!(path = %path.display(), error = %e, "failed to process file");
                }
            }
        }
    }

    info!("watcher stopped");
    Ok(())
}
