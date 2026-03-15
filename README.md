# syno-media-organizer

[![CI](https://github.com/JordanAtDown/syno-media-organizer/actions/workflows/ci.yml/badge.svg)](https://github.com/JordanAtDown/syno-media-organizer/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/JordanAtDown/syno-media-organizer)](https://github.com/JordanAtDown/syno-media-organizer/releases)

Automatically organize photos and videos on a **Synology NAS** by reading EXIF
metadata and placing files into a dated folder hierarchy (`ANNEE/MOIS`).

Designed for the **DS216play** (ARMv7l, 512 MB RAM) running **DSM 7.x** with minimal
CPU and memory footprint. Runs as a native Synology SPK service.

---

## Features

- Watches one or more input folders (inotify, 500ms debounce)
- Reads EXIF `DateTimeOriginal` → `DateTimeDigitized` → `DateTime` → mtime fallback
- Moves or copies files to `output/YYYY/MM/` according to a configurable pattern
- Handles filename conflicts: `rename`, `skip`, or `overwrite`
- Structured JSON logging compatible with DSM log center
- Graceful shutdown on SIGTERM (DSM stop command)
- Thread pool capped at 2 to protect the NAS

---

## Architecture

```
inotify event
     │
  watcher (debounce 500ms)
     │
  processor
     ├── validate extension
     ├── read EXIF date
     ├── apply naming pattern
     ├── resolve conflicts
     ├── create directories
     └── move or copy
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
3. **Edit the config** via File Station:
   - Open **File Station** → shared folder `config` → `syno-media-organizer` → `config.toml`
   - Right-click → **Open with Text Editor** (install the free Text Editor package if needed)
   - Or via SSH: `vi /volume1/config/syno-media-organizer/config.toml`
4. **Start the service** from **Package Center → Syno Media Organizer → Run**
5. **Check logs** (SSH):
   ```sh
   tail -f /var/packages/syno-media-organizer/var/syno-media-organizer.log
   ```

> **Upgrading**: existing `config.toml` is automatically migrated and preserved.
> **Uninstall**: Package Center → Syno Media Organizer → Uninstall

---

## Configuration

```toml
[[folders]]
input      = "/volume1/inbox/camera"
output     = "/volume1/Phototheque"
pattern    = "{year}/{month}/{year}-{month}-{day}_{hour}{min}{sec}_{stem}{ext}"
recursive  = true
move_files = true
on_conflict = "rename"   # rename | skip | overwrite
extensions = ["jpg", "jpeg", "png", "heic", "mp4", "mov", "avi", "mkv"]
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
cargo build --release --target armv7-unknown-linux-gnueabihf
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
