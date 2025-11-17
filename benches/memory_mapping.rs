//! Benchmark for memory mapping vs ReadProcessMemory
//!
//! This benchmarks the performance difference between using memory-mapped
//! sections and traditional ReadProcessMemory calls.

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use libmemscan::diff::{MemoryRegionSnapshot, diff_snapshots};

fn benchmark_diff_snapshots(c: &mut Criterion) {
    let mut group = c.benchmark_group("diff_snapshots");

    // Test with different memory sizes
    for size in [1024, 4096, 16384, 65536].iter() {
        let data = vec![0xAA; *size];
        let old_snapshot = MemoryRegionSnapshot::from_slice(&data);
        let mut new_data = vec![0xAA; *size];
        // Add some changes
        if *size >= 100 {
            new_data[50] = 0xBB;
            new_data[100] = 0xCC;
            new_data[*size / 2] = 0xDD;
        }
        let new_snapshot = MemoryRegionSnapshot::from_slice(&new_data);

        group.throughput(Throughput::Bytes(*size as u64));

        group.bench_with_input(BenchmarkId::new("few_changes", size), size, |b, &_size| {
            b.iter(|| diff_snapshots(black_box(&old_snapshot), black_box(&new_snapshot)));
        });
    }

    // Test with many changes
    for size in [1024, 4096, 16384, 65536].iter() {
        let data = vec![0xAA; *size];
        let old_snapshot = MemoryRegionSnapshot::from_slice(&data);
        let mut new_data = vec![0xAA; *size];
        // Change every 10th byte
        for i in (0..*size).step_by(10) {
            new_data[i] = 0xBB;
        }
        let new_snapshot = MemoryRegionSnapshot::from_slice(&new_data);

        group.throughput(Throughput::Bytes(*size as u64));

        group.bench_with_input(BenchmarkId::new("many_changes", size), size, |b, &_size| {
            b.iter(|| diff_snapshots(black_box(&old_snapshot), black_box(&new_snapshot)));
        });
    }

    // Test with no changes (best case)
    for size in [1024, 4096, 16384, 65536].iter() {
        let data = vec![0xAA; *size];
        let old_snapshot = MemoryRegionSnapshot::from_slice(&data);
        let new_snapshot = MemoryRegionSnapshot::from_slice(&data);

        group.throughput(Throughput::Bytes(*size as u64));

        group.bench_with_input(BenchmarkId::new("no_changes", size), size, |b, &_size| {
            b.iter(|| diff_snapshots(black_box(&old_snapshot), black_box(&new_snapshot)));
        });
    }

    group.finish();
}

fn benchmark_snapshot_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("snapshot_creation");

    // Benchmark creating snapshots from different data sizes
    for size in [1024, 4096, 16384, 65536].iter() {
        let data = vec![0xAA; *size];

        group.throughput(Throughput::Bytes(*size as u64));

        group.bench_with_input(BenchmarkId::new("from_vec", size), size, |b, &_size| {
            b.iter(|| {
                let snapshot = MemoryRegionSnapshot::from_slice(&data);
                black_box(snapshot)
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    benchmark_diff_snapshots,
    benchmark_snapshot_creation
);
criterion_main!(benches);
