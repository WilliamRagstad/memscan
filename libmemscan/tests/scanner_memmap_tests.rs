//! Test for scanner with memory mapping enabled

#[cfg(test)]
mod scanner_memmap_tests {
    use libmemscan::scanner::ScanOptions;

    #[test]
    fn test_scan_options_all_modules() {
        // Test that ScanOptions can be created with all_modules enabled
        let opts = ScanOptions {
            verbose: 0,
            all_modules: true,
        };

        assert!(opts.all_modules);
        assert_eq!(opts.verbose, 0);
    }

    #[test]
    fn test_scan_options_without_all_modules() {
        // Test that ScanOptions can be created with all_modules disabled
        let opts = ScanOptions {
            verbose: 1,
            all_modules: false,
        };

        assert!(!opts.all_modules);
        assert_eq!(opts.verbose, 1);
    }
}
