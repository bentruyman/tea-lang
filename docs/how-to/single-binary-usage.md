# Using Tea Single Binary Distribution

If you've downloaded a pre-built `tea` binary, you have everything needed to compile Tea programs to native executables - no additional dependencies required!

## Quick Verification

```bash
# Verify tea is working
./tea --version

# Run a Tea script
echo 'use debug = "std.debug"
debug.print("Hello from Tea!")' > hello.tea

./tea hello.tea
```

## Building Native Executables

The single `tea` binary includes:

- LLVM 17 compiler
- LLD linker
- Tea runtime
- Standard library

No third-party tools needed!

```bash
# Create a program
cat > fib.tea << 'EOF'
use debug = "std.debug"

def fib(n: Int) -> Int
  if n <= 1
    n
  else
    fib(n - 1) + fib(n - 2)
  end
end

debug.print("fib(10) = ${fib(10)}")
EOF

# Compile to native binary
./tea build fib.tea

# Run the compiled program
./bin/fib
# Output: fib(10) = 55
```

## What's Different from Traditional Compilers?

### Traditional Setup

```
User installs: LLVM → Clang → Language compiler
                ↓       ↓           ↓
           (500MB)  (300MB)     (100MB)
                ↓       ↓           ↓
           Multiple tools to manage and update
```

### Tea Single Binary

```
User downloads: tea (100MB)
                  ↓
    Everything included statically
                  ↓
       One file, zero configuration
```

## Command Reference

### tea (Run Script)

```bash
# Execute a Tea script on the VM
./tea script.tea [args...]

# Pass arguments to your script
./tea script.tea arg1 arg2
```

### tea build (Compile to Native)

```bash
# Compile to native executable (defaults to bin/<name>)
./tea build program.tea

# Specify output path
./tea build program.tea -o my-program

# View additional compiler output
./tea build program.tea --emit llvm-ir  # Print LLVM IR
```

### tea fmt (Format Code)

```bash
# Format all .tea files in current directory
./tea fmt

# Format specific files
./tea fmt src/main.tea lib/utils.tea

# Check formatting without modifying
./tea fmt --check
```

### tea test (Run Tests)

```bash
# Run tests in tests/ directory
./tea test

# Filter tests by name
./tea test --filter "http"

# Update snapshots
./tea test --update-snapshots
```

## Performance

Tea produces highly optimized native code:

- **Optimization level**: O3 (aggressive)
- **CPU targeting**: Native (optimized for your machine)
- **Vectorization**: Loop and SLP vectorization enabled
- **Inlining**: Aggressive function inlining
- **Dead code elimination**: Full interprocedural optimization

Compiled binaries are typically as fast as equivalent C/C++ code.

## Binary Size

Your compiled Tea programs will be:

- **Small programs**: 200KB - 1MB (mostly runtime overhead)
- **Medium programs**: 1MB - 5MB
- **Large programs**: 5MB - 20MB

The Tea runtime (`libtea_runtime.a`) provides:

- Memory management
- String operations
- Collection types (List, Dict)
- JSON/YAML parsing
- File system operations
- Process spawning

## System Requirements

### macOS

- macOS 15.0 or later
- Apple Silicon (arm64) or Intel (x86_64)
- No additional dependencies

### Linux (coming soon)

- Any modern Linux (glibc 2.28+ or musl)
- x86_64 or aarch64
- Fully static binary (zero dependencies)

### Windows (coming soon)

- Windows 10 or later
- x86_64 or aarch64
- MSVC runtime included

## Troubleshooting

### "tea: command not found"

Add tea to your PATH or use the full path:

```bash
# Option 1: Add to PATH
export PATH="$PATH:/path/to/tea/directory"

# Option 2: Move to a directory in PATH
sudo mv tea /usr/local/bin/

# Option 3: Use full path
/path/to/tea build program.tea
```

### "Permission denied"

Make sure tea is executable:

```bash
chmod +x tea
./tea --version
```

### Compiled binary doesn't run

The compiled binary should work on any machine with the same OS/architecture:

```bash
# Check your architecture
uname -m
# arm64 (Apple Silicon) or x86_64 (Intel)

# Check macOS version
sw_vers
# ProductVersion should be 15.0+
```

If you're distributing your compiled Tea program to other machines, ensure they match the architecture the binary was compiled for.

## Next Steps

- [Language Tutorial](../README.md)
- [Standard Library Reference](../roadmap/cli-stdlib.md)
- [Examples](../../examples/)

## Building Tea from Source

Want to build Tea yourself with vendored LLVM? See [Static LLVM Embedding](../explanation/static-llvm-embedding.md).
