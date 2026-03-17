# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

---

## [1.0.0] - 2026-03-17

### Added
- GitHub Pages package source: the SPK is now installable and auto-updatable directly from
  DSM Package Center by adding the following custom source URL:
  `https://jordanatdown.github.io/syno-media-organizer/packages.json`
- `docs/packages.json`: package index served via GitHub Pages — updated automatically by
  `scripts/release.sh` on every release.
- `scripts/release.sh` now updates `docs/packages.json` (version, SPK download link,
  changelog excerpt) as part of the release commit.

---

## [0.2.3] - 2026-03-16

### Added
- Persistent no-date cache (`no_date_cache.json`, stored next to `config.toml`): files
  that fail capture-date extraction are remembered across scan cycles and silently skipped
  until their modification time changes or the TTL expires. Previously, the same files
  generated a `WARN` log on every 30-second scan cycle.
- New global config options:
  - `no_date_cache_enabled` (default `true`): set to `false` to disable the cache and
    always re-scan every file.
  - `no_date_cache_ttl_days` (default `0` = never expire): set e.g. `30` to retry files
    without metadata once a month regardless of mtime.
- Cache is invalidated automatically when a file's mtime changes (e.g. EXIF added via
  `exiftool`), so modified files are always re-processed without manual intervention.
- `postuninst()` hook in SPK installer: deletes `no_date_cache.json` on package
  uninstall while preserving the user's `config.toml`.

### Changed
- `processor::process_file()` now returns `Err(ProcessorError::CaptureDataNotFound)`
  instead of `Ok(())` when no capture date is found. The watcher emits the `WARN` log
  once (on first discovery) and then silently skips the file via the cache.

---

## [0.2.2] - 2026-03-16

### Fixed
- Files inside Synology DSM auto-generated directories (`@eaDir`, `@SynoEAStream`, `@Recycle`,
  `#recycle`, `@tmp`) are now silently ignored during scanning. Previously, the watcher processed
  `@eaDir/*/SYNOPHOTO_FILM_M.mp4` (Synology video thumbnails), which had `mvhd.creation_time = 0`
  and were incorrectly moved to `output/1970/01/`.
- `mvhd.creation_time = 0` (unset field in synthetic/encoder-generated MP4 files) now returns
  `NoDateTimeOriginal` instead of producing a `1970-01-01` date.

### Added
- New per-folder config option `excluded_dirs` (`Vec<String>`): list of directory names to skip
  during scanning, matched against every component of the file path. Defaults to
  `["@eaDir", "@SynoEAStream", "@Recycle", "#recycle", "@tmp"]`. Fully customizable — see
  `config.example.toml` for the complete list of Synology DSM directories.

---

## [0.2.1] - 2026-03-16

### Fixed
- QuickTime/MOV files from iPhone (and other Apple devices) are now parsed correctly.
  The `mp4 = "0.14"` crate failed on Apple-specific `trak` sub-boxes (`tapt`, `clef`,
  `prof`, `enof`) which are valid QuickTime extensions but not part of the MPEG-4 spec.
  Replaced with a minimal manual ISOBMFF parser that reads only `moov → mvhd`, skipping
  all other boxes. Works for both QuickTime (`qt  ` brand) and MPEG-4 (`isom`) files.

### Changed
- Removed `mp4 = "0.14"` dependency — no longer needed.

### Added
- Integration test fixture `tests/fixtures/sample_iphone.mov`: header-only extract
  (ftyp + moov, 4.7 KB) from a real iPhone XS `.mov` file, used to guard against
  regressions on Apple-specific QuickTime box structures.

---

## [0.2.0] - 2026-03-16

### Added
- Support MP4 and MOV video files: reads the `creation_time` field from the QuickTime
  `mvhd` (Movie Header Box) inside the `moov` container. The value is stored as UTC
  (seconds since 1904-01-01, Mac epoch) and converted to local time at runtime.
- New `DateReader` trait (`src/date_reader.rs`): decouples the date-extraction strategy
  from the processing pipeline. Swapping the concrete implementation (e.g. changing the
  MP4 parsing crate) requires only a new `impl DateReader` — no changes to `processor.rs`.
- `ExifDateReader` struct — wraps `kamadak-exif` for photos (JPEG, HEIC, PNG, TIFF).
- `QuickTimeDateReader` struct — wraps the `mp4` crate for videos (MP4, MOV, AVI, MKV…).

### Changed
- Version bumped to 0.2.0 (video support is a significant new capability).
- `processor.rs` now dispatches to the appropriate `DateReader` based on file extension
  via `date_reader::for_extension()`, instead of calling `exif::read_date()` directly.

---

## [0.1.22] - 2026-03-15

### Changed
- Date extraction now requires the `DateTimeOriginal` EXIF tag exclusively.
  Files without this tag (including MP4/MOV videos and JPEGs with no EXIF) are
  silently skipped with a warning log. The previous fallback chain
  (`DateTimeDigitized` → `DateTime` → mtime) has been removed to avoid
  organizing files under an incorrect date.

---

## [0.1.21] - 2026-03-15

### Added
- New pattern token `{prefix}`: resolves to `photo_prefix` for image files and `video_prefix`
  for video files, making it easy to distinguish photos from videos in the output folder.
- New per-folder config options `photo_prefix` (default: `""`) and `video_prefix` (default: `""`):
  configure the string substituted for `{prefix}` based on the file's media type.
  Video extensions: mp4, mov, avi, mkv, 3gp, m4v, wmv, flv, webm, ts, mts, m2ts.
  All other allowed extensions are treated as photos.

---

## [0.1.20] - 2026-03-15

### Changed
- Files are now always moved to the output folder; the `move_files` config option has been
  removed. Moving is the only safe strategy: it guarantees EXIF metadata is fully preserved
  (rename is a pure filesystem operation) and prevents files with old capture timestamps from
  being silently ignored on subsequent scans.

### Fixed
- Watcher no longer ignores files whose filesystem modification time predates the last scan
  (e.g. photos copied from a camera or phone where the original mtime is preserved). The
  mtime-based filter has been removed entirely since moved files cannot reappear in the input
  folder.

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
