//! Required functions not included in the winapi crate
//! See: https://github.com/retep998/winapi-rs/blob/5b1829956ef645f3c2f8236ba18bb198ca4c2468/src/um/memoryapi.rs#L344-L383

#![cfg(windows)]
#![allow(non_snake_case, dead_code)]
use winapi::{
    shared::{
        basetsd::{PSIZE_T, SIZE_T, ULONG64},
        minwindef::{BOOL, ULONG},
    },
    um::{
        memoryapi::WIN32_MEMORY_INFORMATION_CLASS,
        minwinbase::NUMA_NO_PREFERRED_NODE,
        winnt::{HANDLE, PVOID, VOID},
    },
};

unsafe extern "system" {
    pub unsafe fn QueryVirtualMemoryInformation(
        Process: HANDLE,
        VirtualAddress: *const VOID,
        MemoryInformationClass: WIN32_MEMORY_INFORMATION_CLASS,
        MemoryInformation: PVOID,
        MemoryInformationSize: SIZE_T,
        ReturnSize: PSIZE_T,
    ) -> BOOL;
    pub unsafe fn MapViewOfFileNuma2(
        FileMappingHandle: HANDLE,
        ProcessHandle: HANDLE,
        Offset: ULONG64,
        BaseAddress: PVOID,
        ViewSize: SIZE_T,
        AllocationType: ULONG,
        PageProtection: ULONG,
        PreferredNode: ULONG,
    ) -> PVOID;
}

#[inline]
pub unsafe fn MapViewOfFile2(
    FileMappingHandle: HANDLE,
    ProcessHandle: HANDLE,
    Offset: ULONG64,
    BaseAddress: PVOID,
    ViewSize: SIZE_T,
    AllocationType: ULONG,
    PageProtection: ULONG,
) -> PVOID {
    unsafe {
        MapViewOfFileNuma2(
            FileMappingHandle,
            ProcessHandle,
            Offset,
            BaseAddress,
            ViewSize,
            AllocationType,
            PageProtection,
            NUMA_NO_PREFERRED_NODE,
        )
    }
}
