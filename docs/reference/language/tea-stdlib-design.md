# Tea Standard Library Design

## Overview

The Tea standard library is being migrated from Rust implementations to Tea code built on native intrinsics. This design provides:

1. **Easier maintenance**: Stdlib written in Tea, not Rust
2. **Better documentation**: Docstrings in Tea become the source of truth
3. **Flexibility**: Easier to add high-level helpers without touching Rust
4. **Embedded distribution**: Stdlib compiled and embedded in binaries

## Architecture

```
┌─────────────────────────────────────────┐
│ User Code (my_app.tea)                  │
│   use util = "std.util"                 │
│   util.is_string("hello")               │
└─────────────────┬───────────────────────┘
                  │
                  │ imports std.util
                  ▼
┌─────────────────────────────────────────┐
│ stdlib/util/mod.tea                     │
│   fn is_string(value)                   │
│     __intrinsic_is_string(value)        │
│   end                                   │
└─────────────────┬───────────────────────┘
                  │
                  │ calls intrinsic
                  ▼
┌─────────────────────────────────────────┐
│ VM Intrinsic Handler (Rust)             │
│   Intrinsic::IsString => {              │
│     // existing type check impl         │
│   }                                     │
└─────────────────────────────────────────┘
```

## Components

### 1. Native Intrinsics (`tea-compiler/src/runtime/intrinsics.rs`)

Functions prefixed with `__intrinsic_` that provide low-level OS/runtime functionality:

- **Type predicates**: `__intrinsic_is_string()`, etc.
- **Conversions**: `__intrinsic_to_string()`
- **I/O**: `__intrinsic_fs_read_text()`, `__intrinsic_io_write()`, etc.
- **Process**: `__intrinsic_process_run()`, etc.
- **Codecs**: `__intrinsic_json_encode()`, etc.

### 2. Tea Stdlib Modules (`stdlib/*/mod.tea`)

Tea code that wraps intrinsics and provides ergonomic APIs:

- `stdlib/util/mod.tea` - Type inspection utilities
- `stdlib/assert/mod.tea` - Testing assertions
- `stdlib/env/mod.tea` - Environment variables
- `stdlib/fs/mod.tea` - Filesystem operations
- `stdlib/path/mod.tea` - Path manipulation
- `stdlib/io/mod.tea` - Standard I/O
- `stdlib/cli/mod.tea` - CLI parsing
- `stdlib/process/mod.tea` - Process spawning
- `stdlib/json/mod.tea` - JSON codec
- `stdlib/yaml/mod.tea` - YAML codec

### 3. Snapshot Format (`tea-compiler/src/stdlib_snapshot.rs`)

Compiled stdlib packaged for embedding:

```rust
pub struct Snapshot {
    pub version: String,
    pub modules: HashMap<String, SnapshotModule>,
}

pub struct SnapshotModule {
    pub path: String,           // "std.util"
    pub bytecode: Vec<u8>,      // compiled Tea code
    pub exports: Vec<Export>,   // function signatures
}

pub struct Export {
    pub name: String,
    pub arity: usize,
    pub variadic: bool,
    pub doc: Option<String>,
}
```

### 4. Build Process (`tea-compiler/build.rs`)

1. Compile each `stdlib/*/mod.tea` to bytecode
2. Extract exported function signatures
3. Serialize to JSON snapshot
4. Generate `src/embedded_stdlib.rs`:
   ```rust
   pub const EMBEDDED_STDLIB: &[u8] = include_bytes!("../target/stdlib.snapshot");
   ```

### 5. Module Resolution

Order of precedence for `use util = "std.util"`:

1. **Embedded snapshot** (if module path starts with `std.`)
2. **User modules** (relative or absolute paths)
3. **Disk override** (if `TEA_STDLIB_PATH` env var set)

### 6. Typechecking

Export signatures from snapshot provide type information:

```tea
# In stdlib/util/mod.tea
fn is_string(value)  # Exports: is_string/1
fn len(value)        # Exports: len/1
```

Typechecker reads exports from snapshot → knows `is_string` takes 1 arg.

## Migration Path

### Phase 1: Foundation ✅

- [x] Create stdlib/ directory structure
- [x] Define intrinsics enum
- [x] Create snapshot format
- [x] Port std.util and std.assert

### Phase 2: Build Infrastructure 🚧

- [ ] Implement build.rs compilation
- [ ] Generate and embed snapshot
- [ ] Support TEA_STDLIB_PATH override

### Phase 3: Runtime Integration

- [ ] Wire intrinsics to VM
- [ ] Update resolver for snapshot loading
- [ ] Feature flag: `tea-stdlib` vs `rust-stdlib`

### Phase 4: Complete Port

- [ ] Port all 10 stdlib modules
- [ ] Test parity with existing tests
- [ ] Benchmark performance

### Phase 5: Rollout

- [ ] Default to Tea stdlib
- [ ] Remove Rust stdlib (deprecated)
- [ ] Update all documentation

## Design Decisions

### Why Intrinsics Instead of FFI?

- **Simpler**: No ABI, marshaling, or dynamic loading
- **Faster**: Direct VM dispatch, no overhead
- **Type-safe**: Intrinsics validated at compile-time
- **Familiar**: Similar to Lua/Ruby/Python built-ins

### Why Snapshot Instead of JIT Compilation?

- **Fast startup**: No parsing/compilation at runtime
- **Deterministic**: Same bytecode across platforms
- **Small size**: Bytecode smaller than source
- **Secure**: No runtime code generation

### Why JSON for Snapshot Format?

- **Initial simplicity**: Easy debugging and inspection
- **Human-readable**: Can inspect snapshot contents
- **Future migration**: Can switch to bincode/CBOR/MessagePack later
- **Good enough**: ~100KB uncompressed for full stdlib

### Why Embed Instead of Install?

- **Zero config**: Programs just work, no `tea install`
- **Hermetic builds**: Stdlib version locked to compiler
- **Single binary**: Distribute one file, runs anywhere
- **Offline-first**: No network required

## Example: std.util

### Source (`stdlib/util/mod.tea`)

```tea
# Returns the length of a string, list, or dict.
fn len(value)
  length(value)
end

# Returns true if value is a string.
fn is_string(value)
  __intrinsic_is_string(value)
end
```

### Snapshot Entry

```json
{
  "path": "std.util",
  "bytecode": [0x12, 0x34, ...],
  "exports": [
    {"name": "len", "arity": 1, "variadic": false, "doc": "Returns the length..."},
    {"name": "is_string", "arity": 1, "variadic": false, "doc": "Returns true..."}
  ]
}
```

### Usage

```tea
use util = "std.util"

fn main()
  print(util.len("hello"))      # => 5
  print(util.is_string(42))     # => false
end
```

### Execution Flow

1. Resolver sees `"std.util"` → loads from embedded snapshot
2. Compiler generates call to `util.is_string`
3. VM executes stdlib bytecode → calls `__intrinsic_is_string`
4. Intrinsic handler executes Rust type check
5. Result returned to caller

## Developer Experience

### Writing Stdlib

```bash
# Edit stdlib module
$ vim stdlib/util/mod.tea

# Rebuild (compiles stdlib automatically)
$ cargo build

# Test with dev override (load from disk, not snapshot)
$ TEA_STDLIB_PATH=./stdlib tea run my_test.tea
```

### Debugging

```bash
# Inspect snapshot contents
$ cat target/stdlib.snapshot | jq .modules.\"std.util\".exports

# Compare Rust vs Tea stdlib
$ cargo test --features rust-stdlib
$ cargo test --features tea-stdlib
```

## Performance Considerations

### Cold Path (Startup)

- Snapshot deserialization: ~1ms
- Bytecode loading: ~500μs per module
- **Total overhead**: <5ms for full stdlib

### Hot Path (Runtime)

- Intrinsic calls: same overhead as current stdlib calls
- Tea wrapper functions: ~10 bytecode instructions
- **No measurable difference** vs current Rust stdlib

### Binary Size

- Snapshot (uncompressed): ~100KB
- Snapshot (zstd): ~30KB
- **Acceptable tradeoff** for embedded distribution

## Future Enhancements

1. **AOT Compilation**: Compile stdlib to native code alongside user code
2. **Tree Shaking**: Only include used stdlib modules in snapshot
3. **Lazy Loading**: Load stdlib modules on-demand, not upfront
4. **Compression**: Use zstd/lz4 for snapshot compression
5. **Cache**: Cache compiled stdlib across builds (content-addressed)

## Security & Stability

- **Version lock**: Stdlib version tied to compiler version
- **No surprises**: Stdlib behavior deterministic across platforms
- **Audit trail**: Git history shows all stdlib changes
- **Type safety**: Exports validated at compile-time and runtime

## Compatibility

- **Backward compatible**: Existing programs continue to work
- **Forward compatible**: Snapshot format versioned, can evolve
- **Cross-platform**: Intrinsics abstract platform differences
- **No breaking changes**: Migrations feature-flagged
