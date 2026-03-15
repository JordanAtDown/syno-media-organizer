#!/usr/bin/env bash
# Manual release helper.
# Usage: bash scripts/release.sh X.Y.Z
# This script bumps versions, commits, tags, and pushes — GitHub Actions then
# picks up the tag and creates the GitHub Release + SPK automatically.
set -euo pipefail

if [ $# -ne 1 ]; then
    echo "Usage: $0 <version>   e.g.  $0 1.2.3"
    exit 1
fi

VERSION="$1"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

# Validate semver format
if ! echo "${VERSION}" | grep -qE '^[0-9]+\.[0-9]+\.[0-9]+$'; then
    echo "ERROR: version must be in semver format X.Y.Z"
    exit 1
fi

echo "==> Releasing version ${VERSION}"

# 1. Check clean working tree
if ! git -C "${ROOT}" diff --quiet || ! git -C "${ROOT}" diff --cached --quiet; then
    echo "ERROR: working tree is not clean. Commit or stash changes first."
    exit 1
fi

# 2. Bump version in Cargo.toml
sed -i "s/^version = \".*\"/version = \"${VERSION}\"/" "${ROOT}/Cargo.toml"

# 3. Bump version in spk/INFO
sed -i "s/^version=\".*\"/version=\"${VERSION}\"/" "${ROOT}/spk/INFO"

# 4. Verify CHANGELOG has an entry for this version
if ! grep -q "## \[${VERSION}\]" "${ROOT}/CHANGELOG.md"; then
    echo "ERROR: No entry for [${VERSION}] found in CHANGELOG.md"
    echo "       Add a '## [${VERSION}]' section before releasing."
    exit 1
fi

# 5. Commit
git -C "${ROOT}" add Cargo.toml spk/INFO
git -C "${ROOT}" commit -m "chore(release): bump version to ${VERSION}"

# 6. Tag
git -C "${ROOT}" tag -a "v${VERSION}" -m "Release ${VERSION}"

echo ""
echo "==> Version bumped and tagged locally."
echo "    Push to trigger the release workflow:"
echo ""
echo "    git push origin main --tags"
echo ""
echo "    GitHub Actions will then:"
echo "    1. Cross-compile for ARMv7"
echo "    2. Build the .spk package"
echo "    3. Create a GitHub Release with the SPK and SHA256 checksum"
