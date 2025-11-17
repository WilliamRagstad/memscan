//! No direct Windows or Linux API usage here; platform-specific reads are in OS modules

use crate::memmap::MappedMemory;
use crate::process::ProcessHandle;
use crate::process::{MemoryRegion, MemoryRegionIterator, SystemInfo, read_process_memory};
use anyhow::Result;
use owo_colors::OwoColorize;
use std::cmp::min;

#[cfg(unix)]
use crate::linux;
#[cfg(windows)]
use crate::windows;

pub struct ScanOptions<'a> {
    pub pattern: Option<&'a [u8]>,
    pub verbose: u8,
    pub all_modules: bool,
    /// Use memory mapping instead of ReadProcessMemory (default: true)
    pub use_memmap: bool,
}

/// Perform static, single-pass scan all readable regions.
pub fn scan_process(
    proc: &ProcessHandle,
    sys: &SystemInfo,
    opts: &ScanOptions<'_>,
    modules: &[MemoryRegion],
) -> Result<()> {
    let page_size = sys.page_size;
    let mut page_buf = vec![0u8; page_size];

    let mut total_regions = 0usize;
    let mut total_bytes = 0usize;
    let mut matches_found = 0usize;

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

        total_regions += 1;

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

        // Try to use memory mapping if enabled, fall back to ReadProcessMemory on error
        if opts.use_memmap {
            match MappedMemory::new(proc, &region) {
                Ok(mapped) => {
                    // Successfully mapped, use the mapped memory
                    let memory_slice = mapped.data();
                    total_bytes += memory_slice.len();

                    if let Some(pattern) = opts.pattern {
                        let mut prev_off = 0;
                        while prev_off < memory_slice.len() {
                            if let Some(rel_off) = naive_search(&memory_slice[prev_off..], pattern)
                            {
                                let abs_addr = region.base_address + prev_off + rel_off;
                                matches_found += 1;
                                println!("{}  {:016x}", "[match]".bright_green(), abs_addr);
                                if opts.verbose > 0 {
                                    // Display surrounding bytes and highlight match
                                    const CONTEXT_BYTES: usize = 8;
                                    let match_offset = prev_off + rel_off;
                                    let start = match_offset.saturating_sub(CONTEXT_BYTES);
                                    let end = min(
                                        match_offset + pattern.len() + CONTEXT_BYTES,
                                        memory_slice.len(),
                                    );
                                    print!("{}", " ... ".bright_black());
                                    let mut i = start;
                                    while i < end {
                                        if i == match_offset {
                                            // Highlight match
                                            for b in &memory_slice[i..i + pattern.len()] {
                                                print!(
                                                    "{}",
                                                    format!("{:02x} ", b).bright_green().bold()
                                                );
                                            }
                                            i += pattern.len();
                                        } else {
                                            print!(
                                                "{}",
                                                format!("{:02x} ", memory_slice[i]).bright_black()
                                            );
                                            i += 1;
                                        }
                                    }
                                    println!("{}", " ... ".bright_black());
                                }
                                prev_off += rel_off + 1; // continue searching after this match
                            } else {
                                break;
                            }
                        }
                    }
                    continue; // Skip to next region
                }
                Err(_) => {
                    // Fall back to page-by-page reading
                    if opts.verbose > 2 {
                        println!(
                            "{} memory mapping failed for region {:016x}, falling back to ReadProcessMemory",
                            "[warn]".yellow(),
                            region.base_address
                        );
                    }
                }
            }
        }

        // Original page-by-page reading (fallback or when use_memmap is false)
        let mut offset = 0usize;
        while offset < region.size {
            let to_read = min(page_size, region.size - offset);
            let read_n =
                read_process_memory(proc, region.base_address + offset, &mut page_buf[..to_read]);
            let ok = read_n > 0;

            if ok {
                total_bytes += to_read;

                if let Some(pattern) = opts.pattern {
                    let mut prev_off = 0;
                    while prev_off < to_read {
                        if let Some(rel_off) = naive_search(&page_buf[prev_off..to_read], pattern) {
                            let page_offset = prev_off + rel_off;
                            let abs_addr = region.base_address + offset + page_offset;
                            matches_found += 1;
                            println!("{}  {:016x}", "[match]".bright_green(), abs_addr);
                            if opts.verbose > 0 {
                                // Display surrounding bytes and highlight match
                                const CONTEXT_BYTES: usize = 8;
                                let start = page_offset.saturating_sub(CONTEXT_BYTES);
                                let end = min(page_offset + pattern.len() + CONTEXT_BYTES, to_read);
                                print!("{}", " ... ".bright_black());
                                let mut i = start;
                                while i < end {
                                    if i == page_offset {
                                        // Highlight match
                                        for b in &page_buf[i..i + pattern.len()] {
                                            print!(
                                                "{}",
                                                format!("{:02x} ", b).bright_green().bold()
                                            );
                                        }
                                        i += pattern.len();
                                    } else {
                                        print!(
                                            "{}",
                                            format!("{:02x} ", page_buf[i]).bright_black()
                                        );
                                        i += 1;
                                    }
                                }
                                println!("{}", " ... ".bright_black());
                            }
                            prev_off += rel_off + 1; // continue searching after this match
                        } else {
                            break;
                        }
                    }
                }
            } else if opts.verbose > 1 {
                println!(
                    "{} failed to read page at {:016x}",
                    "[warn]".yellow(),
                    region.base_address + offset
                );
            }

            offset += to_read;
        }
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
}
