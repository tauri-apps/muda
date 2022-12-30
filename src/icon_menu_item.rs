// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use crate::{accelerator::Accelerator, icon::Icon, MenuItemExt, MenuItemType};

/// A check menu item inside a [`Menu`] or [`Submenu`]
/// and usually contains a text and a check mark or a similar toggle
/// that corresponds to a checked and unchecked states.
///
/// [`Menu`]: crate::Menu
/// [`Submenu`]: crate::Submenu
#[derive(Clone)]
pub struct IconMenuItem(pub(crate) crate::platform_impl::IconMenuItem);

unsafe impl MenuItemExt for IconMenuItem {
    fn type_(&self) -> MenuItemType {
        MenuItemType::Icon
    }
    fn as_any(&self) -> &(dyn std::any::Any + 'static) {
        self
    }

    fn id(&self) -> u32 {
        self.id()
    }
}

impl IconMenuItem {
    /// Create a new check menu item.
    ///
    /// - `text` could optionally contain an `&` before a character to assign this character as the mnemonic
    /// for this check menu item. To display a `&` without assigning a mnemenonic, use `&&`
    pub fn new<S: AsRef<str>>(
        text: S,
        enabled: bool,
        icon: Option<Icon>,
        acccelerator: Option<Accelerator>,
    ) -> Self {
        Self(crate::platform_impl::IconMenuItem::new(
            text.as_ref(),
            enabled,
            icon,
            acccelerator,
        ))
    }

    /// Returns a unique identifier associated with this submenu.
    pub fn id(&self) -> u32 {
        self.0.id()
    }

    /// Get the text for this check menu item.
    pub fn text(&self) -> String {
        self.0.text()
    }

    /// Get the text for this check menu item. `text` could optionally contain
    /// an `&` before a character to assign this character as the mnemonic
    /// for this check menu item. To display a `&` without assigning a mnemenonic, use `&&`
    pub fn set_text<S: AsRef<str>>(&self, text: S) {
        self.0.set_text(text.as_ref())
    }

    /// Get whether this check menu item is enabled or not.
    pub fn is_enabled(&self) -> bool {
        self.0.is_enabled()
    }

    /// Enable or disable this check menu item.
    pub fn set_enabled(&self, enabled: bool) {
        self.0.set_enabled(enabled)
    }

    /// Change this menu item icon or remove it.
    pub fn set_icon(&self, icon: Option<Icon>) {
        self.0.set_icon(icon)
    }
}
