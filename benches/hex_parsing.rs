//! Benchmark for hex pattern parsing
//!
//! This benchmarks the parse_hex_pattern function which converts
//! user input like "DEADBEEF" or "4D 5A 90 00" into byte arrays.

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use libmemscan::parse_hex_pattern;

fn benchmark_hex_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("hex_parsing");

    // Test patterns of varying lengths
    let test_patterns = [
        ("short_compact", "4D5A"),
        ("short_spaced", "4D 5A"),
        ("medium_compact", "DEADBEEF12345678"),
        ("medium_spaced", "DE AD BE EF 12 34 56 78"),
        (
            "long_compact",
            "4D5A90000300000004000000FFFF00000800000000000000",
        ),
        (
            "long_spaced",
            "4D 5A 90 00 03 00 00 00 04 00 00 00 FF FF 00 00 08 00 00 00 00 00 00 00",
        ),
        ("very_long", &"DEADBEEF".repeat(32)),
    ];

    for (name, pattern) in test_patterns.iter() {
        group.throughput(Throughput::Bytes(pattern.len() as u64));

        group.bench_with_input(BenchmarkId::new("parse", name), pattern, |b, &pattern| {
            b.iter(|| parse_hex_pattern(black_box(pattern)));
        });
    }

    group.finish();
}

fn benchmark_hex_parsing_realistic(c: &mut Criterion) {
    let mut group = c.benchmark_group("hex_parsing_realistic");

    // Realistic patterns that users might search for
    let realistic_patterns = [
        ("pe_header", "4D 5A 90 00"),
        ("elf_header", "7F 45 4C 46"),
        ("zip_signature", "50 4B 03 04"),
        ("png_header", "89 50 4E 47 0D 0A 1A 0A"),
        ("jpeg_header", "FF D8 FF"),
        ("shellcode_pattern", "90 90 90 90 31 C0 50 68 2F 2F 73 68"),
        ("function_prologue", "55 8B EC 83 EC"),
        ("ret_instruction", "C3"),
    ];

    for (name, pattern) in realistic_patterns.iter() {
        group.throughput(Throughput::Bytes(pattern.len() as u64));

        group.bench_with_input(
            BenchmarkId::new("realistic", name),
            pattern,
            |b, &pattern| {
                b.iter(|| parse_hex_pattern(black_box(pattern)));
            },
        );
    }

    group.finish();
}

fn benchmark_hex_parsing_variations(c: &mut Criterion) {
    let mut group = c.benchmark_group("hex_parsing_variations");

    let _base_pattern = "DEADBEEF";

    // Compare different whitespace patterns
    let variations = [
        ("no_spaces", "DEADBEEF12345678CAFEBABE"),
        ("single_spaces", "DE AD BE EF 12 34 56 78 CA FE BA BE"),
        (
            "double_spaces",
            "DE  AD  BE  EF  12  34  56  78  CA  FE  BA  BE",
        ),
        ("mixed_spaces", "DEAD BEEF  1234 5678   CAFE BABE"),
        ("lowercase", "deadbeef12345678cafebabe"),
        ("mixed_case", "DeAdBeEf12345678CaFeBaBe"),
    ];

    for (name, pattern) in variations.iter() {
        group.throughput(Throughput::Bytes(pattern.len() as u64));

        group.bench_with_input(
            BenchmarkId::new("variation", name),
            pattern,
            |b, &pattern| {
                b.iter(|| parse_hex_pattern(black_box(pattern)));
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    benchmark_hex_parsing,
    benchmark_hex_parsing_realistic,
    benchmark_hex_parsing_variations
);
criterion_main!(benches);
