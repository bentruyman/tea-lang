# Tea Language Documentation

This directory contains all documentation for the Tea programming language.

## 📖 Language Documentation

Organized using the [Diátaxis](https://diataxis.fr/) framework for clarity.

### Tutorials

Step-by-step lessons for learning Tea concepts from scratch.

_Coming soon: Getting Started with Tea, Building Your First CLI Tool_

### How-To Guides

Task-focused guides for specific goals.

- [Setting up the Tea LSP](how-to/lsp-setup.md)

### Reference

Technical specifications and API documentation.

- [Language Semantics](reference/language/semantics.md)
- [Type System](reference/language/type-system.md)
- [Tea Stdlib Design](reference/language/tea-stdlib-design.md)
- [Standard Library Reference](stdlib-reference.md)
- [Intrinsics Reference](intrinsics-reference.md)

### Explanation

Architecture decisions, design rationale, and deep dives.

- [Compiler Architecture](explanation/aot-backend.md)
- [LLVM Implementation](explanation/aot-llvm-implementation.md)
- [Compiler Code Generation](explanation/compiler-codegen.md)

## 🤝 Project Documentation

Documentation for contributors and maintainers.

### [`project/`](project/) - Project Management

- [Contributing Guide](project/CONTRIBUTING.md) - How to contribute to Tea
- [Changelog](project/CHANGELOG.md) - Version history and release notes

### [`maintenance/`](maintenance/) - Maintainer Resources

- [Release Checklist](maintenance/RELEASE_CHECKLIST.md) - Release process steps
- [Beta Release TODO](maintenance/BETA_RELEASE_TODO.md) - Current release status
- [Maintainer Guide](maintenance/maintainer-guide.md) - Extending Tea with intrinsics/stdlib
- [Domain Setup](maintenance/DOMAINS.md) - tea-lang.com/dev setup guide
- [Domain Quickstart](maintenance/QUICKSTART-DOMAINS.md) - Fast domain configuration
- [New Stdlib](maintenance/new-stdlib.md) - Standard library development

## External Resources

- [Tea Specification](../spec/) - Canonical grammar, AST, and token definitions
- [Examples](../examples/) - Executable Tea programs demonstrating language features
