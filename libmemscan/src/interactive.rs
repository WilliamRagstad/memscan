//! Interactive memory scanning with filtering and modification
//!
//! This module provides a REPL-like interface for iterative memory scanning.
//! Users can progressively filter memory addresses by value changes and types
//! until only a few candidates remain.

use crate::diff::MemoryDiff;
use crate::process::{MemoryRegion, ProcessHandle, write_process_memory};
use crate::values::{Value, ValueType, MathOp, values_equal, value_less_than, value_greater_than, apply_math_op};
use anyhow::Result;

/// Filter operation for comparing values
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterOp {
    /// Value equals a specific value
    Equals,
    /// Value is less than a specific value
    LessThan,
    /// Value is greater than a specific value
    GreaterThan,
    /// Value increased compared to previous scan
    Increased,
    /// Value decreased compared to previous scan
    Decreased,
    /// Value changed compared to previous scan
    Changed,
    /// Value unchanged compared to previous scan
    Unchanged,
}

/// A memory address that matches the current filter criteria
#[derive(Debug, Clone)]
pub struct MatchedAddress {
    /// Address in the target process
    pub address: usize,
    /// Current value at this address
    pub current_value: Value,
    /// Previous value (if available)
    pub previous_value: Option<Value>,
}

/// Interactive memory scanner that maintains state between scans
pub struct InteractiveScanner<'a> {
    /// Target process handle
    process: &'a ProcessHandle,
    /// Memory diff tracker for managing snapshots and regions
    diff: MemoryDiff<'a>,
    /// Matched addresses from current filter
    matches: Vec<MatchedAddress>,
    /// Value type being searched for
    value_type: ValueType,
    /// Alignment requirement (1, 2, 4, or 8 bytes)
    alignment: usize,
}

impl<'a> InteractiveScanner<'a> {
    /// Create a new interactive scanner
    pub fn new(process: &'a ProcessHandle, regions: Vec<MemoryRegion>, value_type: ValueType) -> Self {
        let mut diff = MemoryDiff::new(process);
        
        // Map all regions using MemoryDiff's mapper
        for region in regions {
            let _ = diff.mapper.map_region(region);
        }
        
        Self {
            process,
            diff,
            matches: Vec::new(),
            value_type,
            alignment: value_type.size(), // Default to natural alignment
        }
    }
    
    /// Set the alignment requirement
    pub fn set_alignment(&mut self, alignment: usize) {
        self.alignment = alignment;
    }
    
    /// Perform initial scan to find all possible addresses
    pub fn initial_scan(&mut self) -> Result<usize> {
        self.matches.clear();
        
        // Use mapped memory from the diff tracker
        for mapped in self.diff.mapper.iter() {
            let base_address = mapped.remote_region.base_address;
            let data = mapped.data();
            
            // Scan through the region with proper alignment
            let mut offset = 0;
            while offset + self.value_type.size() <= data.len() {
                if offset % self.alignment == 0 {
                    if let Some(value) = Value::from_bytes(data, offset, self.value_type) {
                        self.matches.push(MatchedAddress {
                            address: base_address + offset,
                            current_value: value,
                            previous_value: None,
                        });
                    }
                }
                offset += self.alignment;
            }
        }
        
        Ok(self.matches.len())
    }
    
    /// Apply a filter to the current matches
    pub fn filter(&mut self, op: FilterOp, compare_value: Option<Value>) -> Result<usize> {
        let mut new_matches = Vec::new();
        
        for match_entry in &self.matches {
            // Find the mapped region containing this address
            let mapped = self.diff.mapper.get_by_address(match_entry.address);
            
            if mapped.is_none() {
                continue; // Region no longer mapped
            }
            
            let mapped = mapped.unwrap();
            let offset = match_entry.address - mapped.remote_region.base_address;
            let data = mapped.data();
            
            // Read current value from mapped memory
            let current = match Value::from_bytes(data, offset, self.value_type) {
                Some(v) => v,
                None => continue,
            };
            
            let keep = match op {
                FilterOp::Equals => {
                    if let Some(ref val) = compare_value {
                        values_equal(&current, val)
                    } else {
                        false
                    }
                }
                FilterOp::LessThan => {
                    if let Some(ref val) = compare_value {
                        value_less_than(&current, val)
                    } else {
                        false
                    }
                }
                FilterOp::GreaterThan => {
                    if let Some(ref val) = compare_value {
                        value_greater_than(&current, val)
                    } else {
                        false
                    }
                }
                FilterOp::Increased => value_greater_than(&current, &match_entry.current_value),
                FilterOp::Decreased => value_less_than(&current, &match_entry.current_value),
                FilterOp::Changed => !values_equal(&current, &match_entry.current_value),
                FilterOp::Unchanged => values_equal(&current, &match_entry.current_value),
            };
            
            if keep {
                new_matches.push(MatchedAddress {
                    address: match_entry.address,
                    current_value: current,
                    previous_value: Some(match_entry.current_value.clone()),
                });
            }
        }
        
        self.matches = new_matches;
        
        // Clean up regions with no matches
        self.cleanup_empty_regions();
        
        Ok(self.matches.len())
    }
    
    /// Remove regions that have no matching addresses
    fn cleanup_empty_regions(&mut self) {
        if self.matches.is_empty() {
            self.diff.mapper.clear();
            return;
        }
        
        // Determine which regions still have matches using MemoryRegion::is_superset_of
        let mut active_addresses = std::collections::HashSet::new();
        for match_entry in &self.matches {
            active_addresses.insert(match_entry.address);
        }
        
        // Remove regions that don't contain any active addresses
        self.diff.mapper.retain(|mapped| {
            let region = &mapped.remote_region;
            active_addresses.iter().any(|&addr| {
                addr >= region.base_address && addr < region.base_address + region.size
            })
        });
    }
    
    /// Write a value to a specific address
    pub fn write_value(&self, address: usize, value: Value) -> Result<()> {
        let bytes = value.to_bytes();
        let bytes_written = write_process_memory(self.process, address, &bytes);
        
        if bytes_written < bytes.len() {
            anyhow::bail!(
                "Failed to write {} bytes to address {:016x}, only wrote {}",
                bytes.len(),
                address,
                bytes_written
            );
        }
        
        Ok(())
    }
    
    /// Write a value to all matched addresses
    pub fn write_all(&self, value: Value) -> Result<usize> {
        let mut written = 0;
        for match_entry in &self.matches {
            if self.write_value(match_entry.address, value.clone()).is_ok() {
                written += 1;
            }
        }
        Ok(written)
    }
    
    /// Apply a math operation to a specific address
    pub fn modify_value(&self, address: usize, op: MathOp, operand: Value) -> Result<()> {
        // Find the mapped region containing this address
        let mapped = self.diff.mapper.get_by_address(address)
            .ok_or_else(|| anyhow::anyhow!("Address {:016x} not in mapped regions", address))?;
        
        let offset = address - mapped.remote_region.base_address;
        let data = mapped.data();
        
        let current = Value::from_bytes(data, offset, self.value_type)
            .ok_or_else(|| anyhow::anyhow!("Failed to parse value at address {:016x}", address))?;
        
        let new_value = apply_math_op(&current, &operand, op)?;
        self.write_value(address, new_value)
    }
    
    /// Apply a math operation to all matched addresses
    pub fn modify_all(&self, op: MathOp, operand: Value) -> Result<usize> {
        let mut modified = 0;
        for match_entry in &self.matches {
            if self.modify_value(match_entry.address, op, operand.clone()).is_ok() {
                modified += 1;
            }
        }
        Ok(modified)
    }
    
    /// Get the current matches
    pub fn matches(&self) -> &[MatchedAddress] {
        &self.matches
    }
    
    /// Get the number of regions being monitored
    pub fn region_count(&self) -> usize {
        self.diff.mapper.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_operations() {
        // FilterOp enum values
        assert_eq!(FilterOp::Equals, FilterOp::Equals);
        assert_ne!(FilterOp::Equals, FilterOp::LessThan);
    }
}
