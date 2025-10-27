#!/usr/bin/env bash

set -euo pipefail

root_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${root_dir}"

echo "Running tree-sitter grammar tests..."
(
  cd "${root_dir}/tree-sitter-tea"
  if [[ ! -d "node_modules" ]]; then
    if ! command -v bun >/dev/null 2>&1; then
      echo "error: bun is required to install tree-sitter dependencies" >&2
      exit 1
    fi

    echo "Installing tree-sitter dependencies..."
    bun install >/tmp/tea-e2e-tree-sitter-install.log 2>&1
  fi

  bunx tree-sitter test
)

echo "✅ tree-sitter tests passed"

echo "Building tea-cli once for example runs..."
cargo build -p tea-cli >/tmp/tea-e2e-build-cli.log 2>&1
tea_bin="${root_dir}/target/debug/tea"

if [[ ! -x "${tea_bin}" ]]; then
  alt_bin="${root_dir}/target/debug/tea-cli"
  if [[ -x "${alt_bin}" ]]; then
    tea_bin="${alt_bin}"
  else
    echo "error: expected tea binary at ${tea_bin} or ${alt_bin} but neither was created" >&2
    exit 1
  fi
fi

echo "✅ tea-cli binary ready at ${tea_bin}"

echo "Running Tea examples..."
while IFS= read -r example; do
  echo "Running ${example}..."
  log_file="/tmp/tea-e2e-$(echo "${example}" | tr '/' '-' | tr '.' '-')-run.log"

  case "${example}" in
  "examples/stdlib/io/pipeline.tea")
    printf '{"value":1}\n' | "${tea_bin}" "${example}" >"${log_file}" 2>&1
    ;;
  "examples/stdlib/http/server.tea")
    "${tea_bin}" "${example}" >"${log_file}" 2>&1 &
    tea_pid=$!
    sleep 2
    if kill -0 "${tea_pid}" 2>/dev/null; then
      kill "${tea_pid}" 2>/dev/null || true
      wait "${tea_pid}" 2>/dev/null || true
    fi
    ;;
  *)
    "${tea_bin}" "${example}" >"${log_file}" 2>&1
    ;;
  esac

  echo "✅ ${example} completed (log: ${log_file})"
done < <(find examples -name '*.tea' | sort)

echo "✅ All examples executed successfully"

full_example="examples/full/team_scoreboard.tea"
echo "Building ${full_example}..."
"${tea_bin}" build "${full_example}" >/tmp/tea-e2e-team_scoreboard-build.log 2>&1

full_example_binary="${root_dir}/bin/team_scoreboard"
if [[ ! -x "${full_example_binary}" ]]; then
  echo "error: expected executable at ${full_example_binary} but it was not created" >&2
  exit 1
fi

echo "✅ full example build produced ${full_example_binary}"
echo "✅ e2e suite completed"
