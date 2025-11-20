//! Python bindings for libmemscan
//!
//! This module provides Python bindings using PyO3 to expose the memscan
//! functionality to Python scripts. The API is explicit and requires specialized
//! function calls for fine-grained control.

use pyo3::prelude::*;
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use std::collections::HashMap;

use crate::process::{self, ProcessHandle, MemoryRegion, SystemInfo, MemoryType, MemoryState, MemoryProtection};
use crate::scanner::{self, ScanOptions};
use crate::interactive::{InteractiveScanner, FilterOp, MatchedAddress};
use crate::values::{ValueType, Value, MathOp};

/// Python wrapper for ProcessHandle
#[pyclass]
struct PyProcessHandle {
    handle: ProcessHandle,
}

/// Python wrapper for MemoryRegion
#[pyclass]
#[derive(Clone)]
struct PyMemoryRegion {
    #[pyo3(get)]
    base_address: usize,
    #[pyo3(get)]
    size: usize,
    #[pyo3(get)]
    region_type: String,
    #[pyo3(get)]
    state: String,
    #[pyo3(get)]
    protect: String,
}

#[pymethods]
impl PyMemoryRegion {
    fn __repr__(&self) -> String {
        format!(
            "MemoryRegion(base=0x{:016x}, size={}, type={}, state={}, protect={})",
            self.base_address, self.size, self.region_type, self.state, self.protect
        )
    }
}

/// Python wrapper for SystemInfo
#[pyclass]
struct PySystemInfo {
    #[pyo3(get)]
    min_app_addr: usize,
    #[pyo3(get)]
    max_app_addr: usize,
    #[pyo3(get)]
    page_size: usize,
    #[pyo3(get)]
    granularity: usize,
}

#[pymethods]
impl PySystemInfo {
    fn __repr__(&self) -> String {
        format!(
            "SystemInfo(min_addr=0x{:016x}, max_addr=0x{:016x}, page_size={}, granularity={})",
            self.min_app_addr, self.max_app_addr, self.page_size, self.granularity
        )
    }
}

/// Python wrapper for InteractiveScanner
/// Uses unsafe code to manage lifetime, but keeps the process handle alive
#[pyclass(unsendable)]
struct PyInteractiveScanner {
    scanner: Option<InteractiveScanner<'static>>,
    process_handle: *const ProcessHandle,
    value_type: ValueType,
    // Keep a reference to the PyProcessHandle to ensure it stays alive
    _phantom: std::marker::PhantomData<&'static ProcessHandle>,
}

/// Python wrapper for matched address
#[pyclass]
#[derive(Clone)]
struct PyMatchedAddress {
    #[pyo3(get)]
    address: usize,
    #[pyo3(get)]
    current_value: f64,
    #[pyo3(get)]
    previous_value: Option<f64>,
}

#[pymethods]
impl PyMatchedAddress {
    fn __repr__(&self) -> String {
        match self.previous_value {
            Some(prev) => format!(
                "MatchedAddress(addr=0x{:016x}, current={}, previous={})",
                self.address, self.current_value, prev
            ),
            None => format!(
                "MatchedAddress(addr=0x{:016x}, current={})",
                self.address, self.current_value
            ),
        }
    }
}

/// Convert Rust Value to f64 for Python
fn value_to_f64(value: &Value) -> f64 {
    match value {
        Value::I8(v) => *v as f64,
        Value::I16(v) => *v as f64,
        Value::I32(v) => *v as f64,
        Value::I64(v) => *v as f64,
        Value::U8(v) => *v as f64,
        Value::U16(v) => *v as f64,
        Value::U32(v) => *v as f64,
        Value::U64(v) => *v as f64,
        Value::F32(v) => *v as f64,
        Value::F64(v) => *v,
    }
}

/// Open a process by its PID
#[pyfunction]
fn open_process(pid: u32) -> PyResult<PyProcessHandle> {
    let handle = process::open_process(pid)
        .map_err(|e| PyRuntimeError::new_err(format!("Failed to open process: {}", e)))?;
    Ok(PyProcessHandle { handle })
}

/// Find a process by its name
#[pyfunction]
fn find_process_by_name(name: &str) -> PyResult<Option<u32>> {
    process::find_process_by_name(name)
        .map_err(|e| PyRuntimeError::new_err(format!("Failed to find process: {}", e)))
}

/// Get system information
#[pyfunction]
fn query_system_info() -> PySystemInfo {
    let info = process::query_system_info();
    PySystemInfo {
        min_app_addr: info.min_app_addr,
        max_app_addr: info.max_app_addr,
        page_size: info.page_size,
        granularity: info.granularity,
    }
}

/// Get process module regions
#[pyfunction]
fn get_process_module_regions(handle: &PyProcessHandle) -> PyResult<Vec<PyMemoryRegion>> {
    let regions = process::get_process_module_regions(&handle.handle)
        .map_err(|e| PyRuntimeError::new_err(format!("Failed to get module regions: {}", e)))?;
    
    Ok(regions.into_iter().map(|r| PyMemoryRegion {
        base_address: r.base_address,
        size: r.size,
        region_type: r.type_.to_string(),
        state: r.state.to_string(),
        protect: r.protect.to_string(),
    }).collect())
}

/// Parse a hex pattern string into bytes
#[pyfunction]
fn parse_hex_pattern(pattern: &str) -> PyResult<Vec<u8>> {
    crate::parse_hex_pattern(pattern)
        .map_err(|e| PyValueError::new_err(format!("Invalid hex pattern: {}", e)))
}

/// Read memory from a process at a specific address
#[pyfunction]
fn read_process_memory(handle: &PyProcessHandle, address: usize, size: usize) -> PyResult<Vec<u8>> {
    let mut buffer = vec![0u8; size];
    let bytes_read = process::read_process_memory(&handle.handle, address, &mut buffer);
    
    if bytes_read == 0 {
        return Err(PyRuntimeError::new_err("Failed to read process memory"));
    }
    
    buffer.truncate(bytes_read);
    Ok(buffer)
}

/// Write memory to a process at a specific address
#[pyfunction]
fn write_process_memory(handle: &PyProcessHandle, address: usize, data: Vec<u8>) -> PyResult<usize> {
    let bytes_written = process::write_process_memory(&handle.handle, address, &data);
    
    if bytes_written == 0 {
        return Err(PyRuntimeError::new_err("Failed to write process memory"));
    }
    
    Ok(bytes_written)
}

/// Create an interactive scanner for a process
#[pyfunction]
fn create_interactive_scanner(
    handle: &PyProcessHandle,
    regions: Vec<PyMemoryRegion>,
    value_type: &str,
) -> PyResult<PyInteractiveScanner> {
    let vtype = match value_type.to_lowercase().as_str() {
        "i8" => ValueType::I8,
        "i16" => ValueType::I16,
        "i32" => ValueType::I32,
        "i64" => ValueType::I64,
        "u8" => ValueType::U8,
        "u16" => ValueType::U16,
        "u32" => ValueType::U32,
        "u64" => ValueType::U64,
        "f32" => ValueType::F32,
        "f64" => ValueType::F64,
        _ => return Err(PyValueError::new_err(format!("Invalid value type: {}", value_type))),
    };
    
    let rust_regions: Vec<MemoryRegion> = regions.into_iter().map(|r| {
        MemoryRegion {
            base_address: r.base_address,
            size: r.size,
            type_: MemoryType::Unknown, // Simplified since we just need the address range
            state: MemoryState { committed: true, free: false, reserved: false },
            protect: MemoryProtection {
                no_access: false,
                read: true,
                write: false,
                execute: false,
                copy_on_write: false,
                guarded: false,
                no_cache: false,
            },
            image_file: None,
        }
    }).collect();
    
    // SAFETY: We're storing a raw pointer to the ProcessHandle
    // The Python wrapper must ensure the handle outlives the scanner
    let process_ptr = &handle.handle as *const ProcessHandle;
    
    // SAFETY: We extend the lifetime of the reference here
    // This is safe because we control both lifetimes through Python's ownership
    let scanner = unsafe {
        let process_ref = &*process_ptr;
        InteractiveScanner::new(process_ref, rust_regions, vtype)
    };
    
    Ok(PyInteractiveScanner {
        scanner: Some(scanner),
        process_handle: process_ptr,
        value_type: vtype,
        _phantom: std::marker::PhantomData,
    })
}

/// Helper to convert f64 to Value based on ValueType
fn f64_to_value(f: f64, vtype: ValueType) -> Value {
    match vtype {
        ValueType::I8 => Value::I8(f as i8),
        ValueType::I16 => Value::I16(f as i16),
        ValueType::I32 => Value::I32(f as i32),
        ValueType::I64 => Value::I64(f as i64),
        ValueType::U8 => Value::U8(f as u8),
        ValueType::U16 => Value::U16(f as u16),
        ValueType::U32 => Value::U32(f as u32),
        ValueType::U64 => Value::U64(f as u64),
        ValueType::F32 => Value::F32(f as f32),
        ValueType::F64 => Value::F64(f),
    }
}

#[pymethods]
impl PyInteractiveScanner {
    /// Perform initial scan to find all possible addresses
    fn initial_scan(&mut self) -> PyResult<usize> {
        let scanner = self.scanner.as_mut()
            .ok_or_else(|| PyRuntimeError::new_err("Scanner not initialized"))?;
        
        scanner.initial_scan()
            .map_err(|e| PyRuntimeError::new_err(format!("Initial scan failed: {}", e)))
    }
    
    /// Filter addresses by value equality
    fn filter_eq(&mut self, value: f64) -> PyResult<usize> {
        let scanner = self.scanner.as_mut()
            .ok_or_else(|| PyRuntimeError::new_err("Scanner not initialized"))?;
        
        let val = f64_to_value(value, self.value_type);
        scanner.filter(FilterOp::Equals, Some(val))
            .map_err(|e| PyRuntimeError::new_err(format!("Filter failed: {}", e)))
    }
    
    /// Filter addresses by value less than
    fn filter_lt(&mut self, value: f64) -> PyResult<usize> {
        let scanner = self.scanner.as_mut()
            .ok_or_else(|| PyRuntimeError::new_err("Scanner not initialized"))?;
        
        let val = f64_to_value(value, self.value_type);
        scanner.filter(FilterOp::LessThan, Some(val))
            .map_err(|e| PyRuntimeError::new_err(format!("Filter failed: {}", e)))
    }
    
    /// Filter addresses by value greater than
    fn filter_gt(&mut self, value: f64) -> PyResult<usize> {
        let scanner = self.scanner.as_mut()
            .ok_or_else(|| PyRuntimeError::new_err("Scanner not initialized"))?;
        
        let val = f64_to_value(value, self.value_type);
        scanner.filter(FilterOp::GreaterThan, Some(val))
            .map_err(|e| PyRuntimeError::new_err(format!("Filter failed: {}", e)))
    }
    
    /// Filter addresses where value increased
    fn filter_increased(&mut self) -> PyResult<usize> {
        let scanner = self.scanner.as_mut()
            .ok_or_else(|| PyRuntimeError::new_err("Scanner not initialized"))?;
        
        scanner.filter(FilterOp::Increased, None)
            .map_err(|e| PyRuntimeError::new_err(format!("Filter failed: {}", e)))
    }
    
    /// Filter addresses where value decreased
    fn filter_decreased(&mut self) -> PyResult<usize> {
        let scanner = self.scanner.as_mut()
            .ok_or_else(|| PyRuntimeError::new_err("Scanner not initialized"))?;
        
        scanner.filter(FilterOp::Decreased, None)
            .map_err(|e| PyRuntimeError::new_err(format!("Filter failed: {}", e)))
    }
    
    /// Filter addresses where value changed
    fn filter_changed(&mut self) -> PyResult<usize> {
        let scanner = self.scanner.as_mut()
            .ok_or_else(|| PyRuntimeError::new_err("Scanner not initialized"))?;
        
        scanner.filter(FilterOp::Changed, None)
            .map_err(|e| PyRuntimeError::new_err(format!("Filter failed: {}", e)))
    }
    
    /// Filter addresses where value unchanged
    fn filter_unchanged(&mut self) -> PyResult<usize> {
        let scanner = self.scanner.as_mut()
            .ok_or_else(|| PyRuntimeError::new_err("Scanner not initialized"))?;
        
        scanner.filter(FilterOp::Unchanged, None)
            .map_err(|e| PyRuntimeError::new_err(format!("Filter failed: {}", e)))
    }
    
    /// Get list of matched addresses
    fn get_matches(&self) -> PyResult<Vec<PyMatchedAddress>> {
        let scanner = self.scanner.as_ref()
            .ok_or_else(|| PyRuntimeError::new_err("Scanner not initialized"))?;
        
        let matches = scanner.matches();
        Ok(matches.iter().map(|m| PyMatchedAddress {
            address: m.address,
            current_value: value_to_f64(&m.current_value),
            previous_value: m.previous_value.as_ref().map(value_to_f64),
        }).collect())
    }
    
    /// Get number of matched addresses
    fn match_count(&self) -> PyResult<usize> {
        let scanner = self.scanner.as_ref()
            .ok_or_else(|| PyRuntimeError::new_err("Scanner not initialized"))?;
        
        Ok(scanner.matches().len())
    }
    
    /// Set value at all matched addresses
    fn set_value(&mut self, value: f64) -> PyResult<usize> {
        let scanner = self.scanner.as_mut()
            .ok_or_else(|| PyRuntimeError::new_err("Scanner not initialized"))?;
        
        let val = f64_to_value(value, self.value_type);
        scanner.write_all(val)
            .map_err(|e| PyRuntimeError::new_err(format!("Set value failed: {}", e)))
    }
    
    /// Set value at a specific address
    fn set_value_at(&mut self, address: usize, value: f64) -> PyResult<()> {
        let scanner = self.scanner.as_mut()
            .ok_or_else(|| PyRuntimeError::new_err("Scanner not initialized"))?;
        
        let val = f64_to_value(value, self.value_type);
        scanner.write_value(address, val)
            .map_err(|e| PyRuntimeError::new_err(format!("Set value failed: {}", e)))
    }
    
    /// Add value to all matched addresses
    fn add_value(&mut self, value: f64) -> PyResult<usize> {
        let scanner = self.scanner.as_mut()
            .ok_or_else(|| PyRuntimeError::new_err("Scanner not initialized"))?;
        
        let val = f64_to_value(value, self.value_type);
        scanner.modify_all(MathOp::Add, val)
            .map_err(|e| PyRuntimeError::new_err(format!("Math operation failed: {}", e)))
    }
    
    /// Subtract value from all matched addresses
    fn sub_value(&mut self, value: f64) -> PyResult<usize> {
        let scanner = self.scanner.as_mut()
            .ok_or_else(|| PyRuntimeError::new_err("Scanner not initialized"))?;
        
        let val = f64_to_value(value, self.value_type);
        scanner.modify_all(MathOp::Subtract, val)
            .map_err(|e| PyRuntimeError::new_err(format!("Math operation failed: {}", e)))
    }
    
    /// Multiply value at all matched addresses
    fn mul_value(&mut self, value: f64) -> PyResult<usize> {
        let scanner = self.scanner.as_mut()
            .ok_or_else(|| PyRuntimeError::new_err("Scanner not initialized"))?;
        
        let val = f64_to_value(value, self.value_type);
        scanner.modify_all(MathOp::Multiply, val)
            .map_err(|e| PyRuntimeError::new_err(format!("Math operation failed: {}", e)))
    }
    
    /// Divide value at all matched addresses
    fn div_value(&mut self, value: f64) -> PyResult<usize> {
        let scanner = self.scanner.as_mut()
            .ok_or_else(|| PyRuntimeError::new_err("Scanner not initialized"))?;
        
        let val = f64_to_value(value, self.value_type);
        scanner.modify_all(MathOp::Divide, val)
            .map_err(|e| PyRuntimeError::new_err(format!("Math operation failed: {}", e)))
    }
    
    /// Save a checkpoint with a given name
    fn save_checkpoint(&mut self, name: &str) -> PyResult<()> {
        let scanner = self.scanner.as_mut()
            .ok_or_else(|| PyRuntimeError::new_err("Scanner not initialized"))?;
        
        scanner.save_checkpoint(name.to_string())
            .map_err(|e| PyRuntimeError::new_err(format!("Save checkpoint failed: {}", e)))
    }
    
    /// List all checkpoint names
    fn list_checkpoints(&self) -> PyResult<Vec<String>> {
        let scanner = self.scanner.as_ref()
            .ok_or_else(|| PyRuntimeError::new_err("Scanner not initialized"))?;
        
        Ok(scanner.list_checkpoints().into_iter().map(|s| s.to_string()).collect())
    }
    
    /// Delete a checkpoint by name
    fn delete_checkpoint(&mut self, name: &str) -> PyResult<()> {
        let scanner = self.scanner.as_mut()
            .ok_or_else(|| PyRuntimeError::new_err("Scanner not initialized"))?;
        
        if scanner.delete_checkpoint(name) {
            Ok(())
        } else {
            Err(PyRuntimeError::new_err(format!("Checkpoint '{}' not found", name)))
        }
    }
    
    /// Filter by relative checkpoint changes
    fn filter_checkpoint(&mut self, cp1: &str, cp2: &str, cp3: &str, margin: f64) -> PyResult<usize> {
        let scanner = self.scanner.as_mut()
            .ok_or_else(|| PyRuntimeError::new_err("Scanner not initialized"))?;
        
        scanner.filter_checkpoint_relative(cp1, cp2, cp3, margin)
            .map_err(|e| PyRuntimeError::new_err(format!("Checkpoint filter failed: {}", e)))
    }
}

/// Python module initialization
#[pymodule]
fn memscan(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(open_process, m)?)?;
    m.add_function(wrap_pyfunction!(find_process_by_name, m)?)?;
    m.add_function(wrap_pyfunction!(query_system_info, m)?)?;
    m.add_function(wrap_pyfunction!(get_process_module_regions, m)?)?;
    m.add_function(wrap_pyfunction!(parse_hex_pattern, m)?)?;
    m.add_function(wrap_pyfunction!(read_process_memory, m)?)?;
    m.add_function(wrap_pyfunction!(write_process_memory, m)?)?;
    m.add_function(wrap_pyfunction!(create_interactive_scanner, m)?)?;
    
    m.add_class::<PyProcessHandle>()?;
    m.add_class::<PyMemoryRegion>()?;
    m.add_class::<PySystemInfo>()?;
    m.add_class::<PyInteractiveScanner>()?;
    m.add_class::<PyMatchedAddress>()?;
    
    Ok(())
}
