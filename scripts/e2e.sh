#!/usr/bin/env bash

set -euo pipefail

root_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${root_dir}"

echo "Building examples/language/basics/fib.tea..."
cargo run -p tea-cli -- build examples/language/basics/fib.tea >/tmp/tea-e2e-build.log

binary_path="${root_dir}/bin/fib"
if [[ ! -x "${binary_path}" ]]; then
  echo "error: expected executable at ${binary_path} but it was not created" >&2
  exit 1
fi

echo "Running ${binary_path}..."
program_output="$("${binary_path}")"
expected_output="832040"

if [[ "${program_output//$'\n'/}" != "${expected_output}" ]]; then
  echo "error: expected program output '${expected_output}' but found '${program_output}'" >&2
  exit 1
fi

echo "✅ fib build produced expected output (${program_output})"

echo "Building examples/language/types/structs.tea..."
cargo run -p tea-cli -- build examples/language/types/structs.tea >/tmp/tea-e2e-structs-build.log

structs_binary="${root_dir}/bin/structs"
if [[ ! -x "${structs_binary}" ]]; then
  echo "error: expected executable at ${structs_binary} but it was not created" >&2
  exit 1
fi

echo "Running ${structs_binary}..."
structs_output="$("${structs_binary}")"
expected_structs_output=$'Ada\n37'

if [[ "${structs_output//$'\r'/}" != "${expected_structs_output}" ]]; then
  echo "error: expected program output '${expected_structs_output}' but found '${structs_output}'" >&2
  exit 1
fi

echo "✅ structs build produced expected output"

echo "Building examples/language/functions/lambdas.tea..."
cargo run -p tea-cli -- build examples/language/functions/lambdas.tea >/tmp/tea-e2e-lambdas-build.log

lambdas_binary="${root_dir}/bin/lambdas"
if [[ ! -x "${lambdas_binary}" ]]; then
  echo "error: expected executable at ${lambdas_binary} but it was not created" >&2
  exit 1
fi

echo "Running ${lambdas_binary}..."
lambdas_output="$(${lambdas_binary})"
expected_lambdas_output="42"

if [[ "${lambdas_output//$'\r'/}" != "${expected_lambdas_output}" ]]; then
  echo "error: expected program output '${expected_lambdas_output}' but found '${lambdas_output}'" >&2
  exit 1
fi

echo "✅ lambdas build produced expected output"

echo "Building examples/stdlib/io/pipeline.tea..."
cargo run -p tea-cli -- build examples/stdlib/io/pipeline.tea >/tmp/tea-e2e-pipeline-build.log

pipeline_binary="${root_dir}/bin/pipeline"
if [[ ! -x "${pipeline_binary}" ]]; then
  echo "error: expected executable at ${pipeline_binary} but it was not created" >&2
  exit 1
fi

echo "Running ${pipeline_binary} with sample input..."
pipeline_output="$(echo '{"value":1}' | "${pipeline_binary}")"
expected_pipeline_output='{"value":1}'

if [[ "${pipeline_output//$'\r'/}" != "${expected_pipeline_output}" ]]; then
  echo "error: expected program output '${expected_pipeline_output}' but found '${pipeline_output}'" >&2
  exit 1
fi

echo "✅ pipeline build produced expected output"

echo "Building examples/stdlib/cli/process.tea..."
cargo run -p tea-cli -- build examples/stdlib/cli/process.tea >/tmp/tea-e2e-process-build.log

process_binary="${root_dir}/bin/process"
if [[ ! -x "${process_binary}" ]]; then
  echo "error: expected executable at ${process_binary} but it was not created" >&2
  exit 1
fi

echo "Running ${process_binary}..."
process_output="$(${process_binary})"
expected_process_output=$'run stdout:\nhello\nspawn chunk:\nworld\nspawn exit:\n0'

if [[ "${process_output//$'\r'/}" != "${expected_process_output}" ]]; then
  echo "error: expected program output '${expected_process_output}' but found '${process_output}'" >&2
  exit 1
fi

echo "✅ process build produced expected output"

echo "Building examples/stdlib/cli/path_utils.tea..."
cargo run -p tea-cli -- build examples/stdlib/cli/path_utils.tea >/tmp/tea-e2e-path-build.log

path_binary="${root_dir}/bin/path_utils"
if [[ ! -x "${path_binary}" ]]; then
  echo "error: expected executable at ${path_binary} but it was not created" >&2
  exit 1
fi

echo "Running ${path_binary}..."
path_output="$("${path_binary}")"

if [[ "${path_output}" != *"joined path"* ]]; then
  echo "error: expected path_utils output to include 'joined path' but it did not" >&2
  exit 1
fi

if [[ "${path_output}" != *"separator"* ]]; then
  echo "error: expected path_utils output to include 'separator' but it did not" >&2
  exit 1
fi

echo "✅ path_utils build ran successfully"

echo "Building examples/stdlib/cli/env.tea..."
cargo run -p tea-cli -- build examples/stdlib/cli/env.tea >/tmp/tea-e2e-env-build.log

env_binary="${root_dir}/bin/env"
if [[ ! -x "${env_binary}" ]]; then
  echo "error: expected executable at ${env_binary} but it was not created" >&2
  exit 1
fi

echo "Running ${env_binary}..."
env_output="$(${env_binary})"

if [[ "${env_output}" != *"cwd"* ]]; then
  echo "error: expected env output to include 'cwd' but it did not" >&2
  exit 1
fi

if [[ "${env_output}" != *"has PATH"* ]]; then
  echo "error: expected env output to include 'has PATH' but it did not" >&2
  exit 1
fi

echo "✅ env build ran successfully"
echo "✅ llvm build e2e tests passed"
