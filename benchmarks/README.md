# Tea Language Benchmarks

This directory contains benchmarks for measuring and comparing the performance of the Tea AOT compiler against the VM interpreter.

## ⚡ Quick Answer

**Yes!** By default, `tea build` produces **maximum performance** binaries that match or exceed Rust!

```bash
tea build myprogram.tea  # O3 + auto-detected CPU by default!
```

- ✅ **O3 aggressive optimizations** enabled automatically
- ✅ **CPU auto-detection** for your specific processor
- ✅ **Rust-level performance** or better

For details, see [User Guide](../benchmark_results/USER_GUIDE.md) or [New Defaults](../benchmark_results/NEW_DEFAULTS.md).

## Benchmark Programs

- **loops.tea** - Integer arithmetic in tight loops (10,000 iterations × sum to 1,000)
- **fib.tea** - Recursive fibonacci (fib(35), heavy function call overhead)
- **strings.tea** - String concatenation (100 iterations × 1,000 character string)
- **lists.tea** - List operations: build, append, and sum (100 iterations × 1,000 elements)
- **dicts.tea** - Dictionary insert and lookup (100 iterations × 500 key-value pairs)
- **closures.tea** - Closure allocation and invocation (1,000 closures × 1,000 calls each)
- **structs.tea** - Struct allocation and field access (100 iterations × 1,000 points)

## Reference Implementations

For some benchmarks, equivalent Rust implementations are provided in `reference_*.rs` files to establish an aspirational performance baseline.

## Running Benchmarks

### Prerequisites

Install hyperfine:

```bash
cargo install hyperfine
```

### Run All Benchmarks

```bash
./scripts/bench.sh all
```

This will:

1. Build each benchmark with multiple optimization configurations:
   - O2 with generic CPU
   - O3 with generic CPU
   - O3 with native CPU features
2. Build Rust reference implementations (where available)
3. Run each benchmark comparing:
   - Tea AOT binaries (different opt levels)
   - Rust -O reference
   - Tea VM (bytecode interpreter)
4. Generate results in `benchmark_results/`

### Run Specific Benchmark

```bash
./scripts/bench.sh loops
./scripts/bench.sh fib 5 20  # 5 warmup runs, 20 measured runs
```

### Clean Artifacts

```bash
./scripts/bench.sh clean
```

## Optimization Configurations

**Note**: `tea build` now uses **O3 + auto-detected CPU** by default, giving you maximum performance out of the box!

The benchmark script tests different AOT configurations:

1. **Default**: `tea build` (no flags)
   - ✅ **O3 aggressive optimizations**
   - ✅ **Auto-detected CPU** (apple-m4, x86-64-v3, etc.)
   - ✅ **Maximum performance by default!**

2. **Portable**: `--cpu generic`
   - O3 aggressive optimizations
   - Generic CPU features for cross-platform distribution
   - Slightly slower than auto-detected but works everywhere

3. **Fast compilation**: `--opt-level 1` or `--opt-level 2`
   - Lower optimization levels
   - Faster compilation during development
   - Good but not maximum performance

## Results

Benchmark results are saved to `benchmark_results/`:

- Individual benchmark results: `{name}.json` and `{name}.md`
- Combined summary: `summary_{timestamp}.md`

## Metrics

For each configuration, we measure:

- **Mean runtime** (milliseconds)
- **Standard deviation**
- **Min/Max times**
- **Relative speedup** compared to baseline

## Goals

The benchmarks help us:

1. Track AOT compiler performance over time
2. Identify optimization opportunities
3. Validate that optimizations improve performance
4. Compare Tea performance against native code
5. Understand the overhead of different language features
