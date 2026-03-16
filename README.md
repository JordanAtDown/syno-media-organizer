# syno-media-organizer

<p align="center">
  <img src="spk/PACKAGE_ICON_256.PNG" alt="Syno Media Organizer" width="128"/>
</p>

[![CI](https://github.com/JordanAtDown/syno-media-organizer/actions/workflows/ci.yml/badge.svg)](https://github.com/JordanAtDown/syno-media-organizer/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/JordanAtDown/syno-media-organizer)](https://github.com/JordanAtDown/syno-media-organizer/releases)

Automatically organize photos and videos on a **Synology NAS** by reading EXIF
metadata and placing files into a dated folder hierarchy (`ANNEE/MOIS`).

Designed for the **DS216play** (ARMv7l, 512 MB RAM) running **DSM 7.x** with minimal
CPU and memory footprint. Runs as a native Synology SPK service.

---

## Features

- Polls one or more input folders every N seconds (default: 30s, configurable)
- **Photos**: reads EXIF `DateTimeOriginal` (tag 0x9003) — JPEG, HEIC, PNG, TIFF
- **Videos**: reads QuickTime `mvhd` creation date (UTC → local time) — MP4, MOV, AVI, MKV…
- Files without the required metadata tag are **skipped** — no fallback, no data loss
- Moves files to `output/YYYY/MM/` according to a configurable naming pattern
- Handles filename conflicts: `rename`, `skip`, or `overwrite`
- Structured JSON logging compatible with DSM log center
- Graceful shutdown on SIGTERM (DSM stop command)
- Statically linked (musl) — no GLIBC dependency, works on any DSM kernel

---

## Architecture

```
every poll_interval_secs (default: 30s)
     │
  watcher (scan input folder)
     │
  processor
     ├── validate extension
     ├── read capture date
     │     ├── photo → EXIF DateTimeOriginal (tag 0x9003)
     │     └── video → QuickTime mvhd creation_time (UTC → local)
     ├── apply naming pattern
     ├── resolve conflicts
     ├── create directories
     └── move (rename syscall, fallback copy+delete)
```

---

## Requirements

| Requirement | Value |
|-------------|-------|
| NAS architecture | DS216play (STM Monaco STiH412 — ARMv7l) |
| DSM version | **7.0 minimum** (tested on DSM 7.1.1) |
| RAM | 512 MB minimum |

> Other monaco-platform models (DS215play, DS214play) should also work.
> Models with a different SoC (DS216, DS116 — armada38x platform) are **not** compatible with this package.

---

## Installation on Synology NAS

1. Download the latest `.spk` from [Releases](https://github.com/JordanAtDown/syno-media-organizer/releases)
2. In DSM: **Package Center → Manual Install** → select the `.spk` file
3. **Grant shared folder permissions** (required — see section below)
4. **Edit the config** via File Station:
   - Open **File Station** → shared folder `config` → `syno-media-organizer` → `config.toml`
   - Right-click → **Open with Text Editor** (install the free Text Editor package if needed)
   - Or via SSH: `vi /volume1/config/syno-media-organizer/config.toml`
5. **Start the service** from **Package Center → Syno Media Organizer → Run**
6. **Check logs** (SSH):
   ```sh
   tail -f /var/packages/syno-media-organizer/var/syno-media-organizer.log
   ```

> **Upgrading**: existing `config.toml` is automatically migrated and preserved.
> **Uninstall**: Package Center → Syno Media Organizer → Uninstall

---

## Shared folder permissions (required)

The service runs as a dedicated system user `syno-media-organizer`. Synology shared folders
restrict access by user — you must explicitly grant this user access to every shared folder
the tool reads from or writes to.

**For each shared folder used as `input` or `output` in your config:**

1. **Panneau de configuration** → **Dossier partagé**
2. Select the folder → **Modifier** → tab **Permissions**
3. Click **Ajouter** → search for user `syno-media-organizer`
4. Grant: **Lecture/Écriture** (Read/Write) on input folders; **Lecture/Écriture** on output folders

> The `config` shared folder (where `config.toml` lives) is handled automatically by the
> installer — no manual action needed for it.

---

## Configuration

```toml
# Scan interval in seconds (optional, default: 30)
poll_interval_secs = 30

[[folders]]
input        = "/volume1/inbox/camera"
output       = "/volume1/Phototheque"
pattern      = "{year}/{month}/{prefix}{year}-{month}-{day}_{hour}{min}{sec}_{stem}{ext}"
recursive    = true
photo_prefix = "IMG_"   # optional, default: ""
video_prefix = "VID_"   # optional, default: ""
on_conflict  = "rename"   # rename | skip | overwrite
extensions   = ["jpg", "jpeg", "png", "heic", "mp4", "mov", "avi", "mkv"]
```

### Pattern tokens

| Token | Description |
|-------|-------------|
| `{year}` | 4-digit year (2024) |
| `{month}` | 2-digit month (01–12) |
| `{day}` | 2-digit day (01–31) |
| `{hour}` | 2-digit hour (00–23) |
| `{min}` | 2-digit minute |
| `{sec}` | 2-digit second |
| `{stem}` | Original filename without extension |
| `{ext}` | Extension with dot (`.jpg`) |
| `{camera}` | Camera model from EXIF (or `unknown`) |
| `{counter}` | Auto-increment counter (0001, 0002, …) |
| `{prefix}` | `photo_prefix` for photos, `video_prefix` for videos (default: `""`) |

---

## Supported formats

### Photos — EXIF `DateTimeOriginal` (tag 0x9003)

| Extension | Format |
|-----------|--------|
| `.jpg` / `.jpeg` | JPEG |
| `.heic` / `.heif` | High Efficiency Image (iPhone) |
| `.png` | PNG with EXIF APP1 block |
| `.tiff` / `.tif` | TIFF |

The file must contain a valid EXIF block with a `DateTimeOriginal` field.
If the tag is absent the file is **skipped** — no fallback, file stays in input.

### Videos — QuickTime `mvhd` creation date

| Extension | Format |
|-----------|--------|
| `.mp4` | MPEG-4 |
| `.mov` | QuickTime Movie (iPhone, cameras) |
| `.avi` | Audio Video Interleave |
| `.mkv` | Matroska |
| `.3gp` | 3GPP (Android) |
| `.m4v` | iTunes Video |
| `.wmv` | Windows Media Video |
| `.flv` | Flash Video |
| `.webm` | WebM |
| `.ts` / `.mts` / `.m2ts` | MPEG-2 Transport Stream |

The `creation_time` field is read from the `mvhd` (Movie Header Box) inside the `moov`
container. It is stored as **UTC** (seconds since 1904-01-01, Mac epoch) and converted
to local time at runtime. If the `moov` box is absent the file is **skipped**.

> Only extensions listed in the `extensions` config key are processed.
> You must explicitly include the extensions you want (e.g. `["jpg", "heic", "mp4", "mov"]`).

---

## Development

### Prerequisites

- WSL Ubuntu 24.04
- Rust stable (`rustup`)
- `git config core.hooksPath .githooks` (activate hooks)

### Setup cross-compilation

```bash
bash scripts/setup-cross.sh
```

### Common commands

```bash
cargo fmt
cargo clippy -- -D warnings
cargo test --lib            # unit tests
cargo test                  # all tests (unit + integration)
cargo zigbuild --release --target armv7-unknown-linux-musleabihf
bash scripts/build-spk.sh   # build the .spk package
```

### Release

```bash
# 1. Update CHANGELOG.md
# 2. Run:
bash scripts/release.sh X.Y.Z
git push origin master --tags
```

GitHub Actions will cross-compile, package the `.spk`, and publish the release.

---

## Contributing

- Follow [Conventional Commits](.github/COMMIT_CONVENTION.md)
- All commits must pass `cargo fmt --check`, `cargo clippy -D warnings`, and unit tests (enforced by pre-commit hook)
- No binary test fixtures — generate them at runtime with `tests/common/`
