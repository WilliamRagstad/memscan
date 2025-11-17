//! Change detection and parallel diffing for mapped memory regions
//!
//! This module provides functionality to detect memory changes by comparing
//! snapshots of mapped memory regions in parallel.

use crate::memmap::MappedMemory;
use crate::process::{MemoryRegion, ProcessHandle};
use anyhow::Result;

/// A snapshot of memory at a specific point in time
pub struct MemorySnapshot {
    /// Base address in the remote process
    pub address: usize,
    /// Snapshot data
    pub data: Vec<u8>,
}

impl MemorySnapshot {
    /// Create a new snapshot from mapped memory
    pub fn from_mapped(mapped: &MappedMemory) -> Self {
        Self {
            address: mapped.remote_address(),
            data: mapped.as_slice().to_vec(),
        }
    }

    /// Create a snapshot by reading directly from process memory
    pub fn from_process(proc: &ProcessHandle, region: &MemoryRegion) -> Result<Self> {
        let mapped = MappedMemory::new(proc, region)?;
        Ok(Self::from_mapped(&mapped))
    }
}

/// Represents a detected change in memory
#[derive(Debug, Clone)]
pub struct MemoryChange {
    /// Address where the change was detected
    pub address: usize,
    /// Old value at this address
    pub old_value: u8,
    /// New value at this address
    pub new_value: u8,
}

/// Compare two snapshots and return the list of changes
pub fn diff_snapshots(old: &MemorySnapshot, new: &MemorySnapshot) -> Vec<MemoryChange> {
    if old.address != new.address || old.data.len() != new.data.len() {
        return Vec::new();
    }

    let mut changes = Vec::new();
    for (offset, (&old_byte, &new_byte)) in old.data.iter().zip(new.data.iter()).enumerate() {
        if old_byte != new_byte {
            changes.push(MemoryChange {
                address: old.address + offset,
                old_value: old_byte,
                new_value: new_byte,
            });
        }
    }
    changes
}

/// Parallel change detector for multiple memory regions
pub struct ChangeDetector {
    snapshots: Vec<MemorySnapshot>,
}

impl ChangeDetector {
    /// Create a new change detector
    pub fn new() -> Self {
        Self {
            snapshots: Vec::new(),
        }
    }

    /// Take initial snapshots of the given regions
    pub fn initialize(&mut self, proc: &ProcessHandle, regions: &[MemoryRegion]) -> Result<()> {
        self.snapshots.clear();
        for region in regions {
            let snapshot = MemorySnapshot::from_process(proc, region)?;
            self.snapshots.push(snapshot);
        }
        Ok(())
    }

    /// Detect changes by comparing current memory state with snapshots
    ///
    /// This performs parallel comparison of all tracked regions
    pub fn detect_changes(&self, proc: &ProcessHandle, regions: &[MemoryRegion]) -> Result<Vec<Vec<MemoryChange>>> {
        if regions.len() != self.snapshots.len() {
            anyhow::bail!("Region count mismatch: expected {}, got {}", self.snapshots.len(), regions.len());
        }

        // For now, implement sequential comparison
        // TODO: Add parallel implementation using rayon when benchmarks show benefit
        let mut all_changes = Vec::new();
        for (old_snapshot, region) in self.snapshots.iter().zip(regions.iter()) {
            let new_snapshot = MemorySnapshot::from_process(proc, region)?;
            let changes = diff_snapshots(old_snapshot, &new_snapshot);
            all_changes.push(changes);
        }

        Ok(all_changes)
    }

    /// Update snapshots to the current memory state
    pub fn update_snapshots(&mut self, proc: &ProcessHandle, regions: &[MemoryRegion]) -> Result<()> {
        if regions.len() != self.snapshots.len() {
            anyhow::bail!("Region count mismatch");
        }

        for (snapshot, region) in self.snapshots.iter_mut().zip(regions.iter()) {
            let new_snapshot = MemorySnapshot::from_process(proc, region)?;
            *snapshot = new_snapshot;
        }

        Ok(())
    }

    /// Get the number of tracked snapshots
    pub fn snapshot_count(&self) -> usize {
        self.snapshots.len()
    }
}

impl Default for ChangeDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diff_snapshots_no_changes() {
        let old = MemorySnapshot {
            address: 0x1000,
            data: vec![1, 2, 3, 4, 5],
        };
        let new = MemorySnapshot {
            address: 0x1000,
            data: vec![1, 2, 3, 4, 5],
        };
        let changes = diff_snapshots(&old, &new);
        assert_eq!(changes.len(), 0);
    }

    #[test]
    fn test_diff_snapshots_with_changes() {
        let old = MemorySnapshot {
            address: 0x1000,
            data: vec![1, 2, 3, 4, 5],
        };
        let new = MemorySnapshot {
            address: 0x1000,
            data: vec![1, 9, 3, 8, 5],
        };
        let changes = diff_snapshots(&old, &new);
        assert_eq!(changes.len(), 2);
        assert_eq!(changes[0].address, 0x1001);
        assert_eq!(changes[0].old_value, 2);
        assert_eq!(changes[0].new_value, 9);
        assert_eq!(changes[1].address, 0x1003);
        assert_eq!(changes[1].old_value, 4);
        assert_eq!(changes[1].new_value, 8);
    }

    #[test]
    fn test_change_detector_new() {
        let detector = ChangeDetector::new();
        assert_eq!(detector.snapshot_count(), 0);
    }

    #[test]
    fn test_change_detector_default() {
        let detector = ChangeDetector::default();
        assert_eq!(detector.snapshot_count(), 0);
    }
}
