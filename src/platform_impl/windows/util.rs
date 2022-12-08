// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use std::ops::{Deref, DerefMut};

use windows_sys::Win32::UI::WindowsAndMessaging::ACCEL;

pub fn encode_wide<S: AsRef<std::ffi::OsStr>>(string: S) -> Vec<u16> {
    std::os::windows::prelude::OsStrExt::encode_wide(string.as_ref())
        .chain(std::iter::once(0))
        .collect()
}

#[allow(non_snake_case)]
pub fn LOWORD(dword: u32) -> u16 {
    (dword & 0xFFFF) as u16
}

pub fn decode_wide(w_str: *mut u16) -> String {
    let len = unsafe { windows_sys::Win32::Globalization::lstrlenW(w_str) } as usize;
    let w_str_slice = unsafe { std::slice::from_raw_parts(w_str, len) };
    String::from_utf16_lossy(w_str_slice)
}

/// ACCEL wrapper to implement Debug
#[derive(Clone)]
#[repr(transparent)]
pub struct Accel(pub ACCEL);

impl std::fmt::Debug for Accel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ACCEL")
            .field("key", &self.0.key)
            .field("cmd", &self.0.cmd)
            .field("fVirt", &self.0.fVirt)
            .finish()
    }
}

impl Deref for Accel {
    type Target = ACCEL;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Accel {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
