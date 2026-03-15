mod common;

use common::{create_jpeg_with_exif, create_jpeg_without_exif, create_mp4_stub, make_date};
use rstest::rstest;
use syno_media_organizer::config::{FolderConfig, OnConflict};
use syno_media_organizer::processor::process_file;
use tempfile::TempDir;

fn make_cfg(
    input: std::path::PathBuf,
    output: std::path::PathBuf,
    on_conflict: OnConflict,
    extensions: Vec<String>,
) -> FolderConfig {
    FolderConfig {
        input,
        output,
        pattern: "{year}/{month}/{year}-{month}-{day}_{stem}{ext}".to_string(),
        recursive: false,
        photo_prefix: String::new(),
        video_prefix: String::new(),
        on_conflict,
        extensions,
    }
}

#[rstest]
#[case("with_exif.jpg", true)]
#[case("no_exif.jpg", false)]
fn test_pipeline_jpeg(#[case] filename: &str, #[case] with_exif: bool) {
    let input = TempDir::new().unwrap();
    let output = TempDir::new().unwrap();
    let date = make_date(2024, 3, 15, 10, 30, 0);

    let file = if with_exif {
        create_jpeg_with_exif(input.path(), filename, date)
    } else {
        create_jpeg_without_exif(input.path(), filename)
    };

    let cfg = make_cfg(
        input.path().to_path_buf(),
        output.path().to_path_buf(),
        OnConflict::Rename,
        vec!["jpg".to_string()],
    );

    process_file(&file, &cfg, false).unwrap();

    // Source should be removed (move mode)
    assert!(!file.exists(), "source should be removed after move");

    // Exactly one file in output
    let files: Vec<_> = walkdir::WalkDir::new(output.path())
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .collect();
    assert_eq!(files.len(), 1);

    // For EXIF case, verify year/month in path
    if with_exif {
        let rel = files[0]
            .path()
            .strip_prefix(output.path())
            .unwrap()
            .to_str()
            .unwrap()
            .replace('\\', "/");
        assert!(
            rel.starts_with("2024/03/"),
            "expected 2024/03/ prefix, got {}",
            rel
        );
    }
}

#[rstest]
#[case(OnConflict::Rename, 2)]
#[case(OnConflict::Skip, 1)]
#[case(OnConflict::Overwrite, 1)]
fn test_conflict_strategies(#[case] strategy: OnConflict, #[case] expected_count: usize) {
    let input = TempDir::new().unwrap();
    let output = TempDir::new().unwrap();
    let date = make_date(2024, 6, 1, 12, 0, 0);

    let f1 = create_jpeg_with_exif(input.path(), "photo1.jpg", date);
    let f2 = create_jpeg_with_exif(input.path(), "photo2.jpg", date);

    // Pattern that ignores {stem} forces a name collision between f1 and f2
    let mut cfg = make_cfg(
        input.path().to_path_buf(),
        output.path().to_path_buf(),
        strategy,
        vec!["jpg".to_string()],
    );
    cfg.pattern = "{year}/{month}/fixed{ext}".to_string();

    process_file(&f1, &cfg, false).unwrap();
    process_file(&f2, &cfg, false).unwrap();

    let count = walkdir::WalkDir::new(output.path())
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .count();

    assert_eq!(count, expected_count);
}

#[test]
fn test_pipeline_mp4_stub() {
    let input = TempDir::new().unwrap();
    let output = TempDir::new().unwrap();
    let file = create_mp4_stub(input.path(), "clip.mp4");

    let cfg = make_cfg(
        input.path().to_path_buf(),
        output.path().to_path_buf(),
        OnConflict::Rename,
        vec!["mp4".to_string()],
    );

    process_file(&file, &cfg, false).unwrap();

    assert!(!file.exists(), "source should be removed after move");
    let count = walkdir::WalkDir::new(output.path())
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .count();
    assert_eq!(count, 1);
}

#[test]
fn test_dry_run_no_side_effects() {
    let input = TempDir::new().unwrap();
    let output = TempDir::new().unwrap();
    let file = create_jpeg_without_exif(input.path(), "photo.jpg");

    let cfg = make_cfg(
        input.path().to_path_buf(),
        output.path().to_path_buf(),
        OnConflict::Rename,
        vec!["jpg".to_string()],
    );

    process_file(&file, &cfg, true).unwrap();

    assert!(file.exists(), "dry-run must not remove source");
    let count = walkdir::WalkDir::new(output.path())
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .count();
    assert_eq!(count, 0, "dry-run must not create files");
}
