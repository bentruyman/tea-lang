#!/usr/bin/env bash
# Tea Language Installer
# 
# This script installs Tea by building from source.
# It checks for required dependencies and guides you through the process.

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
TEA_REPO="${TEA_REPO:-https://github.com/bentruyman/tea-lang}"
TEA_BRANCH="${TEA_BRANCH:-main}"
INSTALL_DIR="${HOME}/.cargo/bin"

# Helper functions
log_info() {
    echo -e "${BLUE}==>${NC} $1"
}

log_success() {
    echo -e "${GREEN}✓${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}!${NC} $1"
}

log_error() {
    echo -e "${RED}✗${NC} $1"
}

command_exists() {
    command -v "$1" >/dev/null 2>&1
}

check_dependency() {
    local cmd=$1
    local name=$2
    local install_instructions=$3
    
    if command_exists "$cmd"; then
        log_success "$name is installed"
        return 0
    else
        log_error "$name is not installed"
        echo "  Install instructions: $install_instructions"
        return 1
    fi
}

detect_os() {
    case "$(uname -s)" in
        Darwin*)    echo "macos" ;;
        Linux*)     echo "linux" ;;
        *)          echo "unknown" ;;
    esac
}

# Main installation process
main() {
    echo ""
    echo "╔══════════════════════════════════════════╗"
    echo "║     Tea Language Installer              ║"
    echo "╚══════════════════════════════════════════╝"
    echo ""
    
    # Detect OS
    OS=$(detect_os)
    log_info "Detected OS: $OS"
    
    if [ "$OS" = "unknown" ]; then
        log_error "Unsupported operating system"
        log_info "Tea currently supports macOS and Linux"
        exit 1
    fi
    
    echo ""
    log_info "Checking dependencies..."
    echo ""
    
    # Check for required dependencies
    local deps_ok=true
    
    # Check Rust/Cargo
    if [ "$OS" = "macos" ]; then
        check_dependency "cargo" "Rust/Cargo" "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh" || deps_ok=false
    else
        check_dependency "cargo" "Rust/Cargo" "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh" || deps_ok=false
    fi
    
    # Check Bun
    if [ "$OS" = "macos" ]; then
        check_dependency "bun" "Bun" "curl -fsSL https://bun.sh/install | bash" || deps_ok=false
    else
        check_dependency "bun" "Bun" "curl -fsSL https://bun.sh/install | bash" || deps_ok=false
    fi
    
    # Check for make
    check_dependency "make" "Make" "Install build-essential (Linux) or Xcode Command Line Tools (macOS)" || deps_ok=false
    
    # Check for LLVM (optional but recommended)
    if ! command_exists "llvm-config"; then
        log_warning "LLVM is not detected (optional, but recommended for AOT compilation)"
        if [ "$OS" = "macos" ]; then
            echo "  Install with: brew install llvm"
        else
            echo "  Install with: apt-get install llvm-dev (Ubuntu/Debian) or yum install llvm-devel (RHEL/CentOS)"
        fi
    else
        log_success "LLVM is installed"
    fi
    
    echo ""
    
    if [ "$deps_ok" = false ]; then
        log_error "Missing required dependencies"
        echo ""
        echo "Please install the missing dependencies and run this script again."
        echo "For more information, visit: https://github.com/bentruyman/tea-lang#installation"
        exit 1
    fi
    
    log_success "All required dependencies are installed!"
    echo ""
    
    # Determine if we're in the Tea repo or need to clone
    if [ -f "Cargo.toml" ] && grep -q "tea-lang" Cargo.toml 2>/dev/null; then
        log_info "Installing from current directory..."
        INSTALL_FROM_PWD=true
    else
        log_info "Cloning Tea repository..."
        TEMP_DIR=$(mktemp -d)
        cd "$TEMP_DIR"
        
        if ! git clone --depth 1 --branch "$TEA_BRANCH" "$TEA_REPO" tea-lang; then
            log_error "Failed to clone repository"
            exit 1
        fi
        
        cd tea-lang
        INSTALL_FROM_PWD=false
    fi
    
    echo ""
    log_info "Setting up build environment..."
    
    # Run setup
    if ! make setup; then
        log_error "Setup failed"
        exit 1
    fi
    
    log_success "Setup complete"
    echo ""
    
    log_info "Building Tea (this may take a few minutes)..."
    
    # Build in release mode
    if ! cargo build --release --workspace; then
        log_error "Build failed"
        exit 1
    fi
    
    log_success "Build complete"
    echo ""
    
    # Install binaries
    log_info "Installing binaries to $INSTALL_DIR..."
    
    # Ensure install directory exists
    mkdir -p "$INSTALL_DIR"
    
    # Copy binaries
    if ! cp target/release/tea-cli "$INSTALL_DIR/tea"; then
        log_error "Failed to install tea binary"
        exit 1
    fi
    
    if ! cp target/release/tea-lsp "$INSTALL_DIR/tea-lsp"; then
        log_error "Failed to install tea-lsp binary"
        exit 1
    fi
    
    # Make executable
    chmod +x "$INSTALL_DIR/tea"
    chmod +x "$INSTALL_DIR/tea-lsp"
    
    log_success "Binaries installed"
    echo ""
    
    # Verify installation
    log_info "Verifying installation..."
    
    if command_exists tea; then
        TEA_VERSION=$(tea --version 2>&1 || echo "unknown")
        log_success "Tea installed successfully: $TEA_VERSION"
    else
        log_warning "Tea binary installed but not found in PATH"
        echo ""
        echo "Add $INSTALL_DIR to your PATH:"
        echo "  export PATH=\"$INSTALL_DIR:\$PATH\""
        echo ""
        echo "Add this line to your shell profile (~/.bashrc, ~/.zshrc, etc.)"
    fi
    
    # Clean up if we cloned
    if [ "$INSTALL_FROM_PWD" = false ]; then
        log_info "Cleaning up temporary files..."
        cd /
        rm -rf "$TEMP_DIR"
    fi
    
    echo ""
    echo "╔══════════════════════════════════════════╗"
    echo "║     Installation Complete! 🎉           ║"
    echo "╚══════════════════════════════════════════╝"
    echo ""
    echo "Get started:"
    echo "  tea --help                    # Show help"
    echo "  tea examples/language/basics/basics.tea  # Run an example"
    echo "  tea build myprogram.tea       # Compile to native binary"
    echo ""
    echo "Learn more:"
    echo "  https://github.com/bentruyman/tea-lang"
    echo ""
}

# Run main function
main "$@"
