# Tea Standard Library Implementation Plan

## Current Progress

### ✅ Phase 1: Foundation

- [x] Created stdlib/ directory structure
- [x] Defined intrinsics enum and documentation
- [x] Created snapshot format (SnapshotModule, Export, Snapshot)
- [x] Ported std.util to Tea
- [x] Ported std.assert to Tea

### 🚧 Phase 2: Build Infrastructure (In Progress)

- [ ] Update build.rs to compile stdlib modules
- [ ] Generate stdlib snapshot during build
- [ ] Embed snapshot using include_bytes!
- [ ] Add TEA_STDLIB_PATH dev override

### 📋 Phase 3: Runtime Integration

- [ ] Wire up intrinsics in VM (map \__intrinsic_\* to native impls)
- [ ] Update resolver to load from embedded snapshot
- [ ] Add feature flag for Tea stdlib vs Rust stdlib
- [ ] Test basic module loading

### 📋 Phase 4: Port Remaining Modules

- [ ] Port std.env (wraps env intrinsics)
- [ ] Port std.fs (wraps fs intrinsics + helpers like ensure_dir)
- [ ] Port std.path (wraps path intrinsics)
- [ ] Port std.io (wraps io intrinsics)
- [ ] Port std.cli (wraps cli intrinsics)
- [ ] Port std.process (wraps process intrinsics)
- [ ] Port std.json (wraps json intrinsics)
- [ ] Port std.yaml (wraps yaml intrinsics)

### 📋 Phase 5: Testing & Parity

- [ ] Run existing tests with Tea stdlib
- [ ] Verify all examples work
- [ ] Performance benchmarks
- [ ] Fix any discrepancies

### 📋 Phase 6: Rollout

- [ ] Default to Tea stdlib
- [ ] Remove deprecated Rust stdlib code
- [ ] Update documentation

## Next Steps

1. **Build script**: Compile stdlib/\*.tea files to bytecode
   - Use the tea-compiler library to compile each module
   - Extract function exports (name, arity, docs)
   - Serialize to snapshot format
   - Write to target/stdlib.snapshot
   - Generate src/embedded_stdlib.rs with include_bytes!

2. **Intrinsics wiring**: Map \__intrinsic_\* calls to StdFunctionKind
   - Add bytecode instruction for intrinsic calls
   - VM execution maps to existing stdlib implementations
   - Maintain existing behavior, just different call path

3. **Resolver update**: Check embedded snapshot before disk
   - For "std.\*" imports, check snapshot first
   - Fall back to disk for user modules
   - Support TEA_STDLIB_PATH override for development

## Notes

- Start simple: get std.util and std.assert working end-to-end
- Keep Rust stdlib working in parallel (feature flag)
- Intrinsics map to existing StdFunctionKind implementations
- No breaking changes to VM or bytecode format (add, don't replace)
