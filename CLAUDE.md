# CLAUDE.md — syno-media-organizer

See [.claude/CLAUDE.md](.claude/CLAUDE.md) for the full architecture guide.

## Quick reference

```
src/main.rs       CLI + entry point
src/config.rs     TOML parsing
src/exif.rs       EXIF reading (kamadak-exif) + mtime fallback
src/naming.rs     Pattern engine + conflict resolution
src/processor.rs  File pipeline: validate → exif → name → mkdir → move/copy
src/watcher.rs    notify watcher (500ms debounce) + SIGTERM
```

**Cross-compile**: `bash scripts/setup-cross.sh` once, then `cargo build --release --target armv7-unknown-linux-gnueabihf`

**Release**: `bash scripts/release.sh X.Y.Z` then `git push origin master --tags`

**Git hooks**: `git config core.hooksPath .githooks`
