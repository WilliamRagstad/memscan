use crate::process::{
    MemoryProtection, MemoryRegion, MemoryState, MemoryType, ProcessHandle, SystemInfo,
    is_region_interesting,
};
use anyhow::Result;
use std::mem::{MaybeUninit, size_of, transmute};
use winapi::{
    shared::{
        basetsd::SIZE_T,
        minwindef::{DWORD, FALSE, HMODULE, LPCVOID, LPVOID, MAX_PATH},
    },
    um::{
        handleapi::CloseHandle,
        memoryapi::{ReadProcessMemory, VirtualQueryEx},
        processthreadsapi::OpenProcess,
        psapi::{EnumProcessModules, GetModuleFileNameExA, GetModuleInformation, MODULEINFO},
        sysinfoapi::{GetNativeSystemInfo, SYSTEM_INFO},
        tlhelp32::{
            CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW,
            TH32CS_SNAPPROCESS,
        },
        winnt::{
            CHAR, HANDLE, MEM_COMMIT, MEM_FREE, MEM_IMAGE, MEM_MAPPED, MEM_PRIVATE, MEM_RESERVE,
            MEMORY_BASIC_INFORMATION, PAGE_EXECUTE, PAGE_EXECUTE_READ, PAGE_EXECUTE_READWRITE,
            PAGE_EXECUTE_WRITECOPY, PAGE_GUARD, PAGE_NOACCESS, PAGE_NOCACHE, PAGE_READONLY,
            PAGE_READWRITE, PAGE_WRITECOPY, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ,
            PROCESS_VM_WRITE, PROCESS_VM_OPERATION,
        },
    },
};

// ================== Windows-specific process types ==================

#[derive(Debug)]
pub struct ProcessHandleWin(pub HANDLE);

unsafe impl Send for ProcessHandleWin {}
unsafe impl Sync for ProcessHandleWin {}

impl ProcessHandleWin {
    pub fn raw(&self) -> HANDLE {
        self.0
    }
}

impl Drop for ProcessHandleWin {
    fn drop(&mut self) {
        unsafe {
            if !self.0.is_null() {
                CloseHandle(self.0);
            }
        }
    }
}

/// For Unicode platforms, TCHAR is defined as synonymous with the WCHAR type.
/// A Win32 character string that can be used to describe ANSI, DBCS, or Unicode strings.
/// See: https://learn.microsoft.com/en-us/office/client-developer/outlook/mapi/tchar
pub type TCHAR = CHAR;

impl From<u32> for MemoryProtection {
    fn from(protect: u32) -> Self {
        MemoryProtection {
            no_access: protect & PAGE_NOACCESS != 0,
            read: protect
                & (PAGE_READONLY
                    | PAGE_READWRITE
                    | PAGE_WRITECOPY
                    | PAGE_EXECUTE_READ
                    | PAGE_EXECUTE_READWRITE
                    | PAGE_EXECUTE_WRITECOPY)
                != 0,
            write: protect
                & (PAGE_READWRITE
                    | PAGE_WRITECOPY
                    | PAGE_EXECUTE_READWRITE
                    | PAGE_EXECUTE_WRITECOPY)
                != 0,
            execute: protect
                & (PAGE_EXECUTE
                    | PAGE_EXECUTE_READ
                    | PAGE_EXECUTE_READWRITE
                    | PAGE_EXECUTE_WRITECOPY)
                != 0,
            copy_on_write: protect & (PAGE_WRITECOPY | PAGE_EXECUTE_WRITECOPY) != 0,
            guarded: protect & PAGE_GUARD != 0,
            no_cache: protect & PAGE_NOCACHE != 0,
        }
    }
}

impl From<u32> for MemoryState {
    fn from(state: u32) -> Self {
        MemoryState {
            committed: state & MEM_COMMIT != 0,
            free: state & MEM_FREE != 0,
            reserved: state & MEM_RESERVE != 0,
        }
    }
}

impl From<u32> for MemoryType {
    fn from(type_: u32) -> Self {
        match type_ {
            MEM_IMAGE => MemoryType::Image,
            MEM_MAPPED => MemoryType::Mapped,
            MEM_PRIVATE => MemoryType::Private,
            _ => MemoryType::Unknown,
        }
    }
}

// ================== Windows-specific process functions ==================

pub(crate) fn open_process(pid: u32) -> Result<ProcessHandle> {
    unsafe {
        let handle = OpenProcess(
            PROCESS_QUERY_INFORMATION | PROCESS_VM_READ | PROCESS_VM_WRITE | PROCESS_VM_OPERATION,
            FALSE,
            pid,
        );
        if handle.is_null() {
            anyhow::bail!("OpenProcess failed for pid {}", pid);
        }
        Ok(ProcessHandleWin(handle))
    }
}

/// Find the PID of the first process whose executable name matches `name` (case-insensitive).
///
/// Example names: `"notepad"` or `"notepad.exe"`.
pub(crate) fn find_process_by_name(name: &str) -> Result<Option<u32>> {
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
pub(crate) fn get_process_module_regions(proc: &ProcessHandleWin) -> Result<Vec<MemoryRegion>> {
    let mut modules = Vec::new();
    let mut h_mods: [HMODULE; 1024];
    let mut cb_needed: DWORD = 0;

    unsafe {
        h_mods = [std::ptr::null_mut(); 1024];
        let res = EnumProcessModules(
            proc.raw(),
            h_mods.as_mut_ptr(),
            (size_of::<HMODULE>() * h_mods.len()) as DWORD,
            &mut cb_needed as *mut DWORD,
        );
        if res == FALSE {
            anyhow::bail!(
                "EnumProcessModules failed: {}",
                std::io::Error::last_os_error()
            );
        }

        let count = (cb_needed as usize) / size_of::<HMODULE>();
        for &h_mod in &h_mods[1..count] {
            //* Skip first module (the main executable)
            //* We only want to get the unrelated DLL modules here.
            let mut modimage: [TCHAR; MAX_PATH] = [0; MAX_PATH];
            let res =
                GetModuleFileNameExA(proc.raw(), h_mod, modimage.as_mut_ptr(), MAX_PATH as DWORD);
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
                h_mod,
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
                protect: PAGE_EXECUTE_READ.into(),
                state: MEM_COMMIT.into(),
                type_: MEM_IMAGE.into(),
                image_file: Some(image_file),
            });
        }
    }
    Ok(modules)
}

pub(crate) fn query_system_info() -> SystemInfo {
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

pub(crate) fn memory_region_iterator_next(
    proc: &ProcessHandleWin,
    cur_addr: &mut usize,
) -> Option<MemoryRegion> {
    let mut mbi = MaybeUninit::<MEMORY_BASIC_INFORMATION>::uninit();
    let res = unsafe {
        VirtualQueryEx(
            proc.raw(),
            *cur_addr as LPCVOID,
            mbi.as_mut_ptr(),
            size_of::<MEMORY_BASIC_INFORMATION>() as SIZE_T,
        )
    };

    // If the function fails, the return value is zero.
    // To get extended error information, call GetLastError.
    // Possible error values include ERROR_INVALID_PARAMETER.
    if res == 0 {
        // Failed (or reached the end of the address space)
        return None;
    }

    let mbi = unsafe { mbi.assume_init() };
    let region_base = mbi.BaseAddress as usize;
    let region_size = mbi.RegionSize as usize;

    let prot: MemoryProtection = mbi.Protect.into();
    let state: MemoryState = mbi.State.into();

    // Advance iterator *before* possible continue
    *cur_addr = region_base.saturating_add(region_size);

    if is_region_interesting(&prot, &state) {
        return Some(MemoryRegion {
            base_address: region_base,
            size: region_size,
            protect: prot,
            state: state,
            type_: mbi.Type.into(),
            image_file: None,
        });
    } else {
        return None;
    }
}

/// Read process memory into the provided buffer. Returns the number of bytes read (0 on failure).
pub(crate) fn read_process_memory(proc: &ProcessHandleWin, addr: usize, buf: &mut [u8]) -> usize {
    unsafe {
        let mut bytes_read: SIZE_T = 0;
        let res = ReadProcessMemory(
            proc.raw(),
            addr as LPCVOID,
            buf.as_mut_ptr() as LPVOID,
            buf.len() as SIZE_T,
            &mut bytes_read as *mut SIZE_T,
        );
        if res == 0 { 0 } else { bytes_read as usize }
    }
}

pub(crate) fn write_process_memory(proc: &ProcessHandleWin, addr: usize, buf: &[u8]) -> usize {
    unsafe {
        let mut bytes_written: SIZE_T = 0;
        let res = winapi::um::memoryapi::WriteProcessMemory(
            proc.raw(),
            addr as LPVOID,
            buf.as_ptr() as LPCVOID,
            buf.len() as SIZE_T,
            &mut bytes_written as *mut SIZE_T,
        );
        if res == 0 { 0 } else { bytes_written as usize }
    }
}
