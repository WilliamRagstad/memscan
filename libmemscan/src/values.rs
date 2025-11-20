//! Value types and operations for memory scanning
//!
//! This module provides type-safe value handling for different data types
//! in memory, including conversions, comparisons, and mathematical operations.

use anyhow::Result;

/// Supported value types for filtering
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueType {
    I8,
    I16,
    I32,
    I64,
    U8,
    U16,
    U32,
    U64,
    F32,
    F64,
}

impl ValueType {
    /// Get the size in bytes for this value type
    pub fn size(&self) -> usize {
        match self {
            ValueType::I8 | ValueType::U8 => 1,
            ValueType::I16 | ValueType::U16 => 2,
            ValueType::I32 | ValueType::U32 | ValueType::F32 => 4,
            ValueType::I64 | ValueType::U64 | ValueType::F64 => 8,
        }
    }
}

/// A value read from memory that can be one of several types
#[derive(Debug, Clone)]
pub enum Value {
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    F32(f32),
    F64(f64),
}

impl Value {
    /// Read a value from bytes at the given offset
    pub fn from_bytes(bytes: &[u8], offset: usize, value_type: ValueType) -> Option<Self> {
        if offset + value_type.size() > bytes.len() {
            return None;
        }
        
        let slice = &bytes[offset..offset + value_type.size()];
        Some(match value_type {
            ValueType::I8 => Value::I8(i8::from_le_bytes([slice[0]])),
            ValueType::I16 => Value::I16(i16::from_le_bytes(slice.try_into().ok()?)),
            ValueType::I32 => Value::I32(i32::from_le_bytes(slice.try_into().ok()?)),
            ValueType::I64 => Value::I64(i64::from_le_bytes(slice.try_into().ok()?)),
            ValueType::U8 => Value::U8(u8::from_le_bytes([slice[0]])),
            ValueType::U16 => Value::U16(u16::from_le_bytes(slice.try_into().ok()?)),
            ValueType::U32 => Value::U32(u32::from_le_bytes(slice.try_into().ok()?)),
            ValueType::U64 => Value::U64(u64::from_le_bytes(slice.try_into().ok()?)),
            ValueType::F32 => Value::F32(f32::from_le_bytes(slice.try_into().ok()?)),
            ValueType::F64 => Value::F64(f64::from_le_bytes(slice.try_into().ok()?)),
        })
    }
    
    /// Convert value to bytes for writing to memory
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            Value::I8(v) => v.to_le_bytes().to_vec(),
            Value::I16(v) => v.to_le_bytes().to_vec(),
            Value::I32(v) => v.to_le_bytes().to_vec(),
            Value::I64(v) => v.to_le_bytes().to_vec(),
            Value::U8(v) => v.to_le_bytes().to_vec(),
            Value::U16(v) => v.to_le_bytes().to_vec(),
            Value::U32(v) => v.to_le_bytes().to_vec(),
            Value::U64(v) => v.to_le_bytes().to_vec(),
            Value::F32(v) => v.to_le_bytes().to_vec(),
            Value::F64(v) => v.to_le_bytes().to_vec(),
        }
    }
}

/// Math operations for modifying values
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MathOp {
    Add,
    Subtract,
    Multiply,
    Divide,
}

/// Compare two values for equality
pub fn values_equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::I8(a), Value::I8(b)) => a == b,
        (Value::I16(a), Value::I16(b)) => a == b,
        (Value::I32(a), Value::I32(b)) => a == b,
        (Value::I64(a), Value::I64(b)) => a == b,
        (Value::U8(a), Value::U8(b)) => a == b,
        (Value::U16(a), Value::U16(b)) => a == b,
        (Value::U32(a), Value::U32(b)) => a == b,
        (Value::U64(a), Value::U64(b)) => a == b,
        (Value::F32(a), Value::F32(b)) => a == b,
        (Value::F64(a), Value::F64(b)) => a == b,
        _ => false,
    }
}

/// Compare if value a is less than value b
pub fn value_less_than(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::I8(a), Value::I8(b)) => a < b,
        (Value::I16(a), Value::I16(b)) => a < b,
        (Value::I32(a), Value::I32(b)) => a < b,
        (Value::I64(a), Value::I64(b)) => a < b,
        (Value::U8(a), Value::U8(b)) => a < b,
        (Value::U16(a), Value::U16(b)) => a < b,
        (Value::U32(a), Value::U32(b)) => a < b,
        (Value::U64(a), Value::U64(b)) => a < b,
        (Value::F32(a), Value::F32(b)) => a < b,
        (Value::F64(a), Value::F64(b)) => a < b,
        _ => false,
    }
}

/// Compare if value a is greater than value b
pub fn value_greater_than(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::I8(a), Value::I8(b)) => a > b,
        (Value::I16(a), Value::I16(b)) => a > b,
        (Value::I32(a), Value::I32(b)) => a > b,
        (Value::I64(a), Value::I64(b)) => a > b,
        (Value::U8(a), Value::U8(b)) => a > b,
        (Value::U16(a), Value::U16(b)) => a > b,
        (Value::U32(a), Value::U32(b)) => a > b,
        (Value::U64(a), Value::U64(b)) => a > b,
        (Value::F32(a), Value::F32(b)) => a > b,
        (Value::F64(a), Value::F64(b)) => a > b,
        _ => false,
    }
}

/// Apply a math operation to two values
pub fn apply_math_op(a: &Value, b: &Value, op: MathOp) -> Result<Value> {
    Ok(match (a, b) {
        (Value::I8(a), Value::I8(b)) => match op {
            MathOp::Add => Value::I8(a.wrapping_add(*b)),
            MathOp::Subtract => Value::I8(a.wrapping_sub(*b)),
            MathOp::Multiply => Value::I8(a.wrapping_mul(*b)),
            MathOp::Divide => Value::I8(a.wrapping_div(*b)),
        },
        (Value::I16(a), Value::I16(b)) => match op {
            MathOp::Add => Value::I16(a.wrapping_add(*b)),
            MathOp::Subtract => Value::I16(a.wrapping_sub(*b)),
            MathOp::Multiply => Value::I16(a.wrapping_mul(*b)),
            MathOp::Divide => Value::I16(a.wrapping_div(*b)),
        },
        (Value::I32(a), Value::I32(b)) => match op {
            MathOp::Add => Value::I32(a.wrapping_add(*b)),
            MathOp::Subtract => Value::I32(a.wrapping_sub(*b)),
            MathOp::Multiply => Value::I32(a.wrapping_mul(*b)),
            MathOp::Divide => Value::I32(a.wrapping_div(*b)),
        },
        (Value::I64(a), Value::I64(b)) => match op {
            MathOp::Add => Value::I64(a.wrapping_add(*b)),
            MathOp::Subtract => Value::I64(a.wrapping_sub(*b)),
            MathOp::Multiply => Value::I64(a.wrapping_mul(*b)),
            MathOp::Divide => Value::I64(a.wrapping_div(*b)),
        },
        (Value::U8(a), Value::U8(b)) => match op {
            MathOp::Add => Value::U8(a.wrapping_add(*b)),
            MathOp::Subtract => Value::U8(a.wrapping_sub(*b)),
            MathOp::Multiply => Value::U8(a.wrapping_mul(*b)),
            MathOp::Divide => Value::U8(a.wrapping_div(*b)),
        },
        (Value::U16(a), Value::U16(b)) => match op {
            MathOp::Add => Value::U16(a.wrapping_add(*b)),
            MathOp::Subtract => Value::U16(a.wrapping_sub(*b)),
            MathOp::Multiply => Value::U16(a.wrapping_mul(*b)),
            MathOp::Divide => Value::U16(a.wrapping_div(*b)),
        },
        (Value::U32(a), Value::U32(b)) => match op {
            MathOp::Add => Value::U32(a.wrapping_add(*b)),
            MathOp::Subtract => Value::U32(a.wrapping_sub(*b)),
            MathOp::Multiply => Value::U32(a.wrapping_mul(*b)),
            MathOp::Divide => Value::U32(a.wrapping_div(*b)),
        },
        (Value::U64(a), Value::U64(b)) => match op {
            MathOp::Add => Value::U64(a.wrapping_add(*b)),
            MathOp::Subtract => Value::U64(a.wrapping_sub(*b)),
            MathOp::Multiply => Value::U64(a.wrapping_mul(*b)),
            MathOp::Divide => Value::U64(a.wrapping_div(*b)),
        },
        (Value::F32(a), Value::F32(b)) => match op {
            MathOp::Add => Value::F32(a + b),
            MathOp::Subtract => Value::F32(a - b),
            MathOp::Multiply => Value::F32(a * b),
            MathOp::Divide => Value::F32(a / b),
        },
        (Value::F64(a), Value::F64(b)) => match op {
            MathOp::Add => Value::F64(a + b),
            MathOp::Subtract => Value::F64(a - b),
            MathOp::Multiply => Value::F64(a * b),
            MathOp::Divide => Value::F64(a / b),
        },
        _ => anyhow::bail!("Type mismatch in math operation"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_value_type_size() {
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
    fn test_value_from_bytes() {
        let bytes = vec![0x42, 0x00, 0x00, 0x00];
        let val = Value::from_bytes(&bytes, 0, ValueType::I32).unwrap();
        match val {
            Value::I32(v) => assert_eq!(v, 0x42),
            _ => panic!("Wrong type"),
        }
    }

    #[test]
    fn test_value_to_bytes() {
        let val = Value::I32(0x42);
        let bytes = val.to_bytes();
        assert_eq!(bytes, vec![0x42, 0x00, 0x00, 0x00]);
    }

    #[test]
    fn test_values_equal() {
        assert!(values_equal(&Value::I32(42), &Value::I32(42)));
        assert!(!values_equal(&Value::I32(42), &Value::I32(43)));
        assert!(!values_equal(&Value::I32(42), &Value::U32(42)));
    }

    #[test]
    fn test_value_comparisons() {
        assert!(value_less_than(&Value::I32(10), &Value::I32(20)));
        assert!(!value_less_than(&Value::I32(20), &Value::I32(10)));
        assert!(value_greater_than(&Value::I32(20), &Value::I32(10)));
        assert!(!value_greater_than(&Value::I32(10), &Value::I32(20)));
    }

    #[test]
    fn test_math_operations() {
        let result = apply_math_op(&Value::I32(10), &Value::I32(5), MathOp::Add).unwrap();
        assert!(values_equal(&result, &Value::I32(15)));

        let result = apply_math_op(&Value::I32(10), &Value::I32(5), MathOp::Subtract).unwrap();
        assert!(values_equal(&result, &Value::I32(5)));

        let result = apply_math_op(&Value::I32(10), &Value::I32(5), MathOp::Multiply).unwrap();
        assert!(values_equal(&result, &Value::I32(50)));

        let result = apply_math_op(&Value::I32(10), &Value::I32(5), MathOp::Divide).unwrap();
        assert!(values_equal(&result, &Value::I32(2)));
    }
}
