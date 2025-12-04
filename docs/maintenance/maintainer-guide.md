# Tea Maintainer Guide

This guide explains how to extend Tea with new intrinsics and standard library surface area now that the project is LLVM/AOT-only. The key idea is to keep business logic in the shared `tea-intrinsics` crate, expose it through the C ABI in `tea-runtime`, and describe it to the compiler via the `tea-compiler/src/stdlib` metadata so type checking and codegen can reason about the API.

---

## Adding a New Intrinsic

Intrinsics are the narrow escape hatch into host functionality. Every intrinsic flows through the same four layers:

1. **Core implementation** – plain Rust in `tea-intrinsics` that performs the real work.
2. **Runtime export** – an `extern "C"` function in `tea-runtime/src/lib.rs` that adapts pointer-based `TeaValue` structs into Rust types and calls the core function.
3. **Compiler metadata** – entries in `tea-compiler/src/stdlib` that let the resolver/type checker understand arity, parameter types, and documentation.
4. **AOT lowering hooks** – optional entries in `tea-compiler/src/aot/intrinsics.rs` when code generation needs to recognise a `__intrinsic_*` function name.

### 1. Implement the Core Function

Add the logic to the appropriate module in `tea-intrinsics/src/`. Keep the API ergonomic (plain `&str`, `Vec<String>`, etc.) and return `anyhow::Result` when fallible.

```rust
// tea-intrinsics/src/fs.rs
use anyhow::Result;
use std::fs;

pub fn copy(source: &str, dest: &str) -> Result<()> {
    fs::copy(source, dest)
        .map_err(|error| anyhow::anyhow!(tea_support::fs_error("copy", source, &error)))?;
    Ok(())
}
```

### 2. Expose the Runtime Entry Point

Declare an `extern "C"` wrapper in `tea-runtime/src/lib.rs`. These functions convert to/from `TeaValue` or `TeaString` pointers and panic with a helpful error if the intrinsic fails.

```rust
// tea-runtime/src/lib.rs
#[no_mangle]
pub extern "C" fn tea_fs_copy(source: *const TeaString, dest: *const TeaString) {
    let source_str = expect_path(source);
    let dest_str = expect_path(dest);
    tea_intrinsics::fs::copy(&source_str, &dest_str)
        .unwrap_or_else(|error| panic!("{}", error));
}
```

If the intrinsic needs to return structured data, build the appropriate `TeaValue` (strings via `tea_alloc_string`, lists via `tea_alloc_list`, etc.) before handing the pointer back to LLVM-generated code.

### 3. Register Compiler Metadata

The compiler learns about intrinsic signatures via the statically-defined modules in `tea-compiler/src/stdlib/`.

1. Add a new variant to `StdFunctionKind` in `tea-compiler/src/stdlib/mod.rs`.
2. Append a `std_function!(...)` entry to `tea-compiler/src/stdlib/intrinsics.rs` (or the module that best matches your category) so the resolver/type checker can validate arity and types.
3. If codegen needs to treat the intrinsic specially (e.g., to emit custom LLVM IR instead of an FFI call), extend `tea-compiler/src/aot/intrinsics.rs` and handle the corresponding `StdFunctionKind` in `tea-compiler/src/aot/mod.rs`.

### 4. Document and Test

- Document the intrinsic in `docs/intrinsics-reference.md` or the relevant stdlib README.
- Add or extend `.tea` examples under `stdlib/` so we exercise the new API end-to-end.
- Run `cargo test --workspace` and any targeted examples to ensure the intrinsic works when invoked from compiled Tea code.

### Intrinsic Checklist

- [ ] Core logic in `tea-intrinsics/src/<category>.rs`
- [ ] Runtime wrapper in `tea-runtime/src/lib.rs`
- [ ] `StdFunctionKind` variant + metadata entry in `tea-compiler/src/stdlib/`
- [ ] Optional `tea-compiler/src/aot/intrinsics.rs` hook if lowering needs special handling
- [ ] Documentation + tests
- [ ] `cargo fmt --all` and `cargo test --workspace`

---

## Adding a New Standard Library Module

Stdlib modules package intrinsic calls (and pure-Tea helpers) behind a cohesive API. Each module has two halves:

1. **User-facing Tea source** under `stdlib/<module>/mod.tea`.
2. **Compiler metadata** in `tea-compiler/src/stdlib/<module>.rs`.

### Steps

1. **Create the Tea module**  
   Write the `.tea` functions under `stdlib/<name>/mod.tea`. Use `pub` to expose functions and rely on existing intrinsics (e.g., `__intrinsic_fs_read_text`) for host interaction.

2. **Describe the module to the compiler**
   - Add a new Rust file under `tea-compiler/src/stdlib/` that declares a `StdModule` with its functions (`std_function!` macro).
   - Import the module in `tea-compiler/src/stdlib/mod.rs` and append it to the `MODULES` slice so `use net = "std.net"` resolves correctly.

3. **Wire new intrinsic helpers (optional)**  
   If the module needs additional intrinsics, follow the intrinsic steps above before referencing them from Tea code.

4. **Document & test**  
   Update `docs/stdlib-reference.md`, add example programs under `examples/`, and add regression tests under `tea-compiler/tests/` if the module exposes behaviour not already covered.

---

## Architecture Overview

```
┌──────────────────────────────────────────────┐
│                tea-intrinsics                │
│  (pure Rust: filesystem, env, path, etc.)    │
└──────────────────────────┬───────────────────┘
                           │
                           ▼
┌──────────────────────────────────────────────┐
│                 tea-runtime                  │
│  extern \"C\" API → TeaValue/TeaString FFI   │
│  + process/fs helpers used by compiled code  │
└──────────────────────────┬───────────────────┘
                           │
                           ▼
┌──────────────────────────────────────────────┐
│                 tea-compiler                 │
│  StdModule metadata + LLVM code generation   │
│  emits calls into tea-runtime for intrinsics │
└──────────────────────────────────────────────┘
```

All Tea programs now compile straight to LLVM IR and link against `tea-runtime`. The metadata layer ensures the resolver/type checker know which functions exist, and the runtime FFI ensures compiled binaries and tools like `tea run` have identical semantics.
