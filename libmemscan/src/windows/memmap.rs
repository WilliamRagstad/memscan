//! Windows-specific memory mapping implementation using file mapping objects

use crate::process::{MemoryRegion, ProcessHandle};
use crate::windows::memoryapi::MapViewOfFile2;
use anyhow::Result;
use std::ptr::{null, null_mut};
use winapi::{
    shared::minwindef::LPVOID,
    um::{
        handleapi::CloseHandle,
        memoryapi::{CreateFileMappingW, UnmapViewOfFile},
        winnt::{HANDLE, PAGE_READONLY, SEC_COMMIT},
    },
};

/// Windows-specific mapped memory implementation
pub struct MappedMemoryWin {
    /// Handle to the file mapping object
    mapping_handle: HANDLE,
    /// Pointer to mapped view in local process
    local_ptr: LPVOID,
    /// Size of mapped region
    size: usize,
}

impl MappedMemoryWin {
    /// Create a new memory mapping for a region of a remote process
    ///
    /// This uses Windows file mapping APIs to create a section object
    /// backed by the remote process's memory.
    pub fn new(proc: &ProcessHandle, region: &MemoryRegion) -> Result<Self> {
        unsafe {
            // Create a file mapping object backed by the remote process memory
            // Using INVALID_HANDLE_VALUE with SEC_COMMIT creates a page-file backed mapping
            let mapping_handle = CreateFileMappingW(
                winapi::um::handleapi::INVALID_HANDLE_VALUE,
                null_mut(),
                PAGE_READONLY | SEC_COMMIT,
                (region.size >> 32) as u32,
                (region.size & 0xFFFFFFFF) as u32,
                null(),
            );

            if mapping_handle.is_null() {
                anyhow::bail!(
                    "CreateFileMappingW failed: {}",
                    std::io::Error::last_os_error()
                );
            }

            // Map the view into the local process
            // MapViewOfFile2 allows us to specify the remote process handle
            let local_ptr = MapViewOfFile2(
                mapping_handle,
                proc.raw(),
                region.base_address as u64,
                null_mut(),
                region.size,
                0,
                PAGE_READONLY,
            );

            if local_ptr.is_null() {
                CloseHandle(mapping_handle);
                anyhow::bail!(
                    "MapViewOfFile2 failed for address {:016x}: {}",
                    region.base_address,
                    std::io::Error::last_os_error()
                );
            }

            Ok(Self {
                mapping_handle,
                local_ptr,
                size: region.size,
            })
        }
    }

    /// Get a slice view of mapped memory
    pub fn as_slice(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.local_ptr as *const u8, self.size) }
    }
}

impl Drop for MappedMemoryWin {
    fn drop(&mut self) {
        unsafe {
            if !self.local_ptr.is_null() {
                UnmapViewOfFile(self.local_ptr);
            }
            if !self.mapping_handle.is_null() {
                CloseHandle(self.mapping_handle);
            }
        }
    }
}

unsafe impl Send for MappedMemoryWin {}
unsafe impl Sync for MappedMemoryWin {}
