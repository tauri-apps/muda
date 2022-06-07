#![allow(unused)]

use std::sync::atomic::{AtomicU64, Ordering};

pub struct Counter(AtomicU64);

impl Counter {
    pub const fn new() -> Self {
        Self(AtomicU64::new(1))
    }

    pub const fn new_with_start(start: u64) -> Self {
        Self(AtomicU64::new(start))
    }

    pub fn next(&self) -> u64 {
        self.0.fetch_add(1, Ordering::Relaxed)
    }

    pub fn current(&self) -> u64 {
        self.0.load(Ordering::Relaxed)
    }
}
