#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

BLUE='\033[0;34m'
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

log_info() {
  echo -e "${BLUE}==>${NC} $1"
}

log_success() {
  echo -e "${GREEN}✓${NC} $1"
}

log_warn() {
  echo -e "${YELLOW}!${NC} $1"
}

log_error() {
  echo -e "${RED}✗${NC} $1" >&2
}

require_repo_root() {
  if [[ ! -f "${REPO_ROOT}/Cargo.toml" || ! -d "${REPO_ROOT}/tea-cli" || ! -d "${REPO_ROOT}/www" ]]; then
    log_error "scripts/setup-worktree.sh must live inside the tea-lang repo"
    exit 1
  fi

  cd "${REPO_ROOT}"
}

require_command() {
  local command_name="$1"
  local install_hint="$2"

  if command -v "${command_name}" >/dev/null 2>&1; then
    log_success "Found ${command_name}"
    return 0
  fi

  log_error "Missing required command: ${command_name}"
  echo "  Install: ${install_hint}" >&2
  exit 1
}

configure_llvm() {
  local llvm_prefix=""
  local llvm_config_path=""
  local install_hint=""

  case "$(uname -s)" in
    Darwin)
      install_hint="brew install llvm@17"
      ;;
    Linux)
      install_hint="sudo apt-get install llvm-17-dev libpolly-17-dev"
      ;;
    *)
      install_hint="Install LLVM 17 and ensure llvm-config is on PATH"
      ;;
  esac

  if command -v brew >/dev/null 2>&1; then
    local brewed_prefix_17
    brewed_prefix_17="$(brew --prefix llvm@17 2>/dev/null || true)"
    if [[ -n "${brewed_prefix_17}" && -x "${brewed_prefix_17}/bin/llvm-config" ]]; then
      llvm_prefix="${brewed_prefix_17}"
      llvm_config_path="${brewed_prefix_17}/bin/llvm-config"
      export PATH="${brewed_prefix_17}/bin:${PATH}"
      log_info "Using Homebrew LLVM 17 from ${brewed_prefix_17}"
    fi
  fi

  if [[ -z "${llvm_config_path}" ]] && command -v llvm-config >/dev/null 2>&1; then
    llvm_config_path="$(command -v llvm-config)"
    llvm_prefix="$(cd "$(dirname "${llvm_config_path}")/.." && pwd)"
  elif [[ -z "${llvm_config_path}" ]] && command -v brew >/dev/null 2>&1; then
    local brewed_prefix
    brewed_prefix="$(brew --prefix llvm 2>/dev/null || true)"
    if [[ -n "${brewed_prefix}" && -x "${brewed_prefix}/bin/llvm-config" ]]; then
      llvm_prefix="${brewed_prefix}"
      llvm_config_path="${brewed_prefix}/bin/llvm-config"
      export PATH="${brewed_prefix}/bin:${PATH}"
      log_info "Using Homebrew LLVM from ${brewed_prefix}"
    fi
  fi

  if [[ -z "${llvm_config_path}" || ! -x "${llvm_config_path}" ]]; then
    log_error "LLVM 17 is required for this repository"
    echo "  Install with: ${install_hint}" >&2
    exit 1
  fi

  local llvm_version
  llvm_version="$("${llvm_config_path}" --version)"
  if [[ "${llvm_version}" != 17.* ]]; then
    log_error "Found LLVM ${llvm_version}, but this repo requires LLVM 17.x"
    echo "  Install with: ${install_hint}" >&2
    exit 1
  fi

  export LLVM_SYS_170_PREFIX="${llvm_prefix}"
  export LLVM_CONFIG_PATH="${llvm_config_path}"
  log_success "Using LLVM ${llvm_version} (${LLVM_CONFIG_PATH})"
}

main() {
  require_repo_root

  log_info "Validating development prerequisites..."
  require_command "bun" "curl -fsSL https://bun.sh/install | bash"
  require_command "cargo" "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
  require_command "rustc" "rustup toolchain install stable"
  require_command "make" "Install make (or Xcode Command Line Tools on macOS)"
  configure_llvm

  if [[ -d "${REPO_ROOT}/www" && ! -f "${REPO_ROOT}/www/package.json" ]]; then
    log_error "Expected docs app at ${REPO_ROOT}/www"
    exit 1
  fi

  log_info "Running root setup (Bun workspace install + codegen)..."
  make setup

  log_info "Prefetching Cargo dependencies..."
  cargo fetch --locked

  log_info "Installing docs site dependencies..."
  bun install --cwd www --frozen-lockfile

  echo ""
  log_success "Worktree bootstrap complete"
  echo "Next steps:"
  echo "  cargo build -p tea-cli"
  echo "  ./scripts/e2e.sh"
  echo "  bun run --cwd www dev"
}

main "$@"
