# Tea Standard Library

This directory contains the Tea standard library, written in Tea itself and built on top of native intrinsics.

## Structure

- `assert/` - Assertion helpers for testing
- `env/` - Environment variable access
- `fs/` - Filesystem operations
- `json/` - JSON encoding/decoding
- `path/` - Path manipulation utilities
- `string/` - String manipulation utilities

## Module Structure

Each module has a `mod.tea` file that exports the module's public API. Functions are documented with comments that serve as the official documentation.

## Native Intrinsics

Functions prefixed with `__intrinsic_` are implemented in Rust and provide low-level functionality. These are not meant to be called directly by user code; instead, the standard library wraps them with ergonomic APIs.

See `docs/reference/language/intrinsics.md` for the complete list of intrinsics.

## Building

The standard library is compiled during the build process (via `build.rs`) and embedded into the `tea-cli` binary as a snapshot. This ensures that compiled Tea programs always have access to the standard library without requiring external files.

## Development

When developing the stdlib:

1. Make changes to `.tea` files in this directory
2. Run `cargo build` to recompile the stdlib snapshot
3. Test with `cargo test --workspace`

Use the `TEA_STDLIB_PATH` environment variable to load stdlib from disk during development instead of the embedded snapshot.
