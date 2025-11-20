//! Windows-specific memory mapping implementation
//!
//! This implementation uses ReadProcessMemory for reliable cross-platform access.
//! True memory mapping is only possible for MEM_MAPPED regions by duplicating file handles,
//! which requires complex handle enumeration. For future enhancement, thread injection
//! could be used to create section mappings in the remote process.

use crate::process::{MemoryRegion, ProcessHandle};
use crate::windows::process::read_process_memory;
use anyhow::Result;

/// Windows-specific mapped memory implementation
///
/// Uses ReadProcessMemory to read remote process memory into a local buffer.
/// This works reliably for all memory types (PRIVATE, MAPPED, IMAGE).
///
/// Future enhancement: For MEM_MAPPED regions, could enumerate handles via
/// NtQuerySystemInformation to duplicate file handles for true memory mapping.
#[derive(Debug)]
pub struct MappedMemoryWin {
    /// Local buffer containing a copy of the remote memory
    buffer: Vec<u8>,
    /// Remote address that was read
    remote_addr: usize,
}

impl MappedMemoryWin {
    /// Create a new memory mapping for a region of a remote process
    ///
    /// Reads the remote memory into a local buffer using ReadProcessMemory.
    /// This is the most reliable approach that works for all memory types.
    pub fn map_region(proc: &ProcessHandle, region: &MemoryRegion) -> Result<Self> {
        // Allocate a buffer for the remote memory
        let mut buffer = vec![0u8; region.size];

        // Read the memory using ReadProcessMemory
        let bytes_read = read_process_memory(proc, region.base_address, &mut buffer);

        if bytes_read == 0 {
            anyhow::bail!(
                "Failed to read memory at {:016x}: {}",
                region.base_address,
                std::io::Error::last_os_error()
            );
        }

        if bytes_read < region.size {
            anyhow::bail!(
                "Partial read: expected {} bytes, got {} bytes at address {:016x}",
                region.size,
                bytes_read,
                region.base_address
            );
        }

        Ok(Self {
            buffer,
            remote_addr: region.base_address,
        })
    }

    /// Get a slice view of mapped memory
    pub fn as_slice(&self) -> &[u8] {
        &self.buffer
    }

    /// Refresh mapped memory by re-reading from the remote process
    #[allow(dead_code)]
    pub fn refresh(&mut self, proc: &ProcessHandle) -> Result<()> {
        let bytes_read = read_process_memory(proc, self.remote_addr, &mut self.buffer);

        if bytes_read == 0 {
            anyhow::bail!(
                "Failed to refresh memory at {:016x}: {}",
                self.remote_addr,
                std::io::Error::last_os_error()
            );
        }

        if bytes_read < self.buffer.len() {
            anyhow::bail!(
                "Partial refresh: expected {} bytes, got {} bytes",
                self.buffer.len(),
                bytes_read
            );
        }

        Ok(())
    }
}

unsafe impl Send for MappedMemoryWin {}
unsafe impl Sync for MappedMemoryWin {}
