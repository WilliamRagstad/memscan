# MemScan

A simple memory scanning tool for Windows process debugging, dynamic analysis, and reverse engineering purposes.
All functionalities are implemented using User-Mode Windows APIs, no kernel-mode components are required.

## Features

- [ ] Scan a process's memory for specific byte patterns.
- [ ] Cross-platform support (currently only Windows is implemented).
- [ ] Support for scanning large memory regions efficiently.
- [ ] Configurable scanning options (e.g., case sensitivity, wildcards).
- [ ] Scriptable interface for automating scans with Python.

## References

The key Windows API is `VirtualQueryEx`, which retrieves information about a range of pages in the virtual address space of **a specified process**. Efficiently scanning a process's memory regions is done via a shared object mapping through  `CreateFileMapping` and `MapViewOfFile` into pre-allocated local pages.

- Functions
  - `VirtualQueryEx`: [msdn](https://learn.microsoft.com/en-us/windows/win32/api/memoryapi/nf-memoryapi-virtualqueryex), [winapi](https://docs.rs/winapi/latest/winapi/um/memoryapi/fn.VirtualQueryEx.html)
  - `VirtualAlloc`: [msdn](https://learn.microsoft.com/en-us/windows/win32/api/memoryapi/nf-memoryapi-virtualalloc), [winapi](https://docs.rs/winapi/latest/winapi/um/memoryapi/fn.VirtualAlloc.html)
  - `ReadProcessMemory`: [msdn](https://learn.microsoft.com/en-us/windows/win32/api/memoryapi/nf-memoryapi-readprocessmemory), [winapi](https://docs.rs/winapi/latest/winapi/um/memoryapi/fn.ReadProcessMemory.html)
  - `OpenProcess`: [msdn](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-openprocess), [winapi](https://docs.rs/winapi/latest/winapi/um/processthreadsapi/fn.OpenProcess.html)
  - `CreateFileMapping`: [msdn](https://learn.microsoft.com/en-us/windows/win32/api/memoryapi/nf-memoryapi-createfilemappingw), [winapi](https://docs.rs/winapi/latest/winapi/um/memoryapi/fn.CreateFileMappingW.html)
  - `MapViewOfFile`: [msdn](https://learn.microsoft.com/en-us/windows/win32/api/memoryapi/nf-memoryapi-mapviewoffile), [winapi](https://docs.rs/winapi/latest/winapi/um/memoryapi/fn.MapViewOfFile.html)
  - `MapViewOfFileEx`: [msdn](https://learn.microsoft.com/en-us/windows/win32/api/memoryapi/nf-memoryapi-mapviewoffileex), [winapi](https://docs.rs/winapi/latest/winapi/um/memoryapi/fn.MapViewOfFileEx.html)
  - `MapViewOfFile2`: [msdn](https://learn.microsoft.com/en-us/windows/win32/api/memoryapi/nf-memoryapi-mapviewoffile2)
- Structures
  - `SYSTEM_INFO`: [msdn](https://learn.microsoft.com/en-us/windows/win32/api/sysinfoapi/ns-sysinfoapi-system_info), [winapi](https://docs.rs/winapi/latest/winapi/um/sysinfoapi/struct.SYSTEM_INFO.html)
  - `MEMORY_BASIC_INFORMATION`: [msdn](https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-memory_basic_information), [winapi](https://docs.rs/winapi/latest/winapi/um/winnt/struct.MEMORY_BASIC_INFORMATION.html)
- Microsoft Docs
  - [Working with Pages](https://learn.microsoft.com/en-us/windows/win32/memory/working-with-pages)
  - [Memory Management Functions](https://learn.microsoft.com/en-us/windows/win32/memory/memory-management-functions)
  - [Memory Protection Constants](https://learn.microsoft.com/en-us/windows/win32/memory/memory-protection-constants)
  - [Process Access Rights](https://learn.microsoft.com/en-us/windows/win32/procthread/process-security-and-access-rights)
- Blogs
  - [Alex Ionescu's Blog: Windows Internals, Thoughts on Security, and Reverse Engineering](https://www.alex-ionescu.com)
  - [Enumerate process modules via VirtualQueryEx. Simple C++ example.](https://cocomelonc.github.io/malware/2023/11/07/malware-trick-37.html)
  - [List Process Modules with VirtualQueryEx](https://medium.com/@s12deff/list-process-modules-with-virtualqueryex-6c7c3e2613a6)
  - [Difference between QueryVirtualMemoryInformation and VirtualQueryEx](https://stackoverflow.com/questions/74704604/difference-between-queryvirtualmemoryinformation-and-virtualqueryex)
