# Changelog

All notable changes to Tea Language will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Preparing for Beta Release

## [0.1.0] - TBD

### Added

#### Language Features

- Static type system with type inference
- Generic types with automatic specialization
- Pattern matching in function parameters
- String interpolation with backticks: `` `Hello, ${name}!` ``
- List and Dictionary collection types
- Struct types for data modeling
- Module system with `use` imports
- Lambda expressions and closures
- For-of loops for iteration
- If-else expressions (expressions, not statements)
- Compound assignment operators (`+=`, `-=`, etc.)

#### Compiler

- Lexer with full token support
- Recursive descent parser
- Type checker with inference
- Resolver for variable scoping and modules
- Code formatter (`tea fmt`)
- Diagnostic system with source location tracking
- AOT compilation to native binaries via LLVM
- Optimized codegen (PHI nodes for loops, SSA for immutables)

#### Standard Library

- `std.assert` - Testing utilities with snapshot support
- `std.env` - Environment variable access
- `std.fs` - File system operations (read, write, directory listing)
- `std.json` - JSON parsing and serialization
- `std.path` - Path manipulation utilities
- `std.string` - String operations

#### CLI Tools

- `tea <script>` - Run Tea scripts directly
- `tea build <script>` - Compile to standalone native binary
- `tea fmt <paths>` - Format Tea source files
- `tea test <paths>` - Run test files (basic support)
- LSP server (`tea-lsp`) for editor integration

#### Development

- Automated installation script for macOS/Linux
- Comprehensive documentation structure (Diátaxis framework)
- CI/CD pipeline with GitHub Actions
- Benchmark suite comparing to Rust and JavaScript
- Tree-sitter grammar for syntax highlighting
- Pre-commit hooks for code quality

### Performance

- Fibonacci benchmark: ~1.17x Rust performance
- Math operations: Optimized integer arithmetic
- Loop performance: SSA-based optimization with PHI nodes
- String operations: Efficient runtime string handling

### Known Limitations

- Windows installation requires manual build (no automated installer yet)
- Test execution in AOT mode is experimental
- Some LLVM optimizations not yet enabled (LTO support planned)
- Documentation is work-in-progress (tutorials coming soon)
- No package manager yet (planned for future release)

### Breaking Changes

None (initial release)

---

## Release Notes Format

### Categories

- **Added**: New features
- **Changed**: Changes in existing functionality
- **Deprecated**: Soon-to-be removed features
- **Removed**: Removed features
- **Fixed**: Bug fixes
- **Security**: Security vulnerability fixes
- **Performance**: Performance improvements

[Unreleased]: https://github.com/bentruyman/tea-lang/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/bentruyman/tea-lang/releases/tag/v0.1.0
