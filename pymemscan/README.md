# pymemscan

Python bindings for the `libmemscan` crate, providing high-performance memory scanning capabilities to Python.

## Overview

This crate wraps the core `libmemscan` library using PyO3 to create Python bindings. It is designed to be built as a Python extension module using `maturin`.

## Building

This crate is not meant to be used directly as a Rust dependency. Instead, it should be built as a Python package:

```bash
# From the repository root
maturin develop
```

For release builds:

```bash
maturin build --release
```

## Architecture

- **Separate crate**: Keeps Python bindings isolated from core library
- **PyO3**: Uses PyO3 for Rust-Python interop
- **Explicit API**: All operations require explicit function calls
- **Type safety**: Proper error handling and type conversions

## See Also

- [Python API Documentation](../python/README.md)
- [Core Library (libmemscan)](../libmemscan/README.md)
- [Example Usage](../examples/python_example.py)
