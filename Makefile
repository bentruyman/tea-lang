.PHONY: help setup codegen codegen-highlights codegen-ast test build fmt install vendor vendor-check

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
	@echo "  install             Install tea and tea-lsp to ~/.cargo/bin"
	@echo "  vendor              Build vendored LLVM + runtime for current platform"
	@echo "  vendor-check        Check if vendored artifacts exist"
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
	@echo "Running Rust tests (excluding LLVM AOT features)..."
	@cargo test --workspace --exclude tea-llvm-vendor --no-default-features
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

install:
	@echo "Building release binaries..."
	@cargo build --release
	@echo "Installing to ~/.cargo/bin..."
	@cp target/release/tea-cli ~/.cargo/bin/tea
	@cp target/release/tea-lsp ~/.cargo/bin/tea-lsp
	@echo "Verifying installation..."
	@tea --version

vendor:
	@echo "Detecting platform..."
	@uname_s=$$(uname -s); \
	uname_m=$$(uname -m); \
	if [ "$$uname_s" = "Darwin" ] && [ "$$uname_m" = "arm64" ]; then \
		echo "Platform: macOS arm64"; \
		echo "Building vendored LLVM + runtime artifacts..."; \
		./scripts/llvm/build-all-macos-arm64.sh; \
	elif [ "$$uname_s" = "Darwin" ] && [ "$$uname_m" = "x86_64" ]; then \
		echo "Platform: macOS x86_64"; \
		echo "ERROR: macOS x86_64 vendoring not yet supported"; \
		echo "See docs/how-to/building-vendor-artifacts.md for supported platforms"; \
		exit 1; \
	elif [ "$$uname_s" = "Linux" ]; then \
		echo "Platform: Linux $$uname_m"; \
		echo "ERROR: Linux vendoring not yet supported"; \
		echo "See docs/how-to/building-vendor-artifacts.md for supported platforms"; \
		exit 1; \
	else \
		echo "Platform: $$uname_s $$uname_m"; \
		echo "ERROR: Unsupported platform"; \
		echo "See docs/how-to/building-vendor-artifacts.md for supported platforms"; \
		exit 1; \
	fi

vendor-check:
	@echo "Checking for vendored artifacts..."
	@uname_s=$$(uname -s); \
	uname_m=$$(uname -m); \
	if [ "$$uname_s" = "Darwin" ] && [ "$$uname_m" = "arm64" ]; then \
		if [ -d "tea-llvm-vendor/install-macos-arm64/lib" ] && \
		   [ -f "tea-llvm-vendor/runtime-artifacts-macos-arm64/libtea_runtime.a" ] && \
		   [ -f "tea-llvm-vendor/runtime-artifacts-macos-arm64/entry_stub.o" ]; then \
			echo "✓ Vendored artifacts found for macOS arm64"; \
			echo "  LLVM libs: tea-llvm-vendor/install-macos-arm64/lib/"; \
			echo "  Runtime:   tea-llvm-vendor/runtime-artifacts-macos-arm64/"; \
			exit 0; \
		else \
			echo "✗ Vendored artifacts NOT found for macOS arm64"; \
			echo "  Run 'make vendor' to build them"; \
			exit 1; \
		fi; \
	else \
		echo "Platform: $$uname_s $$uname_m"; \
		echo "Vendoring only supported for macOS arm64 currently"; \
		exit 1; \
	fi
