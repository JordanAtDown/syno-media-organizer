mod common;

use common::{create_jpeg_without_exif, create_mp4_stub};
use syno_media_organizer::config::{FolderConfig, OnConflict};
use syno_media_organizer::error::ProcessorError;
use syno_media_organizer::processor::process_file;
use tempfile::TempDir;

fn make_cfg(input: std::path::PathBuf, output: std::path::PathBuf) -> FolderConfig {
    FolderConfig {
        input,
        output,
        pattern: "{year}/{month}/{stem}{ext}".to_string(),
        recursive: false,
        photo_prefix: String::new(),
        video_prefix: String::new(),
        on_conflict: OnConflict::Rename,
        extensions: vec!["jpg".to_string(), "mp4".to_string(), "jpeg".to_string()],
        excluded_dirs: vec![],
    }
}

fn output_files(output: &TempDir) -> Vec<String> {
    walkdir::WalkDir::new(output.path())
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|e| {
            e.path()
                .strip_prefix(output.path())
                .unwrap()
                .to_str()
                .unwrap()
                .replace('\\', "/")
        })
        .collect()
}

// --- WhatsApp ---

#[test]
fn test_whatsapp_photo_no_exif_moved_to_correct_year_month() {
    let input = TempDir::new().unwrap();
    let output = TempDir::new().unwrap();
    // No EXIF — date must come from filename: 2025-06-26
    let file = create_jpeg_without_exif(input.path(), "IMG-20250626-WA0002.jpg");

    let cfg = make_cfg(input.path().to_path_buf(), output.path().to_path_buf());
    process_file(&file, &cfg, false).unwrap();

    assert!(!file.exists(), "source must be removed after move");
    let files = output_files(&output);
    assert_eq!(files.len(), 1);
    assert!(
        files[0].starts_with("2025/06/"),
        "expected 2025/06/, got {}",
        files[0]
    );
}

#[test]
fn test_whatsapp_video_no_quicktime_moved_to_correct_year_month() {
    let input = TempDir::new().unwrap();
    let output = TempDir::new().unwrap();
    // MP4 stub with no moov/mvhd — date must come from filename: 2025-04-08
    let file = create_mp4_stub(input.path(), "VID-20250408-WA0022.mp4");

    let cfg = make_cfg(input.path().to_path_buf(), output.path().to_path_buf());
    process_file(&file, &cfg, false).unwrap();

    assert!(!file.exists(), "source must be removed after move");
    let files = output_files(&output);
    assert_eq!(files.len(), 1);
    assert!(
        files[0].starts_with("2025/04/"),
        "expected 2025/04/, got {}",
        files[0]
    );
}

#[test]
fn test_whatsapp_photo_with_suffix_moved_correctly() {
    let input = TempDir::new().unwrap();
    let output = TempDir::new().unwrap();
    // Synology Drive appends " (1)" to conflict copies
    let file = create_jpeg_without_exif(input.path(), "IMG-20250626-WA0003 (1).jpg");

    let cfg = make_cfg(input.path().to_path_buf(), output.path().to_path_buf());
    process_file(&file, &cfg, false).unwrap();

    let files = output_files(&output);
    assert_eq!(files.len(), 1);
    assert!(
        files[0].starts_with("2025/06/"),
        "expected 2025/06/, got {}",
        files[0]
    );
}

// --- Android camera ---

#[test]
fn test_android_img_prefix_no_exif_moved_correctly() {
    let input = TempDir::new().unwrap();
    let output = TempDir::new().unwrap();
    // Date 2019-08-07, time 08:09:39 from filename
    let file = create_jpeg_without_exif(input.path(), "IMG_20190807_080939.jpg");

    let cfg = make_cfg(input.path().to_path_buf(), output.path().to_path_buf());
    process_file(&file, &cfg, false).unwrap();

    let files = output_files(&output);
    assert_eq!(files.len(), 1);
    assert!(
        files[0].starts_with("2019/08/"),
        "expected 2019/08/, got {}",
        files[0]
    );
}

#[test]
fn test_android_bare_yyyymmdd_no_exif_moved_correctly() {
    let input = TempDir::new().unwrap();
    let output = TempDir::new().unwrap();
    let file = create_jpeg_without_exif(input.path(), "20220630_211409_031.jpg");

    let cfg = make_cfg(input.path().to_path_buf(), output.path().to_path_buf());
    process_file(&file, &cfg, false).unwrap();

    let files = output_files(&output);
    assert_eq!(files.len(), 1);
    assert!(
        files[0].starts_with("2022/06/"),
        "expected 2022/06/, got {}",
        files[0]
    );
}

// --- Facebook ---

#[test]
fn test_facebook_timestamp_no_exif_moved_correctly() {
    let input = TempDir::new().unwrap();
    let output = TempDir::new().unwrap();
    // 1484685560948 ms → 2017-01-17 UTC
    let file = create_jpeg_without_exif(input.path(), "FB_IMG_1484685560948.jpg");

    let cfg = make_cfg(input.path().to_path_buf(), output.path().to_path_buf());
    process_file(&file, &cfg, false).unwrap();

    let files = output_files(&output);
    assert_eq!(files.len(), 1);
    assert!(
        files[0].starts_with("2017/"),
        "expected 2017/xx/, got {}",
        files[0]
    );
}

// --- No match → still skipped ---

#[test]
fn test_no_pattern_no_exif_still_returns_capture_data_not_found() {
    let input = TempDir::new().unwrap();
    let output = TempDir::new().unwrap();
    let file = create_jpeg_without_exif(input.path(), "scan_001.jpg");

    let cfg = make_cfg(input.path().to_path_buf(), output.path().to_path_buf());
    let err = process_file(&file, &cfg, false).unwrap_err();

    assert!(
        matches!(err, ProcessorError::CaptureDataNotFound),
        "expected CaptureDataNotFound, got {:?}",
        err
    );
    assert!(file.exists(), "source must not be moved when no date found");
    assert!(output_files(&output).is_empty());
}

// --- Dry-run with filename fallback ---

#[test]
fn test_dry_run_whatsapp_no_side_effects() {
    let input = TempDir::new().unwrap();
    let output = TempDir::new().unwrap();
    let file = create_jpeg_without_exif(input.path(), "IMG-20250101-WA0001.jpg");

    let cfg = make_cfg(input.path().to_path_buf(), output.path().to_path_buf());
    process_file(&file, &cfg, true).unwrap();

    assert!(file.exists(), "dry-run must not remove source");
    assert!(
        output_files(&output).is_empty(),
        "dry-run must not create files"
    );
}
