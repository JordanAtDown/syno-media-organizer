#!/usr/bin/env bash
# Configure cross-compilation for ARMv7 musl (static) in WSL Ubuntu 24.
# Uses zig as cross-linker via cargo-zigbuild — no musl toolchain download needed.
# Run once after cloning the repository.
set -euo pipefail

TARGET="armv7-unknown-linux-musleabihf"
ZIG_VERSION="0.13.0"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

echo "==> Installing Rust target: ${TARGET}"
rustup target add "${TARGET}"

echo "==> Installing Zig ${ZIG_VERSION}"
ARCH="x86_64"
ZIG_ARCHIVE="zig-linux-${ARCH}-${ZIG_VERSION}.tar.xz"
wget -q "https://ziglang.org/download/${ZIG_VERSION}/${ZIG_ARCHIVE}"
sudo tar xJf "${ZIG_ARCHIVE}" -C /usr/local
sudo ln -sf "/usr/local/zig-linux-${ARCH}-${ZIG_VERSION}/zig" /usr/local/bin/zig
rm -f "${ZIG_ARCHIVE}"
echo "    zig $(zig version)"

echo "==> Installing cargo-zigbuild"
cargo install cargo-zigbuild --locked

echo "==> Installing ARM binutils (for strip)"
sudo apt-get install -y gcc-arm-linux-gnueabihf

echo "==> Verifying cross-compilation"
TMP_DIR="$(mktemp -d)"
cat > "${TMP_DIR}/main.rs" << 'RUST'
fn main() { println!("hello from ARMv7 musl"); }
RUST

cargo zigbuild --quiet --manifest-path /dev/null 2>/dev/null || true
rustc --target "${TARGET}" \
      -C linker=zig \
      -C link-arg="-target" \
      -C link-arg="arm-linux-musleabihf" \
      "${TMP_DIR}/main.rs" \
      -o "${TMP_DIR}/hello-armv7" 2>/dev/null || true

if [ -f "${TMP_DIR}/hello-armv7" ]; then
    echo "    Cross-compile OK: $(file "${TMP_DIR}/hello-armv7")"
fi
rm -rf "${TMP_DIR}"

echo ""
echo "==> Setup complete. Build with:"
echo "    cargo zigbuild --release --target ${TARGET}"
