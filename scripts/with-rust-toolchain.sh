#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

activate_proto() {
  if ! command -v proto >/dev/null 2>&1; then
    return 0
  fi

  if [[ ! -f "${REPO_ROOT}/.prototools" && ! -f "${REPO_ROOT}/.tool-versions" ]]; then
    return 0
  fi

  local proto_exports
  proto_exports="$(cd "${REPO_ROOT}" && proto activate bash --export)"
  if [[ -n "${proto_exports}" ]]; then
    eval "${proto_exports}"
  fi
}

resolve_rust_toolchain() {
  if command -v cargo >/dev/null 2>&1 && command -v rustc >/dev/null 2>&1; then
    return 0
  fi

  if command -v proto >/dev/null 2>&1; then
    local proto_cargo
    proto_cargo="$(cd "${REPO_ROOT}" && proto bin rust 2>/dev/null || true)"
    if [[ -n "${proto_cargo}" && -x "${proto_cargo}" ]]; then
      local proto_dir
      proto_dir="$(dirname "${proto_cargo}")"
      if [[ -x "${proto_dir}/rustc" ]]; then
        export PATH="${proto_dir}:${PATH}"
        export RUSTC="${proto_dir}/rustc"
        return 0
      fi
    fi
  fi

  local rustup_cargo
  rustup_cargo="$(find "${HOME}/.rustup/toolchains" -path '*/bin/cargo' -print -quit 2>/dev/null || true)"
  if [[ -n "${rustup_cargo}" && -x "${rustup_cargo}" ]]; then
    local rustup_dir
    rustup_dir="$(dirname "${rustup_cargo}")"
    if [[ -x "${rustup_dir}/rustc" ]]; then
      export PATH="${rustup_dir}:${PATH}"
      export RUSTC="${rustup_dir}/rustc"
      return 0
    fi
  fi

  echo "error: unable to locate cargo/rustc" >&2
  echo "install Rust or expose it on PATH before running this command" >&2
  exit 1
}

main() {
  if [[ "$#" -eq 0 ]]; then
    echo "usage: $0 <command> [args...]" >&2
    exit 1
  fi

  activate_proto
  resolve_rust_toolchain
  export TEA_RUST_TOOLCHAIN_RESOLVED=1

  exec "$@"
}

main "$@"
