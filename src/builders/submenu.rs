// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use crate::{IsMenuItem, Submenu};

/// A builder type for [`Submenu`]
#[derive(Clone, Default)]
pub struct SubmenuBuilder<'a> {
    text: String,
    enabled: bool,
    items: Vec<&'a dyn IsMenuItem>,
}

impl std::fmt::Debug for SubmenuBuilder<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SubmenuBuilder")
            .field("text", &self.text)
            .field("enabled", &self.enabled)
            .finish()
    }
}

impl<'a> SubmenuBuilder<'a> {
    pub fn new() -> Self {
        Default::default()
    }

    /// Set the text for this menu item.
    ///
    /// See [`Submenu::set_text`] for more info.
    pub fn text<S: Into<String>>(mut self, text: S) -> Self {
        self.text = text.into();
        self
    }

    /// Enable or disable this menu item.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Add an item to this submenu.
    pub fn item(mut self, item: &'a dyn IsMenuItem) -> Self {
        self.items.push(item);
        self
    }

    /// Add these items to this submenu.
    pub fn items(mut self, items: &[&'a dyn IsMenuItem]) -> Self {
        self.items.extend_from_slice(items);
        self
    }

    /// Build this menu item.
    pub fn build(self) -> crate::Result<Submenu> {
        Submenu::with_items(self.text, self.enabled, &self.items)
    }
}
