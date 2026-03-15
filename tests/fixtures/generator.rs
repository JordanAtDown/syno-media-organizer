//! Test fixture generators — creates minimal valid media files on the fly.
//! No binary fixtures are committed to the repository.

use chrono::{DateTime, Local, TimeZone};
use std::path::{Path, PathBuf};

/// Minimal valid JPEG SOI + EOI (2 bytes each)
static JPEG_HEADER: &[u8] = &[0xFF, 0xD8, 0xFF, 0xE0];
static JPEG_FOOTER: &[u8] = &[0xFF, 0xD9];

/// Create a minimal JPEG file WITHOUT EXIF metadata.
pub fn create_jpeg_without_exif(dir: &Path, name: &str) -> PathBuf {
    let path = dir.join(name);
    let mut data = Vec::new();
    data.extend_from_slice(JPEG_HEADER);
    // Minimal APP0 JFIF marker
    data.extend_from_slice(&[0x00, 0x10]); // length = 16
    data.extend_from_slice(b"JFIF\x00");   // identifier
    data.extend_from_slice(&[0x01, 0x01]); // version
    data.extend_from_slice(&[0x00]);       // aspect ratio unit
    data.extend_from_slice(&[0x00, 0x01]); // Xdensity
    data.extend_from_slice(&[0x00, 0x01]); // Ydensity
    data.extend_from_slice(&[0x00, 0x00]); // thumbnail size
    data.extend_from_slice(JPEG_FOOTER);
    std::fs::write(&path, &data).expect("Failed to write test JPEG");
    path
}

/// Create a minimal JPEG file WITH EXIF metadata containing the given date.
///
/// Embeds DateTimeOriginal in EXIF format "YYYY:MM:DD HH:MM:SS".
pub fn create_jpeg_with_exif(dir: &Path, name: &str, date: DateTime<Local>) -> PathBuf {
    let path = dir.join(name);
    let date_str = date.format("%Y:%m:%d %H:%M:%S").to_string();
    let date_bytes = date_str.as_bytes();

    // Minimal EXIF APP1 segment with DateTimeOriginal
    // We build a minimal TIFF structure with a single IFD
    let mut exif_data: Vec<u8> = Vec::new();

    // TIFF header: little-endian byte order mark + magic + IFD offset
    exif_data.extend_from_slice(b"II");        // little-endian
    exif_data.extend_from_slice(&[0x2A, 0x00]); // TIFF magic
    exif_data.extend_from_slice(&[0x08, 0x00, 0x00, 0x00]); // IFD offset = 8

    // IFD: 1 entry
    exif_data.extend_from_slice(&[0x01, 0x00]); // 1 directory entry

    // DateTimeOriginal tag = 0x9003, type ASCII = 2, count = 20
    let count = (date_bytes.len() + 1) as u32; // +1 for null terminator
    let value_offset = (8 + 2 + 12 + 4) as u32; // after IFD
    exif_data.extend_from_slice(&[0x03, 0x90]); // tag 0x9003 LE
    exif_data.extend_from_slice(&[0x02, 0x00]); // type ASCII
    exif_data.extend_from_slice(&count.to_le_bytes());
    exif_data.extend_from_slice(&value_offset.to_le_bytes());

    // IFD end
    exif_data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);

    // DateTimeOriginal value
    exif_data.extend_from_slice(date_bytes);
    exif_data.push(0x00); // null terminator

    // Build APP1 segment
    let mut app1: Vec<u8> = Vec::new();
    app1.extend_from_slice(b"Exif\x00\x00");
    app1.extend_from_slice(&exif_data);

    // APP1 marker + length (length includes the 2-byte length field itself)
    let app1_len = (app1.len() + 2) as u16;

    let mut data: Vec<u8> = Vec::new();
    data.extend_from_slice(&[0xFF, 0xD8]); // SOI
    data.extend_from_slice(&[0xFF, 0xE1]); // APP1 marker
    data.extend_from_slice(&app1_len.to_be_bytes());
    data.extend_from_slice(&app1);
    data.extend_from_slice(JPEG_FOOTER); // EOI

    std::fs::write(&path, &data).expect("Failed to write test JPEG with EXIF");
    path
}

/// Create a minimal MP4 stub (just enough bytes to be recognized by extension).
pub fn create_mp4_stub(dir: &Path, name: &str) -> PathBuf {
    let path = dir.join(name);
    // Minimal ftyp box
    let ftyp: &[u8] = &[
        0x00, 0x00, 0x00, 0x14, // box size = 20
        b'f', b't', b'y', b'p', // box type
        b'i', b's', b'o', b'm', // major brand
        0x00, 0x00, 0x02, 0x00, // minor version
        b'i', b's', b'o', b'm', // compatible brands
    ];
    std::fs::write(&path, ftyp).expect("Failed to write test MP4");
    path
}

/// Helper: create a date in local time from components.
pub fn make_date(year: i32, month: u32, day: u32, hour: u32, min: u32, sec: u32) -> DateTime<Local> {
    Local
        .with_ymd_and_hms(year, month, day, hour, min, sec)
        .single()
        .expect("Invalid date components")
}
