//! Test for scanner with memory mapping enabled

#[cfg(test)]
mod scanner_memmap_tests {
    use libmemscan::scanner::ScanOptions;

    #[test]
    fn test_scan_options_with_memmap() {
        // Test that ScanOptions can be created with use_memmap enabled
        let opts = ScanOptions {
            pattern: Some(b"test"),
            verbose: 0,
            all_modules: false,
            use_memmap: true,
        };

        assert!(opts.use_memmap);
        assert_eq!(opts.pattern, Some(b"test" as &[u8]));
        assert_eq!(opts.verbose, 0);
        assert!(!opts.all_modules);
    }

    #[test]
    fn test_scan_options_without_memmap() {
        // Test that ScanOptions can be created with use_memmap disabled
        let opts = ScanOptions {
            pattern: None,
            verbose: 1,
            all_modules: true,
            use_memmap: false,
        };

        assert!(!opts.use_memmap);
        assert_eq!(opts.pattern, None);
        assert_eq!(opts.verbose, 1);
        assert!(opts.all_modules);
    }
}
