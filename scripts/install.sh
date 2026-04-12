#!/usr/bin/env bash

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

TEA_GITHUB_REPO="${TEA_GITHUB_REPO:-bentruyman/tea-lang}"
TEA_VERSION="${TEA_VERSION:-}"
TEA_INSTALL_DIR="${TEA_INSTALL_DIR:-${HOME}/.local/bin}"

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

require_command() {
  local command_name="$1"
  local install_hint="$2"

  if command -v "${command_name}" >/dev/null 2>&1; then
    return 0
  fi

  log_error "Missing required command: ${command_name}"
  echo "  Install: ${install_hint}" >&2
  exit 1
}

detect_os() {
  case "$(uname -s)" in
    Darwin) echo "apple-darwin" ;;
    Linux) echo "unknown-linux-gnu" ;;
    *)
      log_error "Unsupported operating system: $(uname -s)"
      exit 1
      ;;
  esac
}

detect_arch() {
  case "$(uname -m)" in
    x86_64|amd64)
      if [[ "$(uname -s)" == "Darwin" ]]; then
        if [[ "$(sysctl -in hw.optional.arm64 2>/dev/null || true)" == "1" ]]; then
          echo "aarch64"
          return 0
        fi
      fi
      echo "x86_64"
      ;;
    arm64|aarch64) echo "aarch64" ;;
    *)
      log_error "Unsupported architecture: $(uname -m)"
      exit 1
      ;;
  esac
}

show_source_build_instructions() {
  local repo_name="${TEA_GITHUB_REPO##*/}"

  echo "  Build from source instead:" >&2
  echo "    git clone https://github.com/${TEA_GITHUB_REPO}" >&2
  echo "    cd ${repo_name}" >&2
  echo "    ./scripts/setup-worktree.sh" >&2
  echo "    make install" >&2
}

host_target() {
  local target
  target="$(detect_arch)-$(detect_os)"

  case "${target}" in
    x86_64-unknown-linux-gnu|aarch64-apple-darwin)
      echo "${target}"
      ;;
    x86_64-apple-darwin)
      log_error "Prebuilt Tea releases do not support Intel macOS"
      echo "  Supported prebuilt targets: x86_64-unknown-linux-gnu, aarch64-apple-darwin" >&2
      show_source_build_instructions
      exit 1
      ;;
    *)
      log_error "No prebuilt Tea release is available for ${target}"
      echo "  Supported prebuilt targets: x86_64-unknown-linux-gnu, aarch64-apple-darwin" >&2
      show_source_build_instructions
      exit 1
      ;;
  esac
}

require_host_linker() {
  if command -v cc >/dev/null 2>&1; then
    return 0
  fi

  case "$(detect_os)" in
    apple-darwin)
      log_error "Tea needs a host C toolchain to run and build Tea programs"
      echo "  Install: xcode-select --install" >&2
      ;;
    unknown-linux-gnu)
      log_error "Tea needs a host C toolchain to run and build Tea programs"
      echo "  Install: sudo apt-get install build-essential clang" >&2
      ;;
  esac
  exit 1
}

resolve_version() {
  if [[ -n "${TEA_VERSION}" ]]; then
    echo "${TEA_VERSION}"
    return 0
  fi

  local api_url="https://api.github.com/repos/${TEA_GITHUB_REPO}/releases/latest"
  local version
  version="$(curl -fsSL "${api_url}" | sed -n 's/.*"tag_name":[[:space:]]*"\([^"]*\)".*/\1/p' | head -n 1)"
  if [[ -z "${version}" ]]; then
    log_error "Failed to resolve the latest Tea release"
    echo "  Set TEA_VERSION explicitly or check ${api_url}" >&2
    exit 1
  fi

  echo "${version}"
}

checksum_file_name() {
  local version="$1"
  echo "tea-${version}-SHA256SUMS.txt"
}

archive_file_name() {
  local version="$1"
  local target="$2"
  echo "tea-${version}-${target}.tar.gz"
}

download_release_asset() {
  local version="$1"
  local asset="$2"
  local output="$3"
  local asset_url="https://github.com/${TEA_GITHUB_REPO}/releases/download/${version}/${asset}"

  log_info "Downloading ${asset}"
  curl --proto '=https' --tlsv1.2 -fLsS "${asset_url}" -o "${output}"
}

current_sha256() {
  local file_path="$1"
  if command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "${file_path}" | awk '{print $1}'
    return 0
  fi

  sha256sum "${file_path}" | awk '{print $1}'
}

verify_checksum() {
  local archive_path="$1"
  local checksums_path="$2"
  local archive_name
  archive_name="$(basename "${archive_path}")"
  local expected
  expected="$(awk -v asset="${archive_name}" '
    {
      file = $2
      sub(/^.*\//, "", file)
      if (file == asset) {
        print $1
      }
    }
  ' "${checksums_path}")"

  if [[ -z "${expected}" ]]; then
    log_error "Checksum entry for ${archive_name} is missing"
    exit 1
  fi

  local actual
  actual="$(current_sha256 "${archive_path}")"
  if [[ "${expected}" != "${actual}" ]]; then
    log_error "Checksum mismatch for ${archive_name}"
    echo "  expected: ${expected}" >&2
    echo "  actual:   ${actual}" >&2
    exit 1
  fi

  log_success "Verified checksum for ${archive_name}"
}

ensure_install_dir() {
  mkdir -p "${TEA_INSTALL_DIR}"
}

install_extracted_payload() {
  local extract_dir="$1"

  if [[ ! -f "${extract_dir}/tea" ]]; then
    log_error "Release archive did not contain a top-level tea binary"
    exit 1
  fi

  install -m 0755 "${extract_dir}/tea" "${TEA_INSTALL_DIR}/tea"

  local dylib_found=0
  for dylib_path in "${extract_dir}"/*.dylib; do
    if [[ ! -f "${dylib_path}" ]]; then
      continue
    fi
    dylib_found=1
    install -m 0644 "${dylib_path}" "${TEA_INSTALL_DIR}/$(basename "${dylib_path}")"
  done

  log_success "Installed tea to ${TEA_INSTALL_DIR}/tea"
  if [[ "${dylib_found}" -eq 1 ]]; then
    log_info "Installed companion runtime libraries to ${TEA_INSTALL_DIR}"
  fi
}

show_path_hint() {
  case ":${PATH}:" in
    *":${TEA_INSTALL_DIR}:"*) return 0 ;;
  esac

  log_warn "${TEA_INSTALL_DIR} is not currently on PATH"
  echo "Add this line to your shell profile:"
  echo "  export PATH=\"${TEA_INSTALL_DIR}:\$PATH\""
}

main() {
  echo ""
  echo "╔══════════════════════════════════════════╗"
  echo "║     Tea Language Installer              ║"
  echo "╚══════════════════════════════════════════╝"
  echo ""

  require_command "curl" "Install curl using your package manager"
  require_command "tar" "Install tar using your package manager"
  require_command "mktemp" "Install mktemp using your package manager"
  require_host_linker

  local version
  version="$(resolve_version)"
  local target
  target="$(host_target)"
  local archive_name
  archive_name="$(archive_file_name "${version}" "${target}")"
  local checksums_name
  checksums_name="$(checksum_file_name "${version}")"

  log_info "Installing Tea ${version} for ${target}"
  log_info "GitHub repository: ${TEA_GITHUB_REPO}"

  local temp_dir
  temp_dir="$(mktemp -d)"
  trap 'rm -rf "${temp_dir}"' EXIT

  local archive_path="${temp_dir}/${archive_name}"
  local checksums_path="${temp_dir}/${checksums_name}"

  download_release_asset "${version}" "${archive_name}" "${archive_path}"
  download_release_asset "${version}" "${checksums_name}" "${checksums_path}"
  verify_checksum "${archive_path}" "${checksums_path}"

  ensure_install_dir
  tar -xzf "${archive_path}" -C "${temp_dir}"
  install_extracted_payload "${temp_dir}"

  "${TEA_INSTALL_DIR}/tea" --version
  show_path_hint

  echo ""
  echo "Tea is ready."
  echo "  tea --help"
  echo "  tea examples/echo/main.tea hello tea"
  echo ""
}

main "$@"
