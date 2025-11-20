# MemScan Python Bindings

Python bindings for the high-performance memscan library, providing scriptable process memory analysis capabilities.

## Installation

### From Source

```bash
# Install maturin (build tool for Python packages with Rust)
pip install maturin

# Build and install in development mode
maturin develop --features python

# Or build a wheel
maturin build --release --features python
pip install target/wheels/memscan-*.whl
```

## Features

- **Explicit API**: All actions require specialized function calls for fine-grained control
- **High Performance**: Built on the Rust implementation with zero-cost abstractions
- **Interactive Scanning**: Progressive memory filtering to narrow down target addresses
- **Value Type Support**: Works with integers (i8-i64, u8-u64) and floats (f32, f64)
- **Memory Operations**: Read, write, and apply mathematical operations to memory
- **Checkpoint System**: Save and compare memory states for advanced filtering

## Quick Start

### Finding and Opening a Process

```python
import memscan

# Find process by name
pid = memscan.find_process_by_name("notepad")
if pid:
    # Open the process
    proc = memscan.open_process(pid)
    
    # Get system information
    sys_info = memscan.query_system_info()
    print(sys_info)
    
    # Get module regions
    modules = memscan.get_process_module_regions(proc)
    print(f"Found {len(modules)} module regions")
```

### Interactive Memory Scanning

```python
import memscan

# Open a process
pid = memscan.find_process_by_name("game")
proc = memscan.open_process(pid)

# Get memory regions
modules = memscan.get_process_module_regions(proc)

# Create an interactive scanner for 32-bit integers
scanner = memscan.create_interactive_scanner(proc, modules, "i32")

# Perform initial scan
count = scanner.initial_scan()
print(f"Found {count} possible addresses")

# Filter by exact value (e.g., health = 100)
count = scanner.filter_eq(100)
print(f"Filtered to {count} addresses")

# Wait for value to change, then filter by increase
count = scanner.filter_increased()
print(f"Filtered to {count} addresses")

# List current matches (first 20)
matches = scanner.get_matches()
for match in matches[:20]:
    print(f"Address: 0x{match.address:016x}, Value: {match.current_value}")

# Set value at all matched addresses
scanner.set_value(999)
```

### Reading and Writing Memory

```python
import memscan

pid = memscan.find_process_by_name("target")
proc = memscan.open_process(pid)

# Read memory at a specific address
address = 0x7ff6a1234000
size = 64
data = memscan.read_process_memory(proc, address, size)
print("Memory contents:", " ".join(f"{b:02x}" for b in data))

# Write memory
new_data = bytes([0x90, 0x90, 0x90, 0x90])  # NOP instructions
bytes_written = memscan.write_process_memory(proc, address, new_data)
print(f"Wrote {bytes_written} bytes")
```

### Using Checkpoints for Advanced Filtering

```python
import memscan
import time

# Setup scanner (as above)
scanner = memscan.create_interactive_scanner(proc, modules, "i32")
scanner.initial_scan()

# Filter to a reasonable number
scanner.filter_eq(100)

# Save first checkpoint
scanner.save_checkpoint("cp1")
time.sleep(1)

# Wait for value to increase, then save second checkpoint
scanner.filter_increased()
scanner.save_checkpoint("cp2")
time.sleep(1)

# Wait for another increase, save third checkpoint
scanner.filter_increased()
scanner.save_checkpoint("cp3")

# Filter for addresses where the rate of change is consistent
# (cp2 - cp1) ≈ (cp3 - cp2) within 10% margin
scanner.filter_checkpoint("cp1", "cp2", "cp3", 10.0)

# List all checkpoints
checkpoints = scanner.list_checkpoints()
print("Saved checkpoints:", checkpoints)
```

## API Reference

### Process Management

- `open_process(pid: int) -> PyProcessHandle`: Open a process by PID
- `find_process_by_name(name: str) -> Optional[int]`: Find a process PID by name
- `query_system_info() -> PySystemInfo`: Get system memory information
- `get_process_module_regions(handle: PyProcessHandle) -> List[PyMemoryRegion]`: Get loaded module regions

### Memory Access

- `read_process_memory(handle: PyProcessHandle, address: int, size: int) -> bytes`: Read memory
- `write_process_memory(handle: PyProcessHandle, address: int, data: bytes) -> int`: Write memory

### Interactive Scanner

- `create_interactive_scanner(handle: PyProcessHandle, regions: List[PyMemoryRegion], value_type: str) -> PyInteractiveScanner`: Create scanner

#### Scanner Methods

**Scanning:**
- `initial_scan() -> int`: Perform initial scan for all values
- `match_count() -> int`: Get current number of matches
- `get_matches() -> List[PyMatchedAddress]`: Get list of matched addresses

**Filtering:**
- `filter_eq(value: float) -> int`: Filter by exact value
- `filter_lt(value: float) -> int`: Filter by less than value
- `filter_gt(value: float) -> int`: Filter by greater than value
- `filter_increased() -> int`: Filter by increased values
- `filter_decreased() -> int`: Filter by decreased values
- `filter_changed() -> int`: Filter by changed values
- `filter_unchanged() -> int`: Filter by unchanged values

**Value Modification:**
- `set_value(value: float) -> int`: Set value at all matches
- `set_value_at(address: int, value: float) -> None`: Set value at specific address
- `add_value(value: float) -> int`: Add to all matched values
- `sub_value(value: float) -> int`: Subtract from all matched values
- `mul_value(value: float) -> int`: Multiply all matched values
- `div_value(value: float) -> int`: Divide all matched values

**Checkpoints:**
- `save_checkpoint(name: str) -> None`: Save current state
- `list_checkpoints() -> List[str]`: List all checkpoint names
- `delete_checkpoint(name: str) -> None`: Delete a checkpoint
- `filter_checkpoint(cp1: str, cp2: str, cp3: str, margin: float) -> int`: Filter by consistent change rate

### Utilities

- `parse_hex_pattern(pattern: str) -> bytes`: Parse hex string to bytes (e.g., "4D 5A 90 00")

### Value Types

Supported value types for `create_interactive_scanner`:
- Signed integers: `i8`, `i16`, `i32`, `i64`
- Unsigned integers: `u8`, `u16`, `u32`, `u64`
- Floating point: `f32`, `f64`

## Examples

See the [examples/python_example.py](../examples/python_example.py) file for a complete interactive example.

## Platform Support

The Python bindings support the same platforms as the core memscan library:
- Windows (tested on Windows 10+)
- Linux (tested on Ubuntu 20.04+)

## Performance

The Python bindings provide near-native performance through:
- Zero-copy data sharing where possible
- Efficient memory mapping
- Parallel processing for memory diffing
- Minimal overhead from Python/Rust FFI boundary

## License

MIT License - See LICENSE file for details
