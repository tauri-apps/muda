use std::sync::atomic::{AtomicU64, Ordering};

pub struct Counter(AtomicU64);

impl Counter {
    pub const fn new() -> Self {
        Self(AtomicU64::new(1))
    }

    pub fn next(&self) -> u64 {
        self.0.fetch_add(1, Ordering::Relaxed)
    }

    #[allow(unused)]
    pub fn current(&self) -> u64 {
        self.0.load(Ordering::Relaxed)
    }
}

#[cfg(target_os = "windows")]
pub fn encode_wide(string: impl AsRef<std::ffi::OsStr>) -> Vec<u16> {
    std::os::windows::prelude::OsStrExt::encode_wide(string.as_ref())
        .chain(std::iter::once(0))
        .collect()
}

#[cfg(target_os = "windows")]
#[allow(non_snake_case)]
pub fn LOWORD(dword: u32) -> u16 {
    (dword & 0xFFFF) as u16
}

#[cfg(target_os = "windows")]
pub fn wchar_ptr_to_string(wchar: windows_sys::core::PWSTR) -> String {
    let len = unsafe { windows_sys::Win32::Globalization::lstrlenW(wchar) } as usize;
    let wchar_slice = unsafe { std::slice::from_raw_parts(wchar, len) };
    String::from_utf16_lossy(wchar_slice)
}
