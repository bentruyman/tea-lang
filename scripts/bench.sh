#!/usr/bin/env bash
# Benchmark harness for Tea compiler performance
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

# Default: run Tea vs Rust only; JS is opt-in
INCLUDE_JS=false

# Ensure hyperfine is installed
if ! command -v hyperfine &> /dev/null; then
    echo -e "${RED}Error: hyperfine is not installed${NC}"
    echo "Install it with: cargo install hyperfine"
    exit 1
fi

# Ensure jq is installed (for summarizing relative results)
if ! command -v jq &> /dev/null; then
    echo -e "${RED}Error: jq is not installed${NC}"
    echo "Install it with: brew install jq (macOS) or your package manager"
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

echo -e "${BLUE}Tea Language Compiler Benchmark Suite${NC}"
echo "======================================"
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
    local output="${BIN_DIR}/${name}_tea"
    
    if [ ! -f "${input}" ]; then
        echo -e "${RED}Tea file not found: ${input}${NC}" >&2
        return 1
    fi
    
    echo -e "${YELLOW}Building ${name}.tea...${NC}" >&2
    
    cargo run -p tea-cli --quiet --release -- build \
        --output "${output}" \
        "${input}" 2>&1 | grep -v "^Compiling\|^Finished\|^Running" >&2 || true
    
    if [ ! -f "${output}" ]; then
        echo -e "${RED}Failed to build ${name}${NC}" >&2
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
        echo -e "${RED}Rust file not found: ${source}${NC}" >&2
        return 1
    fi
    
    echo -e "${YELLOW}Building ${name}.rs with Rust...${NC}" >&2
    
    rustc -O -C target-cpu=native "${source}" -o "${output}" 2>&1 >&2
    
    if [ ! -f "${output}" ]; then
        echo -e "${RED}Failed to build ${name}${NC}" >&2
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
    
    # Build Tea binary
    local tea_binary
    if ! tea_binary=$(build_tea "${name}"); then
        echo -e "${RED}Failed to build Tea binary for ${name}${NC}"
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
    
    # Add Tea binary
    hyperfine_cmd+=" --command-name 'Tea' '${tea_binary}'"
    
    # Add Rust binary
    hyperfine_cmd+=" --command-name 'Rust' '${rust_binary}'"
    
    # Optionally add JavaScript/Bun benchmark
    if [ "${INCLUDE_JS}" = true ] && [ "${HAS_BUN}" = true ]; then
        local js_file="${JS_BENCHMARKS_DIR}/${name}.js"
        if [ -f "${js_file}" ]; then
            hyperfine_cmd+=" --command-name 'JavaScript (Bun)' 'bun ${js_file}'"
        fi
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
    
    echo "# Tea Benchmark Results" > "${summary_file}"
    echo "" >> "${summary_file}"
    echo "Generated: $(date)" >> "${summary_file}"
    echo "" >> "${summary_file}"
    
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

                # Add a short relative summary (Ratios vs Tea)
                json_file="${RESULTS_DIR}/${bench}.json"
                if [ -f "${json_file}" ]; then
                    tea_mean=$(jq -r '.results[0].mean' "${json_file}")
                    results_len=$(jq -r '.results | length' "${json_file}")

                    if [ -n "${tea_mean}" ] && [ "${tea_mean}" != "null" ]; then
                        echo "Relative to Tea:" >> "${summary_file}"

                        # Rust is always the second entry
                        if [ "${results_len}" -ge 2 ]; then
                            rust_mean=$(jq -r '.results[1].mean' "${json_file}")
                            if [ -n "${rust_mean}" ] && [ "${rust_mean}" != "null" ]; then
                                rust_ratio=$(awk "BEGIN { printf \"%.2f\", ${rust_mean}/${tea_mean} }")
                                echo "- Rust: ${rust_ratio}x" >> "${summary_file}"
                            fi
                        fi

                        # JavaScript (Bun) may be present as the 3rd entry when included
                        if [ "${INCLUDE_JS}" = true ] && [ "${HAS_BUN}" = true ] && [ "${results_len}" -ge 3 ]; then
                            js_mean=$(jq -r '.results[2].mean' "${json_file}")
                            if [ -n "${js_mean}" ] && [ "${js_mean}" != "null" ]; then
                                js_ratio=$(awk "BEGIN { printf \"%.2f\", ${js_mean}/${tea_mean} }")
                                echo "- JavaScript (Bun): ${js_ratio}x" >> "${summary_file}"
                            fi
                        fi

                        echo "" >> "${summary_file}"
                    fi
                fi

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
    rm -f "${BIN_DIR}"/*_tea
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
    echo "  --include-js     Include JavaScript (Bun) in benchmarks"
    echo "  --help           Show this help message"
    echo ""
    echo "Examples:"
    echo "  $0 all                       # Run all benchmarks (Tea vs Rust only)"
    echo "  $0 --include-js all          # Run all benchmarks including JS/Bun"
    echo "  $0 loops                     # Run loops benchmark"
    echo "  $0 --include-js fib 5 20     # Run fib with JS, 5 warmup, 20 runs"
}

# Parse arguments
while [[ $# -gt 0 ]]; do
    case "$1" in
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
        if [ ${#ARGS[@]} -eq 0 ]; then
            benchmark_program "${COMMAND}" 3 10
        else
            benchmark_program "${COMMAND}" "${ARGS[@]}"
        fi
        ;;
esac
