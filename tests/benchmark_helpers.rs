//! Simple tests for benchmark helper functions
//! These ensure the benchmark code itself is correct

/// Simple O(n*m) pattern matcher - copy from benchmarks
fn naive_search(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || needle.len() > haystack.len() {
        return None;
    }
    for i in 0..=haystack.len() - needle.len() {
        if &haystack[i..i + needle.len()] == needle {
            return Some(i);
        }
    }
    None
}

/// Parse a hex string - copy from benchmarks
fn parse_hex_pattern(s: &str) -> Result<Vec<u8>, String> {
    let filtered: String = s.chars().filter(|c| !c.is_whitespace()).collect();

    if filtered.len() % 2 != 0 {
        return Err("hex pattern length must be even".to_string());
    }

    let mut bytes = Vec::with_capacity(filtered.len() / 2);
    for i in (0..filtered.len()).step_by(2) {
        let byte_str = &filtered[i..i + 2];
        let b = u8::from_str_radix(byte_str, 16)
            .map_err(|_| format!("invalid hex byte '{}'", byte_str))?;
        bytes.push(b);
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_naive_search_found() {
        let haystack = b"hello world";
        let needle = b"world";
        assert_eq!(naive_search(haystack, needle), Some(6));
    }

    #[test]
    fn test_naive_search_not_found() {
        let haystack = b"hello world";
        let needle = b"rust";
        assert_eq!(naive_search(haystack, needle), None);
    }

    #[test]
    fn test_naive_search_at_start() {
        let haystack = b"hello world";
        let needle = b"hello";
        assert_eq!(naive_search(haystack, needle), Some(0));
    }

    #[test]
    fn test_naive_search_empty_needle() {
        let haystack = b"hello world";
        let needle = b"";
        assert_eq!(naive_search(haystack, needle), None);
    }

    #[test]
    fn test_naive_search_binary_pattern() {
        let haystack = b"\x4D\x5A\x90\x00\x03\x00\x00\x00";
        let needle = b"\x4D\x5A\x90\x00";
        assert_eq!(naive_search(haystack, needle), Some(0));
    }

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
