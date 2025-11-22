//! Tests for the interactive module

use libmemscan::interactive::FilterOp;
use libmemscan::process::{MemoryRegion, MemoryProtection, MemoryState, MemoryType};
use libmemscan::values::{MathOp, Value, ValueType};

/// Helper function to create a mock memory region for testing
fn create_test_region(base: usize, size: usize) -> MemoryRegion {
    MemoryRegion {
        base_address: base,
        size,
        type_: MemoryType::Private,
        state: MemoryState {
            committed: true,
            free: false,
            reserved: false,
        },
        protect: MemoryProtection {
            no_access: false,
            read: true,
            write: true,
            execute: false,
            copy_on_write: false,
            guarded: false,
            no_cache: false,
        },
        image_file: None,
    }
}

#[test]
fn test_value_type_sizes() {
    assert_eq!(ValueType::I8.size(), 1);
    assert_eq!(ValueType::U8.size(), 1);
    assert_eq!(ValueType::I16.size(), 2);
    assert_eq!(ValueType::U16.size(), 2);
    assert_eq!(ValueType::I32.size(), 4);
    assert_eq!(ValueType::U32.size(), 4);
    assert_eq!(ValueType::F32.size(), 4);
    assert_eq!(ValueType::I64.size(), 8);
    assert_eq!(ValueType::U64.size(), 8);
    assert_eq!(ValueType::F64.size(), 8);
}

#[test]
fn test_value_conversions() {
    // Test I32
    let val = Value::I32(42);
    let bytes = val.to_bytes();
    assert_eq!(bytes.len(), 4);
    assert_eq!(bytes, vec![42, 0, 0, 0]);

    let restored = Value::from_bytes(&bytes, 0, ValueType::I32).unwrap();
    match restored {
        Value::I32(v) => assert_eq!(v, 42),
        _ => panic!("Wrong type"),
    }

    // Test U64
    let val = Value::U64(0x1234567890ABCDEF);
    let bytes = val.to_bytes();
    assert_eq!(bytes.len(), 8);

    let restored = Value::from_bytes(&bytes, 0, ValueType::U64).unwrap();
    match restored {
        Value::U64(v) => assert_eq!(v, 0x1234567890ABCDEF),
        _ => panic!("Wrong type"),
    }

    // Test F32
    let val = Value::F32(3.14);
    let bytes = val.to_bytes();
    assert_eq!(bytes.len(), 4);

    let restored = Value::from_bytes(&bytes, 0, ValueType::F32).unwrap();
    match restored {
        Value::F32(v) => assert!((v - 3.14).abs() < 0.001),
        _ => panic!("Wrong type"),
    }
}

#[test]
fn test_value_from_bytes_offset() {
    let bytes = vec![0x00, 0x00, 0x42, 0x00, 0x00, 0x00];
    
    // Read I32 at offset 0
    let val = Value::from_bytes(&bytes, 0, ValueType::I32).unwrap();
    match val {
        Value::I32(v) => assert_eq!(v, 0x42 << 16),
        _ => panic!("Wrong type"),
    }

    // Read I32 at offset 2
    let val = Value::from_bytes(&bytes, 2, ValueType::I32).unwrap();
    match val {
        Value::I32(v) => assert_eq!(v, 0x42),
        _ => panic!("Wrong type"),
    }

    // Read beyond buffer should return None
    let val = Value::from_bytes(&bytes, 4, ValueType::I32);
    assert!(val.is_none());
}

#[test]
fn test_filter_operations() {
    // FilterOp enum values
    assert_eq!(FilterOp::Equals, FilterOp::Equals);
    assert_ne!(FilterOp::Equals, FilterOp::LessThan);
    
    // MathOp enum values
    assert_eq!(MathOp::Add, MathOp::Add);
    assert_ne!(MathOp::Add, MathOp::Subtract);
}

#[test]
fn test_value_display() {
    // Just verify the values can be created and used
    let vals = vec![
        Value::I8(127),
        Value::I16(32767),
        Value::I32(2147483647),
        Value::I64(9223372036854775807),
        Value::U8(255),
        Value::U16(65535),
        Value::U32(4294967295),
        Value::U64(18446744073709551615),
        Value::F32(3.14),
        Value::F64(2.71828),
    ];

    for val in vals {
        let bytes = val.to_bytes();
        assert!(!bytes.is_empty());
    }
}

#[test]
fn test_value_subtract() {
    use libmemscan::values::value_subtract;
    
    // Test integer subtraction
    let result = value_subtract(&Value::I32(100), &Value::I32(50));
    assert!(result.is_some());
    match result.unwrap() {
        Value::I32(v) => assert_eq!(v, 50),
        _ => panic!("Wrong type"),
    }
    
    // Test float subtraction
    let result = value_subtract(&Value::F64(100.0), &Value::F64(50.0));
    assert!(result.is_some());
    match result.unwrap() {
        Value::F64(v) => assert!((v - 50.0).abs() < 0.001),
        _ => panic!("Wrong type"),
    }
    
    // Test type mismatch
    let result = value_subtract(&Value::I32(100), &Value::U32(50));
    assert!(result.is_none());
}

#[test]
fn test_value_to_f64() {
    use libmemscan::values::value_to_f64;
    
    assert_eq!(value_to_f64(&Value::I32(100)), 100.0);
    assert_eq!(value_to_f64(&Value::F64(3.14)), 3.14);
    assert_eq!(value_to_f64(&Value::U64(1000)), 1000.0);
}

