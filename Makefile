.PHONY: help setup codegen test build fmt install release

INSTALL_DIR ?= $(HOME)/.local/bin
RUST_ENV := ./scripts/with-rust-toolchain.sh

help:
	@echo "Tea Language Build Tasks"
	@echo ""
	@echo "  setup               First-time setup (install deps + codegen)"
	@echo "  codegen             Generate all code from grammar/AST specs"
	@echo "  test                Run all tests"
	@echo "  build               Build all components"
	@echo "  fmt                 Format all code"
	@echo "  install             Install tea to $(INSTALL_DIR)"
	@echo "  release             Update release versions locally (GitHub Release workflow is preferred)"
	@echo ""

setup:
	@echo "Installing dependencies with Bun..."
	@bun install --frozen-lockfile
	@echo "Generating code from specifications..."
	@$(MAKE) codegen
	@echo ""
	@echo "✓ Setup complete! You can now run 'make build' or 'cargo build'"

codegen:
	@echo "Generating tree-sitter highlights..."
	@bun scripts/codegen-highlights.js
	@echo "Generating Rust AST..."
	@$(RUST_ENV) bun scripts/codegen-ast.js

test: codegen
	@echo "Running Rust tests..."
	@$(RUST_ENV) cargo test --workspace
	@echo "Running end-to-end tests..."
	@./scripts/e2e.sh

build: codegen
	@echo "Building compiler and CLI..."
	@$(RUST_ENV) cargo build --workspace

fmt:
	@echo "Formatting Rust code..."
	@$(RUST_ENV) cargo fmt --all
	@echo "Formatting Tea code..."
	@$(RUST_ENV) cargo run -p tea-cli -- fmt .
	@echo "Formatting with Prettier..."
	@npx prettier --write .

install: codegen
	@echo "Building release binary..."
	@$(RUST_ENV) ./scripts/build-bundled-tea.sh
	@echo "Installing to $(INSTALL_DIR)..."
	@mkdir -p $(INSTALL_DIR)
	@cp target/release/tea $(INSTALL_DIR)/tea
	@echo "Verifying installation..."
	@$(INSTALL_DIR)/tea --version

release:
	@if [ -z "$(VERSION)" ]; then echo "Usage: make release VERSION=0.0.1"; exit 1; fi
	@bun scripts/release.mjs prepare $(if $(DRY_RUN),--dry-run,) "$(VERSION)"
