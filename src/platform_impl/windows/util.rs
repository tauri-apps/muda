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
pub fn decode_wide(w_str: *mut u16) -> String {
    let len = unsafe { windows_sys::Win32::Globalization::lstrlenW(w_str) } as usize;
    let w_str_slice = unsafe { std::slice::from_raw_parts(w_str, len) };
    String::from_utf16_lossy(w_str_slice)
}
