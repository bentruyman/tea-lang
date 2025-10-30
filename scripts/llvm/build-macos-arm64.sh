#!/bin/bash
# Build static LLVM 17 + LLD for macOS arm64
# Output: vendored static libraries for embedding in tea

set -euo pipefail

LLVM_VERSION="17.0.6"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
VENDOR_DIR="${WORKSPACE_ROOT}/tea-llvm-vendor"
BUILD_DIR="${VENDOR_DIR}/build-macos-arm64"
INSTALL_DIR="${VENDOR_DIR}/install-macos-arm64"
SRC_DIR="${VENDOR_DIR}/llvm-project"

echo "==> Building LLVM ${LLVM_VERSION} static libs for macOS arm64"
echo "    Vendor dir: ${VENDOR_DIR}"
echo "    Build dir:  ${BUILD_DIR}"
echo "    Install to: ${INSTALL_DIR}"
echo ""

# Check prerequisites
echo "==> Checking prerequisites..."
MISSING_TOOLS=()

if ! command -v cmake &> /dev/null; then
    MISSING_TOOLS+=("cmake")
fi

if ! command -v ninja &> /dev/null; then
    MISSING_TOOLS+=("ninja")
fi

if ! command -v git &> /dev/null; then
    MISSING_TOOLS+=("git")
fi

if [ ${#MISSING_TOOLS[@]} -gt 0 ]; then
    echo "ERROR: Missing required build tools: ${MISSING_TOOLS[*]}"
    echo ""
    echo "Install with Homebrew:"
    echo "  brew install ${MISSING_TOOLS[*]}"
    echo ""
    echo "Or install all at once:"
    echo "  brew install cmake ninja git"
    exit 1
fi

echo "✓ All prerequisites found"
echo ""

# Clone LLVM if not present
if [ ! -d "${SRC_DIR}" ]; then
  echo "==> Cloning LLVM project..."
  git clone --depth 1 --branch "llvmorg-${LLVM_VERSION}" \
    https://github.com/llvm/llvm-project.git "${SRC_DIR}"
else
  echo "==> Using existing LLVM source at ${SRC_DIR}"
fi

# Create build directory
rm -rf "${BUILD_DIR}"
mkdir -p "${BUILD_DIR}"
cd "${BUILD_DIR}"

# CMake configuration for minimal static LLVM+LLD
echo "==> Configuring CMake..."
cmake -G Ninja "${SRC_DIR}/llvm" \
  -DCMAKE_BUILD_TYPE=Release \
  -DCMAKE_INSTALL_PREFIX="${INSTALL_DIR}" \
  -DCMAKE_OSX_DEPLOYMENT_TARGET=11.0 \
  -DCMAKE_OSX_ARCHITECTURES=arm64 \
  -DBUILD_SHARED_LIBS=OFF \
  -DLLVM_ENABLE_PROJECTS="lld" \
  -DLLVM_TARGETS_TO_BUILD="AArch64;X86" \
  -DLLVM_INCLUDE_TESTS=OFF \
  -DLLVM_INCLUDE_EXAMPLES=OFF \
  -DLLVM_INCLUDE_DOCS=OFF \
  -DLLVM_INCLUDE_BENCHMARKS=OFF \
  -DLLVM_ENABLE_BINDINGS=OFF \
  -DLLVM_ENABLE_ZLIB=OFF \
  -DLLVM_ENABLE_ZSTD=OFF \
  -DLLVM_ENABLE_TERMINFO=OFF \
  -DLLVM_ENABLE_LIBXML2=OFF \
  -DLLVM_ENABLE_LIBEDIT=OFF \
  -DLLVM_ENABLE_LIBPFM=OFF \
  -DLLVM_ENABLE_RTTI=ON \
  -DLLVM_ENABLE_EH=ON \
  -DLLVM_OPTIMIZED_TABLEGEN=ON \
  -DLLVM_BUILD_TOOLS=ON \
  -DLLVM_INSTALL_UTILS=OFF

# Build LLVM libraries and LLD
echo "==> Building LLVM and LLD..."
ninja

# Install to vendor directory
echo "==> Installing to ${INSTALL_DIR}..."
ninja install

# Generate link-args file with proper library order
echo "==> Generating link-args.txt..."
LINK_ARGS_FILE="${INSTALL_DIR}/link-args.txt"
cat > "${LINK_ARGS_FILE}" << 'EOF'
# LLVM/LLD static library link order for macOS arm64
# Pass these to rustc in order via cargo:rustc-link-lib=static=<name>

# LLD libraries (must come first)
lldMachO
lldCommon

# LLVM core libraries (in dependency order)
LLVMAArch64CodeGen
LLVMAArch64AsmParser
LLVMAArch64Desc
LLVMAArch64Info
LLVMAArch64Utils
LLVMX86CodeGen
LLVMX86AsmParser
LLVMX86Desc
LLVMX86Info
LLVMAsmPrinter
LLVMDebugInfoDWARF
LLVMGlobalISel
LLVMSelectionDAG
LLVMCodeGen
LLVMScalarOpts
LLVMInstCombine
LLVMAggressiveInstCombine
LLVMTransformUtils
LLVMBitWriter
LLVMAnalysis
LLVMProfileData
LLVMObject
LLVMMCParser
LLVMMCDisassembler
LLVMMC
LLVMDebugInfoCodeView
LLVMDebugInfoMSF
LLVMBitReader
LLVMCore
LLVMRemarks
LLVMBitstreamReader
LLVMBinaryFormat
LLVMSupport
LLVMDemangle

# System libraries (dynamic, already available)
# -lc++
# -lz
EOF

echo "==> Link args written to ${LINK_ARGS_FILE}"

# Create a metadata file
METADATA_FILE="${INSTALL_DIR}/metadata.json"
cat > "${METADATA_FILE}" << EOF
{
  "llvm_version": "${LLVM_VERSION}",
  "target": "aarch64-apple-darwin",
  "build_type": "Release",
  "static": true,
  "targets": ["AArch64", "X86"],
  "built_at": "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
}
EOF

echo "==> Metadata written to ${METADATA_FILE}"

# Print summary
echo ""
echo "==> Build complete!"
echo "    LLVM version: ${LLVM_VERSION}"
echo "    Install dir:  ${INSTALL_DIR}"
echo "    Libraries:    ${INSTALL_DIR}/lib"
echo "    Headers:      ${INSTALL_DIR}/include"
echo "    Link args:    ${LINK_ARGS_FILE}"
echo ""
echo "Next steps:"
echo "  1. Update tea-llvm-vendor/Cargo.toml to reference these libs"
echo "  2. Run 'cargo build -p tea-llvm-vendor' to test linking"
