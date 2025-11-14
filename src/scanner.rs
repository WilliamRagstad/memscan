#![cfg(windows)]

use crate::process::{MemoryRegionIterator, ProcessHandle, SystemInfo};
use anyhow::Result;
use owo_colors::OwoColorize;

use std::cmp::min;

use winapi::shared::basetsd::SIZE_T;
use winapi::shared::minwindef::{LPCVOID, LPVOID};
use winapi::um::memoryapi::ReadProcessMemory;

pub struct ScanOptions<'a> {
    pub pattern: Option<&'a [u8]>,
    pub verbose: u8,
}

/// Perform static, single-pass scan all readable regions.
pub fn scan_process(proc: &ProcessHandle, sys: &SystemInfo, opts: &ScanOptions<'_>) -> Result<()> {
    let page_size = sys.page_size;
    let mut page_buf = vec![0u8; page_size];

    let mut total_regions = 0usize;
    let mut total_bytes = 0usize;
    let mut matches_found = 0usize;

    for region in MemoryRegionIterator::new(proc, sys) {
        total_regions += 1;

        if opts.verbose > 0 {
            println!(
                "{} {:#x} - {:#x} ({} KiB)",
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
                    if let Some(rel_off) = naive_search(&page_buf[..to_read], pattern) {
                        let abs_addr = region.base_address + offset + rel_off;
                        matches_found += 1;
                        println!("{} match at {:#x}", "[hit]".bright_green(), abs_addr);
                    }
                }
            } else if opts.verbose > 1 {
                println!(
                    "{} failed to read page at {:#x}",
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
