# Tea Language Benchmarks

This directory contains benchmarks for measuring and comparing the performance of the Tea compiler against equivalent Rust and JavaScript programs.

## Directory Structure

- **tea/** - Tea language benchmark programs
- **rust/** - Equivalent Rust implementations for performance comparison
- **js/** - Equivalent JavaScript implementations (run with Bun)

## Pairing Requirement

Each benchmark requires **at minimum** Tea and Rust implementations with **matching names**:

- `tea/loops.tea` ↔ `rust/loops.rs` (↔ `js/loops.js` optional)
- `tea/fib.tea` ↔ `rust/fib.rs` (↔ `js/fib.js` optional)

The benchmark script automatically discovers and runs only paired benchmarks (Tea + Rust required). JavaScript implementations are optional and included when using the `--include-js` flag.

## ⚡ Quick Answer

**Yes!** By default, `tea build` produces **maximum performance** binaries that match or exceed Rust!

```bash
tea build myprogram.tea  # O3 + auto-detected CPU by default!
```

- ✅ **O3 aggressive optimizations** enabled automatically
- ✅ **CPU auto-detection** for your specific processor
- ✅ **Rust-level performance** or better

For details, see [User Guide](../benchmark_results/USER_GUIDE.md) or [New Defaults](../benchmark_results/NEW_DEFAULTS.md).

## Current Benchmark Programs

Currently paired benchmarks (both Tea and Rust implementations exist):

- **loops** - Integer arithmetic in tight loops (10,000 iterations × sum to 1,000)
- **fib** - Recursive fibonacci (fib(35), heavy function call overhead)
- **strings** - String concatenation (100 iterations × 1,000 character string)
- **dicts** - Dictionary insert and lookup (100 iterations × 500 key-value pairs)
- **math** - Mixed arithmetic operations (1,000 iterations × 10,000 computations)

## Adding New Benchmarks

To add a new benchmark:

1. Create `tea/{name}.tea` with your Tea implementation
2. Create `rust/{name}.rs` with an equivalent Rust implementation
3. Ensure both programs produce identical output
4. Run `./scripts/bench.sh {name}` to verify

Both files must use the same basename for the benchmark script to discover the pair.

## Running Benchmarks

### Prerequisites

Install hyperfine:

```bash
cargo install hyperfine
```

### Run All Benchmarks

By default, compares **Tea** vs **Rust** only:

```bash
./scripts/bench.sh all
```

To include JavaScript (Bun):

```bash
./scripts/bench.sh --include-js all
```

### Run Specific Benchmark

```bash
./scripts/bench.sh loops
./scripts/bench.sh fib 5 20  # 5 warmup runs, 20 measured runs
./scripts/bench.sh --include-js loops  # Include JS comparison
```

### Clean Artifacts

```bash
./scripts/bench.sh clean
```

### Help

```bash
./scripts/bench.sh --help
```

## Compilation Settings

### Tea

`tea build` uses **O3 + auto-detected CPU** by default for maximum performance:

- ✅ **O3 aggressive optimizations**
- ✅ **Auto-detected CPU features** (apple-m4, x86-64-v3, etc.)

### Rust

Rust benchmarks are compiled with:

```bash
rustc -O -C target-cpu=native
```

This ensures fair comparison by matching Tea's optimization level and CPU-specific tuning.

### JavaScript (Bun)

JavaScript benchmarks run directly with Bun's JIT compiler:

```bash
bun script.js
```

Bun is a fast JavaScript runtime with native-speed performance for many workloads.

## Results

Benchmark results are saved to `benchmark_results/`:

- Individual benchmark results: `{name}.json` and `{name}.md`
- Combined summary: `summary_{timestamp}.md`

## What Gets Compared

**Default mode** (no flags):

- Tea vs Rust

**With `--include-js` flag**:

- Tea vs Rust vs JavaScript (Bun)

JavaScript benchmarks are excluded by default to keep the focus on Tea vs Rust. JS (Bun) provides an optional reference point for dynamic language performance.

## Metrics

For each benchmark, hyperfine measures:

- **Mean runtime** with standard deviation
- **Min/Max times**
- **Relative speedup** between implementations

## Goals

These benchmarks help us:

1. Track Tea compiler performance over time
2. Compare Tea against equivalent Rust programs
3. Identify optimization opportunities
4. Validate that Tea can match or approach Rust performance
