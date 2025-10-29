#!/usr/bin/env bash
# Benchmark harness for Tea AOT compiler performance
# Requires: hyperfine (install via: cargo install hyperfine)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
BENCHMARKS_DIR="${REPO_ROOT}/benchmarks"
BIN_DIR="${REPO_ROOT}/bin"
RESULTS_DIR="${REPO_ROOT}/benchmark_results"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Ensure hyperfine is installed
if ! command -v hyperfine &> /dev/null; then
    echo -e "${RED}Error: hyperfine is not installed${NC}"
    echo "Install it with: cargo install hyperfine"
    exit 1
fi

# Create output directories
mkdir -p "${BIN_DIR}"
mkdir -p "${RESULTS_DIR}"

echo -e "${BLUE}Tea Language AOT Compiler Benchmark Suite${NC}"
echo "=========================================="
echo ""

# Benchmark configurations
declare -a BENCHMARKS=(
    "loops"
    "fib"
    "strings"
    "lists"
    "dicts"
    "closures"
    "structs"
)

# Function to build a Tea program with default optimization
build_tea() {
    local name="$1"
    local input="${BENCHMARKS_DIR}/${name}.tea"
    local output="${BIN_DIR}/${name}_aot"
    
    echo -e "${YELLOW}Building ${name} with AOT compiler...${NC}"
    
    cargo run -p tea-cli --quiet --release -- build \
        --output "${output}" \
        "${input}" 2>&1 | grep -v "^Compiling\|^Finished\|^Running" || true
    
    if [ ! -f "${output}" ]; then
        echo -e "${RED}Failed to build ${name}${NC}"
        return 1
    fi
    
    echo "${output}"
}

# Function to build Rust reference if it exists
build_rust_reference() {
    local name="$1"
    local source="${BENCHMARKS_DIR}/reference_${name}.rs"
    local output="${BIN_DIR}/reference_${name}"
    
    if [ -f "${source}" ]; then
        echo -e "${YELLOW}Building Rust reference for ${name}...${NC}"
        rustc -O "${source}" -o "${output}" 2>/dev/null || true
        if [ -f "${output}" ]; then
            echo "${output}"
            return 0
        fi
    fi
    return 1
}

# Run benchmarks for a specific program
benchmark_program() {
    local name="$1"
    local warmup="${2:-3}"
    local min_runs="${3:-10}"
    
    echo ""
    echo -e "${GREEN}=== Benchmarking: ${name} ===${NC}"
    echo ""
    
    # Build AOT binary
    local aot_binary
    if ! aot_binary=$(build_tea "${name}"); then
        echo -e "${RED}Failed to build AOT binary for ${name}${NC}"
        return 1
    fi
    
    # Build Rust reference if available
    local rust_binary
    local has_rust_ref=false
    if rust_binary=$(build_rust_reference "${name}"); then
        has_rust_ref=true
    fi
    
    local tea_source="${BENCHMARKS_DIR}/${name}.tea"
    
    # Prepare hyperfine command
    local hyperfine_cmd="hyperfine"
    hyperfine_cmd+=" --warmup ${warmup}"
    hyperfine_cmd+=" --min-runs ${min_runs}"
    hyperfine_cmd+=" --style full"
    hyperfine_cmd+=" --export-markdown ${RESULTS_DIR}/${name}.md"
    hyperfine_cmd+=" --export-json ${RESULTS_DIR}/${name}.json"
    
    # Add AOT binary
    hyperfine_cmd+=" --command-name 'Tea AOT' '${aot_binary}'"
    
    # Add Rust reference if available
    if [ "${has_rust_ref}" = true ]; then
        hyperfine_cmd+=" --command-name 'Rust -O' '${rust_binary}'"
    fi
    
    # Add VM benchmark
    hyperfine_cmd+=" --command-name 'Tea VM (bytecode)' 'cargo run -p tea-cli --quiet --release -- ${tea_source}'"
    
    # Run benchmark
    echo ""
    eval "${hyperfine_cmd}"
    echo ""
}

# Function to run all benchmarks
run_all() {
    local timestamp=$(date +%Y%m%d_%H%M%S)
    local summary_file="${RESULTS_DIR}/summary_${timestamp}.md"
    
    echo "# Tea AOT Benchmark Results" > "${summary_file}"
    echo "" >> "${summary_file}"
    echo "Generated: $(date)" >> "${summary_file}"
    echo "" >> "${summary_file}"
    
    for bench in "${BENCHMARKS[@]}"; do
        if benchmark_program "${bench}" 3 10; then
            if [ -f "${RESULTS_DIR}/${bench}.md" ]; then
                echo "## ${bench}" >> "${summary_file}"
                echo "" >> "${summary_file}"
                cat "${RESULTS_DIR}/${bench}.md" >> "${summary_file}"
                echo "" >> "${summary_file}"
            fi
        fi
    done
    
    echo -e "${GREEN}All benchmarks complete!${NC}"
    echo -e "Results saved to: ${RESULTS_DIR}/"
    echo -e "Summary: ${summary_file}"
}

# Function to clean build artifacts
clean() {
    echo "Cleaning benchmark artifacts..."
    rm -rf "${BIN_DIR}"/*_aot
    rm -f "${BIN_DIR}"/reference_*
    echo "Done."
}

# Main command dispatcher
case "${1:-all}" in
    all)
        run_all
        ;;
    clean)
        clean
        ;;
    *)
        # Run a specific benchmark
        benchmark_program "$1" "${2:-3}" "${3:-10}"
        ;;
esac
