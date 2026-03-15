//! Shared test utilities — generate minimal valid media files at runtime.
//! No binary fixtures are committed to the repository.
#![allow(dead_code)]

use chrono::{DateTime, Local, TimeZone};
use std::path::{Path, PathBuf};

static JPEG_FOOTER: &[u8] = &[0xFF, 0xD9];

/// Create a minimal JPEG file WITHOUT EXIF metadata.
pub fn create_jpeg_without_exif(dir: &Path, name: &str) -> PathBuf {
    let path = dir.join(name);
    let mut data: Vec<u8> = Vec::new();
    data.extend_from_slice(&[0xFF, 0xD8, 0xFF, 0xE0]); // SOI + APP0 marker
    data.extend_from_slice(&[0x00, 0x10]); // APP0 length = 16
    data.extend_from_slice(b"JFIF\x00");
    data.extend_from_slice(&[0x01, 0x01, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00]);
    data.extend_from_slice(JPEG_FOOTER);
    std::fs::write(&path, &data).expect("Failed to write test JPEG");
    path
}

/// Create a minimal JPEG WITH EXIF embedding `DateTimeOriginal` (tag 0x9003, in ExifIFD).
pub fn create_jpeg_with_exif(dir: &Path, name: &str, date: DateTime<Local>) -> PathBuf {
    let path = dir.join(name);
    let date_str = date.format("%Y:%m:%d %H:%M:%S").to_string();
    let date_bytes = date_str.as_bytes(); // 19 bytes
    let date_count = (date_bytes.len() + 1) as u32; // 20 (including null terminator)

    // TIFF layout (little-endian):
    //   offset  0: TIFF header (8 bytes) — "II", 0x002A, IFD0 offset=8
    //   offset  8: IFD0 — 1 entry: ExifIFD pointer (tag 0x8769)
    //              2 (count) + 12 (entry) + 4 (next IFD) = 18 bytes → ends at 26
    //   offset 26: ExifIFD — 1 entry: DateTimeOriginal (tag 0x9003)
    //              2 + 12 + 4 = 18 bytes → ends at 44
    //   offset 44: DateTimeOriginal ASCII string

    let exif_ifd_offset: u32 = 26;
    let date_value_offset: u32 = 44;

    let mut tiff: Vec<u8> = Vec::new();

    // TIFF header
    tiff.extend_from_slice(b"II"); // little-endian
    tiff.extend_from_slice(&[0x2A, 0x00]); // TIFF magic
    tiff.extend_from_slice(&8u32.to_le_bytes()); // IFD0 at offset 8

    // IFD0: 1 entry — ExifIFD pointer (tag 0x8769, type LONG=4, count=1, value=ExifIFD offset)
    tiff.extend_from_slice(&1u16.to_le_bytes()); // entry count
    tiff.extend_from_slice(&[0x69, 0x87]); // tag 0x8769 LE
    tiff.extend_from_slice(&[0x04, 0x00]); // type LONG
    tiff.extend_from_slice(&1u32.to_le_bytes()); // count
    tiff.extend_from_slice(&exif_ifd_offset.to_le_bytes()); // value = ExifIFD offset
    tiff.extend_from_slice(&0u32.to_le_bytes()); // next IFD = null

    // ExifIFD: 1 entry — DateTimeOriginal (tag 0x9003, type ASCII=2)
    tiff.extend_from_slice(&1u16.to_le_bytes()); // entry count
    tiff.extend_from_slice(&[0x03, 0x90]); // tag 0x9003 LE
    tiff.extend_from_slice(&[0x02, 0x00]); // type ASCII
    tiff.extend_from_slice(&date_count.to_le_bytes()); // count (20)
    tiff.extend_from_slice(&date_value_offset.to_le_bytes()); // value offset
    tiff.extend_from_slice(&0u32.to_le_bytes()); // next IFD = null

    // DateTimeOriginal value
    tiff.extend_from_slice(date_bytes);
    tiff.push(0x00); // null terminator

    let mut app1_body: Vec<u8> = Vec::new();
    app1_body.extend_from_slice(b"Exif\x00\x00");
    app1_body.extend_from_slice(&tiff);

    let app1_len = (app1_body.len() + 2) as u16;

    let mut data: Vec<u8> = Vec::new();
    data.extend_from_slice(&[0xFF, 0xD8]); // SOI
    data.extend_from_slice(&[0xFF, 0xE1]); // APP1 marker
    data.extend_from_slice(&app1_len.to_be_bytes());
    data.extend_from_slice(&app1_body);
    data.extend_from_slice(JPEG_FOOTER); // EOI

    std::fs::write(&path, &data).expect("Failed to write JPEG with EXIF");
    path
}

/// Create a minimal MP4 ftyp stub.
pub fn create_mp4_stub(dir: &Path, name: &str) -> PathBuf {
    let path = dir.join(name);
    let ftyp: &[u8] = &[
        0x00, 0x00, 0x00, 0x14, b'f', b't', b'y', b'p', b'i', b's', b'o', b'm', 0x00, 0x00, 0x02,
        0x00, b'i', b's', b'o', b'm',
    ];
    std::fs::write(&path, ftyp).expect("Failed to write MP4 stub");
    path
}

/// Build a `DateTime<Local>` from components (panics on invalid input).
pub fn make_date(
    year: i32,
    month: u32,
    day: u32,
    hour: u32,
    min: u32,
    sec: u32,
) -> DateTime<Local> {
    Local
        .with_ymd_and_hms(year, month, day, hour, min, sec)
        .single()
        .expect("Invalid date components")
}
