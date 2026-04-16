#!/usr/bin/env bash
#
# Build fully-static nctui binaries for Linux using musl.
# By default this compiles libnetcdf and libhdf5 from source via the
# Cargo "static" feature, producing a self-contained binary with no
# runtime dependencies.
#
# Usage:
#   ./scripts/build-static.sh              # build for host architecture
#   ./scripts/build-static.sh x86_64       # build for x86_64
#   ./scripts/build-static.sh aarch64      # build for aarch64 (arm64)
#   ./scripts/build-static.sh all          # build both
#
# Prerequisites (Debian/Ubuntu):
#   x86_64  — rustup target add x86_64-unknown-linux-musl
#             sudo apt-get install musl-tools cmake g++ m4
#   aarch64 — rustup target add aarch64-unknown-linux-musl
#             sudo apt-get install gcc-aarch64-linux-gnu g++-aarch64-linux-gnu cmake m4
#
# To build without the NetCDF/HDF5 backend (TUI widgets only):
#   cargo build --release --target <triple> --no-default-features
#
# The resulting binaries land in target/<triple>/release/nctui.

set -euo pipefail

build_target() {
    local triple="$1"
    echo "--- Building static binary for ${triple} (with bundled NetCDF/HDF5) ---"
    cargo build --release --target "${triple}" --features static

    local bin="target/${triple}/release/nctui"
    if [ -f "${bin}" ]; then
        local size
        size=$(stat --printf='%s' "${bin}" 2>/dev/null || stat -f '%z' "${bin}" 2>/dev/null)
        echo "OK  ${bin}  (${size} bytes — $(file "${bin}" | cut -d: -f2-))"
    else
        echo "ERROR: expected binary not found at ${bin}" >&2
        exit 1
    fi
}

ARCH="${1:-$(uname -m)}"

case "${ARCH}" in
    x86_64)
        build_target "x86_64-unknown-linux-musl"
        ;;
    aarch64|arm64)
        build_target "aarch64-unknown-linux-musl"
        ;;
    all)
        build_target "x86_64-unknown-linux-musl"
        build_target "aarch64-unknown-linux-musl"
        ;;
    *)
        echo "Usage: $0 [x86_64|aarch64|all]" >&2
        exit 1
        ;;
esac
