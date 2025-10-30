#!/bin/bash
# Build entry_stub.o for macOS arm64
# This is a tiny object that provides main() and calls tea_main()

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
VENDOR_DIR="${WORKSPACE_ROOT}/tea-llvm-vendor"
OUTPUT_DIR="${VENDOR_DIR}/runtime-artifacts-macos-arm64"
STUB_C="${OUTPUT_DIR}/entry_stub.c"
STUB_O="${OUTPUT_DIR}/entry_stub.o"

echo "==> Building entry stub for macOS arm64"

mkdir -p "${OUTPUT_DIR}"

# Create the minimal entry stub
cat > "${STUB_C}" << 'EOF'
// Minimal entry stub for tea executables
// Links against tea_main from compiled tea code

extern int tea_main(void);

int main(int argc, char** argv) {
    (void)argc;
    (void)argv;
    return tea_main();
}
EOF

# Compile to object file
echo "==> Compiling entry_stub.c -> entry_stub.o"
clang -c "${STUB_C}" -o "${STUB_O}" \
  -arch arm64 \
  -mmacosx-version-min=15.0 \
  -O2 \
  -fPIC

# Verify
if [ -f "${STUB_O}" ]; then
  echo "==> Entry stub built: ${STUB_O}"
  file "${STUB_O}"
else
  echo "ERROR: Failed to build entry stub"
  exit 1
fi

echo ""
echo "==> Complete!"
echo "    Entry stub: ${STUB_O}"
