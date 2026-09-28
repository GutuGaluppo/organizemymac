#!/bin/sh
# Builds the Swift helper as a Tauri sidecar (binaries/organize-helper-<target triple>).
set -e
cd "$(dirname "$0")"
TRIPLE="${TAURI_ENV_TARGET_TRIPLE:-$(rustc -vV | sed -n 's/^host: //p')}"
case "$TRIPLE" in
  aarch64-apple-darwin) ARCH=arm64 ;;
  x86_64-apple-darwin) ARCH=x86_64 ;;
  *) echo "unsupported target $TRIPLE" >&2; exit 1 ;;
esac
OUT="../../src-tauri/binaries/organize-helper-$TRIPLE"
mkdir -p "$(dirname "$OUT")"
xcrun swiftc -O -swift-version 5 -target "$ARCH-apple-macos13.0" Sources/main.swift -o "$OUT"
echo "built $OUT"
