#!/usr/bin/env bash
# Build the Apple Intelligence sidecar (`dicto-apple-polish`) and drop it
# at `src-tauri/binaries/dicto-apple-polish-<target-triple>` so Tauri can
# pick it up as an `externalBin`.
#
# Tauri's bundler appends the host target triple to externalBin names so
# multi-arch builds can ship side-by-side. We match that convention even
# on dev so `cargo run` finds the binary at the same path as a packaged
# build.
#
# Requirements: macOS 26 SDK + Swift 6 (ships with Xcode 26 / its CLT).
# The sidecar imports `FoundationModels`, which is macOS-26-only — older
# SDKs will fail to build. CI must run this on a macOS 26 runner.

set -euo pipefail

cd "$(dirname "$0")/.."

SRC="swift/dicto-apple-polish.swift"
TARGETS=("${1:-aarch64-apple-darwin}")  # default to host arch
OUT_DIR="src-tauri/binaries"
mkdir -p "$OUT_DIR"

for TRIPLE in "${TARGETS[@]}"; do
    case "$TRIPLE" in
        aarch64-apple-darwin) SWIFT_ARCH="arm64" ;;
        x86_64-apple-darwin)  SWIFT_ARCH="x86_64" ;;
        *) echo "unsupported triple: $TRIPLE" >&2; exit 1 ;;
    esac

    OUT="$OUT_DIR/dicto-apple-polish-$TRIPLE"
    echo "building $OUT (arch=$SWIFT_ARCH)..."
    swiftc -O \
        -target "${SWIFT_ARCH}-apple-macos26.0" \
        "$SRC" \
        -o "$OUT"
    echo "  ok: $(file "$OUT")"
done
