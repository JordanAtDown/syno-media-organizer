use crate::error::ExifError;
use chrono::{DateTime, Local, TimeZone};
use mp4::Mp4Reader;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

/// Read the capture date from a file's EXIF DateTimeOriginal tag (tag 0x9003).
/// Returns `ExifError::NoDateTimeOriginal` if the tag is absent or unreadable.
pub fn read_exif_date(path: &Path) -> Result<DateTime<Local>, ExifError> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);

    let exif_reader = exif::Reader::new();
    let exif = exif_reader
        .read_from_container(&mut reader)
        .map_err(|_| ExifError::NoDateTimeOriginal)?;

    let field = exif
        .get_field(exif::Tag::DateTimeOriginal, exif::In::PRIMARY)
        .ok_or(ExifError::NoDateTimeOriginal)?;

    if let exif::Value::Ascii(ref vec) = field.value {
        if let Some(bytes) = vec.first() {
            let s = String::from_utf8_lossy(bytes);
            if let Some(dt) = parse_exif_datetime(&s) {
                return Ok(dt);
            }
        }
    }

    Err(ExifError::NoDateTimeOriginal)
}

/// Read the capture date from a QuickTime/MP4 file's mvhd creation_time atom.
/// The value is stored as UTC seconds since the Mac epoch (1904-01-01 00:00:00 UTC)
/// and is converted to local time.
/// Returns `ExifError::NoDateTimeOriginal` if the box is absent or unreadable.
pub fn read_quicktime_date(path: &Path) -> Result<DateTime<Local>, ExifError> {
    let file = File::open(path)?;
    let size = file.metadata()?.len();
    let reader = BufReader::new(file);

    let mp4 = Mp4Reader::read_header(reader, size).map_err(|_| ExifError::NoDateTimeOriginal)?;

    let mac_secs = mp4.moov.mvhd.creation_time;
    // Mac epoch (1904-01-01 UTC) → Unix epoch (1970-01-01 UTC): subtract 2 082 844 800 s
    let unix_secs = mac_secs.saturating_sub(2_082_844_800);

    DateTime::from_timestamp(unix_secs as i64, 0)
        .map(|dt| dt.with_timezone(&Local))
        .ok_or(ExifError::NoDateTimeOriginal)
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
    fn test_read_exif_date_no_exif_returns_error() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let err = read_exif_date(tmp.path()).unwrap_err();
        assert!(matches!(err, ExifError::NoDateTimeOriginal));
    }

    #[test]
    fn test_read_quicktime_date_no_mvhd_returns_error() {
        // A stub with only ftyp (no moov/mvhd) must return NoDateTimeOriginal
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let ftyp: &[u8] = &[
            0x00, 0x00, 0x00, 0x14, b'f', b't', b'y', b'p', b'i', b's', b'o', b'm', 0x00, 0x00,
            0x02, 0x00, b'i', b's', b'o', b'm',
        ];
        std::fs::write(tmp.path(), ftyp).unwrap();
        let err = read_quicktime_date(tmp.path()).unwrap_err();
        assert!(matches!(err, ExifError::NoDateTimeOriginal));
    }

    #[test]
    fn test_read_quicktime_date_valid() {
        // Build a minimal MP4 (ftyp + moov/mvhd) with a known creation_time.
        // Use 2026-01-02 11:41:57 UTC → Mac epoch = unix + 2_082_844_800
        // 2026-01-01 00:00:00 UTC = 1_767_225_600
        // 2026-01-02 11:41:57 UTC = 1_767_225_600 + 86400 + 42117 = 1_767_354_117
        // Mac timestamp            = 1_767_354_117 + 2_082_844_800 = 3_850_198_917
        let mac_secs: u32 = 3_850_198_917;

        let ftyp: &[u8] = &[
            0x00, 0x00, 0x00, 0x14, b'f', b't', b'y', b'p', b'i', b's', b'o', b'm', 0x00, 0x00,
            0x02, 0x00, b'i', b's', b'o', b'm',
        ];

        let mut mvhd: Vec<u8> = Vec::new();
        mvhd.extend_from_slice(&108u32.to_be_bytes());
        mvhd.extend_from_slice(b"mvhd");
        mvhd.push(0); // version 0
        mvhd.extend_from_slice(&[0u8; 3]); // flags
        mvhd.extend_from_slice(&mac_secs.to_be_bytes()); // creation_time
        mvhd.extend_from_slice(&mac_secs.to_be_bytes()); // modification_time
        mvhd.extend_from_slice(&1000u32.to_be_bytes()); // time_scale
        mvhd.extend_from_slice(&0u32.to_be_bytes()); // duration
        mvhd.extend_from_slice(&0x00010000u32.to_be_bytes()); // rate
        mvhd.extend_from_slice(&0x0100u16.to_be_bytes()); // volume
        mvhd.extend_from_slice(&[0u8; 10]); // reserved
        mvhd.extend_from_slice(&[
            0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x40, 0x00, 0x00, 0x00,
        ]); // identity matrix
        mvhd.extend_from_slice(&[0u8; 24]); // pre-defined
        mvhd.extend_from_slice(&1u32.to_be_bytes()); // next_track_id

        let mut moov: Vec<u8> = Vec::new();
        moov.extend_from_slice(&116u32.to_be_bytes());
        moov.extend_from_slice(b"moov");
        moov.extend_from_slice(&mvhd);

        let mut data: Vec<u8> = Vec::new();
        data.extend_from_slice(ftyp);
        data.extend_from_slice(&moov);

        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), &data).unwrap();

        let dt = read_quicktime_date(tmp.path()).unwrap();
        // The stored UTC time is 2026-01-02 11:41:57 UTC.
        // We verify the UTC equivalent matches regardless of local timezone.
        let dt_utc = dt.with_timezone(&chrono::Utc);
        assert_eq!(
            dt_utc.format("%Y-%m-%d %H:%M:%S").to_string(),
            "2026-01-02 11:41:57"
        );
    }
}
