// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use crate::{
    accelerator::{Accelerator, KeyAccelerator, MenuAccelerator},
    icon::{Icon, NativeIcon},
    IconMenuItem, MenuId,
};

/// A builder type for [`IconMenuItem`]
#[derive(Clone, Debug, Default)]
pub struct IconMenuItemBuilder {
    text: String,
    enabled: bool,
    id: Option<MenuId>,
    accelerator: Option<MenuAccelerator>,
    icon: Option<Icon>,
    native_icon: Option<NativeIcon>,
}

impl IconMenuItemBuilder {
    pub fn new() -> Self {
        Default::default()
    }

    /// Set the id this icon menu item.
    pub fn id(mut self, id: MenuId) -> Self {
        self.id.replace(id);
        self
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
    pub fn native_icon(mut self, icon: Option<NativeIcon>) -> Self {
        self.native_icon = icon;
        self.icon = None;
        self
    }

    /// Set this icon menu item accelerator.
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

    /// Set this icon menu item accelerator using a [`KeyAccelerator`].
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

    /// Build this icon menu item.
    pub fn build(self) -> IconMenuItem {
        let item = if let Some(id) = self.id {
            if self.icon.is_some() {
                IconMenuItem::with_id(id, self.text, self.enabled, self.icon, None)
            } else {
                IconMenuItem::with_id_and_native_icon(
                    id,
                    self.text,
                    self.enabled,
                    self.native_icon,
                    None,
                )
            }
        } else if self.icon.is_some() {
            IconMenuItem::new(self.text, self.enabled, self.icon, None)
        } else {
            IconMenuItem::with_native_icon(self.text, self.enabled, self.native_icon, None)
        };
        if let Some(accelerator) = self.accelerator {
            let _ = match accelerator {
                MenuAccelerator::Physical(accelerator) => item.set_accelerator(Some(accelerator)),
                MenuAccelerator::Logical(accelerator) => {
                    item.set_key_accelerator(Some(accelerator))
                }
            };
        }
        item
    }
}
