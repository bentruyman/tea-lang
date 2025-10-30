# Building Vendor Artifacts - Complete Guide

## Why Build Vendor Artifacts?

When you build `tea-cli` without running the vendor build scripts, you get a binary that **appears to work** but has hidden dependencies:

### Without Vendor Artifacts (What Happens Now)

```
cargo build -p tea-cli --release
  ↓
inkwell finds your system LLVM (via llvm-config)
  ↓
Links against system LLVM dynamically or statically
  ↓
tea-cli binary (148MB)
  ↓
Works on YOUR machine
  ❌ Won't work on machines without LLVM installed
  ❌ Not truly zero-dependency
  ❌ Version inconsistencies (you might have LLVM 21, we target 17)
```

### With Vendor Artifacts (True Zero-Dependency)

```
./scripts/llvm/build-all-macos-arm64.sh
  ↓
Builds controlled LLVM 17.0.6 + runtime artifacts
  ↓
cargo build -p tea-cli --release --features tea-cli/llvm-aot
  ↓
Links against vendored LLVM (not system LLVM)
  ↓
tea-cli binary (155MB)
  ↓
✅ Works on ANY macOS arm64 machine
✅ TRUE zero-dependency
✅ Consistent LLVM version (17.0.6)
```

## Prerequisites

### Required Tools

Install via Homebrew:

```bash
brew install cmake ninja git
```

### Verify Installation

```bash
cmake --version   # Should be 3.20+
ninja --version   # Any version
git --version     # Any version
```

### Xcode Command Line Tools

```bash
xcode-select --install
```

## Building Artifacts

### Option 1: Using Make (Recommended)

**Platform-aware build using Make:**

```bash
make vendor
```

This automatically detects your platform and runs the appropriate build script. Currently supports:

- macOS arm64 ✓
- macOS x86_64 (coming soon)
- Linux x86_64/aarch64 (coming soon)

**Check if artifacts already exist:**

```bash
make vendor-check
```

### Option 2: Build Script Directly

**Run the build script for your platform:**

```bash
./scripts/llvm/build-all-macos-arm64.sh  # macOS arm64
```

This runs:

1. LLVM + LLD build (30-60 minutes)
2. tea-runtime staticlib (1-2 minutes)
3. Entry stub object (<1 minute)

**Expected output structure:**

```
tea-llvm-vendor/
├── llvm-project/                    # LLVM source (cloned once)
├── install-macos-arm64/
│   ├── lib/*.a                      # LLVM/LLD static libs (~60-80MB)
│   ├── include/                     # LLVM headers
│   ├── link-args.txt                # Library link order
│   └── metadata.json                # Build info
└── runtime-artifacts-macos-arm64/
    ├── libtea_runtime.a             # Tea runtime (~18MB)
    ├── entry_stub.o                 # Entry point (~4KB)
    └── entry_stub.c                 # Entry source
```

### Option 3: Build Individually

If you need to rebuild just one component:

**LLVM + LLD (30-60 minutes):**

```bash
./scripts/llvm/build-macos-arm64.sh
```

**tea-runtime staticlib (1-2 minutes):**

```bash
./scripts/llvm/build-runtime-staticlib.sh
```

**Entry stub (<1 minute):**

```bash
./scripts/llvm/build-entry-stub.sh
```

## After Building Artifacts

### Clean Build (Recommended)

To ensure tea-cli uses the vendored LLVM (not system LLVM):

```bash
cargo clean
cargo build -p tea-cli --release --features tea-cli/llvm-aot
```

### Verify Zero-Dependency

Check that the binary doesn't depend on system LLVM:

```bash
# Check dependencies
otool -L target/release/tea-cli | grep -i llvm
# Should output nothing

# Check size (should be ~155MB with vendored LLVM)
ls -lh target/release/tea-cli
```

### Test Compilation

```bash
./target/release/tea-cli build examples/language/basics/fib.tea
./bin/fib
```

## Troubleshooting

### "Missing required build tools"

**Error:**

```
ERROR: Missing required build tools: ninja
```

**Solution:**

```bash
brew install cmake ninja git
```

### "CMake was unable to find a build program"

You're missing ninja. Install it:

```bash
brew install ninja
```

### "CMAKE_C_COMPILER not set"

Xcode Command Line Tools not installed:

```bash
xcode-select --install
```

### Build Takes Too Long

The LLVM build is CPU-intensive and will take 30-60 minutes. This is normal.

**Tips to speed up:**

- Close other applications
- Ensure your Mac isn't throttling (plugged in, not overheating)
- The build is only needed once (cached afterwards)

### Already Have System LLVM

If you have LLVM from Homebrew (`brew install llvm`), tea-cli will use it by default **unless** you build the vendored version.

**To force vendored LLVM:**

1. Build vendor artifacts
2. `cargo clean` (important!)
3. Rebuild tea-cli

The `cargo clean` ensures cargo doesn't use cached builds linked against system LLVM.

### Verify Which LLVM Is Used

**Check if using vendored LLVM:**

```bash
# If vendored artifacts exist
ls tea-llvm-vendor/install-macos-arm64/lib/*.a

# Binary should be ~155MB (not ~148MB)
ls -lh target/release/tea-cli

# Should NOT depend on /opt/homebrew/opt/llvm
otool -L target/release/tea-cli | grep llvm
```

## Understanding the Build

### What Gets Built

**LLVM 17.0.6 + LLD:**

- Static libraries: `libLLVM*.a`, `liblld*.a`
- ~60-80 MB total
- Configuration:
  - Targets: AArch64, X86
  - No unnecessary dependencies
  - Optimized for size and performance

**tea-runtime staticlib:**

- Rust static library with all dependencies
- ~18 MB
- Includes: JSON, YAML, file system, process APIs

**Entry stub:**

- Minimal C object file
- Provides `main()` → calls `tea_main()`
- ~4 KB

### Build Configuration

The LLVM build uses these CMake flags:

```cmake
-DCMAKE_BUILD_TYPE=Release
-DBUILD_SHARED_LIBS=OFF              # Static only
-DLLVM_ENABLE_PROJECTS="lld"         # Include LLD
-DLLVM_TARGETS_TO_BUILD="AArch64;X86"  # Only needed targets
-DLLVM_ENABLE_ZLIB=OFF               # Minimize dependencies
-DLLVM_ENABLE_ZSTD=OFF
-DLLVM_ENABLE_TERMINFO=OFF
-DLLVM_ENABLE_LIBXML2=OFF
-DLLVM_OPTIMIZED_TABLEGEN=ON         # Faster builds
```

### Why It Takes So Long

LLVM is a massive project:

- ~2 million lines of C++ code
- Complex template-heavy code
- Multiple optimization passes
- Static linking requires full compilation

**First build:** 30-60 minutes (clean build)
**Subsequent builds:** Cached (unless you clean)

## CI/CD Considerations

For automated builds:

**Cache Strategy:**

```yaml
# Cache vendored LLVM by version
cache-key: llvm-17.0.6-macos-arm64-${{ hashFiles('scripts/llvm/build-macos-arm64.sh') }}
cache-path: tea-llvm-vendor/install-macos-arm64/
```

**Build Matrix:**

```yaml
strategy:
  matrix:
    os: [macos-14] # arm64 runner
steps:
  - name: Install build tools
    run: brew install cmake ninja

  - name: Build vendor artifacts (or restore cache)
    run: ./scripts/llvm/build-all-macos-arm64.sh

  - name: Build tea
    run: cargo build -p tea-cli --release --features tea-cli/llvm-aot
```

## Cleaning Up

### Remove Build Artifacts

To save disk space after building:

```bash
# Remove intermediate build files (keeps final libs)
rm -rf tea-llvm-vendor/build-macos-arm64/

# Remove LLVM source (keeps built libs)
rm -rf tea-llvm-vendor/llvm-project/

# Remove everything (requires full rebuild)
rm -rf tea-llvm-vendor/install-macos-arm64/
rm -rf tea-llvm-vendor/runtime-artifacts-macos-arm64/
```

**Recommendation:** Keep `install-macos-arm64/` and `runtime-artifacts-macos-arm64/`, delete the rest.

## Next Steps

After building vendor artifacts:

1. **Build tea-cli:**

   ```bash
   cargo build -p tea-cli --release --features tea-cli/llvm-aot
   ```

2. **Test compilation:**

   ```bash
   ./target/release/tea-cli build examples/language/basics/fib.tea
   ```

3. **Distribute:**
   The resulting binary is self-contained and can be copied to any macOS arm64 machine!

## References

- [Zero-Dependency Implementation](../explanation/zero-dependency-implementation.md)
- [Build Scripts README](../../scripts/llvm/README.md)
- [Static LLVM Embedding](../explanation/static-llvm-embedding.md)
