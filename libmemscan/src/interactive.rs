//! Interactive memory scanning with filtering and modification
//!
//! This module provides a REPL-like interface for iterative memory scanning.
//! Users can progressively filter memory addresses by value changes and types
//! until only a few candidates remain.

use crate::diff::MemoryDiff;
use crate::process::{MemoryRegion, ProcessHandle, write_process_memory};
use crate::values::{Value, ValueType, MathOp, values_equal, value_less_than, value_greater_than, apply_math_op, value_subtract};
use anyhow::Result;
use std::collections::HashMap;

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

/// Checkpoint snapshot of memory values at a specific point in time
#[derive(Debug, Clone)]
pub struct Checkpoint {
    /// Name of the checkpoint
    pub name: String,
    /// Snapshot of values at each address
    pub values: HashMap<usize, Value>,
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
    /// Named checkpoints for relative filtering
    checkpoints: HashMap<String, Checkpoint>,
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
            checkpoints: HashMap::new(),
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
    
    /// Save a checkpoint with the current memory state
    pub fn save_checkpoint(&mut self, name: String) -> Result<()> {
        let mut values = HashMap::new();
        
        // Read current values for all matched addresses
        for match_entry in &self.matches {
            let mapped = self.diff.mapper.get_by_address(match_entry.address);
            
            if let Some(mapped) = mapped {
                let offset = match_entry.address - mapped.remote_region.base_address;
                let data = mapped.data();
                
                if let Some(value) = Value::from_bytes(data, offset, self.value_type) {
                    values.insert(match_entry.address, value);
                }
            }
        }
        
        self.checkpoints.insert(name.clone(), Checkpoint { name, values });
        Ok(())
    }
    
    /// List all saved checkpoints
    pub fn list_checkpoints(&self) -> Vec<&str> {
        self.checkpoints.keys().map(|s| s.as_str()).collect()
    }
    
    /// Get a checkpoint by name
    pub fn get_checkpoint(&self, name: &str) -> Option<&Checkpoint> {
        self.checkpoints.get(name)
    }
    
    /// Delete a checkpoint by name
    pub fn delete_checkpoint(&mut self, name: &str) -> bool {
        self.checkpoints.remove(name).is_some()
    }
    
    /// Filter addresses by relative checkpoint changes with margin
    /// Keeps addresses where: abs((cp2 - cp1) - (cp3 - cp2)) <= margin
    pub fn filter_checkpoint_relative(
        &mut self,
        cp1_name: &str,
        cp2_name: &str,
        cp3_name: &str,
        margin_percent: f64,
    ) -> Result<usize> {
        let cp1 = self.get_checkpoint(cp1_name)
            .ok_or_else(|| anyhow::anyhow!("Checkpoint '{}' not found", cp1_name))?;
        let cp2 = self.get_checkpoint(cp2_name)
            .ok_or_else(|| anyhow::anyhow!("Checkpoint '{}' not found", cp2_name))?;
        let cp3 = self.get_checkpoint(cp3_name)
            .ok_or_else(|| anyhow::anyhow!("Checkpoint '{}' not found", cp3_name))?;
        
        let mut new_matches = Vec::new();
        
        for match_entry in &self.matches {
            let addr = match_entry.address;
            
            // Get values from all three checkpoints
            let v1 = match cp1.values.get(&addr) {
                Some(v) => v,
                None => continue,
            };
            let v2 = match cp2.values.get(&addr) {
                Some(v) => v,
                None => continue,
            };
            let v3 = match cp3.values.get(&addr) {
                Some(v) => v,
                None => continue,
            };
            
            // Calculate deltas: (cp2 - cp1) and (cp3 - cp2)
            let delta1 = match value_subtract(v2, v1) {
                Some(d) => d,
                None => continue,
            };
            let delta2 = match value_subtract(v3, v2) {
                Some(d) => d,
                None => continue,
            };
            
            // Check if deltas are approximately equal within margin
            if values_within_margin(&delta1, &delta2, margin_percent) {
                // Read current value
                let mapped = self.diff.mapper.get_by_address(addr);
                if let Some(mapped) = mapped {
                    let offset = addr - mapped.remote_region.base_address;
                    let data = mapped.data();
                    
                    if let Some(current) = Value::from_bytes(data, offset, self.value_type) {
                        new_matches.push(MatchedAddress {
                            address: addr,
                            current_value: current,
                            previous_value: Some(match_entry.current_value.clone()),
                        });
                    }
                }
            }
        }
        
        self.matches = new_matches;
        self.cleanup_empty_regions();
        
        Ok(self.matches.len())
    }
}

/// Check if two values are within a percentage margin of each other
fn values_within_margin(a: &Value, b: &Value, margin_percent: f64) -> bool {
    use crate::values::value_to_f64;
    
    let a_f64 = value_to_f64(a);
    let b_f64 = value_to_f64(b);
    
    // Handle case where both values are very small (near zero)
    if a_f64.abs() < 1e-10 && b_f64.abs() < 1e-10 {
        return true;
    }
    
    // Calculate the relative difference as a percentage
    let diff = (a_f64 - b_f64).abs();
    let max_val = a_f64.abs().max(b_f64.abs());
    
    if max_val < 1e-10 {
        return diff < 1e-10;
    }
    
    let percent_diff = (diff / max_val) * 100.0;
    percent_diff <= margin_percent
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
    
    #[test]
    fn test_values_within_margin() {
        // Test exact match
        assert!(values_within_margin(&Value::I32(100), &Value::I32(100), 0.0));
        
        // Test within 10% margin
        assert!(values_within_margin(&Value::I32(100), &Value::I32(105), 10.0));
        assert!(values_within_margin(&Value::I32(100), &Value::I32(95), 10.0));
        
        // Test outside margin
        assert!(!values_within_margin(&Value::I32(100), &Value::I32(120), 10.0));
        
        // Test floats
        assert!(values_within_margin(&Value::F64(100.0), &Value::F64(105.0), 10.0));
        assert!(!values_within_margin(&Value::F64(100.0), &Value::F64(120.0), 10.0));
        
        // Test near-zero values
        assert!(values_within_margin(&Value::F64(0.0), &Value::F64(0.0), 0.0));
    }
}
