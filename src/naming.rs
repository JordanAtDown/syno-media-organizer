use crate::config::OnConflict;
use crate::error::NamingError;
use chrono::{DateTime, Local};
use std::path::{Path, PathBuf};

/// Apply a naming pattern to produce a relative destination path.
///
/// Supported tokens:
/// - `{year}`    — 4-digit year
/// - `{month}`   — 2-digit month (01–12)
/// - `{day}`     — 2-digit day (01–31)
/// - `{hour}`    — 2-digit hour (00–23)
/// - `{min}`     — 2-digit minute
/// - `{sec}`     — 2-digit second
/// - `{stem}`    — original filename without extension
/// - `{ext}`     — extension with leading dot (e.g. `.jpg`)
/// - `{camera}`  — camera model from EXIF, or "unknown"
/// - `{counter}` — zero-padded 4-digit counter for disambiguation
pub fn apply_pattern(
    pattern: &str,
    date: &DateTime<Local>,
    stem: &str,
    ext: &str,
    camera: Option<&str>,
    counter: u32,
) -> String {
    pattern
        .replace("{year}", &date.format("%Y").to_string())
        .replace("{month}", &date.format("%m").to_string())
        .replace("{day}", &date.format("%d").to_string())
        .replace("{hour}", &date.format("%H").to_string())
        .replace("{min}", &date.format("%M").to_string())
        .replace("{sec}", &date.format("%S").to_string())
        .replace("{stem}", stem)
        .replace("{ext}", ext)
        .replace("{camera}", camera.unwrap_or("unknown"))
        .replace("{counter}", &format!("{:04}", counter))
}

/// Resolve a destination path according to conflict strategy.
///
/// - `Overwrite` → return path as-is (caller will overwrite)
/// - `Skip`      → return `None` if path already exists
/// - `Rename`    → append `_1`, `_2`, … until a free path is found
pub fn resolve_conflict(
    dest: &Path,
    strategy: &OnConflict,
) -> Result<Option<PathBuf>, NamingError> {
    if !dest.exists() {
        return Ok(Some(dest.to_path_buf()));
    }

    match strategy {
        OnConflict::Overwrite => Ok(Some(dest.to_path_buf())),
        OnConflict::Skip => Ok(None),
        OnConflict::Rename => {
            let stem = dest.file_stem().and_then(|s| s.to_str()).unwrap_or("file");
            let ext = dest
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| format!(".{}", e))
                .unwrap_or_default();
            let parent = dest.parent().unwrap_or(Path::new("."));

            for i in 1u32..=9999 {
                let candidate = parent.join(format!("{}_{}{}", stem, i, ext));
                if !candidate.exists() {
                    return Ok(Some(candidate));
                }
            }

            Err(NamingError::ConflictUnresolvable(format!(
                "Cannot find free name for {}",
                dest.display()
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use rstest::rstest;

    fn date(year: i32, month: u32, day: u32, h: u32, m: u32, s: u32) -> DateTime<Local> {
        Local
            .with_ymd_and_hms(year, month, day, h, m, s)
            .single()
            .unwrap()
    }

    #[rstest]
    #[case(
        "{year}/{month}/{year}-{month}-{day}_{hour}{min}{sec}_{stem}{ext}",
        date(2024, 3, 15, 10, 30, 45),
        "IMG_001",
        ".jpg",
        None,
        0,
        "2024/03/2024-03-15_103045_IMG_001.jpg"
    )]
    #[case(
        "{year}/{month}/{stem}{ext}",
        date(2022, 12, 1, 0, 0, 0),
        "photo",
        ".png",
        None,
        0,
        "2022/12/photo.png"
    )]
    #[case(
        "{camera}/{year}/{stem}{ext}",
        date(2023, 6, 20, 8, 0, 0),
        "DSC1234",
        ".jpg",
        Some("Canon EOS"),
        0,
        "Canon EOS/2023/DSC1234.jpg"
    )]
    #[case(
        "{year}/{stem}_{counter}{ext}",
        date(2024, 1, 1, 12, 0, 0),
        "file",
        ".mp4",
        None,
        7,
        "2024/file_0007.mp4"
    )]
    #[case(
        "{year}/{month}/{stem}{ext}",
        date(2024, 3, 15, 10, 30, 45),
        "photo",
        ".jpg",
        None,
        0,
        "2024/03/photo.jpg"
    )]
    fn test_apply_pattern(
        #[case] pattern: &str,
        #[case] date: DateTime<Local>,
        #[case] stem: &str,
        #[case] ext: &str,
        #[case] camera: Option<&str>,
        #[case] counter: u32,
        #[case] expected: &str,
    ) {
        assert_eq!(
            apply_pattern(pattern, &date, stem, ext, camera, counter),
            expected
        );
    }

    #[test]
    fn test_apply_pattern_no_camera_token_uses_unknown() {
        let d = date(2024, 1, 1, 0, 0, 0);
        let result = apply_pattern("{camera}", &d, "f", ".jpg", None, 0);
        assert_eq!(result, "unknown");
    }

    #[test]
    fn test_resolve_conflict_no_existing_file() {
        let tmp = tempfile::TempDir::new().unwrap();
        let dest = tmp.path().join("photo.jpg");
        let result = resolve_conflict(&dest, &OnConflict::Rename).unwrap();
        assert_eq!(result, Some(dest));
    }

    #[test]
    fn test_resolve_conflict_skip() {
        let tmp = tempfile::TempDir::new().unwrap();
        let dest = tmp.path().join("photo.jpg");
        std::fs::write(&dest, b"existing").unwrap();
        let result = resolve_conflict(&dest, &OnConflict::Skip).unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_resolve_conflict_overwrite() {
        let tmp = tempfile::TempDir::new().unwrap();
        let dest = tmp.path().join("photo.jpg");
        std::fs::write(&dest, b"existing").unwrap();
        let result = resolve_conflict(&dest, &OnConflict::Overwrite).unwrap();
        assert_eq!(result, Some(dest));
    }

    #[test]
    fn test_resolve_conflict_rename() {
        let tmp = tempfile::TempDir::new().unwrap();
        let dest = tmp.path().join("photo.jpg");
        std::fs::write(&dest, b"existing").unwrap();
        let result = resolve_conflict(&dest, &OnConflict::Rename)
            .unwrap()
            .unwrap();
        assert_eq!(result.file_name().unwrap().to_str().unwrap(), "photo_1.jpg");
    }

    #[test]
    fn test_resolve_conflict_rename_multiple() {
        let tmp = tempfile::TempDir::new().unwrap();
        let dest = tmp.path().join("photo.jpg");
        std::fs::write(&dest, b"existing").unwrap();
        std::fs::write(tmp.path().join("photo_1.jpg"), b"existing").unwrap();
        let result = resolve_conflict(&dest, &OnConflict::Rename)
            .unwrap()
            .unwrap();
        assert_eq!(result.file_name().unwrap().to_str().unwrap(), "photo_2.jpg");
    }
}
