//! Integration tests for QuickTime/MOV metadata parsing using real device fixtures.
//!
//! Fixtures are committed because iPhone-specific box structures (`tapt`, `clef`,
//! `prof`, `enof`, Track Aperture atoms…) cannot be reproduced synthetically and
//! caused full-tree parsers to fail silently.
//!
//! Each fixture is a header-only extract (ftyp + moov, no video data).
//! See `tests/fixtures/README.md` for details.

use std::path::Path;
use syno_media_organizer::exif::read_quicktime_date;

/// Verify that the Create Date is correctly read from a real iPhone XS `.mov` file.
/// Expected: 2026:02:16 12:37:05 UTC (as reported by exiftool -CreateDate).
#[test]
fn test_read_quicktime_date_iphone_xs_mov() {
    let fixture = Path::new("tests/fixtures/sample_iphone.mov");
    let dt = read_quicktime_date(fixture)
        .expect("should parse Create Date from real iPhone .mov fixture");

    // Compare against UTC to stay independent of the test machine's local timezone.
    let dt_utc = dt.with_timezone(&chrono::Utc);
    assert_eq!(
        dt_utc.format("%Y-%m-%d %H:%M:%S").to_string(),
        "2026-02-16 12:37:05",
        "Create Date from mvhd must match exiftool -CreateDate output"
    );
}
