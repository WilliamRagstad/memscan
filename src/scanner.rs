#![cfg(windows)]

use crate::handle::AutoCloseHandle;
use crate::process::{
    MemoryRegion, MemoryRegionIterator, SystemInfo, protect_to_str, state_to_str, type_to_str,
};
use anyhow::Result;
use owo_colors::OwoColorize;
use std::cmp::min;
use winapi::{
    shared::{
        basetsd::SIZE_T,
        minwindef::{LPCVOID, LPVOID},
    },
    um::memoryapi::ReadProcessMemory,
};

pub struct ScanOptions<'a> {
    pub pattern: Option<&'a [u8]>,
    pub verbose: u8,
}

/// Perform static, single-pass scan all readable regions.
pub fn scan_process(
    proc: &AutoCloseHandle,
    sys: &SystemInfo,
    opts: &ScanOptions<'_>,
    ignored: &[MemoryRegion],
) -> Result<()> {
    let page_size = sys.page_size;
    let mut page_buf = vec![0u8; page_size];

    let mut total_regions = 0usize;
    let mut total_bytes = 0usize;
    let mut matches_found = 0usize;

    for region in MemoryRegionIterator::new(proc, sys) {
        if let Some(ign) = ignored.iter().find(|ign| ign.is_superset_of(&region)) {
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

        total_regions += 1;

        if opts.verbose > 1 {
            println!(
                "{} {:016x} - {:016x} ({} KiB) \t{}",
                "[region]".bright_blue(),
                region.base_address,
                region.base_address + region.size,
                region.size / 1024,
                format!(
                    "[{}, {}, {}]",
                    type_to_str(region.type_),
                    state_to_str(region.state),
                    protect_to_str(region.protect),
                )
                .green(),
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

        let mut offset = 0usize;
        while offset < region.size {
            let to_read = min(page_size, region.size - offset);
            let remote_addr = (region.base_address + offset) as LPCVOID;

            let ok = unsafe {
                let mut bytes_read: SIZE_T = 0;
                let res = ReadProcessMemory(
                    proc.raw(),
                    remote_addr,
                    page_buf.as_mut_ptr() as LPVOID,
                    to_read as SIZE_T,
                    &mut bytes_read as *mut SIZE_T,
                );
                if res == 0 || bytes_read == 0 {
                    false
                } else {
                    // Shrink buffer view logically to bytes_read
                    // (underlying vec stays allocated)
                    true
                }
            };

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
