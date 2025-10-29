#!/usr/bin/env bash
# Benchmark harness for Tea AOT compiler performance
# Requires: hyperfine (install via: cargo install hyperfine)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
BENCHMARKS_DIR="${REPO_ROOT}/benchmarks"
TEA_BENCHMARKS_DIR="${BENCHMARKS_DIR}/tea"
RUST_BENCHMARKS_DIR="${BENCHMARKS_DIR}/rust"
JS_BENCHMARKS_DIR="${BENCHMARKS_DIR}/js"
BIN_DIR="${REPO_ROOT}/bin"
RESULTS_DIR="${REPO_ROOT}/benchmark_results"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Default: don't include VM benchmarks or JS
INCLUDE_VM=false
INCLUDE_JS=false

# Ensure hyperfine is installed
if ! command -v hyperfine &> /dev/null; then
    echo -e "${RED}Error: hyperfine is not installed${NC}"
    echo "Install it with: cargo install hyperfine"
    exit 1
fi

# Check if bun is available (optional)
if command -v bun &> /dev/null; then
    HAS_BUN=true
else
    HAS_BUN=false
fi

# Create output directories
mkdir -p "${BIN_DIR}"
mkdir -p "${RESULTS_DIR}"

echo -e "${BLUE}Tea Language AOT Compiler Benchmark Suite${NC}"
echo "=========================================="
echo ""

# Discover benchmarks: find triples that exist in tea/, rust/, and optionally js/
discover_benchmarks() {
    local -a paired_benchmarks=()
    
    for tea_file in "${TEA_BENCHMARKS_DIR}"/*.tea; do
        [ -f "${tea_file}" ] || continue
        local basename=$(basename "${tea_file}" .tea)
        local rust_file="${RUST_BENCHMARKS_DIR}/${basename}.rs"
        
        if [ -f "${rust_file}" ]; then
            paired_benchmarks+=("${basename}")
        fi
    done
    
    echo "${paired_benchmarks[@]}"
}

# Function to build a Tea program with default optimization
build_tea() {
    local name="$1"
    local input="${TEA_BENCHMARKS_DIR}/${name}.tea"
    local output="${BIN_DIR}/${name}_aot"
    
    if [ ! -f "${input}" ]; then
        echo -e "${RED}Tea file not found: ${input}${NC}"
        return 1
    fi
    
    echo -e "${YELLOW}Building ${name}.tea with AOT compiler...${NC}"
    
    cargo run -p tea-cli --quiet --release -- build \
        --output "${output}" \
        "${input}" 2>&1 | grep -v "^Compiling\|^Finished\|^Running" || true
    
    if [ ! -f "${output}" ]; then
        echo -e "${RED}Failed to build ${name}${NC}"
        return 1
    fi
    
    echo "${output}"
}

# Function to build Rust program
build_rust() {
    local name="$1"
    local source="${RUST_BENCHMARKS_DIR}/${name}.rs"
    local output="${BIN_DIR}/${name}_rust"
    
    if [ ! -f "${source}" ]; then
        echo -e "${RED}Rust file not found: ${source}${NC}"
        return 1
    fi
    
    echo -e "${YELLOW}Building ${name}.rs with Rust...${NC}"
    rustc -O -C target-cpu=native "${source}" -o "${output}" 2>/dev/null
    
    if [ ! -f "${output}" ]; then
        echo -e "${RED}Failed to build Rust ${name}${NC}"
        return 1
    fi
    
    echo "${output}"
}

# Run benchmarks for a specific program
benchmark_program() {
    local name="$1"
    local warmup="${2:-3}"
    local min_runs="${3:-10}"
    
    echo ""
    echo -e "${GREEN}=== Benchmarking: ${name} ===${NC}"
    echo ""
    
    # Verify both files exist
    local tea_file="${TEA_BENCHMARKS_DIR}/${name}.tea"
    local rust_file="${RUST_BENCHMARKS_DIR}/${name}.rs"
    
    if [ ! -f "${tea_file}" ]; then
        echo -e "${RED}Tea file not found: ${tea_file}${NC}"
        return 1
    fi
    
    if [ ! -f "${rust_file}" ]; then
        echo -e "${RED}Rust file not found: ${rust_file}${NC}"
        echo -e "${YELLOW}Skipping ${name} - missing Rust equivalent${NC}"
        return 1
    fi
    
    # Build AOT binary
    local aot_binary
    if ! aot_binary=$(build_tea "${name}"); then
        echo -e "${RED}Failed to build AOT binary for ${name}${NC}"
        return 1
    fi
    
    # Build Rust binary
    local rust_binary
    if ! rust_binary=$(build_rust "${name}"); then
        echo -e "${RED}Failed to build Rust binary for ${name}${NC}"
        return 1
    fi
    
    # Prepare hyperfine command
    local hyperfine_cmd="hyperfine"
    hyperfine_cmd+=" --warmup ${warmup}"
    hyperfine_cmd+=" --min-runs ${min_runs}"
    hyperfine_cmd+=" --style full"
    hyperfine_cmd+=" --export-markdown ${RESULTS_DIR}/${name}.md"
    hyperfine_cmd+=" --export-json ${RESULTS_DIR}/${name}.json"
    
    # Add Tea AOT binary
    hyperfine_cmd+=" --command-name 'Tea AOT' '${aot_binary}'"
    
    # Add Rust binary
    hyperfine_cmd+=" --command-name 'Rust' '${rust_binary}'"
    
    # Optionally add JavaScript/Bun benchmark
    if [ "${INCLUDE_JS}" = true ] && [ "${HAS_BUN}" = true ]; then
        local js_file="${JS_BENCHMARKS_DIR}/${name}.js"
        if [ -f "${js_file}" ]; then
            hyperfine_cmd+=" --command-name 'JavaScript (Bun)' 'bun ${js_file}'"
        fi
    fi
    
    # Optionally add VM benchmark
    if [ "${INCLUDE_VM}" = true ]; then
        hyperfine_cmd+=" --command-name 'Tea VM (bytecode)' 'cargo run -p tea-cli --quiet --release -- ${tea_file}'"
    fi
    
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
    
    if [ "${INCLUDE_VM}" = true ]; then
        echo "VM benchmarks: included" >> "${summary_file}"
    else
        echo "VM benchmarks: excluded (use --include-vm to enable)" >> "${summary_file}"
    fi
    
    if [ "${INCLUDE_JS}" = true ] && [ "${HAS_BUN}" = true ]; then
        echo "JavaScript benchmarks: included (Bun)" >> "${summary_file}"
    else
        echo "JavaScript benchmarks: excluded (use --include-js to enable)" >> "${summary_file}"
    fi
    echo "" >> "${summary_file}"
    
    # Discover paired benchmarks
    local benchmarks=($(discover_benchmarks))
    
    if [ ${#benchmarks[@]} -eq 0 ]; then
        echo -e "${RED}No paired benchmarks found!${NC}"
        echo "Each .tea file in benchmarks/tea/ must have a corresponding .rs file in benchmarks/rust/"
        return 1
    fi
    
    echo -e "${BLUE}Found ${#benchmarks[@]} paired benchmark(s): ${benchmarks[*]}${NC}"
    echo ""
    
    for bench in "${benchmarks[@]}"; do
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
    rm -f "${BIN_DIR}"/*_aot
    rm -f "${BIN_DIR}"/*_rust
    echo "Done."
}

# Function to show usage
usage() {
    echo "Usage: $0 [OPTIONS] [COMMAND]"
    echo ""
    echo "Commands:"
    echo "  all              Run all paired benchmarks (default)"
    echo "  clean            Clean build artifacts"
    echo "  <name>           Run specific benchmark"
    echo ""
    echo "Options:"
    echo "  --include-vm     Include Tea VM (bytecode) in benchmarks"
    echo "  --include-js     Include JavaScript (Bun) in benchmarks"
    echo "  --help           Show this help message"
    echo ""
    echo "Examples:"
    echo "  $0 all                       # Run all benchmarks (AOT vs Rust only)"
    echo "  $0 --include-vm all          # Run all benchmarks including VM"
    echo "  $0 --include-js all          # Run all benchmarks including JS/Bun"
    echo "  $0 --include-js --include-vm all  # Include both VM and JS"
    echo "  $0 loops                     # Run loops benchmark"
    echo "  $0 --include-js fib 5 20     # Run fib with JS, 5 warmup, 20 runs"
}

# Parse arguments
while [[ $# -gt 0 ]]; do
    case "$1" in
        --include-vm)
            INCLUDE_VM=true
            shift
            ;;
        --include-js)
            INCLUDE_JS=true
            if [ "${HAS_BUN}" = false ]; then
                echo -e "${YELLOW}Warning: Bun not found. JavaScript benchmarks will be skipped.${NC}"
                echo -e "${YELLOW}Install Bun from: https://bun.sh${NC}"
            fi
            shift
            ;;
        --help|-h)
            usage
            exit 0
            ;;
        all|clean)
            COMMAND="$1"
            shift
            ;;
        *)
            # Assume it's a benchmark name or additional args
            COMMAND="${1:-all}"
            shift
            ARGS=("$@")
            break
            ;;
    esac
done

# Execute command
case "${COMMAND:-all}" in
    all)
        run_all
        ;;
    clean)
        clean
        ;;
    *)
        # Run a specific benchmark
        benchmark_program "${COMMAND}" "${ARGS[@]:-3 10}"
        ;;
esac
