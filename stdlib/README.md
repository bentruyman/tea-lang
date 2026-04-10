# Tea Standard Library

This directory contains the Tea standard library, written in Tea itself and built on top of native intrinsics.

## Structure

- `assert/` - Assertion helpers for testing
- `args/` - Command-line argument helpers
- `env/` - Environment variable access
- `fs/` - Filesystem operations
- `path/` - Path manipulation utilities
- `process/` - Subprocess execution helpers
- `regex/` - Regular-expression helpers
- `string/` - String manipulation utilities

## Module Structure

Each module has a `mod.tea` file that exports the module's public API. Functions are documented with comments that serve as the official documentation.

## Native Intrinsics

Functions prefixed with `__intrinsic_` are implemented in Rust and provide low-level functionality. These are not meant to be called directly by user code; standard library modules wrap them with the supported public APIs.

See `docs/reference/language/intrinsics.md` for the complete list of intrinsics.

## Development

When developing the stdlib:

1. Make changes to `.tea` files in this directory
2. Run `cargo test --workspace` or targeted `tea` compiler checks
3. Update docs and reference artifacts for any public API changes
