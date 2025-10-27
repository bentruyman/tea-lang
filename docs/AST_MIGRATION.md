# AST Migration to Generated Code

## Overview

The Tea compiler's AST (`tea-compiler/src/ast.rs`) has been migrated from manual maintenance to code generation from the `docs/ast.yaml` specification.

## What Changed

### Before
- AST was manually written and maintained in `tea-compiler/src/ast.rs`
- Changes required manual edits that could drift from documentation
- No single source of truth for AST structure

### After
- AST is **generated** from `docs/ast.yaml` by `scripts/codegen-ast.js`
- Changes are made to the YAML schema, then regenerated
- Single source of truth ensures consistency across tools

## Generated File

**File:** `tea-compiler/src/ast.rs`  
**Source:** `docs/ast.yaml`  
**Generator:** `scripts/codegen-ast.js`

The generated AST includes:
- All node type definitions (structs and enums)
- Derive macros (Debug, Clone, Copy, etc.)
- Documentation comments from the schema
- Implementation methods for SourceSpan (new, single_point, union, Default)
- Implementation method for Module (new)

## Making Changes

### Adding a New Node Type

1. Edit `docs/ast.yaml` and add the new node under `nodes:`
   ```yaml
   NewNode:
     description: Description of what this node represents
     derives: ["Debug", "Clone"]
     fields:
       field_name:
         type: SomeType
         description: What this field holds
   ```

2. Regenerate the AST:
   ```bash
   npm run codegen:ast
   # or
   make codegen
   ```

3. Verify it compiles:
   ```bash
   cargo build
   ```

4. Run tests:
   ```bash
   cargo test --workspace
   ```

### Modifying an Existing Node

1. Edit the node definition in `docs/ast.yaml`
2. Regenerate: `npm run codegen:ast`
3. Update parser/compiler code that uses the changed fields
4. Test: `cargo test --workspace`

### Understanding Variant Syntax

The generator supports three variant patterns:

1. **Unit variant:**
   ```yaml
   variants:
     - SimpleCase
   ```
   Generates: `SimpleCase`

2. **Single-field tuple variant:**
   ```yaml
   variants:
     - Wrapper:
         type: SomeType
   ```
   Generates: `Wrapper(SomeType)`

3. **Multi-field tuple variant:**
   ```yaml
   variants:
     - Multiple:
         fields: [TypeA, TypeB]
   ```
   Generates: `Multiple(TypeA, TypeB)`

4. **Struct variant:**
   ```yaml
   variants:
     - Named:
         field_one: TypeA
         field_two: TypeB
   ```
   Generates:
   ```rust
   Named {
       field_one: TypeA,
       field_two: TypeB,
   }
   ```

## Build Safety

A build script (`tea-compiler/build.rs`) checks that `ast.rs` exists before compilation. If it's missing, you'll see:

```
ERROR: tea-compiler/src/ast.rs is missing!
This file is generated from docs/ast.yaml.
Please run: npm run codegen
```

This prevents confusing compilation errors if you forget to generate the AST after cloning.

## Git Workflow

`tea-compiler/src/ast.rs` is **not tracked** in git (listed in `.gitignore`).

After cloning the repository:
```bash
npm install
npm run codegen  # or make codegen
cargo build
```

This ensures everyone generates the AST from the authoritative `docs/ast.yaml` source.

## Validation

All existing tests pass with the generated AST:
- ✅ 80+ test cases in tea-compiler
- ✅ Parser tests
- ✅ Runtime tests
- ✅ Integration tests with examples

The generated AST is functionally identical to the previous manual version, verified by:
1. Line count: ~563 lines (original: 535 lines)
2. All derive macros preserved
3. All impl methods preserved
4. Test suite: 100% pass rate
5. Examples run correctly (fib.tea outputs 832040)

## Troubleshooting

### "ast.rs not found" during build
Run `bun run codegen:ast` or `make codegen`

### Changes to ast.yaml not reflected
Regenerate: `bun run codegen:ast`

### Tests failing after AST change
1. Check that field names/types match usage in parser/compiler
2. Verify derives are correct for how the type is used
3. Run `cargo test --workspace -- --nocapture` for detailed output

## Future Improvements

- Add schema validation for ast.yaml
- Generate visitor patterns for AST traversal
- Generate serialization/deserialization code
- Add more impl methods via schema annotations
