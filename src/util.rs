// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use std::sync::atomic::{AtomicU32, Ordering};

use crate::MenuId;

#[derive(Clone, Copy, Debug)]
pub enum AddOp {
    Append,
    Insert(usize),
}

/// Ids for callers who did not supply one.
static ID_COUNTER: Counter = Counter::new_with_start(0);

/// `id`, or the next generated one.
pub fn next_id(id: Option<MenuId>) -> MenuId {
    id.unwrap_or_else(|| MenuId(ID_COUNTER.next().to_string()))
}

pub struct Counter(AtomicU32);

impl Counter {
    #[allow(unused)]
    pub const fn new() -> Self {
        Self(AtomicU32::new(1))
    }

    #[allow(unused)]
    pub const fn new_with_start(start: u32) -> Self {
        Self(AtomicU32::new(start))
    }

    pub fn next(&self) -> u32 {
        self.0.fetch_add(1, Ordering::Relaxed)
    }
}
