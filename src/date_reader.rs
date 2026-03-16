use crate::error::ExifError;
use chrono::{DateTime, Local};
use std::path::Path;

/// Abstraction over metadata-date extraction strategies.
///
/// Implement this trait to support a new container format without touching
/// the processing pipeline. `processor.rs` depends only on this trait.
pub trait DateReader: Send + Sync {
    fn read_date(&self, path: &Path) -> Result<DateTime<Local>, ExifError>;
}

/// Reads EXIF `DateTimeOriginal` (tag 0x9003) — for photos (JPEG, HEIC, PNG, TIFF…).
pub struct ExifDateReader;

impl DateReader for ExifDateReader {
    fn read_date(&self, path: &Path) -> Result<DateTime<Local>, ExifError> {
        crate::exif::read_exif_date(path)
    }
}

/// Reads QuickTime `mvhd` `creation_time` (UTC, Mac epoch 1904-01-01) — for MP4, MOV…
pub struct QuickTimeDateReader;

impl DateReader for QuickTimeDateReader {
    fn read_date(&self, path: &Path) -> Result<DateTime<Local>, ExifError> {
        crate::exif::read_quicktime_date(path)
    }
}

/// Factory: returns the appropriate [`DateReader`] for a given lowercase file extension.
pub fn for_extension(ext_lower: &str) -> Box<dyn DateReader> {
    if crate::naming::is_video(ext_lower) {
        Box::new(QuickTimeDateReader)
    } else {
        Box::new(ExifDateReader)
    }
}
