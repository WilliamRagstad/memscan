# Memory Mapping API

This document describes the memory mapping API for detecting instant memory changes in remote processes.

## Overview

The memory mapping API provides functionality to:
1. Map remote process memory pages into the local process address space
2. Detect memory changes by comparing snapshots
3. Track multiple memory regions simultaneously

## Architecture

### Cross-Platform Design

The API consists of three main components:

1. **MappedMemory**: Represents a mapped view of remote process memory
2. **MemoryMapper**: Manages multiple mapped memory regions
3. **ChangeDetector**: Detects changes by comparing memory snapshots

### Platform-Specific Implementations

#### Windows
- Uses `ReadProcessMemory` to read remote process memory into local buffers
- Buffer-based approach similar to Linux implementation
- Note: True memory mapping would require handle enumeration for MEM_MAPPED regions

#### Linux/Unix
- Uses `/proc/<pid>/mem` for memory access
- Implements a buffer-based approach that reads memory into local buffers
- Provides `refresh()` method to update snapshots

## API Reference

### MappedMemory

```rust
pub struct MappedMemory {
    pub remote_addr: usize,
    pub size: usize,
    // Platform-specific internals...
}

impl MappedMemory {
    /// Map a region of remote process memory
    pub fn new(proc: &ProcessHandle, region: &MemoryRegion) -> Result<Self>
    
    /// Get a slice to the mapped memory
    pub fn as_slice(&self) -> &[u8]
    
    /// Get the base address in the remote process
    pub fn remote_address(&self) -> usize
    
    /// Get the size of the mapped region
    pub fn size(&self) -> usize
}
```

### MemoryMapper

```rust
pub struct MemoryMapper {
    // Internal tracking...
}

impl MemoryMapper {
    /// Create a new empty memory mapper
    pub fn new() -> Self
    
    /// Map a memory region
    pub fn map_region(&mut self, proc: &ProcessHandle, region: &MemoryRegion) -> Result<usize>
    
    /// Get a mapped region by index
    pub fn get(&self, index: usize) -> Option<&MappedMemory>
    
    /// Get the number of mapped regions
    pub fn len(&self) -> usize
    
    /// Check if there are no mapped regions
    pub fn is_empty(&self) -> bool
    
    /// Clear all mappings
    pub fn clear(&mut self)
}
```

### ChangeDetector

```rust
pub struct ChangeDetector {
    // Internal snapshots...
}

impl ChangeDetector {
    /// Create a new change detector
    pub fn new() -> Self
    
    /// Take initial snapshots of the given regions
    pub fn initialize(&mut self, proc: &ProcessHandle, regions: &[MemoryRegion]) -> Result<()>
    
    /// Detect changes by comparing current memory state with snapshots
    pub fn detect_changes(&self, proc: &ProcessHandle, regions: &[MemoryRegion]) 
        -> Result<Vec<Vec<MemoryChange>>>
    
    /// Update snapshots to the current memory state
    pub fn update_snapshots(&mut self, proc: &ProcessHandle, regions: &[MemoryRegion]) -> Result<()>
    
    /// Get the number of tracked snapshots
    pub fn snapshot_count(&self) -> usize
}
```

### MemoryChange

```rust
pub struct MemoryChange {
    /// Address where the change was detected
    pub address: usize,
    /// Old value at this address
    pub old_value: u8,
    /// New value at this address
    pub new_value: u8,
}
```

## Usage Examples

### Integrated Scanner Usage

The memory mapping functionality is now integrated into the default scanner. By default, the scanner uses memory mapping for better performance:

```bash
# Use memory mapping (default)
memscan scan notepad --pattern "4D 5A 90 00"

# Disable memory mapping and use ReadProcessMemory
memscan scan notepad --pattern "4D 5A 90 00" --no-memmap
```

The scanner automatically falls back to `ReadProcessMemory` if memory mapping fails for a particular region.

### Basic Memory Mapping

```rust
use libmemscan::process::{open_process, query_system_info};
use libmemscan::memmap::MemoryMapper;

// Open the target process
let proc = open_process(pid)?;
let sys_info = query_system_info();

// Create a memory mapper
let mut mapper = MemoryMapper::new();

// Map a specific memory region
let region = /* ... get region from iterator ... */;
let index = mapper.map_region(&proc, &region)?;

// Access the mapped memory
if let Some(mapped) = mapper.get(index) {
    let data = mapped.as_slice();
    println!("Mapped {} bytes at {:016x}", data.len(), mapped.remote_address());
}
```

### Change Detection

```rust
use libmemscan::process::{open_process, MemoryRegionIterator};
use libmemscan::diff::ChangeDetector;

// Open the target process
let proc = open_process(pid)?;
let sys_info = query_system_info();

// Collect interesting regions
let regions: Vec<_> = MemoryRegionIterator::new(&proc, &sys_info)
    .take(10)
    .collect();

// Create a change detector and initialize with snapshots
let mut detector = ChangeDetector::new();
detector.initialize(&proc, &regions)?;

// Wait for some time or condition
std::thread::sleep(std::time::Duration::from_secs(1));

// Detect changes
let changes = detector.detect_changes(&proc, &regions)?;

for (region_idx, region_changes) in changes.iter().enumerate() {
    if !region_changes.is_empty() {
        println!("Region {} has {} changes:", region_idx, region_changes.len());
        for change in region_changes.iter().take(10) {
            println!("  {:016x}: {:02x} -> {:02x}", 
                change.address, change.old_value, change.new_value);
        }
    }
}

// Update snapshots for next comparison
detector.update_snapshots(&proc, &regions)?;
```

### Snapshot Comparison

```rust
use libmemscan::diff::{MemorySnapshot, diff_snapshots};

// Create snapshots
let old_snapshot = MemorySnapshot::from_process(&proc, &region)?;

// Wait for changes
std::thread::sleep(std::time::Duration::from_millis(100));

let new_snapshot = MemorySnapshot::from_process(&proc, &region)?;

// Compare snapshots
let changes = diff_snapshots(&old_snapshot, &new_snapshot);
println!("Found {} changes", changes.len());
```

## Performance Considerations

### Memory Mapping vs ReadProcessMemory

The memory mapping approach offers several advantages:

1. **Reduced System Call Overhead**: Memory-mapped regions can be accessed directly without repeated system calls
2. **Efficient Diffing**: Comparing snapshots is a simple memory comparison operation
3. **Parallel Processing**: Multiple regions can be compared in parallel (future enhancement)

### Benchmarks

To compare performance:

```bash
cargo bench --bench memory_mapping
```

The benchmarks measure:
- Snapshot creation time
- Diff operation speed for various change scenarios
- Scaling with different memory sizes

### Platform Differences

#### Windows
- Buffer-based approach using ReadProcessMemory
- Memory is read once into a local buffer
- Suitable for periodic change detection with refresh

#### Linux/Unix
- Buffer-based approach requires initial read
- `refresh()` method needed to update snapshots
- Suitable for periodic change detection

## Limitations and Future Work

### Current Limitations

1. **Buffer-Based Approach**: Both Windows and Linux use buffer-based reading (not true shared memory mapping)
2. **Windows Memory Mapping**: True memory mapping of remote process memory is only possible for MEM_MAPPED regions by duplicating file handles via handle enumeration (complex and not implemented)
3. **Parallel Diffing**: Sequential comparison (parallel version planned)
4. **Write Support**: Read-only access (by design for safety)

### Future Enhancements

1. **Parallel Diffing**: Use rayon for parallel snapshot comparison
2. **Advanced Windows Support**: Implement handle enumeration for MEM_MAPPED regions to enable true memory mapping
3. **Advanced Linux Support**: Investigate `process_vm_readv` for better performance
4. **Filtering**: Support for change filters (e.g., ignore specific ranges)
5. **Statistics**: Track change frequency and patterns

## Security Considerations

1. **Permissions**: Requires appropriate process access rights
2. **Read-Only**: All mappings are read-only to prevent accidental modification
3. **Resource Management**: Mappings are automatically cleaned up on drop

## Error Handling

All functions return `Result<T>` for proper error handling:

```rust
match mapper.map_region(&proc, &region) {
    Ok(index) => println!("Mapped region at index {}", index),
    Err(e) => eprintln!("Failed to map region: {}", e),
}
```

Common errors:
- `Failed to read memory`: ReadProcessMemory failed - insufficient permissions or invalid region
- `Partial read`: Region became inaccessible during read
- Process terminated or region not accessible

## See Also

- [BENCHMARKING.md](BENCHMARKING.md) - Performance benchmarking guide
- [README.md](README.md) - General project documentation
- Windows API documentation for memory mapping functions
- Linux `/proc` filesystem documentation
