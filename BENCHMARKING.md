# Benchmarking Guide for MemScan

This comprehensive guide explains how to use the benchmarking infrastructure to measure and optimize performance-critical code in MemScan.

## Table of Contents
- [Overview](#overview)
- [Quick Start](#quick-start)
- [Understanding Results](#understanding-results)
- [Performance-Critical Areas](#performance-critical-areas)
- [Optimization Recommendations](#optimization-recommendations)
- [Continuous Benchmarking](#continuous-benchmarking)
- [Best Practices](#best-practices)

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

# Run benchmarks matching a pattern
cargo bench -- realistic
```

### Using Helper Scripts

Unix/Linux/macOS:
```bash
./bench.sh all              # Run all benchmarks
./bench.sh pattern          # Run pattern search only
./bench.sh baseline main    # Save baseline as 'main'
./bench.sh compare main     # Compare against baseline
./bench.sh report           # Open HTML report in browser
```

Windows PowerShell:
```powershell
.\bench.ps1 all             # Run all benchmarks
.\bench.ps1 pattern         # Run pattern search only
.\bench.ps1 baseline main   # Save baseline as 'main'
.\bench.ps1 compare main    # Compare against baseline
.\bench.ps1 report          # Open HTML report in browser
```

## Understanding Results

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

## Performance-Critical Areas

### 1. Pattern Search (HIGHEST PRIORITY)

**Location**: `src/scanner.rs:naive_search`

**Why Critical**: This function is called for every page of memory scanned. With processes having gigabytes of memory across thousands of pages, this becomes the primary performance bottleneck.

**Current Performance**:
- Algorithm: Naive O(n*m) substring search
- Typical page size: 4KB
- Typical pattern size: 2-16 bytes

**Impact**: Called millions of times per scan

### 2. Hex Pattern Parsing (MEDIUM PRIORITY)

**Location**: `src/lib.rs:parse_hex_pattern`

**Why Critical**: Called once per scan operation, but poor performance here impacts user experience, especially in interactive/scripted scenarios.

**Current Performance**:
- Allocates a filtered string (removes whitespace)
- Parses byte-by-byte with radix conversion

**Impact**: Called once per scan, affects startup time

### 3. Memory Reading (LOWER PRIORITY)

**Location**: `src/scanner.rs:scan_process`

**Why Critical**: Windows API call overhead, but less critical than pattern search as it's I/O bound.

**Note**: Cannot be easily benchmarked without actual Windows process access. Consider profiling in real scenarios instead.

## Optimization Recommendations

### Priority 1: Pattern Search Algorithm

#### Current Implementation

```rust
pub fn naive_search(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || needle.len() > haystack.len() {
        return None;
    }
    for i in 0..=haystack.len() - needle.len() {
        if &haystack[i..i + needle.len()] == needle {
            return Some(i);
        }
    }
    None
}
```

**Complexity**: O(n*m) where n = haystack length, m = needle length

#### Option 1: Use `memchr` crate (Easy, High Impact)

**Difficulty**: ⭐ Easy  
**Expected Speedup**: 5-20x for typical patterns  
**Dependencies**: `memchr` crate

```rust
use memchr::memmem;

pub fn optimized_search(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    memmem::find(haystack, needle)
}
```

**Why it's faster**:
- Uses SIMD instructions (SSE2/AVX2)
- Highly optimized C-like implementation
- Well-tested and maintained (used by ripgrep)

**Trade-offs**: Adds dependency, but it's widely used and battle-tested

#### Option 2: Boyer-Moore-Horspool Algorithm (Medium, High Impact)

**Difficulty**: ⭐⭐ Medium  
**Expected Speedup**: 3-10x for patterns > 4 bytes  
**Dependencies**: None (pure Rust implementation)

```rust
pub fn boyer_moore_horspool(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || needle.len() > haystack.len() {
        return None;
    }
    
    // Build bad character table
    let mut bad_char = [needle.len(); 256];
    for (i, &byte) in needle.iter().enumerate().take(needle.len() - 1) {
        bad_char[byte as usize] = needle.len() - 1 - i;
    }
    
    let mut pos = 0;
    while pos <= haystack.len() - needle.len() {
        let mut j = needle.len() - 1;
        while j > 0 && needle[j] == haystack[pos + j] {
            j -= 1;
        }
        if j == 0 && needle[0] == haystack[pos] {
            return Some(pos);
        }
        pos += bad_char[haystack[pos + needle.len() - 1] as usize];
    }
    None
}
```

**Why it's faster**:
- Skips characters based on bad character heuristic
- More efficient for longer patterns
- Sub-linear average case performance

**Trade-offs**: More complex, requires preprocessing

#### Recommendation

**Start with Option 1** (`memchr`):
1. Add to dependencies: `memchr = "2.7"`
2. Replace `naive_search` with `memmem::find`
3. Run benchmarks to verify improvement
4. Profile real-world usage

**Consider Option 2** if:
- You want to avoid dependencies
- You need custom behavior (e.g., wildcards)
- You want to optimize for specific pattern types

### Priority 2: Hex Pattern Parsing

#### Current Implementation

```rust
pub fn parse_hex_pattern(s: &str) -> anyhow::Result<Vec<u8>> {
    let filtered: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    // ... parsing logic
}
```

**Issues**:
1. Allocates intermediate `String` for filtered version
2. Uses `from_str_radix` which is slower than lookup tables
3. Iterates over chars twice (filter + parse)

#### Option 1: Zero-Copy Parsing (Easy, Medium Impact)

**Difficulty**: ⭐ Easy  
**Expected Speedup**: 2-3x  

```rust
pub fn parse_hex_pattern_optimized(s: &str) -> anyhow::Result<Vec<u8>> {
    let bytes = s.as_bytes();
    let mut result = Vec::with_capacity(s.len() / 2);
    
    let mut i = 0;
    while i < bytes.len() {
        // Skip whitespace
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }
        
        // Read two hex digits
        if i + 1 >= bytes.len() {
            anyhow::bail!("hex pattern length must be even");
        }
        
        let high = hex_digit(bytes[i])?;
        let low = hex_digit(bytes[i + 1])?;
        result.push((high << 4) | low);
        i += 2;
    }
    
    Ok(result)
}

#[inline]
fn hex_digit(c: u8) -> anyhow::Result<u8> {
    match c {
        b'0'..=b'9' => Ok(c - b'0'),
        b'a'..=b'f' => Ok(c - b'a' + 10),
        b'A'..=b'F' => Ok(c - b'A' + 10),
        _ => anyhow::bail!("invalid hex character '{}'", c as char),
    }
}
```

**Why it's faster**:
- No intermediate string allocation
- Single pass through input
- Simple lookup instead of radix parsing

#### Option 2: Use `hex` crate (Easiest, Medium Impact)

**Difficulty**: ⭐ Easiest  
**Expected Speedup**: 2-4x  

```rust
use hex;

pub fn parse_hex_pattern_hex_crate(s: &str) -> anyhow::Result<Vec<u8>> {
    let filtered: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    hex::decode(&filtered).map_err(|e| anyhow::anyhow!("invalid hex: {}", e))
}
```

**Why it's faster**:
- Optimized implementation with SIMD
- Well-tested
- Handles edge cases

**Trade-offs**: Still needs to filter whitespace first

#### Recommendation

**Start with Option 1** (Zero-Copy Parsing):
1. Implement as shown above
2. Run benchmarks to verify improvement
3. Consider Option 2 if you want battle-tested code

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

## Best Practices

1. **Benchmark on target hardware**: Performance characteristics differ between dev and production
2. **Use realistic data**: Benchmarks should reflect actual use cases
3. **Measure end-to-end**: Don't optimize micro-benchmarks at the expense of real-world performance
4. **Document assumptions**: Note why certain benchmarks matter
5. **Version your benchmarks**: Keep them in sync with code changes
6. **Automate regression testing**: Catch performance regressions in CI

### Optimization Workflow

1. **Measure First**: Run benchmarks to establish baseline
2. **Identify Bottleneck**: Focus on the slowest operation with highest impact
3. **Hypothesis**: Form a theory about what's slow and why
4. **Implement**: Make targeted optimization
5. **Benchmark**: Run benchmarks to verify improvement
6. **Profile**: Use profiler to understand if bottleneck moved
7. **Iterate**: Repeat until performance goals met

### Noise Reduction Tips

For more accurate benchmarks:

1. **Close other applications**: Reduce CPU contention
2. **Disable CPU frequency scaling**: Use performance governor (Linux)
3. **Pin to specific cores**: Reduce scheduling variance
4. **Run multiple times**: Look for consistency across runs

## Performance Targets

Based on typical use cases:

| Operation | Current | Target | Status |
|-----------|---------|--------|--------|
| Pattern search (4KB) | ~1.2 µs | ~0.3 µs | 🔴 Needs optimization |
| Hex parsing (16 bytes) | ~150 ns | ~50 ns | 🟡 Could be better |
| Memory read (4KB page) | ~5 µs | ~2 µs | 🟢 I/O bound, acceptable |
| Full process scan (1GB) | ~3s | ~1s | 🔴 Needs optimization |

## CI/CD Integration

### GitHub Actions

The project includes a GitHub Actions workflow that:
- Runs benchmarks on Windows (matches target platform)
- Archives results as artifacts
- Generates summary in PR comments

### Running in CI

```yaml
- name: Run benchmarks
  run: cargo bench --no-fail-fast

- name: Archive results
  uses: actions/upload-artifact@v4
  with:
    name: benchmark-results
    path: target/criterion/
```

## Further Reading

- [Criterion.rs User Guide](https://bheisler.github.io/criterion.rs/book/)
- [The Rust Performance Book](https://nnethercote.github.io/perf-book/)
- [Benchmarking and Optimization in Rust](https://www.youtube.com/watch?v=d7pZNJBdU6s)
- [String Searching Algorithms](https://en.wikipedia.org/wiki/String-searching_algorithm)

## Contributing Benchmarks

When adding new performance-critical code:

1. Add corresponding benchmarks
2. Document why the code is performance-critical
3. Include realistic test cases
4. Set performance budgets if applicable
5. Update this guide with the new benchmark details
