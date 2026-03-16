use crate::error::ExifError;
use chrono::{DateTime, Local, TimeZone};
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
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

/// Read the capture date from a QuickTime/MP4 file's `mvhd` `creation_time` field.
///
/// Traverses only `moov → mvhd` — does NOT parse `trak`, `mdia`, `stbl` or any
/// other sub-tree. This makes it robust against Apple-specific boxes (`tapt`, `clef`,
/// `prof`, `enof`) that trip up full-tree parsers.
///
/// The value is stored as UTC seconds since the Mac epoch (1904-01-01 00:00:00 UTC)
/// and is converted to local time at runtime.
///
/// Returns `ExifError::NoDateTimeOriginal` if the `mvhd` box is absent or unreadable.
pub fn read_quicktime_date(path: &Path) -> Result<DateTime<Local>, ExifError> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    read_mvhd_creation_time(&mut reader)
}

// ---------------------------------------------------------------------------
// Internal ISOBMFF / QuickTime box traversal
// ---------------------------------------------------------------------------

fn read_mvhd_creation_time<R: Read + Seek>(r: &mut R) -> Result<DateTime<Local>, ExifError> {
    loop {
        let (box_type, payload_len) = read_box_header(r)?;
        if &box_type == b"moov" {
            return read_mvhd_in_moov(r, payload_len);
        }
        // Skip all other top-level boxes (ftyp, mdat, free, wide, …)
        r.seek(SeekFrom::Current(payload_len as i64))
            .map_err(|_| ExifError::NoDateTimeOriginal)?;
    }
}

fn read_mvhd_in_moov<R: Read + Seek>(
    r: &mut R,
    moov_payload: u64,
) -> Result<DateTime<Local>, ExifError> {
    let start = r
        .stream_position()
        .map_err(|_| ExifError::NoDateTimeOriginal)?;
    let end = start + moov_payload;

    while r
        .stream_position()
        .map_err(|_| ExifError::NoDateTimeOriginal)?
        + 8
        <= end
    {
        let (box_type, payload_len) = read_box_header(r)?;
        if &box_type == b"mvhd" {
            return parse_mvhd_creation_time(r);
        }
        // Skip trak, udta, meta, and all other moov children
        r.seek(SeekFrom::Current(payload_len as i64))
            .map_err(|_| ExifError::NoDateTimeOriginal)?;
    }
    Err(ExifError::NoDateTimeOriginal)
}

fn parse_mvhd_creation_time<R: Read>(r: &mut R) -> Result<DateTime<Local>, ExifError> {
    let mut ver_flags = [0u8; 4];
    r.read_exact(&mut ver_flags)
        .map_err(|_| ExifError::NoDateTimeOriginal)?;
    let version = ver_flags[0];

    // Mac epoch (1904-01-01 UTC) → Unix epoch (1970-01-01 UTC): subtract 2 082 844 800 s
    let mac_secs: u64 = if version == 0 {
        let mut buf = [0u8; 4];
        r.read_exact(&mut buf)
            .map_err(|_| ExifError::NoDateTimeOriginal)?;
        u32::from_be_bytes(buf) as u64
    } else if version == 1 {
        let mut buf = [0u8; 8];
        r.read_exact(&mut buf)
            .map_err(|_| ExifError::NoDateTimeOriginal)?;
        u64::from_be_bytes(buf)
    } else {
        return Err(ExifError::NoDateTimeOriginal);
    };

    let unix_secs = mac_secs.saturating_sub(2_082_844_800);
    DateTime::from_timestamp(unix_secs as i64, 0)
        .map(|dt| dt.with_timezone(&Local))
        .ok_or(ExifError::NoDateTimeOriginal)
}

/// Read an ISOBMFF/QuickTime box header.
/// Returns `(box_type: [u8; 4], payload_size: u64)` where payload_size excludes the header.
fn read_box_header<R: Read>(r: &mut R) -> Result<([u8; 4], u64), ExifError> {
    let mut header = [0u8; 8];
    r.read_exact(&mut header)
        .map_err(|_| ExifError::NoDateTimeOriginal)?;
    let size = u32::from_be_bytes([header[0], header[1], header[2], header[3]]);
    let box_type = [header[4], header[5], header[6], header[7]];
    let payload = match size {
        0 => return Err(ExifError::NoDateTimeOriginal), // box extends to EOF — unsupported
        1 => {
            // 64-bit extended size (rare)
            let mut ext = [0u8; 8];
            r.read_exact(&mut ext)
                .map_err(|_| ExifError::NoDateTimeOriginal)?;
            u64::from_be_bytes(ext).saturating_sub(16)
        }
        s => (s as u64).saturating_sub(8),
    };
    Ok((box_type, payload))
}

// ---------------------------------------------------------------------------
// EXIF datetime string parsing
// ---------------------------------------------------------------------------

/// Parse EXIF datetime string "YYYY:MM:DD HH:MM:SS" into a DateTime<Local>.
fn parse_exif_datetime(s: &str) -> Option<DateTime<Local>> {
    let s = s.trim();
    if s.len() < 19 {
        return None;
    }
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

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

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
        assert!(parse_exif_datetime("2024-03-15 10:30:45").is_none());
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
        // Synthetic MP4 with ftyp + moov/mvhd (version 0).
        // 2026-01-02 11:41:57 UTC:
        //   unix  = 1_767_354_117
        //   mac   = 1_767_354_117 + 2_082_844_800 = 3_850_198_917
        let mac_secs: u32 = 3_850_198_917;

        let ftyp: &[u8] = &[
            0x00, 0x00, 0x00, 0x14, b'f', b't', b'y', b'p', b'i', b's', b'o', b'm', 0x00, 0x00,
            0x02, 0x00, b'i', b's', b'o', b'm',
        ];

        let mut mvhd: Vec<u8> = Vec::new();
        mvhd.extend_from_slice(&108u32.to_be_bytes());
        mvhd.extend_from_slice(b"mvhd");
        mvhd.push(0); // version 0
        mvhd.extend_from_slice(&[0u8; 3]);
        mvhd.extend_from_slice(&mac_secs.to_be_bytes()); // creation_time
        mvhd.extend_from_slice(&mac_secs.to_be_bytes()); // modification_time
        mvhd.extend_from_slice(&1000u32.to_be_bytes()); // time_scale
        mvhd.extend_from_slice(&0u32.to_be_bytes()); // duration
        mvhd.extend_from_slice(&0x00010000u32.to_be_bytes()); // rate
        mvhd.extend_from_slice(&0x0100u16.to_be_bytes()); // volume
        mvhd.extend_from_slice(&[0u8; 10]);
        mvhd.extend_from_slice(&[
            0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x40, 0x00, 0x00, 0x00,
        ]);
        mvhd.extend_from_slice(&[0u8; 24]);
        mvhd.extend_from_slice(&1u32.to_be_bytes());

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
        let dt_utc = dt.with_timezone(&chrono::Utc);
        assert_eq!(
            dt_utc.format("%Y-%m-%d %H:%M:%S").to_string(),
            "2026-01-02 11:41:57"
        );
    }
}
