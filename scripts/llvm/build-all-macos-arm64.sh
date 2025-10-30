#!/bin/bash
# Build all vendored artifacts for macOS arm64
# Run this once to set up static LLVM embedding

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "========================================"
echo "Building Tea Vendor Artifacts"
echo "Platform: macOS arm64"
echo "========================================"
echo ""

# Step 1: Build LLVM + LLD
echo "Step 1/3: Building static LLVM 17 + LLD..."
echo "This may take 30-60 minutes on first build"
echo ""
"${SCRIPT_DIR}/build-macos-arm64.sh"
echo ""

# Step 2: Build runtime staticlib
echo "Step 2/3: Building tea-runtime staticlib..."
echo ""
"${SCRIPT_DIR}/build-runtime-staticlib.sh"
echo ""

# Step 3: Build entry stub
echo "Step 3/3: Building entry stub..."
echo ""
"${SCRIPT_DIR}/build-entry-stub.sh"
echo ""

# Summary
echo "========================================"
echo "Build Complete!"
echo "========================================"
echo ""
echo "Artifacts:"
VENDOR_DIR="$(cd "${SCRIPT_DIR}/../../tea-llvm-vendor" && pwd)"
echo "  LLVM libs:       ${VENDOR_DIR}/install-macos-arm64/lib/"
echo "  Runtime lib:     ${VENDOR_DIR}/runtime-artifacts-macos-arm64/libtea_runtime.a"
echo "  Entry stub:      ${VENDOR_DIR}/runtime-artifacts-macos-arm64/entry_stub.o"
echo ""
echo "Next steps:"
echo "  cargo build -p tea-cli --release --features tea-cli/llvm-aot"
echo "  ./target/release/tea build examples/language/basics/fib.tea"
echo ""
