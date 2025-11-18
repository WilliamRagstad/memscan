//! Library exposing performance-critical functions for benchmarking
//!
//! This module re-exports functions that need to be benchmarked,
//! allowing benchmark code to use the real implementations instead of copies.

// OS-specific modules
pub(crate) mod linux;
pub(crate) mod windows;

// Platform-independent modules
pub mod diff;
pub mod interactive;
pub mod memmap;
pub mod process;
pub mod scanner;

use anyhow::Result;

/// Parse a hex string like "DEADBEEF" or "4D 5A 90 00" into bytes.
pub fn parse_hex_pattern(s: &str) -> Result<Vec<u8>> {
    let filtered: String = s.chars().filter(|c| !c.is_whitespace()).collect();

    if filtered.len() % 2 != 0 {
        anyhow::bail!("hex pattern length must be even");
    }

    let mut bytes = Vec::with_capacity(filtered.len() / 2);
    for i in (0..filtered.len()).step_by(2) {
        let byte_str = &filtered[i..i + 2];
        let b = u8::from_str_radix(byte_str, 16)
            .map_err(|_| anyhow::anyhow!("invalid hex byte '{}'", byte_str))?;
        bytes.push(b);
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hex_simple() {
        let result = parse_hex_pattern("DEADBEEF").unwrap();
        assert_eq!(result, vec![0xDE, 0xAD, 0xBE, 0xEF]);
    }

    #[test]
    fn test_parse_hex_with_spaces() {
        let result = parse_hex_pattern("DE AD BE EF").unwrap();
        assert_eq!(result, vec![0xDE, 0xAD, 0xBE, 0xEF]);
    }

    #[test]
    fn test_parse_hex_lowercase() {
        let result = parse_hex_pattern("deadbeef").unwrap();
        assert_eq!(result, vec![0xDE, 0xAD, 0xBE, 0xEF]);
    }

    #[test]
    fn test_parse_hex_mixed_case() {
        let result = parse_hex_pattern("DeAdBeEf").unwrap();
        assert_eq!(result, vec![0xDE, 0xAD, 0xBE, 0xEF]);
    }

    #[test]
    fn test_parse_hex_odd_length() {
        let result = parse_hex_pattern("ABC");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_hex_invalid_char() {
        let result = parse_hex_pattern("ABGH");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_hex_pe_header() {
        let result = parse_hex_pattern("4D 5A 90 00").unwrap();
        assert_eq!(result, vec![0x4D, 0x5A, 0x90, 0x00]);
    }
}
