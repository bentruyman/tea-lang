# Static LLVM Embedding

This document explains how Tea embeds LLVM and LLD statically to eliminate third-party dependencies for compiling native binaries.

## Goal

Enable users to download a SINGLE `tea` binary that can compile Tea source code to native executables without requiring:

- LLVM installation
- Clang installation
- Rustc at runtime
- Any third-party toolchains

## Architecture

### Components

1. **tea-llvm-vendor** - Vendored static LLVM 17 + LLD libraries
   - Built per-platform with minimal configuration
   - Linked statically into tea-compiler
   - Includes Mach-O/ELF/COFF LLD drivers

2. **Runtime Artifacts** - Prebuilt linking components
   - `libtea_runtime.a` - Tea runtime as static library
   - `entry_stub.o` - Minimal entry point calling `tea_main()`

3. **In-Process Optimization** - TargetMachine optimization
   - No external `opt` tool needed
   - O3 optimizations performed during code generation
   - Configured for maximum performance by default
   - Note: LLVM 17 removed legacy pass manager; optimizations now handled by TargetMachine

4. **In-Process Linking** - LLD integration (planned)
   - Replaces rustc/clang linking
   - Calls LLD driver directly in-process
   - Produces final Mach-O/ELF/PE executable

## Build Process

### Building Vendored Libraries

```bash
# Build LLVM + LLD static libraries (30-60 minutes)
./scripts/llvm/build-macos-arm64.sh

# Build runtime staticlib
./scripts/llvm/build-runtime-staticlib.sh

# Build entry stub
./scripts/llvm/build-entry-stub.sh
```

### Compilation Pipeline

When a user runs `tea build example.tea`:

1. **Parse & Type Check** - Frontend produces typed AST
2. **LLVM IR Generation** - Lower AST to LLVM IR
3. **Code Generation & Optimization** - TargetMachine emits optimized native object file
   - O3 optimization level applied during code generation
   - Includes: inlining, vectorization, dead code elimination, constant propagation
   - No external `opt` tool required (LLVM 17+ uses integrated optimization)
4. **Linking** - Combine:
   - User module object
   - `entry_stub.o`
   - `libtea_runtime.a`
   - System libraries (libSystem on macOS)
5. **Output** - Native executable

## Platform Support

### macOS arm64 (Initial Target)

- Target: `aarch64-apple-darwin`
- Min OS: macOS 15.0
- CPU: `native` (optimized for build machine)
- System deps: libSystem (always available)
- LLD driver: Mach-O

### Future Platforms

- macOS x86_64
- Linux x86_64/aarch64 (musl for full static)
- Windows x86_64/aarch64

## Optimization Strategy

### Default Settings

- **Optimization**: O3 (Aggressive)
- **CPU**: generic (arm64) / x86-64 (x86_64) - enables standard features for broad compatibility
- **Relocation**: PIC (Position Independent Code)
- **Vectorization**: Enabled (NEON on arm64, SSE/AVX on x86_64)

### Optimization Implementation

Tea uses LLVM's TargetMachine optimization during code generation. When the optimization level is set to `Aggressive` (O3), LLVM performs:

**IR-level optimizations:**

- Function inlining
- Dead code elimination
- Constant propagation and folding
- Loop optimizations (unrolling, vectorization)
- GVN (Global Value Numbering)
- Memory to register promotion

**Backend optimizations:**

- Instruction selection
- Register allocation
- Instruction scheduling
- Peephole optimizations
- Auto-vectorization (NEON on arm64, SSE/AVX on x86)

This approach provides equivalent optimization to running a separate `opt` pass while eliminating the external tool dependency.

## Binary Size

Expected `tea` binary size by platform:

- macOS arm64: 80-120 MB
  - LLVM/LLD libs: ~60-80 MB
  - tea-compiler: ~10-20 MB
  - Runtime artifacts: ~5-10 MB
  - Rust stdlib: ~5-10 MB

Mitigation strategies:

- Strip debug symbols in release builds
- LTO for tea crate itself
- Compress with UPX (optional)

## Licensing

### LLVM

LLVM and LLD are licensed under **Apache-2.0 with LLVM Exception**.

The LLVM exception allows embedding portions of LLVM into compiled binaries without requiring Apache-2.0 compliance for object forms.

Key points:

- Static linking is permitted
- No source redistribution required
- Must include LLVM NOTICE in about/credits

License: https://llvm.org/LICENSE.txt

### Tea

Tea is licensed under MIT.

## Development

### Building Tea with Vendored LLVM

```bash
# Ensure vendor artifacts are built
ls tea-llvm-vendor/install-macos-arm64/lib/*.a
ls tea-llvm-vendor/runtime-artifacts-macos-arm64/*.{a,o}

# Build tea with llvm-aot feature
cargo build -p tea-cli --release --features tea-cli/llvm-aot

# The resulting binary includes everything needed
./target/release/tea build examples/language/basics/fib.tea
./bin/fib
```

### CI/CD

See `.github/workflows/` for automated builds that:

1. Cache LLVM build (keyed by version)
2. Build runtime artifacts
3. Compile tea with vendored LLVM
4. Test end-to-end compilation
5. Publish platform-specific binaries

## References

- [AOT Backend](./aot-backend.md)
- [LLVM Build Scripts](../../scripts/llvm/README.md)
- [tea-llvm-vendor](../../tea-llvm-vendor/)
