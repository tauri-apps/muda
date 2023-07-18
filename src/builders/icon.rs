// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use crate::{
    accelerator::Accelerator,
    icon::{Icon, NativeIcon},
    IconMenuItem,
};

/// A builder type for [`IconMenuItem`]
#[derive(Clone, Debug, Default)]
pub struct IconMenuItemBuilder {
    text: String,
    enabled: bool,
    acccelerator: Option<Accelerator>,
    icon: Option<Icon>,
    native_icon: Option<NativeIcon>,
}

impl IconMenuItemBuilder {
    pub fn new() -> Self {
        Default::default()
    }

    /// Set the text for this icon menu item.
    ///
    /// See [`IconMenuItem::set_text`] for more info.
    pub fn text<S: Into<String>>(mut self, text: S) -> Self {
        self.text = text.into();
        self
    }

    /// Enable or disable this menu item.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Set this icon menu item icon.
    pub fn icon(mut self, icon: Option<Icon>) -> Self {
        self.icon = icon;
        self.native_icon = None;
        self
    }

    /// Set this icon menu item native icon.
    pub fn native_icon(mut self, icon: Option<NativeIcon>) -> Self {
        self.native_icon = icon;
        self.icon = None;
        self
    }

    /// Set this icon menu item accelerator.
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

    /// Build this icon menu item.
    pub fn build(self) -> IconMenuItem {
        if self.icon.is_some() {
            IconMenuItem::new(self.text, self.enabled, self.icon, self.acccelerator)
        } else {
            IconMenuItem::with_native_icon(
                self.text,
                self.enabled,
                self.native_icon,
                self.acccelerator,
            )
        }
    }
}
