use std::sync::atomic::{AtomicU64, Ordering};

pub(crate) struct Counter(AtomicU64);

impl Counter {
    pub(crate) const fn new() -> Self {
        Self(AtomicU64::new(1))
    }

    pub(crate) fn next(&self) -> u64 {
        self.0.fetch_add(1, Ordering::Release)
    }
}

#[cfg(target_os = "windows")]
pub fn encode_wide(string: impl AsRef<std::ffi::OsStr>) -> Vec<u16> {
    std::os::windows::prelude::OsStrExt::encode_wide(string.as_ref())
        .chain(std::iter::once(0))
        .collect()
}
