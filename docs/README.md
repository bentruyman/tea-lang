# Tea Language Documentation

This directory contains all documentation for the Tea programming language, organized using the [Diátaxis](https://diataxis.fr/) framework.

## Structure

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
- [Maintainer Guide](maintainer-guide.md) - How to extend Tea with new intrinsics and stdlib functions

### Explanation

Architecture decisions, design rationale, and deep dives.

- [AOT Backend Architecture](explanation/aot-backend.md)
- [LLVM AOT Implementation](explanation/aot-llvm-implementation.md)
- [Compiler Code Generation](explanation/compiler-codegen.md)

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

### Writing Style

- Keep "why" explanations in Explanation docs
- Keep "what" specifications in Reference docs
- Keep "how" instructions in How-To and Tutorial docs
- Link between docs when referencing related concepts

## External Resources

- [Tea Specification](../spec/) - Canonical grammar, AST, and token definitions
- [Examples](../examples/) - Executable Tea programs demonstrating language features
