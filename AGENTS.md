# Agent Guidelines for tea-lang

## Commands

- **Build**: `cargo build -p tea-cli` or `make build` (runs codegen first)
- **Test all**: `cargo test --workspace` then `scripts/e2e.sh`
- **Test single**: `cargo test -p tea-compiler test_name` (e.g., `cargo test -p tea-compiler interpolated_strings`)
- **Format**: `cargo fmt --all` before committing; `cargo run -p tea-cli -- fmt .` for `.tea` files
- **Run script**: `cargo run -p tea-cli -- examples/path/to/file.tea`
- **Lint**: Use `--check` flag with fmt commands; no separate linter configured

## Code Style

- **Rust**: Standard `cargo fmt` rules; use `anyhow::Result` for fallible functions, `thiserror` for custom errors
- **Imports**: Group stdlib → external crates → internal crates (`use crate::`) with blank lines between; sort alphabetically within groups
- **Naming**: `snake_case` for functions/vars, `PascalCase` for types/structs, lowercase module names matching file names
- **Types**: Prefer explicit types on function signatures; rely on inference for local bindings; use `Type` enum from typechecker
- **Error handling**: Propagate with `?`; use `bail!` for early returns with context; avoid `.unwrap()` except in tests
- **Comments**: Document non-obvious design decisions; avoid stating what code does—explain _why_
- **Tests**: Place integration tests in `tea-compiler/tests/feature_name.rs`; use snapshot testing for diagnostics; validate AOT lowering via emitted LLVM IR when necessary
- **Tea language**: 2-space indent, `snake_case` names, terminate blocks with `end`, no semicolons, backticks for string interpolation

## Repository Structure

- `tea-cli/`: CLI binary (`src/main.rs`)
- `tea-compiler/`: Pipeline stages (`lexer/`, `parser/`, `runtime/`, `aot/`)
- `tea-runtime/`: Runtime helpers used by compiled binaries
- `examples/`: Executable `.tea` samples grouped by topic
