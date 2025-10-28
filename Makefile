.PHONY: help setup codegen codegen-highlights codegen-ast test build fmt

help:
	@echo "Tea Language Build Tasks"
	@echo ""
	@echo "  setup               First-time setup (install deps + codegen)"
	@echo "  codegen             Generate all code from grammar/AST specs"
	@echo "  codegen-highlights  Generate tree-sitter highlights.scm from tokens.toml"
	@echo "  codegen-ast         Generate Rust AST from ast.yaml"
	@echo "  test                Run all tests"
	@echo "  build               Build all components"
	@echo "  fmt                 Format all code"
	@echo ""

setup:
	@echo "Installing dependencies with Bun..."
	@bun install
	@echo "Generating code from specifications..."
	@$(MAKE) codegen
	@echo ""
	@echo "✓ Setup complete! You can now run 'make build' or 'cargo build'"

codegen: codegen-highlights codegen-ast

codegen-highlights:
	@echo "Generating tree-sitter highlights..."
	@bun scripts/codegen-highlights.js

codegen-ast:
	@echo "Generating Rust AST..."
	@bun scripts/codegen-ast.js

test: codegen
	@echo "Running Rust tests..."
	@cargo test --workspace
	@echo "Running end-to-end tests..."
	@./scripts/e2e.sh

build: codegen
	@echo "Building compiler and CLI..."
	@cargo build --workspace

fmt:
	@echo "Formatting Rust code..."
	@cargo fmt --all
	@echo "Formatting Tea code..."
	@cargo run -p tea-cli -- fmt .
	@echo "Formatting with Prettier..."
	@npx prettier --write .
