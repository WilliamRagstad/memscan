#!/usr/bin/env bash
#
# Benchmark runner script with common tasks
# Usage: ./bench.sh [command]

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

show_help() {
    cat << EOF
MemScan Benchmark Runner

Usage: ./bench.sh [command]

Commands:
    all                 Run all benchmarks (default)
    pattern             Run pattern search benchmarks only
    hex                 Run hex parsing benchmarks only
    baseline [name]     Save current results as baseline
    compare [baseline]  Compare against saved baseline
    quick               Run quick benchmarks (reduced sample size)
    clean               Remove benchmark cache and results
    report              Open HTML report in browser
    help                Show this help message

Examples:
    ./bench.sh                      # Run all benchmarks
    ./bench.sh baseline main        # Save baseline as 'main'
    ./bench.sh compare main         # Compare against 'main' baseline
    ./bench.sh pattern              # Run only pattern search benchmarks
    ./bench.sh report               # Open results in browser

Environment Variables:
    SAMPLE_SIZE         Override sample size (default: 100)
    MEASUREMENT_TIME    Override measurement time in seconds (default: 5)

EOF
}

run_all() {
    echo "Running all benchmarks..."
    cargo bench --no-fail-fast
}

run_pattern() {
    echo "Running pattern search benchmarks..."
    cargo bench --bench pattern_search --no-fail-fast
}

run_hex() {
    echo "Running hex parsing benchmarks..."
    cargo bench --bench hex_parsing --no-fail-fast
}

save_baseline() {
    local name="${1:-baseline}"
    echo "Saving baseline as '$name'..."
    cargo bench --no-fail-fast -- --save-baseline "$name"
    echo "Baseline '$name' saved successfully"
}

compare_baseline() {
    local name="${1:-baseline}"
    echo "Comparing against baseline '$name'..."
    cargo bench --no-fail-fast -- --baseline "$name"
}

run_quick() {
    echo "Running quick benchmarks (reduced samples)..."
    CRITERION_SAMPLE_SIZE=20 cargo bench --no-fail-fast
}

clean_benchmarks() {
    echo "Cleaning benchmark results..."
    rm -rf target/criterion
    echo "Benchmark results cleaned"
}

open_report() {
    local report="target/criterion/report/index.html"
    if [ ! -f "$report" ]; then
        echo "Error: No benchmark report found. Run benchmarks first."
        exit 1
    fi
    
    echo "Opening benchmark report..."
    if command -v xdg-open > /dev/null; then
        xdg-open "$report"
    elif command -v open > /dev/null; then
        open "$report"
    elif command -v start > /dev/null; then
        start "$report"
    else
        echo "Report location: $report"
        echo "Please open this file in your browser"
    fi
}

# Main command dispatcher
case "${1:-all}" in
    all)
        run_all
        ;;
    pattern)
        run_pattern
        ;;
    hex)
        run_hex
        ;;
    baseline)
        save_baseline "$2"
        ;;
    compare)
        compare_baseline "$2"
        ;;
    quick)
        run_quick
        ;;
    clean)
        clean_benchmarks
        ;;
    report)
        open_report
        ;;
    help|--help|-h)
        show_help
        ;;
    *)
        echo "Error: Unknown command '$1'"
        echo ""
        show_help
        exit 1
        ;;
esac
