//! Benchmark for pattern searching algorithms
//!
//! This benchmarks the naive_search function which is critical for performance
//! when scanning large memory regions for byte patterns.

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use memscan::scanner::naive_search;

fn benchmark_pattern_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("pattern_search");

    // Test with different haystack sizes to understand scaling behavior
    for size in [1024, 4096, 16384, 65536].iter() {
        // Prepare test data
        let haystack = vec![0u8; *size];
        let pattern_short = b"MZ"; // Common PE header pattern
        let pattern_medium = b"\x4D\x5A\x90\x00"; // MZ header with padding
        let pattern_long = b"\x4D\x5A\x90\x00\x03\x00\x00\x00\x04\x00\x00\x00";

        group.throughput(Throughput::Bytes(*size as u64));

        // Benchmark: Pattern not found (worst case - full scan)
        group.bench_with_input(BenchmarkId::new("miss_short", size), size, |b, &_size| {
            b.iter(|| naive_search(black_box(&haystack), black_box(pattern_short)));
        });

        group.bench_with_input(BenchmarkId::new("miss_medium", size), size, |b, &_size| {
            b.iter(|| naive_search(black_box(&haystack), black_box(pattern_medium)));
        });

        group.bench_with_input(BenchmarkId::new("miss_long", size), size, |b, &_size| {
            b.iter(|| naive_search(black_box(&haystack), black_box(pattern_long)));
        });
    }

    // Test with pattern at different positions
    let haystack_with_pattern = {
        let mut data = vec![0xAA; 65536];
        // Place pattern at the beginning
        data[0..4].copy_from_slice(b"\x4D\x5A\x90\x00");
        // Place pattern in the middle
        data[32768..32772].copy_from_slice(b"\x4D\x5A\x90\x00");
        // Place pattern near the end
        data[65530..65534].copy_from_slice(b"\x4D\x5A\x90\x00");
        data
    };
    let pattern = b"\x4D\x5A\x90\x00";

    group.throughput(Throughput::Bytes(65536));

    group.bench_function("hit_beginning", |b| {
        b.iter(|| naive_search(black_box(&haystack_with_pattern), black_box(pattern)));
    });

    group.bench_function("hit_middle", |b| {
        b.iter(|| {
            // Start searching from position that would find middle pattern first
            naive_search(black_box(&haystack_with_pattern[100..]), black_box(pattern))
        });
    });

    group.finish();
}

fn benchmark_pattern_search_realistic(c: &mut Criterion) {
    let mut group = c.benchmark_group("pattern_search_realistic");

    // Simulate realistic memory page scanning scenarios
    // Windows typically uses 4KB pages
    let page_size = 4096;

    // Typical executable memory with various patterns
    let mut realistic_page = vec![0u8; page_size];
    // Add some realistic binary data patterns
    realistic_page[0..2].copy_from_slice(b"MZ"); // PE header
    realistic_page[100..104].copy_from_slice(b"\x55\x8B\xEC\x83"); // Common x86 prologue
    realistic_page[500..508].copy_from_slice(b"\x48\x89\x5C\x24\x08\x48\x89\x74"); // x64 pattern

    group.throughput(Throughput::Bytes(page_size as u64));

    // Common search patterns in reverse engineering
    let patterns = [
        ("pe_header", b"MZ" as &[u8]),
        ("x86_prologue", b"\x55\x8B\xEC\x83" as &[u8]),
        ("x64_pattern", b"\x48\x89\x5C\x24" as &[u8]),
        ("string_ref", b"http://" as &[u8]),
        ("rare_pattern", b"\xDE\xAD\xBE\xEF" as &[u8]),
    ];

    for (name, pattern) in patterns.iter() {
        group.bench_with_input(
            BenchmarkId::new("realistic", name),
            pattern,
            |b, &pattern| {
                b.iter(|| naive_search(black_box(&realistic_page), black_box(pattern)));
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    benchmark_pattern_search,
    benchmark_pattern_search_realistic
);
criterion_main!(benches);
