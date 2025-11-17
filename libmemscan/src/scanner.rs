//! No direct Windows or Linux API usage here; platform-specific reads are in OS modules

use crate::memmap::{MappedMemory, MemoryMapper};
use crate::process::ProcessHandle;
use crate::process::{MemoryRegion, MemoryRegionIterator, SystemInfo};
use anyhow::Result;
use memchr::memmem;
use owo_colors::OwoColorize;

pub struct ScanOptions {
    pub verbose: u8,
    pub all_modules: bool,
}

/// Perform static, single-pass scan all readable regions.
pub fn scan_process(
    proc: &ProcessHandle,
    sys: &SystemInfo,
    pattern: &[u8],
    opts: &ScanOptions,
    modules: &[MemoryRegion],
) -> Result<()> {
    let mut memory_mapper = MemoryMapper::new(proc);
    let mut total_regions = 0usize;
    let mut total_bytes = 0usize;
    let mut matches_found = 0usize;

    // First map all regions
    for region in MemoryRegionIterator::new(proc, sys) {
        let current_module = modules.iter().find(|ign| ign.is_superset_of(&region));
        let current_module_file = current_module.and_then(|ign| ign.image_file.as_deref());
        let current_module_name = current_module_file
            .and_then(|f| Some(f.rsplit(['\\', '/'].as_ref()).next().unwrap_or(f)));

        if !opts.all_modules {
            if let Some(ign) = current_module {
                let image_file = ign.image_file.as_deref().unwrap_or("unknown");
                if opts.verbose > 2 {
                    println!(
                        "{}   {:016x} - {:016x} ({} KiB) \t{}{}{}",
                        "[skip]".bright_yellow(),
                        region.base_address,
                        region.base_address + region.size,
                        region.size / 1024,
                        "[".magenta(),
                        image_file.magenta(),
                        "]".magenta()
                    );
                } else if opts.verbose > 1 {
                    let image_name = image_file
                        .rsplit(['\\', '/'].as_ref())
                        .next()
                        .unwrap_or(image_file);
                    println!(
                        "{}   {:016x} - {:016x} ({} KiB) \t{}{}{}",
                        "[skip]".bright_yellow(),
                        region.base_address,
                        region.base_address + region.size,
                        region.size / 1024,
                        "[".magenta(),
                        image_name.magenta(),
                        "]".magenta()
                    );
                }
                continue;
            }
        }

        if opts.verbose > 1 {
            println!(
                "{} {:016x} - {:016x} ({} KiB) \t[{}, {}, {}, {}]",
                "[region]".bright_blue(),
                region.base_address,
                region.base_address + region.size,
                region.size / 1024,
                region.type_.green(),
                region.state.green(),
                region.protect.green(),
                current_module_name.unwrap_or("unknown").magenta()
            );
        } else if opts.verbose > 0 {
            println!(
                "{} {:016x} - {:016x} ({} KiB)",
                "[region]".bright_blue(),
                region.base_address,
                region.base_address + region.size,
                region.size / 1024
            );
        }

        total_regions += 1;
        total_bytes += region.size;
        let region_base_addr = region.base_address;
        if let Err(err) = memory_mapper.map_region(region) {
            if opts.verbose > 0 {
                println!(
                    "{} memory mapping failed for region {:016x}: {}",
                    "[warn]".yellow(),
                    region_base_addr,
                    err
                );
            }
        }
    }

    println!(
        "{} mapped {} regions, ~{} KiB",
        "[info]".bright_cyan(),
        total_regions,
        total_bytes / 1024,
    );

    // Now scan all mapped regions
    for mapped in memory_mapper.into_iter() {
        total_bytes += mapped.remote_region.size;
        let matches = scan_region(&mapped, pattern, opts)?;
        matches_found += matches;
    }

    println!(
        "{} scanned {} regions, ~{} KiB, {} matches",
        "[done]".bright_cyan(),
        total_regions,
        total_bytes / 1024,
        matches_found,
    );

    Ok(())
}

pub fn scan_region(mapped: &MappedMemory, pattern: &[u8], opts: &ScanOptions) -> Result<usize> {
    let mut matches_found = 0usize;
    let mut prev_off = 0;
    let haystack = mapped.data();
    while prev_off < haystack.len() {
        if let Some(rel_off) = optimized_search(&haystack[prev_off..], pattern) {
            let match_address = mapped.remote_region.base_address + prev_off + rel_off;
            print_match_context(match_address, haystack, pattern, prev_off, rel_off, opts);
            matches_found += 1;

            prev_off += rel_off + 1; // continue searching after this match
        } else {
            break;
        }
    }
    Ok(matches_found)
}

fn print_match_context(
    abs_addr: usize,
    memory_slice: &[u8],
    pattern: &[u8],
    prev_off: usize,
    rel_off: usize,
    opts: &ScanOptions,
) {
    println!("{}  {:016x}", "[match]".bright_green(), abs_addr);
    if opts.verbose > 0 {
        // Display surrounding bytes and highlight match
        const CONTEXT_BYTES: usize = 8;
        let match_offset = prev_off + rel_off;
        let start = match_offset.saturating_sub(CONTEXT_BYTES);
        let end = std::cmp::min(
            match_offset + pattern.len() + CONTEXT_BYTES,
            memory_slice.len(),
        );
        print!("{}", " ... ".bright_black());
        let mut i = start;
        while i < end {
            if i == match_offset {
                // Highlight match
                for b in &memory_slice[i..i + pattern.len()] {
                    print!("{}", format!("{:02x} ", b).bright_green().bold());
                }
                i += pattern.len();
            } else {
                print!("{}", format!("{:02x} ", memory_slice[i]).bright_black());
                i += 1;
            }
        }
        println!("{}", " ... ".bright_black());
    }
}

/// Very simple O(n*m) pattern matcher sufficient for now.
/// This is platform-independent and can be used in benchmarks.
pub fn naive_search(haystack: &[u8], needle: &[u8]) -> Option<usize> {
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

/// Optimized pattern search using the `memchr` crate.
/// This uses SIMD instructions for significantly better performance.
pub fn optimized_search(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return None;
    }
    memmem::find(haystack, needle)
}

// no extra helpers needed on UNIX; we call ProcessHandleUnix::read_mem directly

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
    fn test_optimized_search_found() {
        let haystack = b"hello world";
        let needle = b"world";
        assert_eq!(optimized_search(haystack, needle), Some(6));
    }

    #[test]
    fn test_optimized_search_not_found() {
        let haystack = b"hello world";
        let needle = b"rust";
        assert_eq!(optimized_search(haystack, needle), None);
    }

    #[test]
    fn test_optimized_search_at_start() {
        let haystack = b"hello world";
        let needle = b"hello";
        assert_eq!(optimized_search(haystack, needle), Some(0));
    }

    #[test]
    fn test_optimized_search_empty_needle() {
        let haystack = b"hello world";
        let needle = b"";
        assert_eq!(optimized_search(haystack, needle), None);
    }

    #[test]
    fn test_optimized_search_binary_pattern() {
        let haystack = b"\x4D\x5A\x90\x00\x03\x00\x00\x00";
        let needle = b"\x4D\x5A\x90\x00";
        assert_eq!(optimized_search(haystack, needle), Some(0));
    }

    #[test]
    fn test_both_searches_match() {
        // Ensure both search functions produce the same results
        let haystacks = [
            b"hello world" as &[u8],
            b"\x4D\x5A\x90\x00\x03\x00\x00\x00",
            b"",
            b"a",
            b"aaaaaaaaaa",
        ];
        let needles = [
            b"world" as &[u8],
            b"\x4D\x5A\x90\x00",
            b"",
            b"a",
            b"aa",
            b"not_there",
        ];

        for haystack in &haystacks {
            for needle in &needles {
                let naive_result = naive_search(haystack, needle);
                let optimized_result = optimized_search(haystack, needle);
                assert_eq!(
                    naive_result, optimized_result,
                    "Mismatch for haystack {:?} and needle {:?}",
                    haystack, needle
                );
            }
        }
    }
}
