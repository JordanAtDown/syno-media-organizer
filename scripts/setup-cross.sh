#!/usr/bin/env bash
# Configure cross-compilation for ARMv7 in WSL Ubuntu 24.
# Run once after cloning the repository.
set -euo pipefail

TARGET="armv7-unknown-linux-gnueabihf"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
CARGO_CONFIG="${ROOT}/.cargo/config.toml"

echo "==> Installing Rust target: ${TARGET}"
rustup target add "${TARGET}"

echo "==> Installing ARM cross-compiler (gcc-arm-linux-gnueabihf)"
sudo apt-get update -qq
sudo apt-get install -y gcc-arm-linux-gnueabihf

echo "==> Configuring Cargo linker for ${TARGET}"
mkdir -p "${ROOT}/.cargo"

cat > "${CARGO_CONFIG}" << 'EOF'
[target.armv7-unknown-linux-gnueabihf]
linker = "arm-linux-gnueabihf-gcc"
EOF

echo "    Written: ${CARGO_CONFIG}"

echo "==> Verifying cross-compilation with a hello-world test"
TMP_DIR="$(mktemp -d)"
cat > "${TMP_DIR}/main.rs" << 'RUST'
fn main() { println!("hello from ARMv7"); }
RUST

rustc --target "${TARGET}" \
      -C linker=arm-linux-gnueabihf-gcc \
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
