#![cfg(unix)]
use crate::process::{
    MemoryProtection, MemoryRegion, MemoryState, MemoryType, ProcessHandle, SystemInfo,
    is_region_interesting,
};
use anyhow::Result;
use libc::{_SC_PAGESIZE, pid_t, sysconf};
use std::{
    collections::HashMap,
    fs::{File, read_link},
    io::{BufRead, BufReader},
    os::{
        fd::{AsRawFd, RawFd},
        unix::fs::FileExt,
    },
    path::Path,
};

// ================== Linux/UNIX-specific process types ==================

#[derive(Debug)]
pub struct ProcessHandleUnix {
    pid: pid_t,
    mem: File,
    maps: Vec<MemoryRegion>,
    page_size: usize,
    exe_path: Option<String>,
}

unsafe impl Send for ProcessHandleUnix {}
unsafe impl Sync for ProcessHandleUnix {}

impl ProcessHandleUnix {
    pub fn raw(&self) -> pid_t {
        self.pid
    }

    pub fn mem_fd(&self) -> RawFd {
        self.mem.as_raw_fd()
    }

    pub fn read_mem(&self, addr: usize, buf: &mut [u8]) -> std::io::Result<usize> {
        self.mem.read_at(buf, addr as u64)
    }

    pub fn write_mem(&self, addr: usize, buf: &[u8]) -> std::io::Result<usize> {
        self.mem.write_at(buf, addr as u64)
    }
}

// ================== Linux/UNIX-specific helpers ==================

fn parse_proc_maps(pid: pid_t) -> Result<(Vec<MemoryRegion>, Option<String>)> {
    // NOTE: This function now returns Vec<MemoryRegion> instead of a custom MapEntry
    // to align with the user's request to use the cross-platform struct directly.
    let maps_path = format!("/proc/{pid}/maps");
    let file = File::open(&maps_path)
        .map_err(|e| anyhow::anyhow!("failed to open {}: {}", maps_path, e))?;
    let reader = BufReader::new(file);

    let exe_path = read_link(format!("/proc/{pid}/exe"))
        .ok()
        .and_then(|p| p.to_str().map(|s| s.to_string()));

    let mut entries: Vec<MemoryRegion> = Vec::new();
    for line_res in reader.lines() {
        let line = line_res?;
        // Format:
        // start-end perms offset `dev:inode` pathname
        // Example:
        // `00400000-0040b000 r-xp 00000000 08:01 131104 /usr/bin/cat`
        let mut parts = line.splitn(6, ' ').filter(|s| !s.is_empty());
        let addr = parts.next().unwrap_or("");
        let perms = parts.next().unwrap_or("");
        // `offset`, `dev`, `inode` are currently unused in MemoryRegion abstraction
        let _offset_hex = parts.next().unwrap_or("0");
        let _dev = parts.next().unwrap_or("");
        let _inode = parts.next().unwrap_or("0");
        let pathname_opt = parts.next().and_then(|p| {
            let p = p.trim();
            if p.is_empty() {
                None
            } else {
                Some(p.to_string())
            }
        });

        let mut addr_it = addr.split('-');
        let start = usize::from_str_radix(addr_it.next().unwrap_or("0"), 16).unwrap_or(0);
        let end = usize::from_str_radix(addr_it.next().unwrap_or("0"), 16).unwrap_or(0);
        let size = end.saturating_sub(start);

        // Convert to cross-platform fields immediately
        let protect = perms_to_protection(perms);
        let state = MemoryState {
            committed: true,
            free: false,
            reserved: false,
        };
        let image_file = pathname_opt.as_ref().and_then(|p| {
            // Only keep file-backed paths, skip pseudo like [heap], [stack]
            if p.starts_with('[') {
                None
            } else {
                Some(p.clone())
            }
        });
        let type_ = perms_to_type(perms, &image_file, &exe_path);

        entries.push(MemoryRegion {
            base_address: start,
            size,
            protect,
            state,
            type_,
            image_file,
        });
    }

    // Ensure sorted by start address
    entries.sort_by_key(|e| e.base_address);

    Ok((entries, exe_path))
}

fn perms_to_protection(perms: &str) -> MemoryProtection {
    let bytes = perms.as_bytes();
    let read = bytes.get(0).map(|&c| c == b'r').unwrap_or(false);
    let write = bytes.get(1).map(|&c| c == b'w').unwrap_or(false);
    let exec = bytes.get(2).map(|&c| c == b'x').unwrap_or(false);
    // Linux doesn't expose guarded or no_cache in maps; assume false
    MemoryProtection {
        no_access: false, // Not an exact match; leave false so we attempt reads and handle failures
        read,
        write,
        execute: exec,
        copy_on_write: false,
        guarded: false,
        no_cache: false,
    }
}

fn perms_to_type(perms: &str, pathname: &Option<String>, _exe_path: &Option<String>) -> MemoryType {
    let shared_flag = perms.as_bytes().get(3).map(|&c| c == b's').unwrap_or(false);
    if let Some(path) = pathname {
        // Treat file-backed mappings as images
        if Path::new(path).is_file() {
            return MemoryType::Image;
        }
    }
    if shared_flag {
        MemoryType::Mapped
    } else {
        MemoryType::Private
    }
}

// ================== Linux-specific process functions ==================

pub(crate) fn open_process(pid: u32) -> Result<ProcessHandle> {
    let pid_i = pid as pid_t;
    // Open /proc/<pid>/mem for reading
    let mem_path = format!("/proc/{pid}/mem");
    let mem =
        File::open(&mem_path).map_err(|e| anyhow::anyhow!("failed to open {}: {}", mem_path, e))?;

    let (maps, exe_path) = parse_proc_maps(pid_i)?;
    let page_size = unsafe { sysconf(_SC_PAGESIZE) as usize };

    Ok(ProcessHandleUnix {
        pid: pid_i,
        mem,
        maps,
        page_size,
        exe_path,
    })
}

/// Find the PID of the first process whose executable name matches `name` (case-insensitive).
/// On Linux, we'll try `/proc/<pid>/comm` first; if that doesn't match, fall back to base name of `/proc/<pid>/exe`.
pub(crate) fn find_process_by_name(name: &str) -> Result<Option<u32>> {
    use std::fs;

    let target_raw = name.to_ascii_lowercase();
    let target = target_raw.trim_end_matches(".exe");

    let proc_dir = Path::new("/proc");
    for entry in fs::read_dir(proc_dir)? {
        let entry = entry?;
        let fname = entry.file_name();
        let fname_str = fname.to_string_lossy();
        if !fname_str.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        let pid: u32 = match fname_str.parse() {
            Ok(p) => p,
            Err(_) => continue,
        };

        // Try `/proc/<pid>/comm`
        let comm_path = entry.path().join("comm");
        if let Ok(comm) = fs::read_to_string(&comm_path) {
            let comm_trim = comm.trim().to_ascii_lowercase();
            if comm_trim == target {
                return Ok(Some(pid));
            }
        }
        // Fallback: base name of `/proc/<pid>/exe`
        if let Ok(link) = read_link(entry.path().join("exe")) {
            if let Some(base) = link.file_name().and_then(|s| s.to_str()) {
                let base_lc = base.to_ascii_lowercase();
                let base_no_ext = base_lc.trim_end_matches(".exe");
                if base_no_ext == target {
                    return Ok(Some(pid));
                }
            }
        }
    }
    Ok(None)
}

/// Get a list of module regions (rough approximation) by grouping file-backed mappings by pathname,
/// skipping the main executable image.
pub(crate) fn get_process_module_regions(proc: &ProcessHandleUnix) -> Result<Vec<MemoryRegion>> {
    let mut by_path: HashMap<String, (usize, usize, bool)> = HashMap::new(); // path -> (`min_start`, `max_end`, `any_exec`)

    for m in &proc.maps {
        let Some(path) = &m.image_file else { continue };
        // Skip main executable image
        if let Some(exe) = &proc.exe_path {
            if path == exe {
                continue;
            }
        }
        let start = m.base_address;
        let end = m.base_address.saturating_add(m.size);
        let entry = by_path.entry(path.clone()).or_insert((start, end, false));
        entry.0 = entry.0.min(start);
        entry.1 = entry.1.max(end);
        if m.protect.execute {
            entry.2 = true;
        }
    }

    let mut regions = Vec::new();
    for (path, (start, end, any_exec)) in by_path {
        regions.push(MemoryRegion {
            base_address: start,
            size: end.saturating_sub(start),
            protect: MemoryProtection {
                no_access: false,
                read: true,
                write: false,
                execute: any_exec,
                copy_on_write: false,
                guarded: false,
                no_cache: false,
            },
            state: MemoryState {
                committed: true,
                free: false,
                reserved: false,
            },
            type_: MemoryType::Image,
            image_file: Some(path),
        });
    }

    // Sort by base address to match expectations
    regions.sort_by_key(|r| r.base_address);

    Ok(regions)
}

pub(crate) fn query_system_info() -> SystemInfo {
    let page_size = unsafe { sysconf(_SC_PAGESIZE) as usize };

    // Derive `min`/`max` from current process maps (kernel enforces valid addresses anyway)
    // If maps unavailable, fallback `0..=usize::MAX` range (will quickly terminate as iterator finds none)
    let (min_addr, max_addr) = (0usize, usize::MAX);

    SystemInfo {
        min_app_addr: min_addr,
        max_app_addr: max_addr,
        granularity: page_size,
        page_size,
    }
}

pub(crate) fn memory_region_iterator_next(
    proc: &ProcessHandleUnix,
    cur_addr: &mut usize,
) -> Option<MemoryRegion> {
    // Find the first map whose start >= cur_addr
    let idx = match proc.maps.binary_search_by_key(cur_addr, |m| m.base_address) {
        Ok(i) => i,
        Err(i) => i,
    };
    if idx >= proc.maps.len() {
        // Exhausted; bump cur_addr to max to signal termination to caller
        *cur_addr = usize::MAX;
        return None;
    }
    let m = &proc.maps[idx];
    // Advance iterator address regardless of interest
    *cur_addr = m.base_address.saturating_add(m.size);

    // Regions were parsed already into cross-platform representation; still apply filter
    if is_region_interesting(&m.protect, &m.state) {
        Some(MemoryRegion {
            base_address: m.base_address,
            size: m.size,
            protect: m.protect.clone(),
            state: m.state.clone(),
            type_: m.type_.clone(),
            image_file: None,
        })
    } else {
        None
    }
}

/// Read process memory into the provided buffer. Returns the number of bytes read (0 on failure).
pub(crate) fn read_process_memory(proc: &ProcessHandleUnix, addr: usize, buf: &mut [u8]) -> usize {
    proc.read_mem(addr, buf).unwrap_or(0)
}

pub(crate) fn write_process_memory(proc: &ProcessHandleUnix, addr: usize, buf: &[u8]) -> usize {
    proc.write_mem(addr, buf).unwrap_or(0)
}
