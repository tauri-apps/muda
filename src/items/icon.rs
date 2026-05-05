// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.inner
// SPDX-License-Identifier: MIT

use std::{cell::RefCell, mem, rc::Rc};

use crate::{
    accelerator::{Accelerator, KeyAccelerator, MenuAccelerator},
    icon::{Icon, NativeIcon},
    sealed::IsMenuItemBase,
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

impl IsMenuItemBase for IconMenuItem {}
impl IsMenuItem for IconMenuItem {
    fn kind(&self) -> MenuItemKind {
        MenuItemKind::Icon(self.clone())
    }

    fn id(&self) -> &MenuId {
        self.id()
    }

    fn into_id(self) -> MenuId {
        self.into_id()
    }
}

impl IconMenuItem {
    /// Create a new icon menu item.
    ///
    /// - `text` could optionally contain an `&` before a character to assign this character as the mnemonic
    ///   for this icon menu item. To display a `&` without assigning a mnemenonic, use `&&`.
    pub fn new<S: AsRef<str>>(
        text: S,
        enabled: bool,
        icon: Option<Icon>,
        accelerator: Option<Accelerator>,
    ) -> Self {
        let item = crate::platform_impl::MenuChild::new_icon(
            text.as_ref(),
            enabled,
            icon,
            accelerator.map(MenuAccelerator::Physical),
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
    ///   for this icon menu item. To display a `&` without assigning a mnemenonic, use `&&`.
    pub fn with_id<I: Into<MenuId>, S: AsRef<str>>(
        id: I,
        text: S,
        enabled: bool,
        icon: Option<Icon>,
        accelerator: Option<Accelerator>,
    ) -> Self {
        let id = id.into();
        Self {
            id: Rc::new(id.clone()),
            inner: Rc::new(RefCell::new(crate::platform_impl::MenuChild::new_icon(
                text.as_ref(),
                enabled,
                icon,
                accelerator.map(MenuAccelerator::Physical),
                Some(id),
            ))),
        }
    }

    /// Create a new icon menu item but with a native icon.
    ///
    /// See [`IconMenuItem::new`] for more info.
    ///
    /// ## Platform-specific
    ///
    /// - **macOS**: Known variants map to AppKit image names. Use [`NativeIcon::Raw`] or
    ///   `NativeIcon::from_name` to pass an AppKit [`NSImage.Name`] string.
    /// - **Windows**: Known variants map to stock shell icons where an equivalent exists. Use
    ///   [`NativeIcon::Raw`] or `NativeIcon::from_id` to pass a raw [`SHSTOCKICONID`] value.
    /// - **GTK 3 / GTK 4**: Known variants map to freedesktop-style icon theme names. Use
    ///   [`NativeIcon::Raw`] or `NativeIcon::from_name` to pass an icon theme name resolved by
    ///   `GtkIconTheme` ([GTK 3][gtk3-icon-theme], [GTK 4][gtk4-icon-theme]).
    ///
    /// [`NSImage.Name`]: https://developer.apple.com/documentation/appkit/nsimage/name-swift.typealias
    /// [`SHSTOCKICONID`]: https://learn.microsoft.com/en-us/windows/win32/api/shellapi/ne-shellapi-shstockiconid
    /// [`SHGetStockIconInfo`]: https://learn.microsoft.com/en-us/windows/win32/api/shellapi/nf-shellapi-shgetstockiconinfo
    /// [gtk3-icon-theme]: https://docs.gtk.org/gtk3/class.IconTheme.html
    /// [gtk4-icon-theme]: https://docs.gtk.org/gtk4/class.IconTheme.html
    /// [Icon Naming Specification]: https://specifications.freedesktop.org/icon-naming-spec/latest/
    pub fn with_native_icon<S: AsRef<str>>(
        text: S,
        enabled: bool,
        native_icon: Option<NativeIcon>,
        accelerator: Option<Accelerator>,
    ) -> Self {
        let item = crate::platform_impl::MenuChild::new_native_icon(
            text.as_ref(),
            enabled,
            native_icon,
            accelerator.map(MenuAccelerator::Physical),
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
    /// ## Platform-specific
    ///
    /// - **macOS**: Known variants map to AppKit image names. Use [`NativeIcon::Raw`] or
    ///   `NativeIcon::from_name` to pass an AppKit [`NSImage.Name`] string.
    /// - **Windows**: Known variants map to stock shell icons where an equivalent exists. Use
    ///   [`NativeIcon::Raw`] or `NativeIcon::from_id` to pass a raw [`SHSTOCKICONID`] value.
    /// - **GTK 3 / GTK 4**: Known variants map to freedesktop-style icon theme names. Use
    ///   [`NativeIcon::Raw`] or `NativeIcon::from_name` to pass an icon theme name resolved by
    ///   `GtkIconTheme` ([GTK 3][gtk3-icon-theme], [GTK 4][gtk4-icon-theme]).
    ///
    /// [`NSImage.Name`]: https://developer.apple.com/documentation/appkit/nsimage/name-swift.typealias
    /// [`SHSTOCKICONID`]: https://learn.microsoft.com/en-us/windows/win32/api/shellapi/ne-shellapi-shstockiconid
    /// [`SHGetStockIconInfo`]: https://learn.microsoft.com/en-us/windows/win32/api/shellapi/nf-shellapi-shgetstockiconinfo
    /// [gtk3-icon-theme]: https://docs.gtk.org/gtk3/class.IconTheme.html
    /// [gtk4-icon-theme]: https://docs.gtk.org/gtk4/class.IconTheme.html
    /// [Icon Naming Specification]: https://specifications.freedesktop.org/icon-naming-spec/latest/
    pub fn with_id_and_native_icon<I: Into<MenuId>, S: AsRef<str>>(
        id: I,
        text: S,
        enabled: bool,
        native_icon: Option<NativeIcon>,
        accelerator: Option<Accelerator>,
    ) -> Self {
        let id = id.into();
        Self {
            id: Rc::new(id.clone()),
            inner: Rc::new(RefCell::new(
                crate::platform_impl::MenuChild::new_native_icon(
                    text.as_ref(),
                    enabled,
                    native_icon,
                    accelerator.map(MenuAccelerator::Physical),
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

    /// Set the item's label as a Finder-style two-part string: `primary` rendered in the
    /// default menu label color, optionally followed by `secondary` rendered in
    /// `NSColor.secondaryLabelColor`. Both parts use the standard menu font.
    /// Pass `None` for `secondary` to clear back to plain `setTitle:` styling.
    ///
    /// Useful for labels like `Preview (default)`, `Speakers (current)`, or
    /// `Folder (3 items selected)`.
    ///
    /// ## Platform-specific:
    ///
    /// - **Windows / Linux**: Concatenates `primary` and `secondary` and falls back to
    ///   [`IconMenuItem::set_text`]; the secondary part is not visually distinguished.
    pub fn set_text_with_secondary(&self, primary: &str, secondary: Option<&str>) {
        #[cfg(target_os = "macos")]
        {
            self.inner
                .borrow_mut()
                .set_text_with_secondary(primary, secondary);
        }
        #[cfg(not(target_os = "macos"))]
        {
            let combined = match secondary {
                Some(sec) => format!("{primary}{sec}"),
                None => primary.to_string(),
            };
            self.inner.borrow_mut().set_text(&combined);
        }
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
    ///
    /// (Note that setting an accelerator will override any existing [.set_key_accelerator()](Self::set_key_accelerator))
    pub fn set_accelerator(&self, accelerator: Option<Accelerator>) -> crate::Result<()> {
        self.inner
            .borrow_mut()
            .set_accelerator(accelerator.map(MenuAccelerator::Physical))
    }

    /// Set this icon menu item accelerator using a [`KeyAccelerator`].
    ///
    /// (Note that setting a key_accelerator will override any existing [.set_accelerator()](Self::set_accelerator))
    pub fn set_key_accelerator(&self, accelerator: Option<KeyAccelerator>) -> crate::Result<()> {
        self.inner
            .borrow_mut()
            .set_accelerator(accelerator.map(MenuAccelerator::Logical))
    }

    /// Change this menu item icon or remove it.
    pub fn set_icon(&self, icon: Option<Icon>) {
        self.inner.borrow_mut().set_icon(icon)
    }

    /// Change this menu item icon to a native image or remove it.
    ///
    /// ## Platform-specific
    ///
    /// - **macOS**: Known variants map to AppKit image names. Use [`NativeIcon::Raw`] or
    ///   `NativeIcon::from_name` to pass an AppKit [`NSImage.Name`] string.
    /// - **Windows**: Known variants map to stock shell icons where an equivalent exists. Use
    ///   [`NativeIcon::Raw`] or `NativeIcon::from_id` to pass a raw [`SHSTOCKICONID`] value.
    /// - **GTK 3 / GTK 4**: Known variants map to freedesktop-style icon theme names. Use
    ///   [`NativeIcon::Raw`] or `NativeIcon::from_name` to pass an icon theme name resolved by
    ///   `GtkIconTheme` ([GTK 3][gtk3-icon-theme], [GTK 4][gtk4-icon-theme]).
    ///
    /// [`NSImage.Name`]: https://developer.apple.com/documentation/appkit/nsimage/name-swift.typealias
    /// [`SHSTOCKICONID`]: https://learn.microsoft.com/en-us/windows/win32/api/shellapi/ne-shellapi-shstockiconid
    /// [`SHGetStockIconInfo`]: https://learn.microsoft.com/en-us/windows/win32/api/shellapi/nf-shellapi-shgetstockiconinfo
    /// [gtk3-icon-theme]: https://docs.gtk.org/gtk3/class.IconTheme.html
    /// [gtk4-icon-theme]: https://docs.gtk.org/gtk4/class.IconTheme.html
    /// [Icon Naming Specification]: https://specifications.freedesktop.org/icon-naming-spec/latest/
    pub fn set_native_icon(&self, icon: Option<NativeIcon>) {
        self.inner.borrow_mut().set_native_icon(icon)
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
