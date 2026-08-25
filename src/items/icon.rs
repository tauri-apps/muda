// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.inner
// SPDX-License-Identifier: MIT

use std::{cell::RefCell, mem, rc::Rc};

use crate::{
    accelerator::{Accelerator, KeyAccelerator, MenuAccelerator},
    icon::{Icon, NativeIcon},
    platform_impl::PlatformMenuItem,
    sealed::IsMenuItemBase,
    util, ClickAction, IconType, IsMenuItem, MenuId, MenuItemKind,
};

/// An icon menu item inside a [`Menu`] or [`Submenu`]
/// and usually contains an icon and a text.
///
/// [`Menu`]: crate::Menu
/// [`Submenu`]: crate::Submenu
#[derive(Clone)]
pub struct IconMenuItem {
    pub(crate) id: Rc<MenuId>,
    pub(crate) state: Rc<RefCell<IconMenuItemState>>,
    pub(crate) platform: Rc<RefCell<PlatformMenuItem>>,
}

/// Shared state of an [`IconMenuItem`].
#[derive(Debug, Clone)]
pub(crate) struct IconMenuItemState {
    pub text: String,
    pub enabled: bool,
    pub icon: Option<IconType>,
    pub accelerator: Option<MenuAccelerator>,
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
        Self::new_inner(
            None,
            text.as_ref(),
            enabled,
            icon.map(IconType::Custom),
            accelerator.map(MenuAccelerator::Physical),
        )
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
        Self::new_inner(
            Some(id.into()),
            text.as_ref(),
            enabled,
            icon.map(IconType::Custom),
            accelerator.map(MenuAccelerator::Physical),
        )
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
        Self::new_inner(
            None,
            text.as_ref(),
            enabled,
            native_icon.map(IconType::Native),
            accelerator.map(MenuAccelerator::Physical),
        )
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
        Self::new_inner(
            Some(id.into()),
            text.as_ref(),
            enabled,
            native_icon.map(IconType::Native),
            accelerator.map(MenuAccelerator::Physical),
        )
    }

    fn new_inner(
        id: Option<MenuId>,
        text: &str,
        enabled: bool,
        icon: Option<IconType>,
        accelerator: Option<MenuAccelerator>,
    ) -> Self {
        let id = util::next_id(id);

        let state = IconMenuItemState {
            text: text.to_string(),
            enabled,
            icon,
            accelerator,
        };

        let click = ClickAction::Emit(id.clone());
        let platform = PlatformMenuItem::new(click, crate::MenuItemType::Icon);

        Self {
            id: Rc::new(id),
            state: Rc::new(RefCell::new(state)),
            platform: Rc::new(RefCell::new(platform)),
        }
    }

    /// Returns a unique identifier associated with this submenu.
    pub fn id(&self) -> &MenuId {
        &self.id
    }

    /// Get the text for this check menu item.
    pub fn text(&self) -> String {
        self.platform
            .borrow()
            .text()
            .unwrap_or_else(|| self.state.borrow().text.clone())
    }

    /// Set the text for this check menu item. `text` could optionally contain
    /// an `&` before a character to assign this character as the mnemonic
    /// for this check menu item. To display a `&` without assigning a mnemenonic, use `&&`.
    pub fn set_text<S: AsRef<str>>(&self, text: S) {
        let accelerator = {
            let mut state = self.state.borrow_mut();
            state.text = text.as_ref().to_string();
            state.accelerator.clone()
        };

        self.platform
            .borrow_mut()
            .set_text(text.as_ref(), accelerator.as_ref())
    }

    /// Get whether this check menu item is enabled or not.
    pub fn is_enabled(&self) -> bool {
        self.platform
            .borrow()
            .is_enabled()
            .unwrap_or_else(|| self.state.borrow().enabled)
    }

    /// Enable or disable this check menu item.
    pub fn set_enabled(&self, enabled: bool) {
        self.state.borrow_mut().enabled = enabled;
        self.platform.borrow_mut().set_enabled(enabled)
    }

    /// Set this icon menu item accelerator.
    ///
    /// (Note that setting an accelerator will override any existing [.set_key_accelerator()](Self::set_key_accelerator))
    pub fn set_accelerator(&self, accelerator: Option<Accelerator>) -> crate::Result<()> {
        self.set_accelerator_inner(accelerator.map(MenuAccelerator::Physical))
    }

    /// Set this icon menu item accelerator using a [`KeyAccelerator`].
    ///
    /// (Note that setting a key_accelerator will override any existing [.set_accelerator()](Self::set_accelerator))
    pub fn set_key_accelerator(&self, accelerator: Option<KeyAccelerator>) -> crate::Result<()> {
        self.set_accelerator_inner(accelerator.map(MenuAccelerator::Logical))
    }

    fn set_accelerator_inner(&self, accelerator: Option<MenuAccelerator>) -> crate::Result<()> {
        let text = {
            let mut state = self.state.borrow_mut();
            state.accelerator = accelerator.clone();
            state.text.clone()
        };

        self.platform
            .borrow_mut()
            .set_accelerator(&text, accelerator.as_ref())
    }

    /// Change this menu item icon or remove it.
    ///
    /// (Note that setting an icon will override any existing [.set_native_icon()](Self::set_native_icon))
    pub fn set_icon(&self, icon: Option<Icon>) {
        self.state.borrow_mut().icon = icon.map(IconType::Custom);
        let state = self.state.borrow();
        self.platform.borrow_mut().set_icon(state.icon.as_ref())
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
    ///
    /// (Note that setting a native icon will override any existing [.set_icon()](Self::set_icon))
    pub fn set_native_icon(&self, icon: Option<NativeIcon>) {
        let icon = icon.map(IconType::Native);
        self.state.borrow_mut().icon = icon;
        let state = self.state.borrow();
        self.platform.borrow_mut().set_icon(state.icon.as_ref())
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
