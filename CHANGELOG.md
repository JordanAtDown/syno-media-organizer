# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

---

## [0.1.19] - 2026-03-15

### Fixed
- Service fails to start via DSM Package Center (`synopkg start` returns error 272): the
  package user `syno-media-organizer` had no read access to the `/volume1/config` shared folder
  due to Synology ACL-based shared folder permissions. The installer now calls `synoacltool`
  to grant the package user read+traverse access to `/volume1/config` and its subdirectory
  automatically on install/upgrade.

### Added
- README: documented the required Synology shared folder permission configuration —
  users must grant `syno-media-organizer` read+write access to every input/output shared
  folder they configure, via DSM Panneau de configuration → Dossier partagé → Modifier → Permissions.

---

## [0.1.18] - 2026-03-15

### Fixed
- Definitive fix for startup panic on DSM kernels where `CLOCK_BOOTTIME` returns `EINVAL`:
  provide a `clock_gettime` symbol override in the musl static binary that intercepts any
  failed `CLOCK_BOOTTIME` call and retries with `CLOCK_MONOTONIC`. This covers all callers
  (Rust stdlib `Instant`, ctrlc, future deps) regardless of which one triggers the issue.

---

## [0.1.17] - 2026-03-15

### Fixed
- Startup panic on DSM kernels where `CLOCK_BOOTTIME` returns `EINVAL`: replaced `notify`
  (inotify-based file watcher, depends on `crossbeam-channel` which calls `Instant::now()`)
  and `rayon` (thread-pool init calls `Instant::now()` via crossbeam) with a simple polling
  loop using `walkdir` + `thread::sleep` — no `Instant` anywhere in the critical path

### Changed
- Watcher is now a polling loop: scans input folders every `poll_interval_secs` seconds
  (default: 30) instead of reacting to inotify events
- Only files with `mtime` newer than the previous scan are processed, preventing
  re-processing of existing files in `move_files = false` (copy) mode
- On first startup all pre-existing files in input folders are processed (catch-up)
- New config option `poll_interval_secs` (global, default `30`) to tune scan frequency

### Removed
- Dependencies: `notify`, `rayon` (both caused `CLOCK_BOOTTIME` panics via crossbeam)

---

## [0.1.16] - 2026-03-15

### Fixed
- Binary is now truly statically linked (no GLIBC dependency): switched cross-compiler from
  `arm-linux-gnueabihf-gcc` (glibc toolchain, produced glibc-linked binary despite musl target)
  to `cargo-zigbuild` + zig (correct musl static linking)
- CI now verifies the binary is statically linked before packaging

### Changed
- `scripts/setup-cross.sh`: installs zig + cargo-zigbuild instead of musl.cc toolchain
- CI and Release workflows: use `cargo zigbuild` for cross-compilation

---

## [0.1.15] - 2026-03-15

### Fixed
- Watcher no longer panics at startup on DSM kernels where `CLOCK_BOOTTIME` returns `EINVAL`:
  replaced `mpsc::recv_timeout` (which uses `std::time::Instant` / `CLOCK_BOOTTIME` internally)
  with a `try_recv` + `thread::sleep` + `SystemTime` (`CLOCK_REALTIME`) polling loop

---

## [0.1.14] - 2026-03-15

### Fixed
- Switch cross-compilation target to `armv7-unknown-linux-musleabihf` (musl static) — binary
  no longer depends on system GLIBC; fixes startup crash on DSM 7.1.1 (`GLIBC_2.28 not found`)
- Installer: add `postreplace()` as alias for `postinst()` — DSM calls `postreplace` on
  reinstall/replace operations, causing config directory and file to never be created

### Changed
- `scripts/setup-cross.sh`: downloads musl.cc ARM toolchain instead of apt glibc toolchain
- CI and Release workflows: use musl ARM cross-compiler from musl.cc

---

## [0.1.13] - 2026-03-15

### Fixed
- Reverted `case "$1"` dispatch back to raw `$1` in installer — case statement caused
  "échec de l'installation" on DSM 7.1.1 Update 9 for unknown reasons
- Removed `startable="yes"` from INFO — suspected to cause install failure on this DSM version
  (identical INFO to v0.1.8 which was the last confirmed working install)

---

## [0.1.12] - 2026-03-15

### Fixed
- Fix CI build failure: `build-spk.sh` referenced `PACKAGE_ICON.PNG` (uppercase) but git
  stores the icons as `PACKAGE_ICON.png` (lowercase) — Linux CI is case-sensitive

---

## [0.1.11] - 2026-03-15

### Added
- Package icons: `PACKAGE_ICON.PNG` (72×72) and `PACKAGE_ICON_256.PNG` (256×256)
  displayed in DSM Package Center (no `thirdparty="yes"` needed)
- README: logo displayed at top of page

---

## [0.1.10] - 2026-03-15

### Fixed
- `start-stop-status`: add `mkdir -p var/` at start of `start_daemon` — guarantees runtime
  directory exists even when upgraded without running `postinst`
- `start-stop-status`: replace `stop_daemon && start_daemon` with `stop_daemon; start_daemon`
  in restart — `start_daemon` was silently skipped if `stop_daemon` returned non-zero
- `start-stop-status`: verify process is still alive 1s after launch and report error
  immediately if it crashed (config parse error, missing volume, etc.)

---

## [0.1.9] - 2026-03-15

### Fixed
- Installer script now uses `case "$1"` dispatch instead of raw `$1` function call:
  unknown arguments (e.g. DSM lifecycle hooks not handled) now exit 0 gracefully instead of
  "command not found" which caused DSM to block uninstall with error 299

### Changed
- Re-added `startable="yes"` to INFO — package can now be started/stopped from Package Center

---

## [0.1.8] - 2026-03-15

### Changed
- Removed `startable="yes"` from INFO to test if it causes "format de fichier non valide" on DSM 7.1.1

---

## [0.1.7] - 2026-03-15

### Fixed
- Reverted SPK bundle to v0.1.4 structure: removed `thirdparty="yes"` and package icons
  which caused "format de fichier non valide" on DSM 7.1.1
- Reverted installer to simple `$1` dispatch style (matches v0.1.4 working baseline)

### Changed
- Package is now startable/stoppable from Package Center (`startable="yes"`)

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
