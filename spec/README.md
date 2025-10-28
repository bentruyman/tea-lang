# Tea Language Specification

This directory contains the canonical specifications for the Tea programming language. These files serve as the single source of truth for language structure and are used to generate code for various tools.

## Files

### `grammar.ebnf`

Canonical EBNF grammar defining Tea's syntax.

**Used by:**

- Documentation and language tutorials
- Parser implementation reference
- Future: Railroad diagram generation

### `ast.yaml`

Complete Abstract Syntax Tree node schema.

**Defines:**

- All statement and expression node types
- Field names and types for each node
- Rust derive macros
- Documentation for each node

**Generates:**

- `tea-compiler/src/ast.rs` (via `bun run codegen:ast`)

### `tokens.toml`

Token definitions including keywords, operators, and semantic mappings.

**Defines:**

- Keywords with semantic types
- Operators with precedence
- Punctuation and delimiters
- Tree-sitter highlight mappings
- LSP semantic token types

**Generates:**

- `tree-sitter-tea/queries/highlights.scm` (via `bun run codegen:highlights`)

## Making Changes

When modifying these specifications:

1. Edit the appropriate spec file
2. Run `bun run codegen` to regenerate derived files
3. Verify the build succeeds: `cargo build`
4. Run tests: `cargo test --workspace`

See [docs/explanation/compiler-codegen.md](../docs/explanation/compiler-codegen.md) for detailed workflows.

## Philosophy

These specifications embody the principle that language design decisions should be:

- **Documented once** in a canonical location
- **Machine-readable** for automation
- **Version-controlled** alongside code
- **Validated** through generated tests

This approach ensures consistency across the compiler, tree-sitter grammar, LSP, and documentation.
