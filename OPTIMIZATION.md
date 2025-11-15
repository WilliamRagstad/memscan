# Performance Optimization Recommendations

This document provides specific optimization recommendations for MemScan's performance-critical code paths, based on benchmarking analysis and industry best practices.

## Priority 1: Pattern Search Algorithm

### Current Implementation
**File**: `src/scanner.rs:193-203`

```rust
fn naive_search(haystack: &[u8], needle: &[u8]) -> Option<usize> {
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

### Recommended Optimizations

#### Option 1: Use `memchr` crate (Easy, High Impact)

**Difficulty**: ⭐ Easy  
**Expected Speedup**: 5-20x for typical patterns  
**Dependencies**: `memchr` crate

```rust
use memchr::memmem;

fn optimized_search(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    memmem::find(haystack, needle)
}
```

**Why it's faster**:
- Uses SIMD instructions (SSE2/AVX2)
- Highly optimized C-like implementation
- Well-tested and maintained

**Trade-offs**: Adds dependency, but it's widely used (e.g., by ripgrep)

#### Option 2: Boyer-Moore-Horspool Algorithm (Medium, High Impact)

**Difficulty**: ⭐⭐ Medium  
**Expected Speedup**: 3-10x for patterns > 4 bytes  
**Dependencies**: None (pure Rust implementation)

```rust
fn boyer_moore_horspool(haystack: &[u8], needle: &[u8]) -> Option<usize> {
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

#### Option 3: Two-Way Algorithm (Hard, Medium Impact)

**Difficulty**: ⭐⭐⭐ Hard  
**Expected Speedup**: 2-5x, especially for patterns with repetition  
**Dependencies**: None

This is what Rust's stdlib uses internally. Consider using `std::str::pattern` infrastructure if possible.

**Trade-offs**: Very complex to implement correctly

### Recommendation

**Start with Option 1** (`memchr`):
1. Add to dependencies: `memchr = "2.7"`
2. Replace `naive_search` with `memmem::find`
3. Run benchmarks to verify improvement
4. Profile real-world usage

**Consider Option 2** if:
- You want to avoid dependencies
- You need custom behavior (e.g., wildcards)
- You want to optimize for specific pattern types

## Priority 2: Hex Pattern Parsing

### Current Implementation
**File**: `src/main.rs:76-91`

```rust
fn parse_hex_pattern(s: &str) -> anyhow::Result<Vec<u8>> {
    let filtered: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    // ... parsing logic
}
```

**Issues**:
1. Allocates intermediate `String` for filtered version
2. Uses `from_str_radix` which is slower than lookup tables
3. Iterates over chars twice (filter + parse)

### Recommended Optimizations

#### Option 1: Zero-Copy Parsing (Easy, Medium Impact)

**Difficulty**: ⭐ Easy  
**Expected Speedup**: 2-3x  

```rust
fn parse_hex_pattern_optimized(s: &str) -> anyhow::Result<Vec<u8>> {
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

fn parse_hex_pattern_hex_crate(s: &str) -> anyhow::Result<Vec<u8>> {
    let filtered: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    hex::decode(&filtered).map_err(|e| anyhow::anyhow!("invalid hex: {}", e))
}
```

**Why it's faster**:
- Optimized implementation with SIMD
- Well-tested
- Handles edge cases

**Trade-offs**: Still needs to filter whitespace first

#### Option 3: SIMD Hex Decoding (Hard, High Impact)

**Difficulty**: ⭐⭐⭐⭐ Hard  
**Expected Speedup**: 5-10x for long patterns  

Use crates like `faster-hex` or implement custom SIMD using `std::simd`.

**Trade-offs**: Complex, platform-specific optimizations needed

### Recommendation

**Start with Option 1** (Zero-Copy Parsing):
1. Implement as shown above
2. Run benchmarks to verify improvement
3. Consider Option 2 if you want battle-tested code

## Priority 3: Memory Reading Optimization

### Current Implementation
**File**: `src/scanner.rs:103-123`

**Key observation**: ReadProcessMemory is called for every page, even though we could potentially batch reads.

### Recommended Optimizations

#### Option 1: Increase Read Buffer Size

**Difficulty**: ⭐ Easy  
**Expected Speedup**: 10-30% for large scans  

```rust
// Instead of page_size (4KB), use larger buffer
let read_buffer_size = page_size * 16; // 64KB
let mut page_buf = vec![0u8; read_buffer_size];
```

**Why it's faster**:
- Fewer kernel transitions
- Better memory locality
- Amortizes system call overhead

**Trade-offs**: Higher memory usage (negligible for modern systems)

#### Option 2: Memory-Mapped I/O

**Difficulty**: ⭐⭐⭐ Hard  
**Expected Speedup**: 2-3x for sequential access  

Use `MapViewOfFile2` (already referenced in `memoryapi.rs`) to map remote process memory.

**Why it's faster**:
- Zero-copy access
- Kernel handles paging
- Better for sequential scans

**Trade-offs**: Complex error handling, not all memory can be mapped

## Priority 4: Multi-threading

### Current Status
Single-threaded scanning of all memory regions.

### Recommended Optimization

**Difficulty**: ⭐⭐ Medium  
**Expected Speedup**: 2-4x on multi-core systems  

```rust
use rayon::prelude::*;

// Collect all regions first
let regions: Vec<MemoryRegion> = MemoryRegionIterator::new(proc, sys).collect();

// Scan in parallel
regions.par_iter().for_each(|region| {
    // Scan this region
    // Note: Need thread-safe process handle
});
```

**Why it's faster**:
- CPU-bound pattern searching parallelizes well
- Modern systems have 4+ cores
- Memory reading can overlap with pattern search

**Trade-offs**: 
- Need thread-safe process handle management
- Careful with output ordering
- Consider synchronization overhead

## Benchmark-Driven Development Workflow

1. **Establish Baseline**
   ```bash
   cargo bench -- --save-baseline before
   ```

2. **Implement Optimization**
   - Start with smallest, safest change
   - Keep old implementation for comparison

3. **Verify with Benchmarks**
   ```bash
   cargo bench -- --baseline before
   ```

4. **Profile in Real Scenarios**
   - Benchmarks might not reflect real-world performance
   - Test with actual process scanning

5. **Measure Memory Usage**
   ```bash
   cargo build --release
   # Use Windows Task Manager or WPA to monitor memory
   ```

6. **Iterate**
   - If improvement is significant, commit
   - If not, try next optimization

## Optimization Anti-Patterns to Avoid

### ❌ Don't: Optimize Without Measuring
```rust
// Bad: Adding complexity without benchmarks
fn overly_complex_search(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    // 500 lines of "optimized" code that might be slower
}
```

### ✅ Do: Measure First, Then Optimize
```rust
// Good: Simple implementation with benchmarks
// BENCHMARK: 1.2 µs for 4KB page
fn naive_search(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    // Simple, correct implementation
}

// Only after benchmarking shows it's slow:
// BENCHMARK: 0.3 µs for 4KB page (4x faster)
fn optimized_search(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    // Optimized with proven benefit
}
```

### ❌ Don't: Micro-Optimize Non-Critical Paths
```rust
// Bad: Optimizing one-time initialization
fn parse_args_super_fast() -> Args {
    // Saving nanoseconds on program startup
}
```

### ✅ Do: Focus on Hot Paths
```rust
// Good: Optimizing code called millions of times
fn search_page(page: &[u8], pattern: &[u8]) -> Vec<usize> {
    // Called for every 4KB page in multi-GB process
}
```

## Performance Targets

Based on typical use cases:

| Operation | Current | Target | Status |
|-----------|---------|--------|--------|
| Pattern search (4KB) | ~1.2 µs | ~0.3 µs | 🔴 Needs optimization |
| Hex parsing (16 bytes) | ~150 ns | ~50 ns | 🟡 Could be better |
| Memory read (4KB page) | ~5 µs | ~2 µs | 🟢 I/O bound, acceptable |
| Full process scan (1GB) | ~3s | ~1s | 🔴 Needs optimization |

## Further Reading

- [The Rust Performance Book](https://nnethercote.github.io/perf-book/)
- [Fast String Search in Rust](https://blog.burntsushi.net/ripgrep/)
- [SIMD for Fast String Processing](https://branchfree.org/2018/05/22/bits-to-indexes-in-bmi2-and-avx-512/)
- [Windows Memory Management Internals](https://learn.microsoft.com/en-us/windows/win32/memory/memory-management)

## Contributing

When implementing optimizations:

1. Add benchmarks for the new code path
2. Include performance comparison in PR description
3. Document any trade-offs or edge cases
4. Update this document with lessons learned
