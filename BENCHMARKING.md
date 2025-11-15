# Benchmarking Guide for MemScan

This guide explains how to use the benchmarking infrastructure to measure and optimize performance-critical code in MemScan.

## Overview

MemScan uses **Criterion.rs**, the gold standard for Rust benchmarking, to measure performance of critical operations:

1. **Pattern Search** (`benches/pattern_search.rs`) - Benchmarks the naive byte pattern matching algorithm
2. **Hex Parsing** (`benches/hex_parsing.rs`) - Benchmarks conversion of hex strings to byte arrays

## Quick Start

### Running All Benchmarks

```bash
cargo bench
```

This will run all benchmarks and generate HTML reports in `target/criterion/`.

### Running Specific Benchmarks

```bash
# Run only pattern search benchmarks
cargo bench --bench pattern_search

# Run only hex parsing benchmarks
cargo bench --bench hex_parsing
```

### Running Specific Test Cases

```bash
# Run only benchmarks matching "realistic"
cargo bench -- realistic

# Run only pattern search benchmarks for 4KB pages
cargo bench -- pattern_search/4096
```

## Understanding the Results

### Output Format

Criterion outputs results like this:

```
pattern_search/miss_short/4096
                        time:   [1.2345 µs 1.2456 µs 1.2567 µs]
                        thrpt:  [3.1234 GiB/s 3.1456 GiB/s 3.1678 GiB/s]
```

- **time**: The time taken (lower is better)
  - First value: lower bound of confidence interval
  - Second value: point estimate
  - Third value: upper bound of confidence interval
- **thrpt**: Throughput in bytes/second (higher is better)

### HTML Reports

Open `target/criterion/report/index.html` in your browser for:
- Interactive charts comparing benchmark runs
- Statistical analysis of performance changes
- Detailed breakdowns of each benchmark

## Performance-Critical Areas Identified

### 1. Pattern Search (`naive_search`)

**Location**: `src/scanner.rs:193-203`

**Why Critical**: This function is called for every page of memory scanned. With processes having gigabytes of memory across thousands of pages, this becomes the primary performance bottleneck.

**Current Performance**:
- Algorithm: Naive O(n*m) substring search
- Typical page size: 4KB
- Typical pattern size: 2-16 bytes

**Optimization Opportunities**:
- Implement Boyer-Moore or Boyer-Moore-Horspool algorithm
- Use SIMD instructions for parallel comparison (e.g., via `memchr` crate)
- Consider Two-Way algorithm for small patterns
- Use Aho-Corasick for multiple pattern searching

**Benchmark Coverage**:
- Various haystack sizes (1KB to 64KB)
- Different pattern sizes (2 to 12 bytes)
- Pattern found vs not found scenarios
- Realistic binary data patterns

### 2. Hex Pattern Parsing (`parse_hex_pattern`)

**Location**: `src/main.rs:76-91`

**Why Critical**: Called once per scan operation, but poor performance here impacts user experience, especially in interactive/scripted scenarios.

**Current Performance**:
- Allocates a filtered string (removes whitespace)
- Parses byte-by-byte with radix conversion

**Optimization Opportunities**:
- Parse directly without intermediate allocation
- Use lookup tables instead of `from_str_radix`
- SIMD-based hex decoding
- Zero-copy parsing for compact hex strings

**Benchmark Coverage**:
- Various pattern lengths
- Compact vs spaced format
- Realistic patterns (PE headers, ELF headers, etc.)
- Case sensitivity variations

### 3. Memory Reading (Not Yet Benchmarked)

**Location**: `src/scanner.rs:109-123`

**Why Critical**: Windows API call overhead, but less critical than pattern search as it's I/O bound.

**Note**: Cannot be easily benchmarked without actual Windows process access. Consider profiling in real scenarios instead.

### 4. Memory Region Iteration (Not Yet Benchmarked)

**Location**: `src/process.rs:276-321`

**Why Critical**: VirtualQueryEx system calls, but relatively infrequent (one per region).

**Note**: Cannot be benchmarked without Windows APIs. Consider profiling in real scenarios.

## Continuous Benchmarking

### Baseline Measurements

Before making optimizations, establish a baseline:

```bash
# Run benchmarks and save baseline
cargo bench -- --save-baseline initial

# After making changes, compare against baseline
cargo bench -- --baseline initial
```

### Comparing Changes

```bash
# Make your changes, then:
cargo bench -- --baseline initial

# Criterion will show the difference:
# "change: [-5.0% -2.5% +0.5%] (p = 0.02 < 0.05)"
# Negative values mean improvement (faster)
```

### Performance Regression Detection

Set performance budgets:

```bash
# Fail if performance regresses by more than 10%
cargo bench -- --test --baseline initial --significance-level 0.10
```

## Profiling Integration

While benchmarks measure *what* is slow, profilers show *why*. On Windows:

### Using Windows Performance Analyzer

1. Record ETW trace:
   ```powershell
   xperf -on PROC_THREAD+LOADER+PROFILE -stackwalk Profile
   memscan.exe scan <target> --pattern "..."
   xperf -d trace.etl
   ```

2. Analyze with WPA to see CPU usage per function

### Using Visual Studio Profiler

1. Build in release mode with debug info:
   ```bash
   cargo build --release
   ```

2. Profile with Visual Studio's CPU profiler
3. Correlate hotspots with benchmark results

### Using cargo-flamegraph (Cross-platform)

```bash
cargo install flamegraph
cargo flamegraph --bench pattern_search
```

## Optimization Workflow

1. **Measure First**: Run benchmarks to establish baseline
2. **Identify Bottleneck**: Focus on the slowest operation with highest impact
3. **Hypothesis**: Form a theory about what's slow and why
4. **Implement**: Make targeted optimization
5. **Benchmark**: Run benchmarks to verify improvement
6. **Profile**: Use profiler to understand if bottleneck moved
7. **Iterate**: Repeat until performance goals met

## Advanced Configuration

### Custom Benchmark Settings

Edit benchmark files to adjust:

```rust
use criterion::Criterion;

fn custom_criterion() -> Criterion {
    Criterion::default()
        .sample_size(100)           // Number of samples (default: 100)
        .measurement_time(Duration::from_secs(10))  // Time per benchmark
        .warm_up_time(Duration::from_secs(3))       // Warm-up time
}

criterion_group! {
    name = benches;
    config = custom_criterion();
    targets = benchmark_pattern_search
}
```

### Noise Reduction Tips

For more accurate benchmarks:

1. **Close other applications**: Reduce CPU contention
2. **Disable CPU frequency scaling**: Use performance governor
   ```bash
   # Linux
   echo performance | sudo tee /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor
   ```
3. **Pin to specific cores**: Reduce scheduling variance
4. **Run multiple times**: Look for consistency across runs

## Integration with CI/CD

### GitHub Actions Example

```yaml
name: Benchmark

on: [pull_request]

jobs:
  benchmark:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v2
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - name: Run benchmarks
        run: cargo bench
      - name: Upload results
        uses: actions/upload-artifact@v2
        with:
          name: benchmark-results
          path: target/criterion/
```

### Performance Testing in CI

```bash
# Compare PR against main branch
git checkout main
cargo bench -- --save-baseline main

git checkout feature-branch
cargo bench -- --baseline main

# Add threshold check
if ! cargo bench -- --baseline main --significance-level 0.05; then
    echo "Performance regression detected!"
    exit 1
fi
```

## Best Practices

1. **Benchmark on target hardware**: Performance characteristics differ between dev and production
2. **Use realistic data**: Benchmarks should reflect actual use cases
3. **Measure end-to-end**: Don't optimize micro-benchmarks at the expense of real-world performance
4. **Document assumptions**: Note why certain benchmarks matter
5. **Version your benchmarks**: Keep them in sync with code changes
6. **Automate regression testing**: Catch performance regressions in CI

## Further Reading

- [Criterion.rs User Guide](https://bheisler.github.io/criterion.rs/book/)
- [The Rust Performance Book](https://nnethercote.github.io/perf-book/)
- [Benchmarking and Optimization in Rust](https://www.youtube.com/watch?v=d7pZNJBdU6s)
- [String Searching Algorithms](https://en.wikipedia.org/wiki/String-searching_algorithm)
- [OPTIMIZATION.md](OPTIMIZATION.md) - Specific optimization recommendations for MemScan

## Contributing Benchmarks

When adding new performance-critical code:

1. Add corresponding benchmarks
2. Document why the code is performance-critical
3. Include realistic test cases
4. Set performance budgets if applicable
5. Update this guide with the new benchmark details
