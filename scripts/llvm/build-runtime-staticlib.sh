#!/bin/bash
# Build tea-runtime as a staticlib for macOS arm64

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
VENDOR_DIR="${WORKSPACE_ROOT}/tea-llvm-vendor"
OUTPUT_DIR="${VENDOR_DIR}/runtime-artifacts-macos-arm64"

echo "==> Building tea-runtime staticlib for macOS arm64"

mkdir -p "${OUTPUT_DIR}"

cd "${WORKSPACE_ROOT}"

# Build tea-runtime as staticlib
echo "==> Running cargo build for tea-runtime..."
cargo build -p tea-runtime --release --target aarch64-apple-darwin

# Copy the staticlib to vendor directory
STATICLIB_SRC="${WORKSPACE_ROOT}/target/aarch64-apple-darwin/release/libtea_runtime.a"
STATICLIB_DST="${OUTPUT_DIR}/libtea_runtime.a"

if [ -f "${STATICLIB_SRC}" ]; then
  cp "${STATICLIB_SRC}" "${STATICLIB_DST}"
  echo "==> Runtime staticlib copied to: ${STATICLIB_DST}"
  ls -lh "${STATICLIB_DST}"
else
  echo "ERROR: tea-runtime staticlib not found at ${STATICLIB_SRC}"
  exit 1
fi

echo ""
echo "==> Complete!"
echo "    Runtime staticlib: ${STATICLIB_DST}"
