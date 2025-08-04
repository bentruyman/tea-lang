# Tea-Lang Roadmap

This document tracks the near-term implementation plan for tea-lang. With the resolver/type checker in place and the first LLVM-backed `tea build` flow available behind a feature flag, the roadmap now focuses on closing remaining language gaps, rounding out the runtime, and hardening the new ahead-of-time pipeline.

## Front-End Tightening
- **Resolver & Symbol Tables**  
  - Track scopes for globals, locals, and closures.  
  - Emit diagnostics for undefined bindings, duplicate declarations, and shadowing rules.
  - ✅ Initial resolver pass lands in the compiler, enforcing scope tracking and halting on undefined/shadowed bindings before type checking.  
- **Type Information**  
  - Annotate AST nodes with simple types (Int, Bool, String, Nil).  
  - Enforce arity and type expectations on function calls and operators.  
  - Provide structured error messages (with spans) instead of generic strings.
- **Generics Support**  
  - ✅ Define syntax for parametrized functions and structs (e.g. `def identity[T]`) and allow explicit type arguments at call sites.  
  - ✅ Extend the resolver/type checker to track generic parameters, substitute concrete types, and emit span-rich diagnostics when instantiations mismatch.  
  - ✅ Monomorphise generics across both bytecode and LLVM backends (including functions/structs imported via `use`), leaving future work to explore advanced optimisation strategies beyond specialisation.
- **Grammar Coverage**  
  - ✅ Member/index expressions (`list[0]`) and trailing expression returns behave in both the VM and LLVM backends.  
  - 🚧 Dictionaries/member access still emit “unsupported expression” in LLVM and need lowering (+ runtime helpers).
  - ✅ Lambda literals now lower with closure capture semantics in both the VM and LLVM backends.
- **Immutable Bindings**  
  - Introduce `const` declarations and enforce single assignment semantics through the resolver/type checker.  
  - Ensure runtime and generated code prevent reassignment, surfacing diagnostics when violated.  
  - Document migration guidance for existing samples and tests that rely on mutable bindings.
- **Lambda Expressions**  
  - ✅ Ship closure capture semantics and bytecode/LLVM lowering for `|args| => expr` literals.  
  - ✅ Add runtime support for invoking lambdas, including environment handling and value lifetimes.  
  - ✅ Expand diagnostics/tests so lambda failures are covered in both VM and AOT paths.

## Bytecode & VM Growth
- **Loop Support**  
  - ✅ `while` and `until` loops lower to bytecode/LLVM and execute via jump instructions.  
  - 🚧 Defer `for` loops until iterable semantics are designed.
- **Data Structures**  
- ✅ Lists (literal/indexing) exist in both the VM and LLVM pipelines.  
- 🚧 Dictionaries/member access still require opcode/runtime work.  
- ✅ Struct layout/field access lower in the VM and type system (LLVM support still pending).
- **Built-ins & Modules**  
  - ✅ The runtime helpers (`tea_print_*`, `tea_alloc_*`) live in `tea-runtime` and are linked into LLVM builds.  
  - ✅ CLI-focused std modules ship helpers for stdin/stdout streaming (`std.io`), structured data codecs (`std.json`, `std.yaml`), and argument parsing (`support.cli`'s `args`/`parse`) across both the VM and LLVM backends (with `support.cli.capture` still VM-only).
  - ✅ User modules pulled in via `use` now participate fully in generic specialisation; follow-up work will promote std modules out of hard-coded VM checks and add caching/cycle detection.
  - 🚧 Promote remaining builtins/modules out of hard-coded VM checks and introduce module caching/cycle detection.

## Tooling & Follow-Ups
- ✅ **Testing Harness (`tea test`)**  
  - CLI command discovers `test "name"` blocks, supports `--list`/`--filter`/`--fail-fast`, and reports pass/fail summaries.  
  - Extensible harness lays groundwork for future snapshot and golden-file assertions in addition to Rust coverage.
  - ✅ Snapshot assertions (`assert_snapshot`, `assert_empty`) with `--update-snapshots` flag and `support.cli.capture` helper for end-to-end CLI testing.
- ✅ **Code Formatter (`tea fmt`)**  
  - The CLI ships an in-place formatter that normalises indentation, preserves inline comments, and collapses stray blank lines.  
  - Follow-ups: broaden coverage for multi-line literals/dicts and publish editor integration notes once the rule set stabilises.
- **Cross-Compilation**: surface `--target`/linker overrides in `tea build` and document required toolchains.  
- **Performance Benchmarks**: wire up Criterion runs comparing VM vs LLVM output once the backend stabilises.  
- **REPL**: stretch goal once the runtime + module system are stable.  
- **Documentation**: keep `semantics.md`, `aot-backend.md`, and CLI help up to date with new capabilities; track CLI standard library milestones in `docs/cli-stdlib-roadmap.md`.

### Open Questions & Deferred Work
- **Range Literals**: Clarify the intended runtime value for `start..end`/`start...end` (eager list, lazy iterator, or compile-time error) and queue the implementation or guardrail.
- **Numeric Runtime**: Extend modulo and float support with edge cases (negative modulus behaviour, float formatting) and add regression coverage once priorities settle.

### Current Sprint: LLVM Polishing
1. Land dictionary/member access lowering in the LLVM backend.  
2. Expose cross-compilation hooks (target triple, linker flags) and document the Rust toolchain dependency.  
3. Add executable smoke tests to guard the new `tea build` flow.  
4. Refresh docs/tests alongside the new behaviour.
