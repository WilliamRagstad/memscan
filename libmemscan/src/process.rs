use anyhow::Result;
use std::fmt::{self, Display, Formatter};

#[cfg(unix)]
use crate::linux;
#[cfg(windows)]
use crate::windows;

// ================== Cross-platform aliases ==================

#[cfg(windows)]
pub type ProcessHandle = windows::process::ProcessHandleWin;
#[cfg(unix)]
pub type ProcessHandle = linux::process::ProcessHandleUnix;

/// Cross-platform function to get the next process module region.
fn memory_region_iterator_next(proc: &ProcessHandle, cur_addr: &mut usize) -> Option<MemoryRegion> {
    #[cfg(windows)]
    return windows::process::memory_region_iterator_next(proc, cur_addr);
    #[cfg(unix)]
    return linux::process::memory_region_iterator_next(proc, cur_addr);
}

/// Cross-platform function to open a process by its PID.
pub fn open_process(pid: u32) -> Result<ProcessHandle> {
    #[cfg(windows)]
    return windows::process::open_process(pid);
    #[cfg(unix)]
    return linux::process::open_process(pid);
}

/// Cross-platform function to find a process by its name.
pub fn find_process_by_name(name: &str) -> Result<Option<u32>> {
    #[cfg(windows)]
    return windows::process::find_process_by_name(name);
    #[cfg(unix)]
    return linux::process::find_process_by_name(name);
}

/// Cross-platform function to get the list of module regions of a process.
pub fn get_process_module_regions(proc: &ProcessHandle) -> Result<Vec<MemoryRegion>> {
    #[cfg(windows)]
    return windows::process::get_process_module_regions(proc);
    #[cfg(unix)]
    return linux::process::get_process_module_regions(proc);
}

/// Cross-platform function to get system information about the target process environment.
pub fn query_system_info() -> SystemInfo {
    #[cfg(windows)]
    return windows::process::query_system_info();
    #[cfg(unix)]
    return linux::process::query_system_info();
}

// ================= Cross-platform structures ==================

/// Cross-platform system information about the target process environment.
#[derive(Debug, Clone)]
pub struct SystemInfo {
    pub min_app_addr: usize,
    pub max_app_addr: usize,
    pub granularity: usize,
    pub page_size: usize,
}

/// Cross-platform memory protection flags.
/// Agnostic representation of:
/// - Windows PAGE_* constants, see https://learn.microsoft.com/en-us/windows/win32/Memory/memory-protection-constants
/// - Linux PROT_* constants, see https://man7.org/linux/man-pages/man2/mprotect.2.html
#[derive(Debug, Clone)]
pub struct MemoryProtection {
    /// E.g. `PAGE_TARGETS_INVALID`, `PAGE_ENCLAVE_DECOMMIT`, `PAGE_ENCLAVE_UNVALIDATED`, etc.
    pub no_access: bool,
    pub read: bool,
    pub write: bool,
    pub execute: bool,
    // Extra flags
    pub copy_on_write: bool,
    pub guarded: bool,
    pub no_cache: bool,
}

impl Display for MemoryProtection {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut flags = Vec::new();
        if self.no_access {
            flags.push("NOACCESS");
        }
        if self.no_cache {
            flags.push("NOCACHE");
        }
        if self.read {
            flags.push("READ");
        }
        if self.write && !self.copy_on_write {
            flags.push("WRITE");
        }
        if self.write && self.copy_on_write {
            flags.push("WRITECOPY");
        }
        if self.execute {
            flags.push("EXECUTE");
        }
        if self.guarded {
            flags.push("GUARDED");
        }
        write!(f, "{}", flags.join("_"))
    }
}

/// Cross-platform memory state flags.
/// Agnostic representation of:
/// - Windows MEM_* constants, see https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-memory_basic_information
/// - Linux `mmap` flags, see https://man7.org/linux/man-pages/man2/mmap.2.html
#[derive(Debug, Clone)]
pub struct MemoryState {
    pub committed: bool,
    /// E.g. `MEM_FREE`
    pub free: bool,
    /// E.g. `MAP_ANONYMOUS`
    pub reserved: bool,
}

impl Display for MemoryState {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut states = Vec::new();
        if self.committed {
            states.push("COMMITTED");
        }
        if self.free {
            states.push("FREE");
        }
        if self.reserved {
            states.push("RESERVED");
        }
        write!(f, "{}", states.join("|"))
    }
}

/// Cross-platform memory type flags.
/// Agnostic representation of:
/// - Windows MEM_* constants, see https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-memory_basic_information
/// - Linux `mmap` flags, see https://man7.org/linux/man-pages/man2/mmap.2.html
#[derive(Debug, Clone)]
pub enum MemoryType {
    Unknown = 0b0,
    Private = 0b1,
    Mapped = 0b10,
    Image = 0b100,
}

impl Display for MemoryType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let type_str = match self {
            MemoryType::Private => "PRIVATE",
            MemoryType::Mapped => "MAPPED",
            MemoryType::Image => "IMAGE",
            MemoryType::Unknown => "UNKNOWN",
        };
        write!(f, "{}", type_str)
    }
}

/// Cross-platform memory region representation in the target process.
#[derive(Debug, Clone)]
pub struct MemoryRegion {
    pub base_address: usize,
    pub size: usize,
    pub protect: MemoryProtection,
    pub state: MemoryState,
    pub type_: MemoryType,
    pub image_file: Option<String>,
}

impl MemoryRegion {
    pub fn is_superset_of(&self, other: &MemoryRegion) -> bool {
        self.base_address <= other.base_address
            && self.base_address + self.size >= other.base_address + other.size
    }
}

/// Iterates committed readable memory regions of the process.
pub struct MemoryRegionIterator<'a> {
    proc: &'a ProcessHandle,
    cur_addr: usize,
    max_addr: usize,
}

impl<'a> MemoryRegionIterator<'a> {
    pub fn new(proc: &'a ProcessHandle, sys: &SystemInfo) -> Self {
        Self {
            proc,
            cur_addr: sys.min_app_addr,
            max_addr: sys.max_app_addr,
        }
    }
}

impl<'a> Iterator for MemoryRegionIterator<'a> {
    type Item = MemoryRegion;

    fn next(&mut self) -> Option<Self::Item> {
        while self.cur_addr < self.max_addr {
            if let Some(region) = memory_region_iterator_next(self.proc, &mut self.cur_addr) {
                return Some(region);
            } else {
                continue;
            }
        }
        None
    }
}

pub fn is_region_interesting(prot: &MemoryProtection, state: &MemoryState) -> bool {
    if !state.committed || state.free || state.reserved || prot.no_access || prot.guarded {
        false // Only committed regions
    } else {
        true
    }
}
