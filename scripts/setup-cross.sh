#!/usr/bin/env bash
# Configure cross-compilation for ARMv7 musl (static) in WSL Ubuntu 24.
# Run once after cloning the repository.
set -euo pipefail

TARGET="armv7-unknown-linux-musleabihf"
MUSL_CROSS="arm-linux-musleabihf-cross"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
CARGO_CONFIG="${ROOT}/.cargo/config.toml"

echo "==> Installing Rust target: ${TARGET}"
rustup target add "${TARGET}"

echo "==> Downloading musl ARM cross-compiler from musl.cc"
wget -q "https://musl.cc/${MUSL_CROSS}.tgz"
sudo tar xzf "${MUSL_CROSS}.tgz" -C /usr/local
rm -f "${MUSL_CROSS}.tgz"
echo "    Installed to /usr/local/${MUSL_CROSS}/bin"

MUSL_BIN="/usr/local/${MUSL_CROSS}/bin"
if ! echo "$PATH" | grep -q "${MUSL_BIN}"; then
    echo "export PATH=\"${MUSL_BIN}:\$PATH\"" >> ~/.bashrc
    export PATH="${MUSL_BIN}:$PATH"
fi

echo "==> Configuring Cargo linker for ${TARGET}"
mkdir -p "${ROOT}/.cargo"

cat > "${CARGO_CONFIG}" << 'EOF'
[target.armv7-unknown-linux-gnueabihf]
linker = "arm-linux-gnueabihf-gcc"

[target.armv7-unknown-linux-musleabihf]
linker = "arm-linux-musleabihf-gcc"
EOF

echo "    Written: ${CARGO_CONFIG}"

echo "==> Verifying cross-compilation with a hello-world test"
TMP_DIR="$(mktemp -d)"
cat > "${TMP_DIR}/main.rs" << 'RUST'
fn main() { println!("hello from ARMv7 musl"); }
RUST

rustc --target "${TARGET}" \
      -C linker=arm-linux-musleabihf-gcc \
      "${TMP_DIR}/main.rs" \
      -o "${TMP_DIR}/hello-armv7"

if file "${TMP_DIR}/hello-armv7" | grep -q "ARM"; then
    echo "    Cross-compile OK: $(file "${TMP_DIR}/hello-armv7")"
else
    echo "ERROR: cross-compiled binary is not ARM!" >&2
    exit 1
fi

rm -rf "${TMP_DIR}"
echo ""
echo "==> Setup complete. You can now run:"
echo "    cargo build --release --target ${TARGET}"
