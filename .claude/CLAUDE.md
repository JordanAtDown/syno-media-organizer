# CLAUDE.md — syno-media-organizer

Architecture guide and workflow reference for AI-assisted development.

---

## Project overview

`syno-media-organizer` is a Rust daemon that watches one or more input folders,
reads EXIF metadata from photo/video files, and moves/copies them into a dated
folder hierarchy (`output/YYYY/MM/`) according to a configurable naming pattern.

**Target hardware**: Synology DS216play — ARMv7l, 512 MB RAM, DSM 6/7.

---

## Module architecture

```
src/
├── main.rs       CLI parsing (clap) + tracing init + entry point
├── lib.rs        Re-exports all modules as a library (for integration tests)
├── config.rs     TOML config parsing — Config, FolderConfig, OnConflict
├── error.rs      Typed errors for each module (thiserror)
├── exif.rs       EXIF date reading (kamadak-exif) + mtime fallback
├── naming.rs     Pattern engine + conflict resolution (Rename/Skip/Overwrite)
├── processor.rs  Full file pipeline: validate → exif → name → mkdir → move/copy
└── watcher.rs    notify watcher with 500ms debounce + SIGTERM via ctrlc
```

---

## Data flow

```
inotify event
     │
     ▼
watcher::run()          ← debounce 500ms, dedup paths
     │
     ▼
processor::process_file()
     │
     ├── validate extension (FolderConfig.extensions)
     ├── exif::read_date()   ← DateTimeOriginal → DateTimeDigitized → DateTime → mtime
     ├── naming::apply_pattern()   ← {year}/{month}/...
     ├── naming::resolve_conflict()  ← Rename | Skip | Overwrite
     ├── std::fs::create_dir_all()
     └── std::fs::rename() or copy+delete
```

---

## Key technical decisions

| Decision | Rationale |
|----------|-----------|
| `notify` crate | Cross-platform inotify/kqueue/FSEvents; debounce avoids multi-event floods on large files |
| `kamadak-exif` | Pure Rust, no C deps, no_std-compatible core; avoids linking libexif |
| `rayon` (≤2 threads) | Parallel file processing without saturating the DS216play dual-core |
| `tracing` + JSON mode | Structured logs parseable by DSM log center; JSON enabled with `--log-format json` |
| `thiserror` | Typed errors per module — callers can match precisely; no `anyhow` in lib code |
| Streaming IO | Files are never fully loaded into RAM; only metadata is read |
| Move = rename + fallback | `rename()` is a metadata-only op on same FS; falls back to copy+delete across filesystems |

---

## Cross-compilation (WSL Ubuntu 24)

### One-time setup

```bash
bash scripts/setup-cross.sh
```

This installs `rustup target add armv7-unknown-linux-gnueabihf`,
`gcc-arm-linux-gnueabihf`, and writes `.cargo/config.toml`.

### Build for ARMv7

```bash
cargo build --release --target armv7-unknown-linux-gnueabihf
```

### Build SPK

```bash
bash scripts/build-spk.sh
# Output: dist/syno-media-organizer-X.Y.Z.spk
```

---

## Development workflow

### Daily cycle

```bash
# Write code
cargo fmt
cargo clippy -- -D warnings
cargo test --lib        # fast unit tests

# Stage atomically
git add -p
git commit              # pre-commit hook runs automatically
git push origin master
```

### Activate git hooks (once)

```bash
git config core.hooksPath .githooks
```

### Run integration tests

```bash
cargo test              # unit + integration
```

### Release

```bash
# 1. Update CHANGELOG.md with a [X.Y.Z] section
# 2. Run the release script:
bash scripts/release.sh X.Y.Z
# 3. Push:
git push origin master --tags
# GitHub Actions creates the release automatically.
```

---

## Environment variables

| Variable | Effect |
|----------|--------|
| `RUST_LOG` | Log level filter (`debug`, `info`, `warn`, `error`) |
| `RUST_LOG_FORMAT` | `json` for structured output |

---

## NAS-specific constraints

- **No `thread::sleep` > 1s** in hot paths — causes stalls on a 512 MB system
- **Signal handling**: watcher reacts to SIGTERM within 200ms (polling interval)
- **No excessive allocation**: use iterators, avoid collecting large `Vec`s
- **Thread pool**: global rayon pool capped at 2 (`rayon::ThreadPoolBuilder`)
- **Log level in production**: `info` by default; `debug` only for troubleshooting

---

## Test strategy

- **Unit tests** (`src/*.rs` `#[cfg(test)]`): fast, no filesystem, rstest parametrized
- **Integration tests** (`tests/*.rs`): use `tempfile::TempDir`, generate minimal JPEG/MP4
  fixtures at runtime — **no binary fixtures committed**
- **Coverage goal**: >80% on business logic (`naming`, `processor`, `config`, `exif`)
