use winapi::um::{handleapi::CloseHandle, winnt::HANDLE};

pub struct AutoCloseHandle(pub HANDLE);

unsafe impl Send for AutoCloseHandle {}
unsafe impl Sync for AutoCloseHandle {}

impl AutoCloseHandle {
    pub fn raw(&self) -> HANDLE {
        self.0
    }
}

impl Drop for AutoCloseHandle {
    fn drop(&mut self) {
        unsafe {
            if !self.0.is_null() {
                CloseHandle(self.0);
            }
        }
    }
}
