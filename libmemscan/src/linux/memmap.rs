//! Linux-specific memory mapping implementation using /proc/pid/mem

#![cfg(unix)]

use crate::process::{MemoryRegion, ProcessHandle};
use anyhow::Result;

/// Linux-specific mapped memory implementation
///
/// On Linux, we use process_vm_readv or direct memory mapping through /proc/pid/mem.
/// Since true shared memory mapping of another process's address space is complex
/// and requires specific kernel features, we implement a cached read approach that
/// reads memory into a local buffer.
pub struct MappedMemoryUnix {
    /// Local buffer containing a copy of the remote memory
    buffer: Vec<u8>,
    /// Remote address that was read
    #[allow(dead_code)]
    remote_addr: usize,
}

impl MappedMemoryUnix {
    /// Create a new memory mapping for a region of a remote process
    ///
    /// On Linux, this reads the remote memory into a local buffer.
    /// For true instant change detection, Linux would require using
    /// process_vm_readv repeatedly or custom kernel modules.
    pub fn new(proc: &ProcessHandle, region: &MemoryRegion) -> Result<Self> {
        // Allocate a buffer for the remote memory
        let mut buffer = vec![0u8; region.size];

        // Read the memory from /proc/pid/mem
        let bytes_read = proc
            .read_mem(region.base_address, &mut buffer)
            .map_err(|e| anyhow::anyhow!("Failed to read memory at {:016x}: {}", region.base_address, e))?;

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

    /// Get a slice view of the mapped memory
    pub fn as_slice(&self) -> &[u8] {
        &self.buffer
    }

    /// Refresh the mapped memory by re-reading from the remote process
    #[allow(dead_code)]
    pub fn refresh(&mut self, proc: &ProcessHandle) -> Result<()> {
        let bytes_read = proc
            .read_mem(self.remote_addr, &mut self.buffer)
            .map_err(|e| anyhow::anyhow!("Failed to refresh memory at {:016x}: {}", self.remote_addr, e))?;

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

unsafe impl Send for MappedMemoryUnix {}
unsafe impl Sync for MappedMemoryUnix {}
