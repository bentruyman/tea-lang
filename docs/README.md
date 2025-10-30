# Tea Language Documentation

This directory contains all documentation for the Tea programming language, organized using the [Diátaxis](https://diataxis.fr/) framework.

## Structure

### Tutorials

Step-by-step lessons for learning Tea concepts from scratch.

_Coming soon: Getting Started with Tea, Building Your First CLI Tool_

### How-To Guides

Task-focused guides for specific goals.

- [Setting up the Tea LSP](how-to/lsp-setup.md)
- [Building Vendor Artifacts](how-to/building-vendor-artifacts.md)
- [Single Binary Usage](how-to/single-binary-usage.md)

### Reference

Technical specifications and API documentation.

- [Language Semantics](reference/language/semantics.md)
- [Type System](reference/language/type-system.md)

### Explanation

Architecture decisions, design rationale, and deep dives.

- [AOT Backend Architecture](explanation/aot-backend.md)
- [Compiler Code Generation](explanation/compiler-codegen.md)
- [Zero-Dependency Implementation](explanation/zero-dependency-implementation.md)
- [Static LLVM Embedding](explanation/static-llvm-embedding.md)

### Roadmap

Project planning and feature roadmaps.

- [Project Roadmap](roadmap/project-roadmap.md)
- [CLI & Standard Library Roadmap](roadmap/cli-stdlib.md)

### Migrations

Version migration guides and breaking change documentation.

- [AST Migration Guide](migrations/ast-migration.md)

### RFCs

Proposals and technical plans for major features.

- [LLVM AOT Backend Plan](rfcs/aot-llvm-plan.md)

## Contributing to Documentation

### File Naming Conventions

- Use `kebab-case` for all filenames: `lsp-setup.md`, not `LSP_SETUP.md`
- Match the H1 heading to the filename in Title Case
- First paragraph should state the document's scope and purpose

### Organization Guidelines

- **Tutorials**: Learning-oriented, step-by-step, safe to follow
- **How-To**: Problem-oriented, practical steps, assumes knowledge
- **Reference**: Information-oriented, accurate, complete
- **Explanation**: Understanding-oriented, context, alternatives
- **Roadmap**: Future plans, timelines, priorities
- **Migrations**: Version transitions, breaking changes
- **RFCs**: Proposals, design discussions, not-yet-implemented

### Writing Style

- Keep "why" explanations in Explanation docs
- Keep "what" specifications in Reference docs
- Keep "how" instructions in How-To and Tutorial docs
- Link between docs when referencing related concepts

## External Resources

- [Tea Specification](../spec/) - Canonical grammar, AST, and token definitions
- [Examples](../examples/) - Executable Tea programs demonstrating language features
