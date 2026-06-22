mod common;

use common::{create_jpeg_with_exif, create_jpeg_without_exif, create_mp4_stub, make_date};
use syno_media_organizer::config::{FolderConfig, OnConflict};
use syno_media_organizer::error::ProcessorError;
use syno_media_organizer::exif::read_exif_date;
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

// --- EXIF injection ---

#[test]
fn test_whatsapp_jpeg_has_exif_date_after_processing() {
    // After processing a WhatsApp JPEG, the moved file must have DateTimeOriginal in EXIF
    let input = TempDir::new().unwrap();
    let output = TempDir::new().unwrap();
    let file = create_jpeg_without_exif(input.path(), "IMG-20250626-WA0002.jpg");

    let cfg = make_cfg(input.path().to_path_buf(), output.path().to_path_buf());
    process_file(&file, &cfg, false).unwrap();

    let moved = output_files(&output);
    assert_eq!(moved.len(), 1);

    let moved_path = output.path().join(&moved[0]);
    let dt = read_exif_date(&moved_path).expect("moved file must have DateTimeOriginal in EXIF");
    assert_eq!(dt.format("%Y-%m-%d").to_string(), "2025-06-26");
}

#[test]
fn test_android_jpeg_has_exif_date_after_processing() {
    let input = TempDir::new().unwrap();
    let output = TempDir::new().unwrap();
    let file = create_jpeg_without_exif(input.path(), "IMG_20190807_080939.jpg");

    let cfg = make_cfg(input.path().to_path_buf(), output.path().to_path_buf());
    process_file(&file, &cfg, false).unwrap();

    let moved = output_files(&output);
    let moved_path = output.path().join(&moved[0]);
    let dt = read_exif_date(&moved_path).unwrap();
    assert_eq!(
        dt.format("%Y-%m-%d %H:%M:%S").to_string(),
        "2019-08-07 08:09:39"
    );
}

// --- Atomic write safety (integration) ---

#[test]
fn test_no_temp_file_left_in_input_after_successful_processing() {
    // After process_file completes, no .syno_exif_tmp_ file must remain in the input directory
    let input = TempDir::new().unwrap();
    let output = TempDir::new().unwrap();
    let file = create_jpeg_without_exif(input.path(), "IMG-20250626-WA0002.jpg");

    let cfg = make_cfg(input.path().to_path_buf(), output.path().to_path_buf());
    process_file(&file, &cfg, false).unwrap();

    let orphans: Vec<_> = std::fs::read_dir(input.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_name()
                .to_string_lossy()
                .starts_with(".syno_exif_tmp_")
        })
        .collect();
    assert!(
        orphans.is_empty(),
        "no temp file must remain in input after processing: {:?}",
        orphans
    );
}

#[test]
fn test_existing_exif_not_overwritten_by_filename_date() {
    // A JPEG that already has DateTimeOriginal must keep its metadata date,
    // even when its filename matches a WhatsApp pattern with a different date.
    // 2024-03-15 in EXIF, filename suggests 2025-06-26 → EXIF must win.
    let input = TempDir::new().unwrap();
    let output = TempDir::new().unwrap();
    let exif_date = make_date(2024, 3, 15, 10, 30, 0);
    // WhatsApp name → 2025-06-26, but EXIF date is 2024-03-15
    let file = create_jpeg_with_exif(input.path(), "IMG-20250626-WA0099.jpg", exif_date);

    let cfg = make_cfg(input.path().to_path_buf(), output.path().to_path_buf());
    process_file(&file, &cfg, false).unwrap();

    // File must land in 2024/03 (EXIF date), not 2025/06 (filename date)
    let files = output_files(&output);
    assert_eq!(files.len(), 1);
    assert!(
        files[0].starts_with("2024/03/"),
        "EXIF date must take priority over filename date — got {}",
        files[0]
    );

    // The EXIF tag in the moved file must still be the original 2024-03-15
    let moved_path = output.path().join(&files[0]);
    let dt = read_exif_date(&moved_path).unwrap();
    assert_eq!(
        dt.format("%Y-%m-%d").to_string(),
        "2024-03-15",
        "original EXIF date must not be overwritten"
    );
}

#[test]
fn test_multiple_whatsapp_photos_all_get_exif_injected() {
    // Process 3 WhatsApp photos in sequence — each must have DateTimeOriginal in the output
    let input = TempDir::new().unwrap();
    let output = TempDir::new().unwrap();
    let cfg = make_cfg(input.path().to_path_buf(), output.path().to_path_buf());

    let cases = [
        ("IMG-20250101-WA0001.jpg", "2025-01-01"),
        ("IMG-20240615-WA0002.jpg", "2024-06-15"),
        ("IMG-20231225-WA0003.jpg", "2023-12-25"),
    ];

    for (name, _) in &cases {
        let file = create_jpeg_without_exif(input.path(), name);
        process_file(&file, &cfg, false).unwrap();
    }

    let mut files = output_files(&output);
    files.sort();
    assert_eq!(files.len(), 3, "all 3 files must be moved");

    // Verify each moved file has a readable DateTimeOriginal
    for rel in &files {
        let moved_path = output.path().join(rel);
        assert!(
            read_exif_date(&moved_path).is_ok(),
            "{} must have DateTimeOriginal after processing",
            rel
        );
    }

    // Verify correct year routing for each
    let years: std::collections::HashSet<_> = files.iter().map(|f| &f[..4]).collect();
    assert!(years.contains("2025"), "2025 folder expected");
    assert!(years.contains("2024"), "2024 folder expected");
    assert!(years.contains("2023"), "2023 folder expected");
}
