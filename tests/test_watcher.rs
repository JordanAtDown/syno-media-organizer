mod common;

use common::{create_jpeg_with_exif, create_mp4_with_quicktime_date, make_date};
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
            recursive: true,
            photo_prefix: String::new(),
            video_prefix: String::new(),
            on_conflict: OnConflict::Rename,
            extensions: vec!["jpg".to_string(), "mp4".to_string()],
            excluded_dirs: vec!["@eaDir".to_string()],
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
    create_jpeg_with_exif(
        &input_path,
        "watch_test.jpg",
        make_date(2024, 6, 1, 12, 0, 0),
    );

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

/// Files inside `@eaDir` (Synology DSM thumbnails/metadata) must never be processed.
#[test]
fn test_watcher_skips_eadir_contents() {
    let input = TempDir::new().unwrap();
    let output = TempDir::new().unwrap();
    let input_path = input.path().to_path_buf();
    let output_path = output.path().to_path_buf();

    // Create the Synology-style hidden folder: @eaDir/video.mov/SYNOPHOTO_FILM_M.mp4
    let ea_dir = input_path.join("@eaDir").join("video.mov");
    std::fs::create_dir_all(&ea_dir).unwrap();
    create_mp4_with_quicktime_date(
        &ea_dir,
        "SYNOPHOTO_FILM_M.mp4",
        make_date(2026, 2, 17, 13, 38, 8),
    );

    let cfg = single_folder_config(input_path.clone(), output_path.clone());

    let shutdown = Arc::new(AtomicBool::new(false));
    let shutdown_watcher = Arc::clone(&shutdown);
    let handle = std::thread::spawn(move || run_with_shutdown(cfg, false, Some(shutdown_watcher)));

    std::thread::sleep(Duration::from_millis(2500));
    shutdown.store(true, Ordering::SeqCst);
    let _ = handle.join();

    let count = walkdir::WalkDir::new(&output_path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .count();

    assert_eq!(
        count, 0,
        "@eaDir contents must be ignored — output must be empty"
    );
}
