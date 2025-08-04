# LLVM AOT Backend (Preview)

This document describes the ahead-of-time (AOT) code generation pipeline that lowers tea-lang programs to LLVM IR. The backend now ships in the default CLI build; expect ongoing iteration as we flesh out lowering, runtime integration, and packaging.

## Enabling the Backend

`tea build` invokes the LLVM path directly:

```bash
cargo run -p tea-cli -- build examples/fib.tea
```

The command lowers `examples/fib.tea` to LLVM IR, emits an object file, links it against the shared runtime with `rustc`, and writes the resulting binary to `bin/fib`. Pass `--target <triple>` if you need to override the detected host triple (useful when LLVM was built without support for your platform). You can still emit intermediate artefacts without producing an executable:

```bash
# Dump IR only
cargo run -p tea-cli -- --emit llvm-ir --no-run examples/fib.tea

# Keep the object file alongside the IR
cargo run -p tea-cli -- --emit llvm-ir --emit obj --no-run examples/fib.tea
```

## Current Capabilities

- **Inputs**: The backend reuses the existing front-end pipeline (lexer → parser → resolver → type checker). It consumes the same expanded `Module` that the bytecode generator sees, so diagnostics stay consistent.
- **Supported constructs**:
  - Integer and float literals with arithmetic (`+`, `-`, `*`, `/`, `%`) and mixed-type promotion (Int → Float).
  - Integer comparisons (`==`, `!=`, `<`, `<=`, `>`, `>=`).
  - If/unless conditionals with logical `and`/`or`.
  - `var` declarations for `Int`, `Float`, `Bool`, `String`, `List`, and struct locals (initializer required, type inferred or annotated).
  - Assignment to existing locals (`x = x + 1`, reassigning strings/lists/structs).
  - `while` / `until` loops in functions and at module top-level.
  - Function definitions, recursion, and explicit `return`.
  - List literals/indexing and dictionary literals/member access (lowered via runtime helpers for consistent semantics with the VM).
  - Struct definitions, constructors (positional and named arguments), member access, equality, and `print` support (forwarded to the runtime).
  - Builtin `print` for `Int`, `Float`, `Bool`, `String`, `List`, `Dict`, and Struct through the `tea-runtime` crate.
  - Generic functions and structs. The type checker emits the concrete instantiations it sees (including those defined in `use`-able modules), and the backend monomorphises each specialisation into a distinct LLVM function/struct template.
  - Standard library helpers: `std.assert` (`assert`, `assert_eq`, `assert_ne`, `fail`), `std.util` (`len`, `to_string`, `clamp_int`, type guards), `std.fs` (text/byte IO, directory helpers, metadata, and chunked streaming), plus CLI-focused helpers in `std.io`, `std.json`, `std.yaml`, and `support.cli` (`args`, `parse`) for argument-driven tooling.
- **Output**: LLVM IR, optional object files, and (via `tea build`) host executables that link the runtime automatically.
- **Packaging Helpers**: `tea build` can emit deterministic bundles (`--bundle`), SHA-256 checksums (`--checksum`), and HMAC signatures (`--signature-key <path>`), all honouring `SOURCE_DATE_EPOCH` so release artifacts stay reproducible. Cross-compilation tunables (`--target`, `--cpu`, `--features`, `--opt-level`) are surfaced directly on the CLI.

## Limitations (for now)

- Lambda literals lower with captured environments, but nested generic closures are still routed through the VM until we add specialisation support there.
- Cross-compilation is not wired up. The linker assumes a host build and relies on `rustc` locating the Rust standard library.
- LLVM-specific failures still surface coarse errors (e.g., missing toolchains), but front-end diagnostics now include spans and import hints.
- `for` loops will remain VM-only until iterator semantics settle.
- `std.json.decode` / `std.yaml.decode` currently materialise dictionaries/lists with unspecified value types; richer typing for mixed JSON data is planned.
- `support.cli.capture` still routes through the VM path; only `args` and `parse` lower through LLVM today.

## Next Steps

- Lower remaining language features (dictionaries, member access, `for` loops once iterables land).
- Add cross-compilation hooks (`--target`, custom linker flags) so `tea build` can produce artefacts for other platforms.
- Add Criterion benchmarks to compare interpreter vs LLVM output once execution is wired up.
- Improve diagnostics (include spans/labels) and surface linker hints for missing runtime symbols/toolchains.

Track detailed milestones in `docs/aot-llvm-plan.md`; this document will evolve as the backend matures.
