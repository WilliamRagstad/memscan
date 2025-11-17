# `libmemscan` crate

`libmemscan` is a Rust library for scanning and manipulating the memory of other processes on Windows and Linux operating systems.
It provides functionality for reading, writing, and searching memory regions, making it useful for applications such as game hacking, live process analysis, debugging, and reverse engineering.

## Features

- [`scanner`](./src/scanner.rs): Scan a process's memory for specific byte patterns.
- [`memmap`](./src/memmap.rs): Map and unmap memory regions in a target process.
- [`process`](./src/process.rs): Interact with processes, including opening and closing process handles.
