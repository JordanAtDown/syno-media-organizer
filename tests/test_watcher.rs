mod common;

use common::create_jpeg_without_exif;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use syno_media_organizer::config::{Config, FolderConfig, OnConflict};
use syno_media_organizer::watcher::run_with_shutdown;
use tempfile::TempDir;

fn single_folder_config(input: std::path::PathBuf, output: std::path::PathBuf) -> Config {
    Config {
        folders: vec![FolderConfig {
            input,
            output,
            pattern: "{year}/{month}/{stem}{ext}".to_string(),
            recursive: false,
            move_files: true,
            on_conflict: OnConflict::Rename,
            extensions: vec!["jpg".to_string()],
        }],
        poll_interval_secs: 1,
    }
}

/// Smoke test: watcher detects a file dropped into the input folder, processes it,
/// then shuts down gracefully via the external shutdown flag.
#[test]
fn test_watcher_detects_and_processes_file() {
    let input = TempDir::new().unwrap();
    let output = TempDir::new().unwrap();
    let input_path = input.path().to_path_buf();
    let output_path = output.path().to_path_buf();

    let cfg = single_folder_config(input_path.clone(), output_path.clone());

    let shutdown = Arc::new(AtomicBool::new(false));
    let shutdown_watcher = Arc::clone(&shutdown);

    let handle = std::thread::spawn(move || run_with_shutdown(cfg, false, Some(shutdown_watcher)));

    // Allow watcher to complete its first scan before we drop the file.
    // The first scan runs immediately with last_scan = UNIX_EPOCH, so any
    // file dropped before it completes would be caught on that first pass.
    // We wait just enough for the scan to finish and last_scan to be updated.
    std::thread::sleep(Duration::from_millis(200));

    // Drop a file into the watched folder
    create_jpeg_without_exif(&input_path, "watch_test.jpg");

    // Wait for one full poll cycle (1 s) plus processing slack
    std::thread::sleep(Duration::from_millis(2500));

    // Signal shutdown and wait
    shutdown.store(true, Ordering::SeqCst);
    let _ = handle.join();

    let count = walkdir::WalkDir::new(&output_path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .count();

    assert_eq!(count, 1, "watcher should have moved the file to output");
    assert!(
        !input_path.join("watch_test.jpg").exists(),
        "source should have been moved"
    );
}
