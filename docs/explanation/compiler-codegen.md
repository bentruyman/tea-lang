# Compiler Code Generation

Tea uses a specification-driven approach where language structure is defined in machine-readable formats and code is generated for different consumers (tree-sitter, compiler, LSP).

## Specification Files

### `spec/grammar.ebnf`

Canonical EBNF grammar defining Tea's syntax. This serves as:

- Documentation for language syntax
- Reference for implementing parsers
- Basis for railroad diagrams and language tutorials

### `spec/ast.yaml`

Complete AST node schema matching `tea-compiler/src/ast.rs`. Includes:

- All node types (statements, expressions, patterns)
- Field names and types
- Derive macros
- Documentation comments

This is the single source of truth for AST structure.

### `spec/tokens.toml`

Token definitions including:

- **Keywords** - with semantic types and contextual usage
- **Operators** - with precedence and associativity
- **Punctuation** - brackets, delimiters
- **Literals** - true, false, nil
- **Builtins** - Func, List, Dict, etc.
- **Semantic token mappings** - for LSP
- **Tree-sitter mappings** - for syntax highlighting

## Generated Files

### `tree-sitter-tea/queries/highlights.scm`

Generated from `spec/tokens.toml` by `scripts/codegen-highlights.js`.

Contains tree-sitter query patterns for syntax highlighting:

- Safe keywords (in top-level array)
- Contextual keywords (in node-specific queries to avoid conflicts with tree-sitter internal nodes)
- Node-type based captures (functions, types, properties, etc.)

**Important:** This file handles the tree-sitter ABI 14 requirement for Neovim 0.11 compatibility.

### `tea-compiler/src/ast.rs`

Generated from `spec/ast.yaml` by `scripts/codegen-ast.js`.

This is the **primary AST** used by the entire compiler, parser, typechecker, and runtime. It includes:

- All statement and expression node types
- Type annotations, patterns, and operators
- Implementation methods for SourceSpan and Module
- Complete derive macros for each type

**Important:** This file is generated and should not be edited directly. All changes should be made to `spec/ast.yaml`.

## Running Code Generation

```bash
# Generate all files
make codegen
# or
bun run codegen

# Generate specific files
bun run codegen:highlights
bun run codegen:ast
```

## Development Workflow

1. **Adding a new keyword:**
   - Add to `spec/tokens.toml` under `[keywords]`
   - Add to `[tree_sitter]` section (safe or contextual)
   - Run `bun run codegen:highlights`
   - Regenerate tree-sitter parser with ABI 14: `cd tree-sitter-tea && bunx tree-sitter generate --abi 14`

2. **Adding a new AST node:**
   - Add to `spec/ast.yaml` under `nodes`
   - Run `bun run codegen:ast`
   - Verify output in `tea-compiler/src/ast.rs` matches intended structure
   - Run `cargo test` to ensure no regressions

3. **Updating grammar:**
   - Update `spec/grammar.ebnf`
   - Update tree-sitter grammar in `tree-sitter-tea/grammar.js`
   - Update parser in `tea-compiler/src/parser/`
   - Regenerate tree-sitter with `npx tree-sitter generate --abi 14`

## Git Workflow

Generated files are **not tracked** in git:

- `tree-sitter-tea/queries/highlights.scm`
- `tea-compiler/src/ast.rs`

After cloning the repo:

```bash
make setup    # Installs deps with Bun and runs codegen
make build    # Builds the project (runs codegen automatically)
```

Or manually:

```bash
bun install
make codegen
cargo build
```

## Tree-sitter and Neovim

The generated `highlights.scm` includes special handling for keywords that conflict with tree-sitter internal node types (`error`, `throw`, `try`, `catch`, `case`).

**Neovim setup:**

1. Generate parser with tree-sitter-cli 0.22.x and `--abi 14` flag
2. Compile to shared library: `cc -o tea.so -shared src/parser.c -I./src -fPIC`
3. Install: `cp tea.so ~/.local/share/nvim/site/parser/tea.so`
4. Install queries: `cp queries/*.scm ~/.local/share/nvim/site/queries/tea/`

Or use the install script: `cd tree-sitter-tea && ./install-nvim.sh`

## Future Enhancements

- Generate LSP semantic token mappings from `spec/tokens.toml`
- Generate railroad diagrams from `spec/grammar.ebnf`
- Generate parser scaffolding from `spec/ast.yaml`
- Add validation that generated `ast.rs` compiles before committing changes to `ast.yaml`
- Generate visitor patterns and AST traversal helpers
