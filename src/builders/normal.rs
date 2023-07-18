// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use crate::{accelerator::Accelerator, MenuItem};

/// A builder type for [`MenuItem`]
#[derive(Clone, Debug, Default)]
pub struct MenuItemBuilder {
    text: String,
    enabled: bool,
    acccelerator: Option<Accelerator>,
}

impl MenuItemBuilder {
    pub fn new() -> Self {
        Default::default()
    }

    /// Set the text for this menu item.
    ///
    /// See [`MenuItem::set_text`] for more info.
    pub fn text<S: Into<String>>(mut self, text: S) -> Self {
        self.text = text.into();
        self
    }

    /// Enable or disable this menu item.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Set this menu item accelerator.
    pub fn acccelerator<A: TryInto<Accelerator>>(
        mut self,
        acccelerator: Option<A>,
    ) -> crate::Result<Self>
    where
        crate::Error: From<<A as TryInto<Accelerator>>::Error>,
    {
        self.acccelerator = acccelerator.map(|a| a.try_into()).transpose()?;
        Ok(self)
    }

    /// Build this menu item.
    pub fn build(self) -> MenuItem {
        MenuItem::new(self.text, self.enabled, self.acccelerator)
    }
}
