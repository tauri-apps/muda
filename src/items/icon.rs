// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use std::{cell::RefCell, rc::Rc};

use crate::{
    accelerator::Accelerator,
    icon::{Icon, NativeIcon},
    IsMenuItem, MenuItemType,
};

/// An icon menu item inside a [`Menu`] or [`Submenu`]
/// and usually contains an icon and a text.
///
/// [`Menu`]: crate::Menu
/// [`Submenu`]: crate::Submenu
#[derive(Clone)]
pub struct IconMenuItem(pub(crate) Rc<RefCell<crate::platform_impl::MenuChild>>);

unsafe impl IsMenuItem for IconMenuItem {
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
    /// Create a new icon menu item.
    ///
    /// - `text` could optionally contain an `&` before a character to assign this character as the mnemonic
    /// for this icon menu item. To display a `&` without assigning a mnemenonic, use `&&`.
    pub fn new<S: AsRef<str>>(
        text: S,
        enabled: bool,
        icon: Option<Icon>,
        acccelerator: Option<Accelerator>,
    ) -> Self {
        Self(Rc::new(RefCell::new(
            crate::platform_impl::MenuChild::new_icon(text.as_ref(), enabled, icon, acccelerator),
        )))
    }

    /// Returns a unique identifier associated with this submenu.
    pub fn id(&self) -> u32 {
        self.0.borrow().id()
    }

    /// Get the text for this check menu item.
    pub fn text(&self) -> String {
        self.0.borrow().text()
    }

    /// Get the text for this check menu item. `text` could optionally contain
    /// an `&` before a character to assign this character as the mnemonic
    /// for this check menu item. To display a `&` without assigning a mnemenonic, use `&&`.
    pub fn set_text<S: AsRef<str>>(&self, text: S) {
        self.0.borrow_mut().set_text(text.as_ref())
    }

    /// Get whether this check menu item is enabled or not.
    pub fn is_enabled(&self) -> bool {
        self.0.borrow().is_enabled()
    }

    /// Enable or disable this check menu item.
    pub fn set_enabled(&self, enabled: bool) {
        self.0.borrow_mut().set_enabled(enabled)
    }

    /// Set this icon menu item accelerator.
    pub fn set_accelerator(&self, acccelerator: Option<Accelerator>) -> crate::Result<()> {
        self.0.borrow_mut().set_accelerator(acccelerator)
    }

    /// Change this menu item icon or remove it.
    pub fn set_icon(&self, icon: Option<Icon>) {
        self.0.borrow_mut().set_icon(icon)
    }

    /// Change this menu item icon to a native image or remove it.
    ///
    /// ## Platform-specific:
    ///
    /// - **Windows / Linux**: Unsupported.
    pub fn set_native_icon(&mut self, _icon: Option<NativeIcon>) {
        #[cfg(target_os = "macos")]
        self.0.borrow_mut().set_native_icon(_icon)
    }
}
