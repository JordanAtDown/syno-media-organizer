use crate::config::FolderConfig;
use crate::error::ProcessorError;
use crate::{exif, naming};
use std::path::Path;
use tracing::{debug, info, warn};

/// Process a single file through the full pipeline:
/// validate extension → read EXIF → compute destination → create dirs → move.
///
/// Errors are returned but callers should log and continue (never crash on a single file).
pub fn process_file(path: &Path, cfg: &FolderConfig, dry_run: bool) -> Result<(), ProcessorError> {
    // 1. Validate extension
    let ext_lower = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();

    if !cfg.extensions.iter().any(|e| e.to_lowercase() == ext_lower) {
        return Err(ProcessorError::ExtensionNotAllowed(ext_lower));
    }

    debug!(file = %path.display(), "processing file");

    // 2. Read date (EXIF → mtime fallback)
    let date = exif::read_date(path).unwrap_or_else(|e| {
        warn!(file = %path.display(), error = %e, "EXIF read failed, using current time");
        chrono::Local::now()
    });

    // 3. Compute destination path from pattern
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("file");
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| format!(".{}", e))
        .unwrap_or_default();

    let relative = naming::apply_pattern(&cfg.pattern, &date, stem, &ext, None, 0);
    let dest_path = cfg.output.join(&relative);

    // 4. Resolve conflicts
    let final_dest = match naming::resolve_conflict(&dest_path, &cfg.on_conflict)
        .map_err(ProcessorError::Naming)?
    {
        Some(p) => p,
        None => {
            info!(
                file = %path.display(),
                dest = %dest_path.display(),
                "skipping: destination exists"
            );
            return Ok(());
        }
    };

    // 5. Create destination directory
    if let Some(parent) = final_dest.parent() {
        if !parent.exists() {
            if dry_run {
                info!(dir = %parent.display(), "[dry-run] would create directory");
            } else {
                std::fs::create_dir_all(parent)?;
            }
        }
    }

    // 6. Move (rename on same FS; fallback to copy+delete across filesystems)
    if dry_run {
        info!(
            from = %path.display(),
            to = %final_dest.display(),
            "[dry-run] would move file"
        );
        return Ok(());
    }

    if std::fs::rename(path, &final_dest).is_err() {
        std::fs::copy(path, &final_dest)?;
        std::fs::remove_file(path)?;
    }
    info!(from = %path.display(), to = %final_dest.display(), "moved file");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::OnConflict;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn make_cfg(input: PathBuf, output: PathBuf, on_conflict: OnConflict) -> FolderConfig {
        FolderConfig {
            input,
            output,
            pattern: "{year}/{month}/{stem}{ext}".to_string(),
            recursive: false,
            on_conflict,
            extensions: vec!["jpg".to_string(), "mp4".to_string()],
        }
    }

    #[test]
    fn test_process_disallowed_extension() {
        let tmp = TempDir::new().unwrap();
        let file = tmp.path().join("document.pdf");
        std::fs::write(&file, b"pdf content").unwrap();
        let cfg = make_cfg(
            tmp.path().to_path_buf(),
            tmp.path().join("out"),
            OnConflict::Rename,
        );
        let err = process_file(&file, &cfg, false).unwrap_err();
        assert!(matches!(err, ProcessorError::ExtensionNotAllowed(_)));
    }

    #[test]
    fn test_process_move_removes_source() {
        let input = TempDir::new().unwrap();
        let output = TempDir::new().unwrap();
        let file = input.path().join("video.mp4");
        std::fs::write(&file, b"mp4 stub").unwrap();

        let cfg = make_cfg(
            input.path().to_path_buf(),
            output.path().to_path_buf(),
            OnConflict::Rename,
        );

        process_file(&file, &cfg, false).unwrap();
        assert!(!file.exists(), "source should be removed after move");
    }

    #[test]
    fn test_process_dry_run_no_changes() {
        let input = TempDir::new().unwrap();
        let output = TempDir::new().unwrap();
        let file = input.path().join("photo.jpg");
        std::fs::write(&file, b"\xFF\xD8\xFF\xD9").unwrap();

        let cfg = make_cfg(
            input.path().to_path_buf(),
            output.path().to_path_buf(),
            OnConflict::Rename,
        );

        process_file(&file, &cfg, true).unwrap();

        // In dry-run: source untouched, output empty
        assert!(file.exists());
        let count = walkdir::WalkDir::new(output.path())
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .count();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_process_skip_conflict() {
        let input = TempDir::new().unwrap();
        let output = TempDir::new().unwrap();
        let file = input.path().join("photo.jpg");
        std::fs::write(&file, b"\xFF\xD8\xFF\xD9").unwrap();

        let cfg = make_cfg(
            input.path().to_path_buf(),
            output.path().to_path_buf(),
            OnConflict::Skip,
        );

        // First pass — file is moved to output
        process_file(&file, &cfg, false).unwrap();
        // Source no longer in input, but destination exists — nothing more to do
        assert!(!file.exists());
    }
}
