# MemScan

A simple memory scanning tool for **Windows** and **Linux** process debugging, dynamic analysis, and reverse engineering purposes.
The functionality is implemented using only user-mode APIs. However, elevated privileges may still be required.

## Features

- [X] Scan a process's memory for specific byte patterns
- [X] Cross-platform support
- [X] **Memory mapping for instant change detection**
- [X] **Parallel diffing of watched memory regions**
- [X] **Interactive mode with REPL for iterative scanning**
- [X] **Value filtering by type (integers, floats) and comparison operations**
- [X] **Memory modification with math operations (add, subtract, multiply, divide)**
- [X] **Relative checkpoint filtering for finding values with consistent change rates**
- [X] **Dynamic region cleanup for efficient memory usage**
- [X] Support for scanning large memory regions efficiently
- [X] Filter memory regions based on module ownership
- [X] **Python bindings for scriptable (automated) scans**
- [ ] Configurable scanning options (e.g., case sensitivity, wildcards)
- [ ] Identify dynamic memory regions (e.g., images, stack, all heaps, allocated virtual pages)

## Usage

### Pattern Scanning

Scan a process's memory for a specific byte pattern:

```sh
memscan scan <process_id/name> --pattern <byte_pattern> [options]
```

### Interactive Mode

Launch an interactive REPL to iteratively filter memory addresses by value:

```sh
memscan interactive <process_id/name> [--value-type <type>] [--all-modules]
```

Value types: `i8`, `i16`, `i32` (default), `i64`, `u8`, `u16`, `u32`, `u64`, `f32`, `f64`

#### Interactive Mode Commands

- `help` - Show available commands
- `list` - List current matched addresses (max 20)
- `filter <op> [value]` - Filter addresses by condition
  - Comparison ops: `eq`, `lt`, `gt` (requires value)
  - Change ops: `inc`, `dec`, `changed`, `unchanged` (no value required)
  - Relative checkpoint filter: `checkpoint <cp1> <cp2> <cp3> <margin%>`
- `checkpoint <subcommand>` - Manage memory checkpoints
  - `save <name>` - Save current memory state
  - `list` - List all saved checkpoints
  - `delete <name>` - Delete a checkpoint
- `set <value> [address]` - Set value at address(es)
- `add/sub/mul/div <value> [address]` - Apply math operation
- `quit` - Exit interactive mode

#### Example Interactive Session

```sh
$ memscan interactive 1234 --value-type i32
[info] system info: min_addr=0000000000010000, max_addr=00007ffffffeffff, page_size=4096, granularity=65536
[info] found 133 module regions
=== Interactive Memory Scanner ===
[info] Type 'help' for available commands

[info] Performing initial scan for I32 values...
[done] Found 5000000 possible addresses across 250 regions

> filter eq 100
[done] Filtered from 5000000 to 50 addresses (12 regions)

> list
50 matches found
  0: 00007ff6a1234000 = 100
  1: 00007ff6a1234100 = 100
  ...

> filter inc
[done] Filtered from 50 to 5 addresses (2 regions)

> list
5 matches found
  0: 00007ff6a1234000 = 105 (was: 100)
  1: 00007ff6a1234100 = 103 (was: 100)
  ...

> set 200
[done] Set value at 5 addresses

> quit
[info] Exiting...
```

#### Checkpoint-based Relative Filtering

Find values that change at a consistent rate across multiple observations:

```sh
> filter eq 100
[done] Filtered to 50 addresses

> checkpoint save cp1
[done] Saved checkpoint 'cp1'

# Wait for values to change (e.g., +10)
> filter inc
[done] Filtered to 10 addresses

> checkpoint save cp2
[done] Saved checkpoint 'cp2'

# Wait for values to change again (e.g., +10 more)
> filter inc
[done] Filtered to 5 addresses

> checkpoint save cp3
[done] Saved checkpoint 'cp3'

# Filter for addresses where (cp2-cp1) ≈ (cp3-cp2) within 10% margin
> filter checkpoint cp1 cp2 cp3 10.0
[done] Filtered to 3 addresses
```

This technique is useful for finding values that increment at a consistent rate, such as timers, counters, or resource values.

### Pattern Scan Example

<details>
<summary>Verbose memory scan output for Notepad.exe searching for "MZ" header pattern</summary>

```sh
$ memscan scan notepad --pattern "4D 5A 90 00" -vv
[info] looking up process by name: notepad
[info] found pid=2872
[info] system info: min_addr=0000000000010000, max_addr=00007ffffffeffff, page_size=4096, granularity=65536
[info] found 133 module regions
[region] 000000007ffe0000 - 000000007ffe1000 (4 KiB)    [PRIVATE, COMMIT, READONLY, unknown]
[region] 000000007ffea000 - 000000007ffeb000 (4 KiB)    [PRIVATE, COMMIT, READONLY, unknown]
[region] 000000b005baa000 - 000000b005bb0000 (24 KiB)   [PRIVATE, COMMIT, READWRITE, unknown]
--- snip ---
[region] 000001f1a30b0000 - 000001f1a30b1000 (4 KiB)    [MAPPED, COMMIT, READWRITE, unknown]
[region] 000001f1a30c0000 - 000001f1a30c7000 (28 KiB)   [MAPPED, COMMIT, READONLY, unknown]
[match]  000001f1a30c0000
 ... 4d 5a 90 00 03 00 00 00 04 00 00 00  ...
[region] 000001f1a30d0000 - 000001f1a30d3000 (12 KiB)   [MAPPED, COMMIT, READONLY, unknown]
[region] 000001f1a30e0000 - 000001f1a30e1000 (4 KiB)    [MAPPED, COMMIT, READWRITE, unknown]
[region] 000001f1a30f0000 - 000001f1a30fb000 (44 KiB)   [MAPPED, COMMIT, READONLY, unknown]
[region] 000001f1a3100000 - 000001f1a3247000 (1308 KiB)         [MAPPED, COMMIT, READONLY, unknown]
[region] 000001f1a3250000 - 000001f1a3263000 (76 KiB)   [MAPPED, COMMIT, READONLY, unknown]
[region] 000001f1a3270000 - 000001f1a3271000 (4 KiB)    [MAPPED, COMMIT, READONLY, unknown]
[region] 000001f1a3280000 - 000001f1a338f000 (1084 KiB)         [IMAGE, COMMIT, READONLY, unknown]
[match]  000001f1a3280000
 ... 4d 5a 90 00 03 00 00 00 04 00 00 00  ...
[region] 000001f1a3390000 - 000001f1a341a000 (552 KiB)  [IMAGE, COMMIT, READONLY, unknown]
[match]  000001f1a3390000
 ... 4d 5a 90 00 03 00 00 00 04 00 00 00  ...
[region] 000001f1a3420000 - 000001f1a3424000 (16 KiB)   [MAPPED, COMMIT, READONLY, unknown]
[region] 000001f1a3430000 - 000001f1a3432000 (8 KiB)    [MAPPED, COMMIT, READONLY, unknown]
--- snip ---
[region] 00007ffb85d3b000 - 00007ffb85d3c000 (4 KiB)    [IMAGE, COMMIT, EXECUTE_READ, unknown]
[region] 00007ffb85d50000 - 00007ffb85d51000 (4 KiB)    [IMAGE, COMMIT, READONLY, SHLWAPI.dll]
[match]  00007ffb85d50000
 ... 4d 5a 90 00 03 00 00 00 04 00 00 00  ...
[region] 00007ffb85d51000 - 00007ffb85d8d000 (240 KiB)  [IMAGE, COMMIT, EXECUTE_READ, SHLWAPI.dll]
[region] 00007ffb85d8d000 - 00007ffb85dae000 (132 KiB)  [IMAGE, COMMIT, READONLY, SHLWAPI.dll]
[region] 00007ffb85dae000 - 00007ffb85db0000 (8 KiB)    [IMAGE, COMMIT, READWRITE, SHLWAPI.dll]
[region] 00007ffb85db0000 - 00007ffb85db7000 (28 KiB)   [IMAGE, COMMIT, READONLY, SHLWAPI.dll]
[region] 00007ffb85db7000 - 00007ffb85db8000 (4 KiB)    [IMAGE, COMMIT, EXECUTE_READ, unknown]
[region] 00007ffb85e60000 - 00007ffb85e61000 (4 KiB)    [IMAGE, COMMIT, READONLY, ntdll.dll]
[match]  00007ffb85e60000
 ... 4d 5a 90 00 03 00 00 00 04 00 00 00  ...
[region] 00007ffb85e61000 - 00007ffb85fd3000 (1480 KiB)         [IMAGE, COMMIT, EXECUTE_READ, ntdll.dll]
[region] 00007ffb85fd3000 - 00007ffb8602c000 (356 KiB)  [IMAGE, COMMIT, READONLY, ntdll.dll]
[region] 00007ffb8602c000 - 00007ffb86036000 (40 KiB)   [IMAGE, COMMIT, READWRITE, ntdll.dll]
[region] 00007ffb86036000 - 00007ffb860c8000 (584 KiB)  [IMAGE, COMMIT, READONLY, ntdll.dll]
[region] 00007ffb860c8000 - 00007ffb860c9000 (4 KiB)    [IMAGE, COMMIT, EXECUTE_READ, unknown]
[done] scanned 1544 regions, ~507292 KiB, 140 matches
```

</details>

## `libmemscan` crate

**Programmatic change detection:**
```rust
use libmemscan::process::open_process;
use libmemscan::diff::ChangeDetector;

let proc = open_process(pid)?;
let mut detector = ChangeDetector::new();
detector.initialize(&proc, &regions)?;

// ... wait for changes ...

let changes = detector.detect_changes(&proc, &regions)?;
```

For detailed information, see [MEMORY_MAPPING.md](llm/MEMORY_MAPPING.md).

## Python Bindings

MemScan provides Python bindings for scriptable and automated memory analysis. The Python API is explicit and provides fine-grained control over all operations.

### Installation

```bash
# Install maturin (build tool)
pip install maturin

# Build and install in development mode
maturin develop

# Or build a wheel
maturin build --release
pip install target/wheels/memscan-*.whl
```

### Quick Example

```python
import memscan

# Find and open a process
pid = memscan.find_process_by_name("game")
proc = memscan.open_process(pid)

# Get memory regions
modules = memscan.get_process_module_regions(proc)

# Create interactive scanner for 32-bit integers
scanner = memscan.create_interactive_scanner(proc, modules, "i32")

# Perform initial scan
scanner.initial_scan()

# Filter by exact value (e.g., health = 100)
scanner.filter_eq(100)

# Wait for value to increase
scanner.filter_increased()

# List matches
matches = scanner.get_matches()
for match in matches[:10]:
    print(f"Address: 0x{match.address:016x}, Value: {match.current_value}")

# Set value at all matched addresses
scanner.set_value(999)
```

For detailed documentation and more examples, see [python/README.md](python/README.md) and [examples/python_example.py](examples/python_example.py).

## Performance Benchmarking

MemScan includes comprehensive benchmarking infrastructure using `Criterion.rs`. For detailed information, see [BENCHMARKING.md](./llm/BENCHMARKING.md).

Quick start:

```sh
cargo bench                           # Run all benchmarks
cargo bench --bench pattern_search   # Run specific benchmark
cargo bench --bench memory_mapping   # Benchmark memory mapping
./bench.sh report                     # Open HTML report (Unix)
.\bench.ps1 report                    # Open HTML report (Windows)
```

## References

The key Windows API is `VirtualQueryEx`, which retrieves information about a range of pages in the virtual address space of **a specified process**. Efficiently scanning a process's memory regions is done via a shared object mapping through  `CreateFileMapping` and `MapViewOfFile` into pre-allocated local pages.

- Windows Functions
  - `GetNativeSystemInfo`: [msdn](https://learn.microsoft.com/en-us/windows/win32/api/sysinfoapi/nf-sysinfoapi-getnativesysteminfo), [winapi](https://docs.rs/winapi/latest/winapi/um/sysinfoapi/fn.GetNativeSystemInfo.html)
  - `EnumProcessModules`: [msdn](https://learn.microsoft.com/en-us/windows/win32/api/psapi/nf-psapi-enumprocessmodules), [winapi](https://docs.rs/winapi/latest/winapi/um/psapi/fn.EnumProcessModules.html)
  - `GetModuleInformation`: [msdn](https://learn.microsoft.com/en-us/windows/win32/api/psapi/nf-psapi-getmoduleinformation), [winapi](https://docs.rs/winapi/latest/winapi/um/psapi/fn.GetModuleInformation.html)
  - `VirtualQueryEx`: [msdn](https://learn.microsoft.com/en-us/windows/win32/api/memoryapi/nf-memoryapi-virtualqueryex), [winapi](https://docs.rs/winapi/latest/winapi/um/memoryapi/fn.VirtualQueryEx.html)
  - `VirtualAlloc`: [msdn](https://learn.microsoft.com/en-us/windows/win32/api/memoryapi/nf-memoryapi-virtualalloc), [winapi](https://docs.rs/winapi/latest/winapi/um/memoryapi/fn.VirtualAlloc.html)
  - `ReadProcessMemory`: [msdn](https://learn.microsoft.com/en-us/windows/win32/api/memoryapi/nf-memoryapi-readprocessmemory), [winapi](https://docs.rs/winapi/latest/winapi/um/memoryapi/fn.ReadProcessMemory.html)
  - `OpenProcess`: [msdn](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-openprocess), [winapi](https://docs.rs/winapi/latest/winapi/um/processthreadsapi/fn.OpenProcess.html)
  - `CreateFileMapping`: [msdn](https://learn.microsoft.com/en-us/windows/win32/api/memoryapi/nf-memoryapi-createfilemappingw), [winapi](https://docs.rs/winapi/latest/winapi/um/memoryapi/fn.CreateFileMappingW.html)
  - `MapViewOfFile`: [msdn](https://learn.microsoft.com/en-us/windows/win32/api/memoryapi/nf-memoryapi-mapviewoffile), [winapi](https://docs.rs/winapi/latest/winapi/um/memoryapi/fn.MapViewOfFile.html)
  - `MapViewOfFileEx`: [msdn](https://learn.microsoft.com/en-us/windows/win32/api/memoryapi/nf-memoryapi-mapviewoffileex), [winapi](https://docs.rs/winapi/latest/winapi/um/memoryapi/fn.MapViewOfFileEx.html)
  - `MapViewOfFile2`: [msdn](https://learn.microsoft.com/en-us/windows/win32/api/memoryapi/nf-memoryapi-mapviewoffile2)
- Windows Structures
  - `SYSTEM_INFO`: [msdn](https://learn.microsoft.com/en-us/windows/win32/api/sysinfoapi/ns-sysinfoapi-system_info), [winapi](https://docs.rs/winapi/latest/winapi/um/sysinfoapi/struct.SYSTEM_INFO.html)
  - `MODULEINFO`: [msdn](https://learn.microsoft.com/en-us/windows/win32/api/psapi/ns-psapi-moduleinfo), [winapi](https://docs.rs/winapi/latest/winapi/um/psapi/struct.MODULEINFO.html)
  - `MEMORY_BASIC_INFORMATION`: [msdn](https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-memory_basic_information), [winapi](https://docs.rs/winapi/latest/winapi/um/winnt/struct.MEMORY_BASIC_INFORMATION.html)
- Microsoft Docs
  - [Working with Pages](https://learn.microsoft.com/en-us/windows/win32/memory/working-with-pages)
  - [Memory Management Functions](https://learn.microsoft.com/en-us/windows/win32/memory/memory-management-functions)
  - [Memory Protection Constants](https://learn.microsoft.com/en-us/windows/win32/memory/memory-protection-constants)
  - [Process Access Rights](https://learn.microsoft.com/en-us/windows/win32/procthread/process-security-and-access-rights)
  - [Enumerating All Modules For a Process](https://learn.microsoft.com/en-us/windows/win32/psapi/enumerating-all-modules-for-a-process)
- Blogs
  - [Alex Ionescu's Blog: Windows Internals, Thoughts on Security, and Reverse Engineering](https://www.alex-ionescu.com)
  - [Enumerate process modules via VirtualQueryEx. Simple C++ example.](https://cocomelonc.github.io/malware/2023/11/07/malware-trick-37.html)
  - [List Process Modules with VirtualQueryEx](https://medium.com/@s12deff/list-process-modules-with-virtualqueryex-6c7c3e2613a6)
  - [Difference between QueryVirtualMemoryInformation and VirtualQueryEx](https://stackoverflow.com/questions/74704604/difference-between-queryvirtualmemoryinformation-and-virtualqueryex)
