#!/usr/bin/env bash

set -euo pipefail

profile="${1:-release}"

if [[ "${profile}" != "release" ]]; then
  echo "unsupported profile: ${profile}" >&2
  echo "usage: $0 [release]" >&2
  exit 1
fi

target_dir="${CARGO_TARGET_DIR:-target}"
runtime_log="$(mktemp)"
trap 'rm -f "${runtime_log}"' EXIT

host_target="$("${RUSTC:-rustc}" -vV | sed -n 's/^host: //p' | head -n 1)"
if [[ -z "${host_target}" ]]; then
  echo "failed to determine Rust host target" >&2
  exit 1
fi

cargo rustc -p tea-runtime --release -- --print=native-static-libs 2>&1 | tee "${runtime_log}"
runtime_native_libs="$(sed -n 's/^note: native-static-libs: //p' "${runtime_log}" | tail -n 1)"
if [[ -z "${runtime_native_libs}" ]]; then
  echo "failed to resolve native static libs for tea-runtime" >&2
  exit 1
fi

runtime_archive="$(pwd)/${target_dir}/release/libtea_runtime.a"
if [[ ! -f "${runtime_archive}" ]]; then
  echo "expected runtime archive at ${runtime_archive}" >&2
  exit 1
fi

TEA_BUNDLED_RUNTIME_ARCHIVE="${runtime_archive}" \
TEA_BUNDLED_RUNTIME_NATIVE_LIBS="${runtime_native_libs}" \
TEA_BUNDLED_RUNTIME_TARGET="${host_target}" \
  cargo build --release -p tea-cli
