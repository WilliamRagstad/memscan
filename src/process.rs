#![cfg(windows)]
#![allow(non_snake_case, dead_code)]
use crate::handle::AutoCloseHandle;
use anyhow::Result;
use std::mem::{MaybeUninit, size_of, transmute};
use winapi::{
    shared::{
        basetsd::SIZE_T,
        minwindef::{DWORD, FALSE, HMODULE, LPCVOID, MAX_PATH},
    },
    um::{
        handleapi::CloseHandle,
        memoryapi::VirtualQueryEx,
        processthreadsapi::OpenProcess,
        psapi::{EnumProcessModules, GetModuleFileNameExA, GetModuleInformation, MODULEINFO},
        sysinfoapi::{GetNativeSystemInfo, SYSTEM_INFO},
        tlhelp32::{
            CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW,
            TH32CS_SNAPPROCESS,
        },
        winnt::{
            CHAR, MEM_COMMIT, MEM_FREE, MEM_IMAGE, MEM_MAPPED, MEM_PRIVATE, MEM_RESERVE,
            MEMORY_BASIC_INFORMATION, PAGE_EXECUTE, PAGE_EXECUTE_READ, PAGE_EXECUTE_READWRITE,
            PAGE_EXECUTE_WRITECOPY, PAGE_GUARD, PAGE_NOACCESS, PAGE_READONLY, PAGE_READWRITE,
            PAGE_WRITECOPY, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ,
        },
    },
};

/// For Unicode platforms, TCHAR is defined as synonymous with the WCHAR type.
/// A Win32 character string that can be used to describe ANSI, DBCS, or Unicode strings.
/// See: https://learn.microsoft.com/en-us/office/client-developer/outlook/mapi/tchar
pub type TCHAR = CHAR;

pub fn open_process(pid: u32) -> Result<AutoCloseHandle> {
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, FALSE, pid);
        if handle.is_null() {
            anyhow::bail!("OpenProcess failed for pid {}", pid);
        }
        Ok(AutoCloseHandle(handle))
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

/// Get a list of module base addresses for the given process using EnumProcessModules.
pub fn get_process_module_regions(proc: &AutoCloseHandle) -> Result<Vec<MemoryRegion>> {
    let mut modules = Vec::new();
    let mut hMods: [HMODULE; 1024];
    let mut cbNeeded: DWORD = 0;

    unsafe {
        hMods = [std::ptr::null_mut(); 1024];
        let res = EnumProcessModules(
            proc.raw(),
            hMods.as_mut_ptr(),
            (size_of::<HMODULE>() * hMods.len()) as DWORD,
            &mut cbNeeded as *mut DWORD,
        );
        if res == FALSE {
            anyhow::bail!(
                "EnumProcessModules failed: {}",
                std::io::Error::last_os_error()
            );
        }

        let count = (cbNeeded as usize) / size_of::<HMODULE>();
        for &hMod in &hMods[1..count] {
            //* Skip first module (the main executable)
            //* We only want to get the unrelated DLL modules here.
            let mut modimage: [TCHAR; MAX_PATH] = [0; MAX_PATH];
            let res =
                GetModuleFileNameExA(proc.raw(), hMod, modimage.as_mut_ptr(), MAX_PATH as DWORD);
            if res == 0 {
                anyhow::bail!(
                    "GetModuleFileNameExA failed: {}",
                    std::io::Error::last_os_error()
                );
            }
            let image_file = {
                let len = modimage
                    .iter()
                    .position(|&c| c == 0)
                    .unwrap_or(modimage.len());
                let modimage_u8: [u8; MAX_PATH] = transmute(modimage);
                String::from_utf8_lossy(&modimage_u8[..len]).to_string()
            };
            let mut modinfo = MaybeUninit::<MODULEINFO>::uninit();
            let res = GetModuleInformation(
                proc.raw(),
                hMod,
                modinfo.as_mut_ptr(),
                size_of::<MODULEINFO>() as DWORD,
            );
            if res == FALSE {
                anyhow::bail!(
                    "GetModuleInformation failed: {}",
                    std::io::Error::last_os_error()
                );
            }
            let modinfo = modinfo.assume_init();
            modules.push(MemoryRegion {
                base_address: modinfo.lpBaseOfDll as usize,
                size: modinfo.SizeOfImage as usize,
                protect: PAGE_EXECUTE_READ,
                state: MEM_COMMIT,
                type_: MEM_IMAGE,
                image_file: Some(image_file),
            });
        }
    }
    Ok(modules)
}

#[derive(Debug, Clone)]
pub struct SystemInfo {
    pub min_app_addr: usize,
    pub max_app_addr: usize,
    pub granularity: usize,
    pub page_size: usize,
}

pub fn query_system_info() -> SystemInfo {
    unsafe {
        let mut info = MaybeUninit::<SYSTEM_INFO>::uninit();
        GetNativeSystemInfo(info.as_mut_ptr());
        let info = info.assume_init();
        SystemInfo {
            min_app_addr: info.lpMinimumApplicationAddress as usize,
            max_app_addr: info.lpMaximumApplicationAddress as usize,
            granularity: info.dwAllocationGranularity as usize,
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
    pub image_file: Option<String>,
}

impl MemoryRegion {
    pub fn is_superset_of(&self, other: &MemoryRegion) -> bool {
        self.base_address <= other.base_address
            && self.base_address + self.size >= other.base_address + other.size
    }
}

fn is_region_interesting(mbi: &MEMORY_BASIC_INFORMATION) -> bool {
    if (mbi.State & MEM_COMMIT) != MEM_COMMIT // Not committed
	|| (mbi.State & MEM_FREE) == MEM_FREE // Free or reserved region
	|| (mbi.State & MEM_RESERVE) == MEM_RESERVE
    {
        return false; // Only committed regions
    }

    // Basic access/protection filtering
    let protect = mbi.Protect;
    if protect == PAGE_NOACCESS || (protect & PAGE_GUARD) == PAGE_GUARD {
        return false;
    }

    // Low byte encodes the primary protection flags.
    // Make sure it's something readable.
    matches!(
        protect & 0xFF,
        PAGE_READONLY
            | PAGE_READWRITE
            | PAGE_WRITECOPY
            | PAGE_EXECUTE_READ
            | PAGE_EXECUTE_READWRITE
            | PAGE_EXECUTE_WRITECOPY
    )
}

pub fn protect_to_str(protect: DWORD) -> &'static str {
    match protect {
        PAGE_EXECUTE => "EXECUTE",
        PAGE_NOACCESS => "NOACCESS",
        PAGE_READONLY => "READONLY",
        PAGE_READWRITE => "READWRITE",
        PAGE_WRITECOPY => "WRITECOPY",
        PAGE_EXECUTE_READ => "EXECUTE_READ",
        PAGE_EXECUTE_READWRITE => "EXECUTE_READWRITE",
        PAGE_EXECUTE_WRITECOPY => "EXECUTE_WRITECOPY",
        _ => "UNKNOWN",
    }
}

pub fn state_to_str(state: DWORD) -> &'static str {
    match state {
        MEM_COMMIT => "COMMIT",
        MEM_FREE => "FREE",
        MEM_RESERVE => "RESERVE",
        _ => "UNKNOWN",
    }
}

pub fn type_to_str(type_: DWORD) -> &'static str {
    match type_ {
        MEM_IMAGE => "IMAGE",
        MEM_MAPPED => "MAPPED",
        MEM_PRIVATE => "PRIVATE",
        _ => "UNKNOWN",
    }
}

/// Iterates committed readable memory regions of the process.
pub struct MemoryRegionIterator<'a> {
    proc: &'a AutoCloseHandle,
    cur_addr: usize,
    max_addr: usize,
}

impl<'a> MemoryRegionIterator<'a> {
    pub fn new(proc: &'a AutoCloseHandle, sys: &SystemInfo) -> Self {
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

                // If the function fails, the return value is zero.
                // To get extended error information, call GetLastError.
                // Possible error values include ERROR_INVALID_PARAMETER.
                if res == 0 {
                    // Failed (or reached the end of the address space)
                    return None;
                }

                let mbi = mbi.assume_init();
                let region_base = mbi.BaseAddress as usize;
                let region_size = mbi.RegionSize as usize;

                // Advance iterator *before* possible continue
                self.cur_addr = region_base.saturating_add(region_size);

                if !is_region_interesting(&mbi) {
                    continue;
                }

                return Some(MemoryRegion {
                    base_address: region_base,
                    size: region_size,
                    protect: mbi.Protect,
                    state: mbi.State,
                    type_: mbi.Type,
                    image_file: None,
                });
            }
        }
        None
    }
}
