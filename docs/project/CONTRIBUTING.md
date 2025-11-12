# Contributing to Tea Language

Thank you for your interest in contributing to Tea! This document provides guidelines and instructions for contributing to the project.

## Getting Started

### Prerequisites

Before you begin, ensure you have:

- **Rust** (1.70+) - [Install from rustup.rs](https://rustup.rs)
- **Bun** - [Install from bun.sh](https://bun.sh)
- **Make** - Usually pre-installed on macOS/Linux
- **LLVM** (optional but recommended) - For AOT compilation testing
- **Git** - For version control

### Setting Up Your Development Environment

1. **Fork the repository** on GitHub

2. **Clone your fork**:

   ```bash
   git clone https://github.com/YOUR_USERNAME/tea-lang
   cd tea-lang
   ```

3. **Add upstream remote**:

   ```bash
   git remote add upstream https://github.com/bentruyman/tea-lang
   ```

4. **Install dependencies and build**:

   ```bash
   make setup
   make build
   ```

5. **Run tests to verify setup**:
   ```bash
   make test
   ```

## Development Workflow

### Making Changes

1. **Create a new branch** from `main`:

   ```bash
   git checkout main
   git pull upstream main
   git checkout -b feature/your-feature-name
   ```

2. **Make your changes** following our coding standards (see below)

3. **Test your changes**:

   ```bash
   cargo test --workspace
   scripts/e2e.sh
   ```

4. **Format your code**:

   ```bash
   cargo fmt --all
   cargo run -p tea-cli -- fmt .
   ```

5. **Commit your changes**:

   ```bash
   git add .
   git commit -m "Add feature: your feature description"
   ```

6. **Push to your fork**:

   ```bash
   git push origin feature/your-feature-name
   ```

7. **Create a Pull Request** on GitHub

### Coding Standards

We follow the guidelines in [AGENTS.md](../AGENTS.md). Key points:

#### Rust Code Style

- Use `cargo fmt` for all Rust code (enforced by CI)
- Follow standard Rust naming conventions:
  - `snake_case` for functions and variables
  - `PascalCase` for types and structs
  - Lowercase for module names
- Use `anyhow::Result` for fallible functions
- Use `thiserror` for custom error types
- Avoid `.unwrap()` except in tests

#### Imports Organization

Group imports with blank lines between groups:

```rust
use std::path::PathBuf;

use anyhow::Result;
use thiserror::Error;

use crate::compiler::Compiler;
use crate::diagnostics::Diagnostic;
```

#### Tea Language Style

- 2-space indentation
- `snake_case` for identifiers
- Terminate blocks with `end`
- No semicolons
- Backticks for string interpolation

#### Comments and Documentation

- Document non-obvious design decisions
- Explain **why**, not just **what**
- Add doc comments for public APIs
- Keep comments up to date with code changes

### Testing

#### Running Tests

```bash
# All tests
make test

# Specific package
cargo test -p tea-compiler

# Specific test
cargo test -p tea-compiler test_name

# E2E tests
scripts/e2e.sh
```

#### Writing Tests

- Place integration tests in `tea-compiler/tests/`
- Use snapshot testing for diagnostics
- Test both success and failure cases
- Add tests for bug fixes to prevent regressions

Example test:

```rust
#[test]
fn test_feature_name() {
    let source = r#"
        var x = 42
        print(x)
    "#;

    let result = compile_and_run(source);
    assert!(result.is_ok());
}
```

### Commit Messages

Write clear, descriptive commit messages:

```
Add feature: string interpolation in templates

- Implement lexer support for template literals
- Add parser rules for interpolated expressions
- Update typechecker to validate interpolations
- Add tests for edge cases

Fixes #123
```

**Format:**

- First line: Brief summary (50 chars or less)
- Blank line
- Detailed explanation if needed
- Reference related issues

### Pull Request Guidelines

1. **One feature per PR** - Keep changes focused
2. **Update documentation** - If you change behavior, update docs
3. **Add tests** - New features need test coverage
4. **Pass CI checks** - All tests must pass
5. **Respond to feedback** - Be open to suggestions

#### PR Description Template

```markdown
## Description

Brief description of what this PR does

## Motivation

Why is this change needed?

## Changes

- List of specific changes made

## Testing

How was this tested?

## Checklist

- [ ] Tests pass locally
- [ ] Code follows style guidelines
- [ ] Documentation updated
- [ ] CHANGELOG.md updated (if needed)
```

## Project Structure

Understanding the codebase:

```
tea-cli/          # Command-line interface
tea-compiler/     # Compiler pipeline
  src/
    lexer/        # Tokenization
    parser/       # AST generation
    resolver.rs   # Symbol resolution
    typechecker.rs # Type checking
    aot/          # LLVM code generation
tea-runtime/      # Runtime support library
tea-lsp/          # Language Server Protocol
tea-intrinsics/   # Native intrinsic functions
spec/             # Language specification
examples/         # Example programs
docs/             # Documentation
```

## Areas for Contribution

### Good First Issues

Look for issues labeled `good first issue` in the GitHub issue tracker. These are:

- Well-defined problems
- Limited scope
- Good for newcomers

### Priority Areas

1. **Documentation**
   - Tutorials and guides
   - API documentation
   - Example programs

2. **Standard Library**
   - New modules (HTTP, regex, etc.)
   - Additional utilities
   - Performance improvements

3. **Compiler Optimizations**
   - LLVM IR improvements
   - Type inference enhancements
   - Error message clarity

4. **Tooling**
   - IDE integrations
   - Debugger support
   - Build tool improvements

5. **Testing**
   - Test coverage
   - Edge case testing
   - Performance benchmarks

## Reporting Bugs

### Before Reporting

1. Check existing issues
2. Verify it's reproducible
3. Test with the latest version

### Bug Report Template

````markdown
**Description**
Clear description of the bug

**To Reproduce**

1. Step one
2. Step two
3. See error

**Expected Behavior**
What should happen

**Actual Behavior**
What actually happens

**Code Sample**

```tea
# Minimal example that reproduces the issue
```
````

**Environment**

- OS: [e.g., macOS 14.0]
- Tea version: [e.g., 0.1.0]
- Rust version: [e.g., 1.75.0]

```

## Suggesting Features

We welcome feature suggestions! Please:

1. **Check existing issues** - Your idea might already be proposed
2. **Describe the use case** - Why is this feature needed?
3. **Provide examples** - Show how it would work
4. **Consider alternatives** - Are there other ways to solve this?

## Code Review Process

1. Maintainers review PRs as time permits
2. Feedback may include requested changes
3. Once approved, a maintainer will merge
4. PRs may be closed if inactive for 30+ days

## Community Guidelines

- Be respectful and inclusive
- Welcome newcomers
- Give constructive feedback
- Assume good intentions

## Questions?

- Open a [GitHub Discussion](https://github.com/bentruyman/tea-lang/discussions)
- Ask in issues with the `question` label
- Refer to documentation in `docs/`

## License

By contributing, you agree that your contributions will be licensed under the MIT License.

---

**Thank you for contributing to Tea Language!** 🍵
```
