//! Shared test utilities — generate minimal valid media files at runtime.
//! No binary fixtures are committed to the repository.

use chrono::{DateTime, Local, TimeZone};
use std::path::{Path, PathBuf};

static JPEG_FOOTER: &[u8] = &[0xFF, 0xD9];

/// Create a minimal JPEG file WITHOUT EXIF metadata.
pub fn create_jpeg_without_exif(dir: &Path, name: &str) -> PathBuf {
    let path = dir.join(name);
    let mut data: Vec<u8> = Vec::new();
    data.extend_from_slice(&[0xFF, 0xD8, 0xFF, 0xE0]); // SOI + APP0 marker
    data.extend_from_slice(&[0x00, 0x10]);              // APP0 length = 16
    data.extend_from_slice(b"JFIF\x00");
    data.extend_from_slice(&[0x01, 0x01, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00]);
    data.extend_from_slice(JPEG_FOOTER);
    std::fs::write(&path, &data).expect("Failed to write test JPEG");
    path
}

/// Create a minimal JPEG WITH EXIF embedding `DateTimeOriginal`.
pub fn create_jpeg_with_exif(dir: &Path, name: &str, date: DateTime<Local>) -> PathBuf {
    let path = dir.join(name);
    let date_str = date.format("%Y:%m:%d %H:%M:%S").to_string();
    let date_bytes = date_str.as_bytes();
    let count = (date_bytes.len() + 1) as u32;

    // Minimal TIFF/EXIF block (little-endian)
    let mut exif_ifd: Vec<u8> = Vec::new();
    exif_ifd.extend_from_slice(b"II");                          // LE byte order
    exif_ifd.extend_from_slice(&[0x2A, 0x00]);                 // TIFF magic
    exif_ifd.extend_from_slice(&[0x08, 0x00, 0x00, 0x00]);     // IFD offset = 8
    exif_ifd.extend_from_slice(&[0x01, 0x00]);                  // 1 IFD entry

    // Tag 0x9003 = DateTimeOriginal, type 2 = ASCII
    let value_offset = (8u32 + 2 + 12 + 4).to_le_bytes();
    exif_ifd.extend_from_slice(&[0x03, 0x90]);                  // tag LE
    exif_ifd.extend_from_slice(&[0x02, 0x00]);                  // ASCII
    exif_ifd.extend_from_slice(&count.to_le_bytes());
    exif_ifd.extend_from_slice(&value_offset);
    exif_ifd.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);     // next IFD = null
    exif_ifd.extend_from_slice(date_bytes);
    exif_ifd.push(0x00);                                         // null terminator

    let mut app1_body: Vec<u8> = Vec::new();
    app1_body.extend_from_slice(b"Exif\x00\x00");
    app1_body.extend_from_slice(&exif_ifd);

    let app1_len = (app1_body.len() + 2) as u16;

    let mut data: Vec<u8> = Vec::new();
    data.extend_from_slice(&[0xFF, 0xD8]);          // SOI
    data.extend_from_slice(&[0xFF, 0xE1]);          // APP1 marker
    data.extend_from_slice(&app1_len.to_be_bytes());
    data.extend_from_slice(&app1_body);
    data.extend_from_slice(JPEG_FOOTER);            // EOI

    std::fs::write(&path, &data).expect("Failed to write JPEG with EXIF");
    path
}

/// Create a minimal MP4 ftyp stub.
pub fn create_mp4_stub(dir: &Path, name: &str) -> PathBuf {
    let path = dir.join(name);
    let ftyp: &[u8] = &[
        0x00, 0x00, 0x00, 0x14, b'f', b't', b'y', b'p',
        b'i', b's', b'o', b'm', 0x00, 0x00, 0x02, 0x00,
        b'i', b's', b'o', b'm',
    ];
    std::fs::write(&path, ftyp).expect("Failed to write MP4 stub");
    path
}

/// Build a `DateTime<Local>` from components (panics on invalid input).
pub fn make_date(year: i32, month: u32, day: u32, hour: u32, min: u32, sec: u32) -> DateTime<Local> {
    Local
        .with_ymd_and_hms(year, month, day, hour, min, sec)
        .single()
        .expect("Invalid date components")
}
