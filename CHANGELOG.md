# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

---

## [0.1.1] - 2026-03-15

### Fixed
- SPK package rejected by DSM 7 ("invalid file format"): added required `checksum` field (MD5 of `package.tgz`) to `INFO` at build time
- SPK scripts (`installer`, `start-stop-status`) now packaged with executable permissions
- Updated minimum firmware to DSM 7.0 (`7.0-40000`) to match actual target platform

### Changed
- README: added Requirements section documenting supported NAS architecture and DSM version

---

## [0.1.0] - 2026-03-15

### Added
- TOML configuration with multiple `[[folders]]` entries (input, output, pattern, extensions, conflict strategy)
- EXIF metadata reading via `kamadak-exif` with priority chain: `DateTimeOriginal` → `DateTimeDigitized` → `DateTime` → mtime fallback
- Pattern engine supporting tokens: `{year}`, `{month}`, `{day}`, `{hour}`, `{min}`, `{sec}`, `{stem}`, `{ext}`, `{camera}`, `{counter}`
- Conflict resolution strategies: `rename` (auto-increment), `skip`, `overwrite`
- inotify file watcher using `notify` with 500ms debounce
- File pipeline: validate extension → read EXIF → compute name → create dirs → move or copy
- `--dry-run` mode: logs planned actions without touching the filesystem
- Graceful shutdown on SIGTERM (DSM-compatible)
- Structured JSON logging via `tracing` (configurable with `--log-format json`)
- Synology SPK packaging with DSM start/stop/status scripts
- GitHub Actions CI: fmt + clippy + tests + ARMv7 cross-compile
- GitHub Actions Release: auto-publish `.spk` + SHA256 on version tags
- Git hooks for Conventional Commits validation and pre-commit quality checks
