use crate::error::ExifError;
use chrono::{DateTime, Local, TimeZone};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

/// Read the capture date from a file's EXIF metadata.
/// Falls back to the file's last-modification time if no EXIF date is found.
pub fn read_date(path: &Path) -> Result<DateTime<Local>, ExifError> {
    match read_exif_date(path) {
        Ok(Some(dt)) => Ok(dt),
        Ok(None) => read_mtime(path),
        Err(_) => read_mtime(path),
    }
}

fn read_exif_date(path: &Path) -> Result<Option<DateTime<Local>>, ExifError> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);

    let exif_reader = exif::Reader::new();
    let exif = match exif_reader.read_from_container(&mut reader) {
        Ok(e) => e,
        Err(_) => return Ok(None),
    };

    // Priority: DateTimeOriginal > DateTimeDigitized > DateTime
    let tag_priority = [
        exif::Tag::DateTimeOriginal,
        exif::Tag::DateTimeDigitized,
        exif::Tag::DateTime,
    ];

    for tag in &tag_priority {
        if let Some(field) = exif.get_field(*tag, exif::In::PRIMARY) {
            if let exif::Value::Ascii(ref vec) = field.value {
                if let Some(bytes) = vec.first() {
                    let s = String::from_utf8_lossy(bytes);
                    if let Some(dt) = parse_exif_datetime(&s) {
                        return Ok(Some(dt));
                    }
                }
            }
        }
    }

    Ok(None)
}

/// Parse EXIF datetime string "YYYY:MM:DD HH:MM:SS" into a DateTime<Local>.
fn parse_exif_datetime(s: &str) -> Option<DateTime<Local>> {
    // EXIF format: "2024:01:15 14:30:00" — separators must be exactly ':', ':', ' ', ':', ':'
    let s = s.trim();
    if s.len() < 19 {
        return None;
    }
    // Validate separators at fixed positions
    let bytes = s.as_bytes();
    if bytes[4] != b':'
        || bytes[7] != b':'
        || bytes[10] != b' '
        || bytes[13] != b':'
        || bytes[16] != b':'
    {
        return None;
    }
    let year: i32 = s[0..4].parse().ok()?;
    let month: u32 = s[5..7].parse().ok()?;
    let day: u32 = s[8..10].parse().ok()?;
    let hour: u32 = s[11..13].parse().ok()?;
    let min: u32 = s[14..16].parse().ok()?;
    let sec: u32 = s[17..19].parse().ok()?;

    Local
        .with_ymd_and_hms(year, month, day, hour, min, sec)
        .single()
}

fn read_mtime(path: &Path) -> Result<DateTime<Local>, ExifError> {
    let meta = std::fs::metadata(path)?;
    let modified = meta.modified().map_err(ExifError::Io)?;
    Ok(DateTime::from(modified))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_exif_datetime_valid() {
        let dt = parse_exif_datetime("2024:03:15 10:30:45").unwrap();
        assert_eq!(
            dt.format("%Y-%m-%d %H:%M:%S").to_string(),
            "2024-03-15 10:30:45"
        );
    }

    #[test]
    fn test_parse_exif_datetime_invalid() {
        assert!(parse_exif_datetime("not a date").is_none());
        assert!(parse_exif_datetime("2024-03-15 10:30:45").is_none()); // wrong separator
        assert!(parse_exif_datetime("").is_none());
    }

    #[test]
    fn test_parse_exif_datetime_with_trailing_whitespace() {
        let dt = parse_exif_datetime("  2024:06:01 08:00:00  ").unwrap();
        assert_eq!(dt.format("%Y").to_string(), "2024");
    }

    #[test]
    fn test_read_date_fallback_mtime() {
        // Create a temp file with no EXIF — should fall back to mtime
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let dt = read_date(tmp.path()).unwrap();
        // Mtime should be recent (within last minute)
        let now = Local::now();
        let diff = now.signed_duration_since(dt);
        assert!(
            diff.num_seconds().abs() < 60,
            "mtime fallback should be recent"
        );
    }
}
