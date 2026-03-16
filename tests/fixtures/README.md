# Test Fixtures

Binary fixtures for integration tests. These files are intentionally committed
because real-device metadata quirks (Apple QuickTime boxes such as `tapt`, `clef`,
`prof`, `enof`) cannot be reliably reproduced with synthetic data.

Each fixture is a **header-only extract** — it contains the `ftyp` and `moov` boxes
only, with no video or audio payload. This keeps file sizes small while preserving
the exact metadata structure of the original device file.

| File | Device | Size | Expected Create Date (UTC) |
|------|--------|------|---------------------------|
| `sample_iphone.mov` | iPhone XS (iOS 18.7.4) | 4.7 KB | `2026-02-16 12:37:05` |

## How fixtures are created

```bash
# Extract ftyp + moov only (no video data).
# Media Data Offset (exiftool) gives the byte boundary.
head -c <media_data_offset> <source.mov> > tests/fixtures/<name>.mov

# Verify the expected date is readable:
exiftool -CreateDate tests/fixtures/<name>.mov
```
