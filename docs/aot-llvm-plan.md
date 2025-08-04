# LLVM AOT Integration Plan

Target outcome: tea-lang programs compile Ahead-Of-Time to native binaries with performance comparable to Rust/Go, while preserving the current compiler pipeline for diagnostics and interpreter fallback. The first end-to-end slice now ships in the default CLI build; the bullets below track the remaining polish items.

## Current Pipeline Snapshot
- **Front-end** – `Lexer` → `Parser` → AST (`tea-compiler/src/parser/mod.rs`).
- **Semantic passes** – `Resolver` (scope rules) and `TypeChecker` (inferred/annotated types). The type checker already produces precise types for expressions, which the bytecode generator ignores today.
- **Code generation** – `CodeGenerator` lowers AST to stack-based bytecode (`Program`) executed by `Vm`. The LLVM backend sits alongside it in `tea-compiler::aot`.
- **CLI** – defaults to the VM backend for `tea run`, while `tea build` lowers to IR/object/executable via LLVM. `--emit {llvm-ir,obj}` work for both inspection and build pipelines.

The AOT backend will hook in after type checking, using the same expanded `Module` and diagnostics infrastructure.

## High-Level Architecture
1. **Intermediate Representation**
   - Define a typed, SSA-friendly IR (or go directly to LLVM IR) that uses the type checker's results.
   - Represent locals/globals with explicit types (Int, Float, Bool, etc.) to avoid boxing.
   - Model control flow with basic blocks (if/else, loops, returns).

2. **LLVM Integration** *(initial pass complete)*
   - ✅ Adopted `inkwell` for safe bindings and initialise the LLVM context/module/builder per compilation unit.
   - ✅ Map tea types to LLVM types (`i1`, `i64`, `double`, pointers to runtime structs for strings/lists/structs).
   - ✅ Declare runtime helper functions and link against the `tea-runtime` staticlib via `rustc` during the build step.

3. **Codegen Pass** *(ongoing)*
- ✅ `LlvmCodeGenerator` implements arithmetic, logical operators, strings, lists, structs (constructors/member access/equality), loops, and function calls.
- ✅ Lambda literals now lower to closure structs with capture handling and callable function pointers.
- 🚧 Pending: dictionaries/member access on dictionaries and `for` loops once iterable semantics settle.
   - ✅ Diagnostics bubble up through the same error tracker; unsupported constructs still report early.
   - ✅ `support.cli`'s `args`/`parse` helpers lower through LLVM (dispatching to the runtime); `capture` remains VM-only for now.

4. **Runtime Alignment**
   - Decide on memory model for compound types:
     - Option A: reuse existing runtime (lists/dicts) via FFI calls.
     - Option B: introduce native structs and garbage collection later.
   - Expose builtins (`print`) as external functions from the CLI binary.

5. **Tooling & CLI Integration** *(initial pass complete)*
   - ✅ `tea build` lowers to IR/object code and links an executable under `bin/`.
   - ✅ `--emit llvm-ir` / `--emit obj` are available for inspection or to keep intermediates.
   - 🚧 Cross-compilation, linker flag overrides, and nicer diagnostics for missing toolchains/linkers.

6. **Testing & Benchmarks**
   - Create criterion benchmarks (`fib`, numeric loops) to compare interpreter vs LLVM AOT.
   - Add integration tests ensuring emitted binaries run expected outputs.

## Immediate Tasks
1. Land lowering for dictionaries and member/index assignment so container-heavy programs can target LLVM.
2. Add CLI options for cross-compiling (`--target`, linker override) and document the `rustc` dependency for final linking.
3. Introduce regression coverage for `tea build` (e.g. smoke tests that run the produced binary) and document manual testing steps for unsupported platforms.
4. Improve diagnostics surfaced from LLVM verification/linking (linker not found, missing runtime artefacts).
5. Run performance experiments (Criterion micro-benchmarks) comparing VM vs LLVM output to validate the payoff.

## Open Questions
- **Garbage Collection**: stick with current reference-counted containers initially, or plan a tracing GC for native values?
- **Module Linking**: support multi-file compilation upfront or stitch object files per module?
- **Error Reporting**: how to surface LLVM verification errors with tea-lang spans?
- **Optimization Strategy**: rely on LLVM defaults (`-O2`) or tailor passes per target?

This document should evolve as we prototype; the next step is adding the dependency skeleton and a `hello world` IR emitter.
