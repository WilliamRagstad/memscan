"""
memscan - Python bindings for high-performance memory scanning

This package provides Python bindings to the memscan Rust library,
enabling scriptable process memory analysis with excellent performance.

Example usage:
    >>> import memscan
    >>> # Find process by name
    >>> pid = memscan.find_process_by_name("notepad")
    >>> if pid:
    ...     # Open the process
    ...     proc = memscan.open_process(pid)
    ...     # Get system info
    ...     sys_info = memscan.query_system_info()
    ...     print(sys_info)
    ...     # Get module regions
    ...     modules = memscan.get_process_module_regions(proc)
    ...     print(f"Found {len(modules)} module regions")
"""

# Import the native Rust module
from .memscan import *

__version__ = "0.1.0"
__all__ = [
    # Functions
    "open_process",
    "find_process_by_name",
    "query_system_info",
    "get_process_module_regions",
    "parse_hex_pattern",
    "read_process_memory",
    "write_process_memory",
    "create_interactive_scanner",
    # Classes
    "PyProcessHandle",
    "PyMemoryRegion",
    "PySystemInfo",
    "PyInteractiveScanner",
    "PyMatchedAddress",
]
