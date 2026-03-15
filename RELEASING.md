# Release Process

This document describes how to cut a new release of `syno-media-organizer`.

---

## Checklist

### 1. Update `CHANGELOG.md`

Move items from `[Unreleased]` into a new `## [X.Y.Z] - YYYY-MM-DD` section.

```markdown
## [1.2.3] - 2026-05-01

### Added
- ...

### Fixed
- ...
```

### 2. Run the release script

```bash
bash scripts/release.sh X.Y.Z
```

This script:
- Verifies the working tree is clean
- Bumps `version` in `Cargo.toml` and `spk/INFO`
- Verifies a `[X.Y.Z]` entry exists in `CHANGELOG.md`
- Creates a commit: `chore(release): bump version to X.Y.Z`
- Creates an annotated tag: `vX.Y.Z`

### 3. Push

```bash
git push origin main --tags
```

### 4. GitHub Actions takes over

The `release.yml` workflow triggers on the `v*.*.*` tag and:
1. Cross-compiles for `armv7-unknown-linux-gnueabihf`
2. Strips the binary
3. Runs `scripts/build-spk.sh` to produce `dist/syno-media-organizer-X.Y.Z.spk`
4. Extracts release notes from `CHANGELOG.md`
5. Publishes a GitHub Release with:
   - The `.spk` file
   - The `SHA256` checksum
   - The release notes

### 5. Verify the release

- Check the [GitHub Releases](https://github.com/JordanAtDown/syno-media-organizer/releases) page
- Verify the `.spk` SHA256 matches the checksum file
- Install on a test NAS if available

---

## Hotfix process

For urgent fixes on a tagged release:

1. Fix the bug on `main`
2. Bump the patch version (e.g. `1.2.3` → `1.2.4`)
3. Follow the normal release checklist above

There are no long-lived release branches — all development happens on `main`.
