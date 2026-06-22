use crate::error::ExifError;
use chrono::{DateTime, Local, TimeZone};
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

/// Wraps a primary reader and falls back to [`parse_date_from_filename`] when metadata is absent.
struct FallbackDateReader {
    primary: Box<dyn DateReader>,
}

impl DateReader for FallbackDateReader {
    fn read_date(&self, path: &Path) -> Result<DateTime<Local>, ExifError> {
        self.primary
            .read_date(path)
            .or_else(|_| parse_date_from_filename(path).ok_or(ExifError::NoDateTimeOriginal))
    }
}

/// Factory: returns a [`DateReader`] for a given lowercase file extension.
///
/// Always wraps the metadata reader in a [`FallbackDateReader`] so filename-based
/// date extraction is attempted automatically when no metadata is found.
pub fn for_extension(ext_lower: &str) -> Box<dyn DateReader> {
    let primary: Box<dyn DateReader> = if crate::naming::is_video(ext_lower) {
        Box::new(QuickTimeDateReader)
    } else {
        Box::new(ExifDateReader)
    };
    Box::new(FallbackDateReader { primary })
}

// ---------------------------------------------------------------------------
// Filename-based date extraction
// ---------------------------------------------------------------------------

/// Attempt to extract a capture date from the file's name when no metadata is available.
///
/// Recognized patterns (tried in order):
///   1. WhatsApp — `IMG-YYYYMMDD-WAxxxx`, `VID-YYYYMMDD-WAxxxx`, `AUD-YYYYMMDD-WAxxxx`
///   2. Android camera — `IMG_YYYYMMDD_HHMMSS[…]` or bare `YYYYMMDD_HHMMSS[…]`
///   3. Facebook — `FB_IMG_<unix_milliseconds>[…]`
///
/// Returns `None` when no pattern matches or when the extracted components are not a valid date.
pub fn parse_date_from_filename(path: &Path) -> Option<DateTime<Local>> {
    let stem = path.file_stem()?.to_str()?;
    parse_whatsapp_date(stem)
        .or_else(|| parse_android_date(stem))
        .or_else(|| parse_facebook_date(stem))
}

/// WhatsApp: `IMG-YYYYMMDD-WAxxxx`, `VID-YYYYMMDD-WAxxxx`, `AUD-YYYYMMDD-WAxxxx`.
/// Time is set to noon local because only the date is encoded in the filename.
fn parse_whatsapp_date(stem: &str) -> Option<DateTime<Local>> {
    let after = stem
        .strip_prefix("IMG-")
        .or_else(|| stem.strip_prefix("VID-"))
        .or_else(|| stem.strip_prefix("AUD-"))?;
    // Expect at least "YYYYMMDD-WA" (11 chars)
    if after.len() < 11 {
        return None;
    }
    let date_str = &after[..8];
    if !after[8..].starts_with("-WA") {
        return None;
    }
    parse_ymd_at_noon(date_str)
}

/// Android camera: `IMG_YYYYMMDD_HHMMSS[…]` or bare `YYYYMMDD_HHMMSS[…]`.
fn parse_android_date(stem: &str) -> Option<DateTime<Local>> {
    let s = stem.strip_prefix("IMG_").unwrap_or(stem);
    // Need at least YYYYMMDD_HHMMSS (15 chars)
    if s.len() < 15 {
        return None;
    }
    let date_part = &s[..8];
    if s.as_bytes()[8] != b'_' {
        return None;
    }
    let time_part = &s[9..15];
    if !date_part.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    if !time_part.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    parse_ymd_hms(date_part, time_part)
}

/// Facebook: `FB_IMG_<unix_milliseconds>`. Trailing non-digit chars are ignored.
fn parse_facebook_date(stem: &str) -> Option<DateTime<Local>> {
    let ms_str = stem.strip_prefix("FB_IMG_")?;
    let digits: &str = ms_str
        .split(|c: char| !c.is_ascii_digit())
        .next()
        .filter(|s| !s.is_empty())?;
    let ms: i64 = digits.parse().ok()?;
    DateTime::from_timestamp(ms / 1000, 0).map(|dt| dt.with_timezone(&Local))
}

fn parse_ymd_at_noon(date: &str) -> Option<DateTime<Local>> {
    parse_ymd_hms(date, "120000")
}

fn parse_ymd_hms(date: &str, time: &str) -> Option<DateTime<Local>> {
    if date.len() != 8 || time.len() != 6 {
        return None;
    }
    let year: i32 = date[0..4].parse().ok()?;
    let month: u32 = date[4..6].parse().ok()?;
    let day: u32 = date[6..8].parse().ok()?;
    let hour: u32 = time[0..2].parse().ok()?;
    let min: u32 = time[2..4].parse().ok()?;
    let sec: u32 = time[4..6].parse().ok()?;
    Local
        .with_ymd_and_hms(year, month, day, hour, min, sec)
        .single()
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- filename pattern tests ---

    #[test]
    fn test_whatsapp_photo_date() {
        let path = std::path::Path::new("IMG-20250626-WA0002.jpg");
        let dt = parse_date_from_filename(path).unwrap();
        assert_eq!(dt.format("%Y-%m-%d").to_string(), "2025-06-26");
        assert_eq!(dt.format("%H:%M:%S").to_string(), "12:00:00");
    }

    #[test]
    fn test_whatsapp_video_date() {
        let path = std::path::Path::new("VID-20250408-WA0022.mp4");
        let dt = parse_date_from_filename(path).unwrap();
        assert_eq!(dt.format("%Y-%m-%d").to_string(), "2025-04-08");
    }

    #[test]
    fn test_whatsapp_audio_date() {
        let path = std::path::Path::new("AUD-20231115-WA0001.opus");
        let dt = parse_date_from_filename(path).unwrap();
        assert_eq!(dt.format("%Y-%m-%d").to_string(), "2023-11-15");
    }

    #[test]
    fn test_whatsapp_with_suffix() {
        // Synology Drive conflict copies append extra chars after the WA number
        let path = std::path::Path::new("IMG-20250626-WA0003 (1).jpg");
        let dt = parse_date_from_filename(path).unwrap();
        assert_eq!(dt.format("%Y-%m-%d").to_string(), "2025-06-26");
    }

    #[test]
    fn test_whatsapp_invalid_month_returns_none() {
        let path = std::path::Path::new("IMG-20251340-WA0001.jpg");
        assert!(parse_date_from_filename(path).is_none());
    }

    #[test]
    fn test_android_with_img_prefix() {
        let path = std::path::Path::new("IMG_20190807_080939.jpg");
        let dt = parse_date_from_filename(path).unwrap();
        assert_eq!(
            dt.format("%Y-%m-%d %H:%M:%S").to_string(),
            "2019-08-07 08:09:39"
        );
    }

    #[test]
    fn test_android_with_img_prefix_extra_digits() {
        // Some Android files append a 7th digit after HHMMSS
        let path = std::path::Path::new("IMG_20190807_0809391.jpg");
        let dt = parse_date_from_filename(path).unwrap();
        assert_eq!(
            dt.format("%Y-%m-%d %H:%M:%S").to_string(),
            "2019-08-07 08:09:39"
        );
    }

    #[test]
    fn test_android_bare_yyyymmdd_hhmmss() {
        let path = std::path::Path::new("20220630_211409.jpg");
        let dt = parse_date_from_filename(path).unwrap();
        assert_eq!(
            dt.format("%Y-%m-%d %H:%M:%S").to_string(),
            "2022-06-30 21:14:09"
        );
    }

    #[test]
    fn test_android_bare_with_sequence_suffix() {
        let path = std::path::Path::new("20220630_211409_031.jpg");
        let dt = parse_date_from_filename(path).unwrap();
        assert_eq!(dt.format("%Y-%m-%d").to_string(), "2022-06-30");
    }

    #[test]
    fn test_facebook_timestamp_ms() {
        // 1484685560948 ms → 1484685560 s → 2017-01-17 22:19:20 UTC
        let path = std::path::Path::new("FB_IMG_1484685560948.jpg");
        let dt = parse_date_from_filename(path).unwrap();
        let dt_utc = dt.with_timezone(&chrono::Utc);
        assert_eq!(dt_utc.format("%Y-%m-%d").to_string(), "2017-01-17");
    }

    #[test]
    fn test_no_pattern_returns_none() {
        assert!(parse_date_from_filename(std::path::Path::new("random_file.jpg")).is_none());
        assert!(parse_date_from_filename(std::path::Path::new("4.jpg")).is_none());
        assert!(parse_date_from_filename(std::path::Path::new("no_exif.jpg")).is_none());
        assert!(parse_date_from_filename(std::path::Path::new("clip.mp4")).is_none());
    }
}
