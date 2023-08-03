// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use std::{cell::RefCell, rc::Rc};

use crate::{util::AddOp, ContextMenu, IsMenuItem, MenuItemKind, Position};

/// A menu that can be added to a [`Menu`] or another [`Submenu`].
///
/// [`Menu`]: crate::Menu
#[derive(Clone)]
pub struct Submenu(pub(crate) Rc<RefCell<crate::platform_impl::MenuChild>>);

unsafe impl IsMenuItem for Submenu {
    fn kind(&self) -> MenuItemKind {
        MenuItemKind::Submenu(self.clone())
    }
}

impl Submenu {
    /// Create a new submenu.
    ///
    /// - `text` could optionally contain an `&` before a character to assign this character as the mnemonic
    /// for this submenu. To display a `&` without assigning a mnemenonic, use `&&`.
    pub fn new<S: AsRef<str>>(text: S, enabled: bool) -> Self {
        Self(Rc::new(RefCell::new(
            crate::platform_impl::MenuChild::new_submenu(text.as_ref(), enabled),
        )))
    }

    /// Creates a new submenu with given `items`. It calls [`Submenu::new`] and [`Submenu::append_items`] internally.
    pub fn with_items<S: AsRef<str>>(
        text: S,
        enabled: bool,
        items: &[&dyn IsMenuItem],
    ) -> crate::Result<Self> {
        let menu = Self::new(text, enabled);
        menu.append_items(items)?;
        Ok(menu)
    }

    /// Returns a unique identifier associated with this submenu.
    pub fn id(&self) -> u32 {
        self.0.borrow().id()
    }

    /// Add a menu item to the end of this menu.
    pub fn append(&self, item: &dyn IsMenuItem) -> crate::Result<()> {
        self.0.borrow_mut().add_menu_item(item, AddOp::Append)
    }

    /// Add menu items to the end of this submenu. It calls [`Submenu::append`] in a loop.
    pub fn append_items(&self, items: &[&dyn IsMenuItem]) -> crate::Result<()> {
        for item in items {
            self.append(*item)?
        }

        Ok(())
    }

    /// Add a menu item to the beginning of this submenu.
    pub fn prepend(&self, item: &dyn IsMenuItem) -> crate::Result<()> {
        self.0.borrow_mut().add_menu_item(item, AddOp::Insert(0))
    }

    /// Add menu items to the beginning of this submenu.
    /// It calls [`Menu::prepend`](crate::Menu::prepend) on the first element and
    /// passes the rest to [`Menu::insert_items`](crate::Menu::insert_items) with position of `1`.
    pub fn prepend_items(&self, items: &[&dyn IsMenuItem]) -> crate::Result<()> {
        self.insert_items(items, 0)
    }

    /// Insert a menu item at the specified `postion` in the submenu.
    pub fn insert(&self, item: &dyn IsMenuItem, position: usize) -> crate::Result<()> {
        self.0
            .borrow_mut()
            .add_menu_item(item, AddOp::Insert(position))
    }

    /// Insert menu items at the specified `postion` in the submenu.
    pub fn insert_items(&self, items: &[&dyn IsMenuItem], position: usize) -> crate::Result<()> {
        for (i, item) in items.iter().enumerate() {
            self.insert(*item, position + i)?
        }

        Ok(())
    }

    /// Remove a menu item from this submenu.
    pub fn remove(&self, item: &dyn IsMenuItem) -> crate::Result<()> {
        self.0.borrow_mut().remove(item)
    }

    /// Returns a list of menu items that has been added to this submenu.
    pub fn items(&self) -> Vec<MenuItemKind> {
        self.0.borrow().items()
    }

    /// Get the text for this submenu.
    pub fn text(&self) -> String {
        self.0.borrow().text()
    }

    /// Set the text for this submenu. `text` could optionally contain
    /// an `&` before a character to assign this character as the mnemonic
    /// for this submenu. To display a `&` without assigning a mnemenonic, use `&&`.
    pub fn set_text<S: AsRef<str>>(&self, text: S) {
        self.0.borrow_mut().set_text(text.as_ref())
    }

    /// Get whether this submenu is enabled or not.
    pub fn is_enabled(&self) -> bool {
        self.0.borrow().is_enabled()
    }

    /// Enable or disable this submenu.
    pub fn set_enabled(&self, enabled: bool) {
        self.0.borrow_mut().set_enabled(enabled)
    }

    // TODO: in a minor release, rename the following two functions to be `set_as_*`

    /// Set this submenu as the Window menu for the application on macOS.
    ///
    /// This will cause macOS to automatically add window-switching items and
    /// certain other items to the menu.
    #[cfg(target_os = "macos")]
    pub fn set_as_windows_menu_for_nsapp(&self) {
        self.0.borrow_mut().set_as_windows_menu_for_nsapp()
    }

    /// Set this submenu as the Help menu for the application on macOS.
    ///
    /// This will cause macOS to automatically add a search box to the menu.
    ///
    /// If no menu is set as the Help menu, macOS will automatically use any menu
    /// which has a title matching the localized word "Help".
    #[cfg(target_os = "macos")]
    pub fn set_as_help_menu_for_nsapp(&self) {
        self.0.borrow_mut().set_as_help_menu_for_nsapp()
    }
}

impl ContextMenu for Submenu {
    #[cfg(target_os = "windows")]
    fn hpopupmenu(&self) -> windows_sys::Win32::UI::WindowsAndMessaging::HMENU {
        self.0.borrow().hpopupmenu()
    }

    #[cfg(target_os = "windows")]
    fn show_context_menu_for_hwnd(&self, hwnd: isize, position: Option<Position>) {
        self.0
            .borrow_mut()
            .show_context_menu_for_hwnd(hwnd, position)
    }

    #[cfg(target_os = "windows")]
    fn attach_menu_subclass_for_hwnd(&self, hwnd: isize) {
        self.0.borrow_mut().attach_menu_subclass_for_hwnd(hwnd)
    }

    #[cfg(target_os = "windows")]
    fn detach_menu_subclass_from_hwnd(&self, hwnd: isize) {
        self.0.borrow_mut().detach_menu_subclass_from_hwnd(hwnd)
    }

    #[cfg(target_os = "linux")]
    fn show_context_menu_for_gtk_window(
        &self,
        w: &gtk::ApplicationWindow,
        position: Option<Position>,
    ) {
        self.0
            .borrow_mut()
            .show_context_menu_for_gtk_window(w, position)
    }

    #[cfg(target_os = "linux")]
    fn gtk_context_menu(&self) -> gtk::Menu {
        self.0.borrow_mut().gtk_context_menu()
    }

    #[cfg(target_os = "macos")]
    fn show_context_menu_for_nsview(&self, view: cocoa::base::id, position: Option<Position>) {
        self.0
            .borrow_mut()
            .show_context_menu_for_nsview(view, position)
    }

    #[cfg(target_os = "macos")]
    fn ns_menu(&self) -> *mut std::ffi::c_void {
        self.0.borrow().ns_menu()
    }
}
