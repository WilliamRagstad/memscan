#![allow(non_snake_case, dead_code)]
use anyhow::Result;
use std::mem::{MaybeUninit, size_of};
use winapi::{
    shared::{
        basetsd::SIZE_T,
        minwindef::{DWORD, FALSE, LPCVOID},
    },
    um::{
        handleapi::CloseHandle,
        memoryapi::VirtualQueryEx,
        processthreadsapi::OpenProcess,
        sysinfoapi::{GetSystemInfo, SYSTEM_INFO},
        tlhelp32::{
            CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW,
            TH32CS_SNAPPROCESS,
        },
        winnt::{
            HANDLE, MEM_COMMIT, MEM_FREE, MEM_RESERVE, MEMORY_BASIC_INFORMATION, PAGE_EXECUTE_READ,
            PAGE_EXECUTE_READWRITE, PAGE_EXECUTE_WRITECOPY, PAGE_GUARD, PAGE_NOACCESS,
            PAGE_READONLY, PAGE_READWRITE, PAGE_WRITECOPY, PROCESS_QUERY_INFORMATION,
            PROCESS_VM_READ,
        },
    },
};

pub struct ProcessHandle(HANDLE);

unsafe impl Send for ProcessHandle {}
unsafe impl Sync for ProcessHandle {}

impl ProcessHandle {
    pub fn raw(&self) -> HANDLE {
        self.0
    }
}

impl Drop for ProcessHandle {
    fn drop(&mut self) {
        unsafe {
            if !self.0.is_null() {
                CloseHandle(self.0);
            }
        }
    }
}

pub fn open_process(pid: u32) -> Result<ProcessHandle> {
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, 0, pid);
        if handle.is_null() {
            anyhow::bail!("OpenProcess failed for pid {}", pid);
        }
        Ok(ProcessHandle(handle))
    }
}

/// Find the PID of the first process whose executable name matches `name` (case-insensitive).
///
/// Example names: `"notepad"` or `"notepad.exe"`.
pub fn find_process_by_name(name: &str) -> Result<Option<u32>> {
    let name = name.to_ascii_lowercase();

    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot == winapi::um::handleapi::INVALID_HANDLE_VALUE {
            anyhow::bail!("CreateToolhelp32Snapshot failed");
        }

        let mut entry: PROCESSENTRY32W = std::mem::zeroed();
        entry.dwSize = size_of::<PROCESSENTRY32W>() as u32;

        let mut found_pid: Option<u32> = None;

        if Process32FirstW(snapshot, &mut entry) == FALSE {
            CloseHandle(snapshot);
            return Ok(None);
        }

        loop {
            let exe_name = {
                let len = entry
                    .szExeFile
                    .iter()
                    .position(|&c| c == 0)
                    .unwrap_or(entry.szExeFile.len());
                String::from_utf16_lossy(&entry.szExeFile[..len]).to_ascii_lowercase()
            };

            if exe_name.starts_with(&name) {
                found_pid = Some(entry.th32ProcessID);
                break;
            }

            if Process32NextW(snapshot, &mut entry) == FALSE {
                break;
            }
        }

        CloseHandle(snapshot);
        Ok(found_pid)
    }
}

#[derive(Debug, Clone)]
pub struct SystemInfo {
    pub min_app_addr: usize,
    pub max_app_addr: usize,
    pub page_size: usize,
}

pub fn query_system_info() -> SystemInfo {
    unsafe {
        let mut info = MaybeUninit::<SYSTEM_INFO>::uninit();
        GetSystemInfo(info.as_mut_ptr());
        let info = info.assume_init();
        SystemInfo {
            min_app_addr: info.lpMinimumApplicationAddress as usize,
            max_app_addr: info.lpMaximumApplicationAddress as usize,
            page_size: info.dwPageSize as usize,
        }
    }
}

/// A committed readable region in the target process.
#[derive(Debug, Clone)]
pub struct MemoryRegion {
    pub base_address: usize,
    pub size: usize,
    pub protect: DWORD,
    pub state: DWORD,
    pub type_: DWORD,
}

fn is_region_interesting(mbi: &MEMORY_BASIC_INFORMATION) -> bool {
    // Only committed regions
    if mbi.State != MEM_COMMIT {
        return false;
    }

    // Basic access/protection filtering
    let protect = mbi.Protect;
    if protect == PAGE_NOACCESS || (protect & PAGE_GUARD) == PAGE_GUARD {
        return false;
    }

    // Low byte encodes the primary protection flags.
    // Make sure it's something readable.
    let primary = protect & 0xFF;

    matches!(
        primary,
        PAGE_READONLY
            | PAGE_READWRITE
            | PAGE_WRITECOPY
            | PAGE_EXECUTE_READ
            | PAGE_EXECUTE_READWRITE
            | PAGE_EXECUTE_WRITECOPY
    )
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
        unsafe {
            while self.cur_addr < self.max_addr {
                let mut mbi = MaybeUninit::<MEMORY_BASIC_INFORMATION>::uninit();
                let res = VirtualQueryEx(
                    self.proc.raw(),
                    self.cur_addr as LPCVOID,
                    mbi.as_mut_ptr(),
                    size_of::<MEMORY_BASIC_INFORMATION>() as SIZE_T,
                );

                if res == 0 {
                    // Failed (or reached the end of the address space)
                    return None;
                }

                let mbi = mbi.assume_init();
                let region_base = mbi.BaseAddress as usize;
                let region_size = mbi.RegionSize as usize;

                // Advance iterator *before* possible continue
                self.cur_addr = region_base.saturating_add(region_size);

                if mbi.State == MEM_FREE || mbi.State == MEM_RESERVE {
                    continue;
                }

                if !is_region_interesting(&mbi) {
                    continue;
                }

                return Some(MemoryRegion {
                    base_address: region_base,
                    size: region_size,
                    protect: mbi.Protect,
                    state: mbi.State,
                    type_: mbi.Type,
                });
            }
        }
        None
    }
}
