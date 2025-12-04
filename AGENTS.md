# Agent Guidelines for tea-lang

## Development Commands

### Build & Setup

- **First-time setup**: `make setup` — Installs Bun dependencies and runs codegen
- **Build**: `cargo build --release` or `make build` (runs codegen first)
- **Install locally**: `make install` — Copies binaries to `~/.cargo/bin`
- **During development**: `cargo run -p tea-cli -- script.tea` — Run without installing

### Testing

- **All tests**: `make test` — Runs `cargo test --workspace` then `scripts/e2e.sh`
- **Single test**: `cargo test -p tea-compiler test_name` (e.g., `cargo test -p tea-compiler interpolated_strings`)
- **E2E tests only**: `./scripts/e2e.sh` — Runs tree-sitter tests and validates all examples
- **AOT tests**: Integration tests validate LLVM IR output in `tea-compiler/tests/aot_examples.rs`

### Formatting

- **Rust code**: `cargo fmt --all`
- **Tea code**: `cargo run -p tea-cli -- fmt .`
- **All code**: `make fmt` — Formats Rust, Tea, and runs Prettier

### Code Generation

- **Regenerate AST**: `make codegen-ast` — Generates `tea-compiler/src/ast.rs` from `spec/ast.yaml`
- **Regenerate highlights**: `make codegen-highlights` — Generates tree-sitter `highlights.scm` from `spec/tokens.toml`
- **All codegen**: `make codegen` — Runs both

**Important**: AST and tree-sitter files are code-generated. Edit `spec/ast.yaml` or `spec/tokens.toml`, then run codegen.

## Architecture Overview

Tea is a strongly typed scripting language that compiles to native code via LLVM. The compiler follows a multi-stage pipeline architecture.

### Crate Structure

**Workspace members**:

- `tea-cli/` — CLI binary; orchestrates compiler and runtime modes
- `tea-compiler/` — Core compilation pipeline (lexer → parser → resolver → typechecker → codegen)
- `tea-runtime/` — C runtime library linked into compiled binaries (FFI helpers, stdlib hooks)
- `tea-intrinsics/` — Rust implementations of intrinsic functions called via FFI
- `tea-lsp/` — Language server for editor integration
- `tea-support/` — Shared utilities

**Other directories**:

- `spec/` — Canonical grammar (`ast.yaml`, `tokens.toml`) used for codegen
- `stdlib/` — Standard library written in Tea (modules like `std.fs`, `std.path`)
- `examples/` — Sample Tea programs organized by feature
- `tree-sitter-tea/` — Tree-sitter grammar for syntax highlighting

### Compilation Pipeline

**Five-stage pipeline** in `tea-compiler/src/`:

1. **Lexer** (`lexer/`) — Tokenizes source into `Token` stream
2. **Parser** (`parser/`) — Builds AST from tokens
3. **Resolver** (`resolver.rs`) — Resolves module imports, builds symbol table
4. **Type Checker** (`typechecker.rs`) — Validates types, infers generics
5. **Code Generation** (`aot/`) — Lowers to LLVM IR for native compilation

**Two execution modes**:

- **Interpreter** (default): JIT-style compilation and execution
- **AOT** (via `tea build`): Emits standalone native binary via LLVM

### Type System

Tea uses **bidirectional type checking** with **full type inference**:

- Type annotations optional on local bindings, required on function signatures
- Generics use monomorphization (generates specialized copies per type)
- `Type` enum in typechecker represents all types: `Int`, `Float`, `String`, `List[T]`, `Struct`, `Function`, etc.

### Standard Library

**Two-tier architecture**:

1. **Intrinsics** (`tea-intrinsics/src/`) — Low-level Rust implementations (filesystem, environment, string ops)
2. **Stdlib modules** (`stdlib/`) — High-level Tea wrappers (e.g., `std.fs` calls intrinsics)

Stdlib modules are resolved by the compiler's resolver and inlined during compilation.

### AST Generation

The AST is **code-generated** from `spec/ast.yaml`:

- Edit `spec/ast.yaml` to modify AST structure
- Run `make codegen-ast` to regenerate `tea-compiler/src/ast.rs`
- Schema defines all node types, fields, and derives

**Pattern**: All AST nodes include `SourceSpan` for error reporting.

## Code Style

- **Rust**: Standard `cargo fmt`; use `anyhow::Result` for fallible functions, `thiserror` for custom errors
- **Imports**: Group stdlib → external crates → internal (`use crate::`) with blank lines between
- **Naming**: `snake_case` functions/vars, `PascalCase` types, lowercase modules
- **Error handling**: Propagate with `?`; use `bail!` for early returns; avoid `.unwrap()` except tests
- **Tests**: Integration tests in `tea-compiler/tests/feature_name.rs`; use snapshot testing for diagnostics
- **Tea syntax**: 2-space indent, `snake_case`, terminate blocks with `end`, backticks for string interpolation

## Running Tea Programs

**Script mode** (interpret):

```bash
tea script.tea
cargo run -p tea-cli -- script.tea  # during development
```

**Build mode** (AOT compile):

```bash
tea build script.tea
./bin/script
```

**With arguments**:

```bash
tea script.tea arg1 arg2
```

**Emit intermediate output**:

```bash
tea --emit ast script.tea      # Show AST
tea --emit llvm-ir script.tea  # Show LLVM IR
```

## Common Development Tasks

### Adding a New Intrinsic Function

1. Add Rust implementation in `tea-intrinsics/src/lib.rs`
2. Register in `tea-compiler/src/stdlib/builtins.rs`
3. Link FFI in `tea-runtime/src/lib.rs`
4. Add Tea wrapper in appropriate `stdlib/*/mod.tea` module
5. Add tests in `tea-compiler/tests/`

### Modifying AST

1. Edit `spec/ast.yaml`
2. Run `make codegen-ast`
3. Update parser in `tea-compiler/src/parser/`
4. Update typechecker if needed
5. Update AOT codegen if needed

### Adding a Language Feature

1. Update lexer if new tokens needed (`tea-compiler/src/lexer/`)
2. Extend AST in `spec/ast.yaml`, run codegen
3. Update parser to recognize new syntax
4. Add type checking rules in `typechecker.rs`
5. Add LLVM codegen in `tea-compiler/src/aot/`
6. Add tests in `tea-compiler/tests/`
7. Add examples in `examples/language/`

## Debugging

**Compiler crashes**: Run with `RUST_BACKTRACE=1` or `RUST_BACKTRACE=full`

**Type errors**: Enable verbose output in typechecker (add debug prints in `typechecker.rs`)

**AOT issues**: Use `--emit llvm-ir` to inspect generated LLVM IR

**Parser issues**: Check `--emit ast` to see parsed structure

**E2E test failures**: Check `scripts/e2e.sh` output; validates all examples run correctly
