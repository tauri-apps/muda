// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.inner
// SPDX-License-Identifier: MIT

use std::{cell::RefCell, mem, rc::Rc};

use crate::{
    accelerator::Accelerator,
    icon::{Icon, NativeIcon},
    IsMenuItem, MenuId, MenuItemKind,
};

/// An icon menu item inside a [`Menu`] or [`Submenu`]
/// and usually contains an icon and a text.
///
/// [`Menu`]: crate::Menu
/// [`Submenu`]: crate::Submenu
#[derive(Clone)]
pub struct IconMenuItem {
    pub(crate) id: Rc<MenuId>,
    pub(crate) inner: Rc<RefCell<crate::platform_impl::MenuChild>>,
}

unsafe impl IsMenuItem for IconMenuItem {
    fn kind(&self) -> MenuItemKind {
        MenuItemKind::Icon(self.clone())
    }

    fn id(&self) -> &MenuId {
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
        let item = crate::platform_impl::MenuChild::new_icon(
            text.as_ref(),
            enabled,
            icon,
            acccelerator,
            None,
        );
        Self {
            id: Rc::new(item.id().clone()),
            inner: Rc::new(RefCell::new(item)),
        }
    }

    /// Create a new icon menu item with the specified id.
    ///
    /// - `text` could optionally contain an `&` before a character to assign this character as the mnemonic
    /// for this icon menu item. To display a `&` without assigning a mnemenonic, use `&&`.
    pub fn with_id<I: Into<MenuId>, S: AsRef<str>>(
        id: I,
        text: S,
        enabled: bool,
        icon: Option<Icon>,
        acccelerator: Option<Accelerator>,
    ) -> Self {
        let id = id.into();
        Self {
            id: Rc::new(id.clone()),
            inner: Rc::new(RefCell::new(crate::platform_impl::MenuChild::new_icon(
                text.as_ref(),
                enabled,
                icon,
                acccelerator,
                Some(id),
            ))),
        }
    }

    /// Create a new icon menu item but with a native icon.
    ///
    /// See [`IconMenuItem::new`] for more info.
    ///
    /// ## Platform-specific:
    ///
    /// - **Windows / Linux**: Unsupported.
    pub fn with_native_icon<S: AsRef<str>>(
        text: S,
        enabled: bool,
        native_icon: Option<NativeIcon>,
        acccelerator: Option<Accelerator>,
    ) -> Self {
        let item = crate::platform_impl::MenuChild::new_native_icon(
            text.as_ref(),
            enabled,
            native_icon,
            acccelerator,
            None,
        );
        Self {
            id: Rc::new(item.id().clone()),
            inner: Rc::new(RefCell::new(item)),
        }
    }

    /// Create a new icon menu item but with the specified id and a native icon.
    ///
    /// See [`IconMenuItem::new`] for more info.
    ///
    /// ## Platform-specific:
    ///
    /// - **Windows / Linux**: Unsupported.
    pub fn with_id_and_native_icon<I: Into<MenuId>, S: AsRef<str>>(
        id: I,
        text: S,
        enabled: bool,
        native_icon: Option<NativeIcon>,
        acccelerator: Option<Accelerator>,
    ) -> Self {
        let id = id.into();
        Self {
            id: Rc::new(id.clone()),
            inner: Rc::new(RefCell::new(
                crate::platform_impl::MenuChild::new_native_icon(
                    text.as_ref(),
                    enabled,
                    native_icon,
                    acccelerator,
                    Some(id),
                ),
            )),
        }
    }

    /// Returns a unique identifier associated with this submenu.
    pub fn id(&self) -> &MenuId {
        &self.id
    }

    /// Get the text for this check menu item.
    pub fn text(&self) -> String {
        self.inner.borrow().text()
    }

    /// Set the text for this check menu item. `text` could optionally contain
    /// an `&` before a character to assign this character as the mnemonic
    /// for this check menu item. To display a `&` without assigning a mnemenonic, use `&&`.
    pub fn set_text<S: AsRef<str>>(&self, text: S) {
        self.inner.borrow_mut().set_text(text.as_ref())
    }

    /// Get whether this check menu item is enabled or not.
    pub fn is_enabled(&self) -> bool {
        self.inner.borrow().is_enabled()
    }

    /// Enable or disable this check menu item.
    pub fn set_enabled(&self, enabled: bool) {
        self.inner.borrow_mut().set_enabled(enabled)
    }

    /// Set this icon menu item accelerator.
    pub fn set_accelerator(&self, acccelerator: Option<Accelerator>) -> crate::Result<()> {
        self.inner.borrow_mut().set_accelerator(acccelerator)
    }

    /// Change this menu item icon or remove it.
    pub fn set_icon(&self, icon: Option<Icon>) {
        self.inner.borrow_mut().set_icon(icon)
    }

    /// Change this menu item icon to a native image or remove it.
    ///
    /// ## Platform-specific:
    ///
    /// - **Windows / Linux**: Unsupported.
    pub fn set_native_icon(&self, _icon: Option<NativeIcon>) {
        #[cfg(target_os = "macos")]
        self.inner.borrow_mut().set_native_icon(_icon)
    }

    /// Convert this menu item into its menu ID.
    pub fn into_id(mut self) -> MenuId {
        // Note: `Rc::into_inner` is available from Rust 1.70
        if let Some(id) = Rc::get_mut(&mut self.id) {
            mem::take(id)
        } else {
            self.id().clone()
        }
    }
}
