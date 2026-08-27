// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use crate::{
    accelerator::{Accelerator, KeyAccelerator, MenuAccelerator},
    CheckMenuItem, MenuId, TextStyle,
};

/// A builder type for [`CheckMenuItem`]
#[derive(Clone, Debug, Default)]
pub struct CheckMenuItemBuilder {
    text: String,
    enabled: bool,
    checked: bool,
    accelerator: Option<MenuAccelerator>,
    id: Option<MenuId>,
    styled_text: Option<Vec<(String, TextStyle)>>,
}

impl CheckMenuItemBuilder {
    pub fn new() -> Self {
        Default::default()
    }

    /// Set the id this check menu item.
    pub fn id(mut self, id: MenuId) -> Self {
        self.id.replace(id);
        self
    }

    /// Set the text for this check menu item.
    ///
    /// See [`CheckMenuItem::set_text`] for more info.
    pub fn text<S: Into<String>>(mut self, text: S) -> Self {
        self.text = text.into();
        self
    }

    /// Enable or disable this menu item.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Check or uncheck this menu item.
    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    /// Set this check menu item accelerator.
    ///
    /// (Note that setting an accelerator will override any existing [.key_accelerator()](Self::key_accelerator))
    pub fn accelerator<A: TryInto<Accelerator>>(
        mut self,
        accelerator: Option<A>,
    ) -> crate::Result<Self>
    where
        crate::Error: From<<A as TryInto<Accelerator>>::Error>,
    {
        self.accelerator = accelerator
            .map(|a| a.try_into().map(MenuAccelerator::Physical))
            .transpose()?;
        Ok(self)
    }

    /// Set this check menu item accelerator using a [`KeyAccelerator`].
    ///
    /// (Note that setting a key_accelerator will override any existing [.accelerator()](Self::accelerator))
    pub fn key_accelerator<A: TryInto<KeyAccelerator>>(
        mut self,
        accelerator: Option<A>,
    ) -> crate::Result<Self>
    where
        crate::Error: From<<A as TryInto<KeyAccelerator>>::Error>,
    {
        self.accelerator = accelerator
            .map(|a| a.try_into().map(MenuAccelerator::Logical))
            .transpose()?;
        Ok(self)
    }

    /// Build this check menu item.
    /// Set the text for this menu item as a sequence of styled text, so one part of the
    /// label can be de-emphasized relative to the rest.
    ///
    /// Overrides any text set with [`.text()`](Self::text).
    ///
    /// See [`CheckMenuItem::set_styled_text`] for more info.
    pub fn styled_text<S: AsRef<str>>(
        mut self,
        parts: impl IntoIterator<Item = (S, TextStyle)>,
    ) -> Self {
        self.styled_text = Some(
            parts
                .into_iter()
                .map(|(text, style)| (text.as_ref().to_string(), style))
                .collect(),
        );
        self
    }

    pub fn build(self) -> CheckMenuItem {
        let item = if let Some(id) = self.id {
            CheckMenuItem::with_id(id, self.text, self.enabled, self.checked, None)
        } else {
            CheckMenuItem::new(self.text, self.enabled, self.checked, None)
        };
        if let Some(accelerator) = self.accelerator {
            let _ = match accelerator {
                MenuAccelerator::Physical(accelerator) => item.set_accelerator(Some(accelerator)),
                MenuAccelerator::Logical(accelerator) => {
                    item.set_key_accelerator(Some(accelerator))
                }
            };
        }
        if let Some(parts) = self.styled_text {
            item.set_styled_text(parts);
        }
        item
    }
}
