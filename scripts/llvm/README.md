# LLVM Vendor Build Scripts

These scripts build static LLVM, LLD, and runtime artifacts for embedding into the tea compiler.

## macOS arm64 Build

### Prerequisites

**Required:**

- macOS 15+ on Apple Silicon
- Xcode Command Line Tools (`xcode-select --install`)
- CMake 3.20+
- Ninja build system
- Git

**Install via Homebrew:**

```bash
brew install cmake ninja git
```

### Build All Artifacts

**Using Make (recommended):**

```bash
# Platform-aware build
make vendor

# Check if artifacts exist
make vendor-check
```

**Or run the script directly:**

```bash
# Builds LLVM, runtime, and entry stub (30-60 minutes)
./scripts/llvm/build-all-macos-arm64.sh
```

**Or build individually:**

```bash
# 1. Build static LLVM + LLD (takes ~30-60 minutes)
./scripts/llvm/build-macos-arm64.sh

# 2. Build tea-runtime staticlib (1-2 minutes)
./scripts/llvm/build-runtime-staticlib.sh

# 3. Build entry stub object (<1 minute)
./scripts/llvm/build-entry-stub.sh
```

### Output Structure

After building, you'll have:

```
tea-llvm-vendor/
├── install-macos-arm64/          # LLVM + LLD static libs
│   ├── lib/                       # .a files
│   ├── include/                   # LLVM headers
│   ├── link-args.txt              # Library link order
│   └── metadata.json              # Build metadata
└── runtime-artifacts-macos-arm64/ # Runtime artifacts
    ├── libtea_runtime.a           # Tea runtime staticlib
    ├── entry_stub.o               # Entry point object
    └── entry_stub.c               # Entry point source
```

## CI/CD

For automated builds, run all three scripts in sequence. Cache the `install-macos-arm64` directory keyed by LLVM version to avoid rebuilding.

## Troubleshooting

### LLVM build fails

- Ensure you have at least 8GB RAM and 20GB disk space
- Check CMake and Ninja versions
- Verify Xcode Command Line Tools are installed: `xcode-select -p`

### Staticlib not found

- Run `cargo clean` and rebuild tea-runtime
- Verify target `aarch64-apple-darwin` is installed: `rustup target add aarch64-apple-darwin`

### Entry stub compilation fails

- Ensure clang is available: `clang --version`
- Check macOS SDK is present: `xcrun --show-sdk-path`
