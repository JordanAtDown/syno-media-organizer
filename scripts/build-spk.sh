#!/usr/bin/env bash
# Build a Synology .spk package for syno-media-organizer (ARMv7).
# Run this script from WSL Ubuntu 24 inside the project root.
set -euo pipefail

TARGET="armv7-unknown-linux-gnueabihf"
PACKAGE="syno-media-organizer"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

# Read version from Cargo.toml
VERSION="$(grep '^version' "${ROOT}/Cargo.toml" | head -1 | sed 's/version = "\(.*\)"/\1/')"
SPK_NAME="${PACKAGE}-${VERSION}.spk"

echo "==> Building ${PACKAGE} v${VERSION} for ${TARGET}"

# 1. Cross-compile
cargo build --release --target "${TARGET}" --manifest-path "${ROOT}/Cargo.toml"

# 2. Strip the binary
BINARY="${ROOT}/target/${TARGET}/release/${PACKAGE}"
arm-linux-gnueabihf-strip "${BINARY}"
echo "    Binary size after strip: $(du -sh "${BINARY}" | cut -f1)"

# 3. Prepare SPK staging directory
STAGE="${ROOT}/spk/package"
rm -rf "${STAGE}"
mkdir -p "${STAGE}/bin"
cp "${BINARY}" "${STAGE}/bin/${PACKAGE}"

# 4. Copy SPK metadata and scripts
mkdir -p "${STAGE}/conf"
cp "${ROOT}/spk/conf/syno-media-organizer.toml.example" "${STAGE}/conf/"

# 5. Create the package tarball (content.tar.gz inside the .spk)
cd "${STAGE}"
tar czf "${ROOT}/spk/package.tgz" .
cd "${ROOT}"

# 6. Bundle into .spk (which is itself a tar archive)
mkdir -p "${ROOT}/dist"
SPK_PATH="${ROOT}/dist/${SPK_NAME}"

cd "${ROOT}/spk"
tar cf "${SPK_PATH}" \
    INFO \
    package.tgz \
    scripts/

rm -f "${ROOT}/spk/package.tgz"
rm -rf "${STAGE}"

# 7. Compute checksum
sha256sum "${SPK_PATH}" > "${SPK_PATH}.sha256"

echo ""
echo "==> Package ready:"
echo "    ${SPK_PATH}"
echo "    $(cat "${SPK_PATH}.sha256")"
