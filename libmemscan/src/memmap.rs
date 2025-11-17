//! Cross-platform memory mapping for detecting instant memory changes
//!
//! This module provides functionality to map remote process memory pages into
//! the local process address space, enabling faster access and parallel diffing
//! compared to traditional `ReadProcessMemory` calls.

use crate::process::{MemoryRegion, ProcessHandle};
use anyhow::Result;

#[cfg(unix)]
use crate::linux;
#[cfg(windows)]
use crate::windows;

/// Represents a mapped view of remote process memory
pub struct MappedMemory {
    /// Base address in the remote process
    pub remote_addr: usize,
    /// Size of the mapped region
    pub size: usize,
    /// Platform-specific handle
    #[cfg(windows)]
    inner: windows::memmap::MappedMemoryWin,
    #[cfg(unix)]
    inner: linux::memmap::MappedMemoryUnix,
}

impl MappedMemory {
    /// Map a region of remote process memory into the local address space
    ///
    /// # Arguments
    /// * `proc` - Process handle
    /// * `region` - Memory region to map
    ///
    /// # Returns
    /// A `MappedMemory` object that provides access to the mapped memory
    pub fn new(proc: &ProcessHandle, region: &MemoryRegion) -> Result<Self> {
        #[cfg(windows)]
        {
            let inner = windows::memmap::MappedMemoryWin::new(proc, region)?;
            Ok(Self {
                remote_addr: region.base_address,
                size: region.size,
                inner,
            })
        }
        #[cfg(unix)]
        {
            let inner = linux::memmap::MappedMemoryUnix::new(proc, region)?;
            Ok(Self {
                remote_addr: region.base_address,
                size: region.size,
                inner,
            })
        }
    }

    /// Get a slice to the mapped memory
    ///
    /// # Safety
    /// The returned slice is only valid as long as the MappedMemory exists.
    /// The remote process may modify this memory at any time.
    pub fn as_slice(&self) -> &[u8] {
        #[cfg(windows)]
        return self.inner.as_slice();
        #[cfg(unix)]
        return self.inner.as_slice();
    }

    /// Get the base address in the remote process
    pub fn remote_address(&self) -> usize {
        self.remote_addr
    }

    /// Get the size of the mapped region
    pub fn size(&self) -> usize {
        self.size
    }
}

/// Manager for tracking multiple mapped memory regions
pub struct MemoryMapper {
    mappings: Vec<MappedMemory>,
}

impl MemoryMapper {
    /// Create a new empty memory mapper
    pub fn new() -> Self {
        Self {
            mappings: Vec::new(),
        }
    }

    /// Map a memory region
    pub fn map_region(&mut self, proc: &ProcessHandle, region: &MemoryRegion) -> Result<usize> {
        let mapped = MappedMemory::new(proc, region)?;
        self.mappings.push(mapped);
        Ok(self.mappings.len() - 1)
    }

    /// Get a mapped region by index
    pub fn get(&self, index: usize) -> Option<&MappedMemory> {
        self.mappings.get(index)
    }

    /// Get the number of mapped regions
    pub fn len(&self) -> usize {
        self.mappings.len()
    }

    /// Check if there are no mapped regions
    pub fn is_empty(&self) -> bool {
        self.mappings.is_empty()
    }

    /// Clear all mappings
    pub fn clear(&mut self) {
        self.mappings.clear();
    }
}

impl Default for MemoryMapper {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_mapper_new() {
        let mapper = MemoryMapper::new();
        assert_eq!(mapper.len(), 0);
        assert!(mapper.is_empty());
    }

    #[test]
    fn test_memory_mapper_default() {
        let mapper = MemoryMapper::default();
        assert_eq!(mapper.len(), 0);
    }
}
