//! Cross-platform memory mapping for detecting instant memory changes
//!
//! This module provides functionality to map remote process memory pages into
//! the local process address space, enabling faster access and parallel diffing
//! compared to traditional `ReadProcessMemory` calls.

use std::collections::HashMap;

use crate::process::{MemoryRegion, ProcessHandle};
use anyhow::Result;

#[cfg(unix)]
use crate::linux;
#[cfg(windows)]
use crate::windows;

/// Represents a mapped memory view of remote process memory
#[derive(Debug)]
pub struct MappedMemory {
    /// Base address in the remote process
    pub remote_region: MemoryRegion,
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
    pub fn map_region(proc: &ProcessHandle, region: MemoryRegion) -> Result<Self> {
        #[cfg(windows)]
        let inner = windows::memmap::MappedMemoryWin::map_region(proc, &region)?;
        #[cfg(unix)]
        let inner = linux::memmap::MappedMemoryUnix::map_region(proc, &region)?;
        Ok(Self {
            remote_region: region,
            inner,
        })
    }

    /// Get a slice to the mapped memory
    ///
    /// # Safety
    /// The returned slice is only valid as long as the MappedMemory exists.
    /// The remote process may modify this memory at any time.
    pub fn data(&self) -> &[u8] {
        return self.inner.as_slice();
    }
}

/// Manager for tracking multiple mapped memory regions
pub struct MemoryMapper<'a> {
    process: &'a ProcessHandle,
    mappings: HashMap<usize, MappedMemory>,
}

impl<'a> MemoryMapper<'a> {
    /// Create a new empty memory mapper
    pub fn new(process: &'a ProcessHandle) -> Self {
        Self {
            process,
            mappings: HashMap::new(),
        }
    }

    /// Map a memory region.
    ///
    /// ## Returns
    /// The remote base address of the mapped region.
    pub fn map_region(&mut self, region: MemoryRegion) -> Result<&MappedMemory> {
        let mapped = MappedMemory::map_region(self.process, region)?;
        let remote_base_address = mapped.remote_region.base_address;
        self.mappings.insert(remote_base_address, mapped);
        Ok(self.get(remote_base_address).unwrap())
    }

    /// Get a mapped region by index
    pub fn get(&self, remote_base_address: usize) -> Option<&MappedMemory> {
        self.mappings.get(&remote_base_address)
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

impl IntoIterator for MemoryMapper<'_> {
    type Item = MappedMemory;
    type IntoIter = std::vec::IntoIter<MappedMemory>;

    fn into_iter(self) -> Self::IntoIter {
        self.mappings.into_values().collect::<Vec<_>>().into_iter()
    }
}

#[cfg(test)]
mod tests {
    use winapi::um::handleapi::INVALID_HANDLE_VALUE;

    use super::*;

    #[test]
    fn test_memory_mapper_new() {
        let mapper = MemoryMapper::new(&ProcessHandle(INVALID_HANDLE_VALUE));
        assert_eq!(mapper.len(), 0);
        assert!(mapper.is_empty());
    }

    #[test]
    fn test_memory_mapper_default() {
        let mapper = MemoryMapper::default();
        assert_eq!(mapper.len(), 0);
    }
}
