# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

---

## [0.1.6] - 2026-03-15

### Fixed
- "Format de fichier non valide" at install: installer script used wrong DSM argument
  names (`pre-install`/`post-install`) instead of DSM 7 convention (`preinst`/`postinst`)
- Added `.gitattributes` to enforce LF line endings for shell scripts and SPK metadata

---

## [0.1.5] - 2026-03-15

### Fixed
- Package not visible in Package Center: added `startable="yes"` and `thirdparty="yes"` to INFO
- Added package icons (PACKAGE_ICON.PNG 72×72 and PACKAGE_ICON_256.PNG 256×256)

### Changed
- Config file moved from `/var/packages/.../etc/` to `/volume1/config/syno-media-organizer/config.toml` — editable via File Station without SSH
- Installer migrates existing config from old location on upgrade
- README: updated installation instructions with File Station config editing steps

---

## [0.1.4] - 2026-03-15

### Fixed
- DSM 7 blocks install with "runs with root privileges": added `conf/privilege` file declaring `run-as: package` (dedicated non-root user)
- PID file and log file moved from root-only `/var/run/` and `/var/log/` to `/var/packages/.../var/` (writable by package user)
- Installer now creates the `var/` runtime directory on post-install

### Changed
- README: clarified compatible models — DS215play and DS214play (monaco platform) should work; DS216/DS116 (armada38x) are not compatible

---

## [0.1.3] - 2026-03-15

### Fixed
- SPK rejected with "incompatible platform": DS216play uses STM Monaco STiH412 (ARMv7l), Synology platform identifier is `monaco` not `armada38x`
- README: corrected compatible hardware to DS216play (STM Monaco STiH412)

---

## [0.1.2] - 2026-03-15

### Fixed
- SPK rejected with "incompatible platform": changed `arch` from generic `armv7` to Synology platform identifier `armada38x` (Marvell Armada 385 — DS216play, DS216j, DS116, DS216+II)

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
