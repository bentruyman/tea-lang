# Tea Standard Library Migration Plan

## Executive Summary

This document outlines the plan to migrate the Tea standard library from Rust implementations to Tea code built on native intrinsics. This provides better maintainability, documentation, and flexibility while preserving performance and enabling embedded distribution.

## Goals

1. **Developer experience**: Write stdlib in Tea, not Rust
2. **Better docs**: Docstrings become source of truth
3. **Maintainability**: Easier to extend without touching compiler internals
4. **Distribution**: Stdlib compiled and embedded in binaries (no external dependencies)

## Current State

The stdlib is currently implemented entirely in Rust:

- `tea-compiler/src/stdlib/*.rs` - Rust implementations
- Each module exports functions via `StdFunctionKind` enum
- VM executes stdlib functions directly in Rust
- Docs maintained separately in `stdlib/docs.rs`

## Target Architecture

```
┌──────────────────────┐
│ User Code            │
│   use util = "..."   │
└──────┬───────────────┘
       │
       ▼
┌──────────────────────┐
│ Tea Stdlib Modules   │ ← Written in Tea
│   stdlib/util/*.tea  │
└──────┬───────────────┘
       │
       ▼
┌──────────────────────┐
│ Native Intrinsics    │ ← Minimal Rust surface
│   __intrinsic_*      │
└──────────────────────┘
```

## Implementation Plan

### Phase 1: Foundation ✅ COMPLETE

**Deliverables:**

- [x] `stdlib/` directory with module structure
- [x] `docs/reference/language/intrinsics.md` - Intrinsic specification
- [x] `tea-compiler/src/runtime/intrinsics.rs` - Intrinsic enum
- [x] `tea-compiler/src/stdlib_snapshot.rs` - Snapshot format
- [x] `stdlib/util/mod.tea` - Example pure module
- [x] `stdlib/assert/mod.tea` - Example with failure handling

**Files Created:**

```
stdlib/
├── README.md
├── util/mod.tea
├── assert/mod.tea
└── {env,fs,path,io,cli,process,json,yaml}/   (empty dirs)

tea-compiler/src/
├── runtime/intrinsics.rs    (new)
└── stdlib_snapshot.rs       (new)

docs/reference/language/
├── intrinsics.md           (new)
└── tea-stdlib-design.md    (new)
```

### Phase 2: Build Infrastructure (NEXT)

**Objective**: Compile stdlib to bytecode and embed in binary

**Tasks:**

1. Update `tea-compiler/build.rs`:
   - Compile `stdlib/**/*.tea` using tea-compiler as library
   - Extract function exports (name, arity, docs)
   - Serialize to `target/stdlib.snapshot` (JSON format)
   - Generate `src/embedded_stdlib.rs` with `include_bytes!`

2. Environment variable override:
   - `TEA_STDLIB_PATH=./stdlib` → load from disk (dev mode)
   - Default → load from embedded snapshot

3. Test infrastructure:
   - Verify snapshot loads correctly
   - Validate module resolution
   - Check bytecode execution

**Acceptance Criteria:**

- `cargo build` produces `target/stdlib.snapshot`
- Snapshot embedded in `tea-cli` binary
- Can load and execute `std.util` functions
- TEA_STDLIB_PATH override works

### Phase 3: Runtime Integration

**Objective**: Wire intrinsics into the VM and resolver

**Tasks:**

1. **Bytecode**: Add `CallIntrinsic` instruction (or reuse `CallStd`)
2. **VM execution**:
   ```rust
   match intrinsic {
       Intrinsic::IsString => /* call existing type check */,
       Intrinsic::FsReadText => /* call existing fs impl */,
       // ... map all intrinsics to StdFunctionKind impls
   }
   ```
3. **Resolver**:
   - For `use x = "std.*"` → check embedded snapshot first
   - Load module bytecode from snapshot
   - Expose function exports to typechecker
4. **Feature flag**: `--features tea-stdlib` vs default `rust-stdlib`

**Acceptance Criteria:**

- All `__intrinsic_*` calls execute correctly
- Module imports from snapshot work
- Existing tests pass with `--features tea-stdlib`

### Phase 4: Port All Modules

**Objective**: Migrate all 10 stdlib modules to Tea

**Modules to Port:**

1. ✅ `std.util` - Type predicates (DONE)
2. ✅ `std.assert` - Test assertions (DONE)
3. `std.env` - Environment variables
4. `std.fs` - Filesystem operations
5. `std.path` - Path manipulation
6. `std.io` - Standard I/O
7. `std.cli` - CLI argument parsing
8. `std.process` - Process management
9. `std.json` - JSON codec
10. `std.yaml` - YAML codec

**Pattern for Each Module:**

```tea
# stdlib/fs/mod.tea

# Read file contents as text
fn read_text(path)
  __intrinsic_fs_read_text(path)
end

# Write with automatic parent directory creation
fn write_text_safe(path, content)
  ensure_parent(path)
  write_text(path, content)
end

fn ensure_parent(path)
  # Pure Tea helper - no intrinsic needed
  # ...
end
```

**Acceptance Criteria:**

- All modules compile to snapshot
- All exports match current Rust stdlib API
- Examples in `examples/stdlib/*` work unchanged

### Phase 5: Testing & Performance

**Objective**: Verify parity and benchmark performance

**Tasks:**

1. **Parity Tests**:
   - Run all `tea-compiler/tests/*` with tea-stdlib
   - Run all `examples/stdlib/*` examples
   - Compare output with rust-stdlib

2. **Performance Benchmarks**:
   - Benchmark hot paths: string ops, list ops, I/O
   - Compare tea-stdlib vs rust-stdlib
   - Ensure no significant regression (<5%)

3. **Integration Tests**:
   - Test module loading
   - Test error handling
   - Test snapshot versioning

**Acceptance Criteria:**

- 100% test parity with rust-stdlib
- Performance within 5% of rust-stdlib
- All benchmarks pass

### Phase 6: Rollout & Cleanup

**Objective**: Make tea-stdlib the default, remove rust-stdlib

**Tasks:**

1. Switch default to `tea-stdlib` feature
2. Keep `rust-stdlib` for one release (deprecated)
3. Remove `tea-compiler/src/stdlib/*.rs` (except `mod.rs` for intrinsics)
4. Update all documentation
5. Update CHANGELOG

**Acceptance Criteria:**

- tea-stdlib is default
- No breaking changes to user code
- Documentation updated

## Technical Details

### Intrinsic Design

**Naming Convention**: `__intrinsic_<category>_<function>`

- Examples: `__intrinsic_fs_read_text`, `__intrinsic_is_string`
- Prefix prevents collision with user code
- Category groups related functions

**Type Signature** (conceptual):

```tea
# Type predicates
__intrinsic_is_string(value: any) -> bool

# Filesystem
__intrinsic_fs_read_text(path: string) -> string
__intrinsic_fs_write_text(path: string, content: string) -> void

# Process
__intrinsic_process_run(cmd: string, args: list[string]) -> dict
```

### Snapshot Format

**Structure**:

```json
{
  "version": "0.1.0",
  "modules": {
    "std.util": {
      "path": "std.util",
      "bytecode": [18, 52, ...],
      "exports": [
        {
          "name": "is_string",
          "arity": 1,
          "variadic": false,
          "doc": "Returns true if value is a string."
        }
      ]
    }
  }
}
```

**Size Estimates**:

- JSON (uncompressed): ~100KB
- JSON (zstd): ~30KB
- Future (bincode): ~20KB

### Module Resolution

**Search Order**:

1. Embedded snapshot (for `std.*` paths)
2. Project-relative paths (`./`, `../`)
3. Absolute paths
4. Dev override (`TEA_STDLIB_PATH` env var)

**Example**:

```tea
use util = "std.util"     # → embedded snapshot
use local = "./lib.tea"   # → disk
```

### Build Process

**Flow**:

```
stdlib/*.tea
    ↓ (compile via tea-compiler lib)
bytecode + exports
    ↓ (serialize to JSON)
target/stdlib.snapshot
    ↓ (include_bytes! in build.rs)
src/embedded_stdlib.rs
    ↓ (compiled into tea-cli)
final binary
```

**Build Time**: ~500ms for full stdlib compilation

## Migration Benefits

### For Users

- ✅ No behavior changes (drop-in compatible)
- ✅ Better error messages (Tea stack traces)
- ✅ Smaller binaries (bytecode vs Rust)
- ✅ No external dependencies

### For Contributors

- ✅ Write stdlib in Tea, not Rust
- ✅ Easier to understand and modify
- ✅ Docs in code (docstrings)
- ✅ Faster iteration (no Rust recompile for stdlib changes)

### For Maintainers

- ✅ Less Rust code to maintain
- ✅ Stable intrinsic boundary (fewer breaking changes)
- ✅ Easier to add high-level helpers
- ✅ Better test coverage (test Tea code)

## Risks & Mitigations

| Risk                   | Mitigation                                                 |
| ---------------------- | ---------------------------------------------------------- |
| Performance regression | Benchmark all hot paths; keep critical paths as intrinsics |
| Binary size increase   | Compress snapshot; tree-shake unused modules               |
| Debugging difficulty   | Preserve stack traces; add introspection tools             |
| Breaking changes       | Feature flag; parallel rust-stdlib for one release         |
| Build complexity       | Clear error messages; document build process               |

## Timeline

- **Week 1**: Phase 1 (Foundation) ✅ COMPLETE
- **Week 2**: Phase 2 (Build Infrastructure)
- **Week 3**: Phase 3 (Runtime Integration)
- **Week 4**: Phase 4 (Port All Modules)
- **Week 5**: Phase 5 (Testing & Performance)
- **Week 6**: Phase 6 (Rollout)

**Total**: ~6 weeks for full migration

## Success Metrics

1. ✅ All existing tests pass with tea-stdlib
2. ✅ Performance within 5% of rust-stdlib
3. ✅ Binary size <200KB (currently ~150KB)
4. ✅ Stdlib compilation <1s
5. ✅ Zero breaking changes to user code

## Documentation

- ✅ `docs/reference/language/intrinsics.md` - Intrinsic reference
- ✅ `docs/reference/language/tea-stdlib-design.md` - Architecture
- ✅ `stdlib/README.md` - Developer guide
- ✅ `stdlib/STDLIB_PLAN.md` - Detailed task breakdown

## Next Actions

1. ✅ Create foundation (Phase 1) - COMPLETE
2. **Implement build.rs stdlib compilation** - IN PROGRESS
3. Wire up intrinsics in VM
4. Update resolver for snapshot loading
5. Port remaining 8 modules
6. Test and benchmark

---

## Related Documents

- [Intrinsics Reference](docs/reference/language/intrinsics.md)
- [Tea Stdlib Design](docs/reference/language/tea-stdlib-design.md)
- [Stdlib Plan](stdlib/STDLIB_PLAN.md)
- [Stdlib README](stdlib/README.md)
