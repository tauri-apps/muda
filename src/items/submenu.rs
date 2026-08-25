// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.inner
// SPDX-License-Identifier: MIT

use std::{cell::RefCell, mem, rc::Rc};

use crate::{
    dpi::Position,
    menu::positions_of,
    platform_impl::PlatformMenuItem,
    sealed::IsMenuItemBase,
    util::{self, AddOp},
    ClickAction, ContextMenu, Icon, IconType, IsMenuItem, MenuId, MenuItemKind, NativeIcon,
};

/// A menu that can be added to a [`Menu`] or another [`Submenu`].
///
/// [`Menu`]: crate::Menu
#[derive(Clone)]
pub struct Submenu {
    pub(crate) id: Rc<MenuId>,
    pub(crate) state: Rc<RefCell<SubmenuState>>,
    pub(crate) platform: Rc<RefCell<PlatformMenuItem>>,
}

/// Shared state of a [`Submenu`].
pub(crate) struct SubmenuState {
    pub text: String,
    pub enabled: bool,
    pub icon: Option<IconType>,
    pub children: Vec<MenuItemKind>,
}

#[cfg(all(
    any(
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd"
    ),
    feature = "gtk"
))]
impl Drop for Submenu {
    fn drop(&mut self) {
        if Rc::strong_count(&self.state) == 1 {
            let state = self.state.borrow();
            self.platform.borrow_mut().destroy(&state.children);
        }
    }
}

impl IsMenuItemBase for Submenu {}
impl IsMenuItem for Submenu {
    fn kind(&self) -> MenuItemKind {
        MenuItemKind::Submenu(self.clone())
    }

    fn id(&self) -> &MenuId {
        self.id()
    }

    fn into_id(self) -> MenuId {
        self.into_id()
    }
}

impl Submenu {
    /// Create a new submenu.
    ///
    /// - `text` could optionally contain an `&` before a character to assign this character as the mnemonic
    ///   for this submenu. To display a `&` without assigning a mnemenonic, use `&&`.
    pub fn new<S: AsRef<str>>(text: S, enabled: bool) -> Self {
        Self::new_inner(None, text.as_ref(), enabled)
    }

    /// Create a new submenu with the specified id.
    ///
    /// - `text` could optionally contain an `&` before a character to assign this character as the mnemonic
    ///   for this submenu. To display a `&` without assigning a mnemenonic, use `&&`.
    pub fn with_id<I: Into<MenuId>, S: AsRef<str>>(id: I, text: S, enabled: bool) -> Self {
        Self::new_inner(Some(id.into()), text.as_ref(), enabled)
    }

    fn new_inner(id: Option<MenuId>, text: &str, enabled: bool) -> Self {
        let id = util::next_id(id);

        let state = SubmenuState {
            text: text.to_string(),
            enabled,
            icon: None,
            children: Vec::new(),
        };
        let click = ClickAction::Emit(id.clone());
        let platform = PlatformMenuItem::new_submenu(click);

        Self {
            id: Rc::new(id.clone()),
            state: Rc::new(RefCell::new(state)),
            platform: Rc::new(RefCell::new(platform)),
        }
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

    /// Creates a new submenu with the specified id and given `items`. It calls [`Submenu::new`] and [`Submenu::append_items`] internally.
    pub fn with_id_and_items<I: Into<MenuId>, S: AsRef<str>>(
        id: I,
        text: S,
        enabled: bool,
        items: &[&dyn IsMenuItem],
    ) -> crate::Result<Self> {
        let menu = Self::with_id(id, text, enabled);
        menu.append_items(items)?;
        Ok(menu)
    }

    /// Returns a unique identifier associated with this submenu.
    pub fn id(&self) -> &MenuId {
        &self.id
    }

    /// Add a menu item to the end of this menu.
    pub fn append(&self, item: &dyn IsMenuItem) -> crate::Result<()> {
        self.add_menu_item(item, AddOp::Append)
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
        self.add_menu_item(item, AddOp::Insert(0))
    }

    /// Add menu items to the beginning of this submenu.
    /// It calls [`Menu::prepend`](crate::Menu::prepend) on the first element and
    /// passes the rest to [`Menu::insert_items`](crate::Menu::insert_items) with position of `1`.
    pub fn prepend_items(&self, items: &[&dyn IsMenuItem]) -> crate::Result<()> {
        self.insert_items(items, 0)
    }

    /// Insert a menu item at the specified `position` in the submenu.
    pub fn insert(&self, item: &dyn IsMenuItem, position: usize) -> crate::Result<()> {
        self.add_menu_item(item, AddOp::Insert(position))
    }

    /// Insert menu items at the specified `position` in the submenu.
    pub fn insert_items(&self, items: &[&dyn IsMenuItem], position: usize) -> crate::Result<()> {
        for (i, item) in items.iter().enumerate() {
            self.insert(*item, position + i)?
        }

        Ok(())
    }

    fn add_menu_item(&self, item: &dyn IsMenuItem, op: AddOp) -> crate::Result<()> {
        let kind = item.kind();

        {
            let mut platform = self.platform.borrow_mut();
            platform.attach(&kind, op)?;
        }

        let mut state = self.state.borrow_mut();
        match op {
            AddOp::Append => state.children.push(kind),
            AddOp::Insert(position) => state.children.insert(position, kind),
        }

        Ok(())
    }

    /// Remove all occurrences of a menu item from this submenu.
    pub fn remove(&self, item: &dyn IsMenuItem) -> crate::Result<()> {
        let positions = positions_of(&self.state.borrow().children, item.id());

        if positions.is_empty() {
            return Err(crate::Error::NotAChildOfThisMenu);
        }

        // Back to front, so that each removal leaves the positions still to come untouched.
        for position in positions.into_iter().rev() {
            self.remove_at(position);
        }

        Ok(())
    }

    /// Remove the menu item at the specified position from this submenu and returns it.
    pub fn remove_at(&self, position: usize) -> Option<MenuItemKind> {
        let kind = {
            let mut state = self.state.borrow_mut();
            if position >= state.children.len() {
                return None;
            }
            state.children.remove(position)
        };

        self.platform.borrow_mut().remove_at(position, &kind);

        Some(kind)
    }

    /// Returns a list of menu items that has been added to this submenu.
    pub fn items(&self) -> Vec<MenuItemKind> {
        self.state.borrow().children.clone()
    }

    /// Get the text for this submenu.
    pub fn text(&self) -> String {
        self.platform
            .borrow()
            .text()
            .unwrap_or_else(|| self.state.borrow().text.clone())
    }

    /// Set the text for this submenu. `text` could optionally contain
    /// an `&` before a character to assign this character as the mnemonic
    /// for this submenu. To display a `&` without assigning a mnemenonic, use `&&`.
    pub fn set_text<S: AsRef<str>>(&self, text: S) {
        self.state.borrow_mut().text = text.as_ref().to_string();
        // A submenu carries no accelerator: there is no `Submenu::set_accelerator`.
        self.platform.borrow_mut().set_text(text.as_ref(), None)
    }

    /// Get whether this submenu is enabled or not.
    pub fn is_enabled(&self) -> bool {
        self.platform
            .borrow()
            .is_enabled()
            .unwrap_or_else(|| self.state.borrow().enabled)
    }

    /// Enable or disable this submenu.
    pub fn set_enabled(&self, enabled: bool) {
        self.state.borrow_mut().enabled = enabled;
        self.platform.borrow_mut().set_enabled(enabled)
    }

    /// Set this submenu as the Window menu for the application on macOS.
    /// This will cause macOS to automatically add window-switching items and
    /// certain other items to the menu.
    ///
    /// Must be called after adding this submenu to [`Menu`](crate::Menu)
    /// and after calling [`Menu::init_for_nsapp`](crate::Menu::init_for_nsapp) on that menu.
    ///
    ///
    /// # Note
    ///
    /// Because a [`Submenu`] can be added multiple times to the same [`Menu`](crate::Menu)
    /// this method will set the first instance of this submenu as the Window menu for the application.
    ///
    /// It is not recommended to add the same submenu multiple times to the same menu, but if you do, be aware of this behavior.
    #[cfg(target_os = "macos")]
    pub fn set_as_windows_menu_for_nsapp(&self) {
        self.platform.borrow_mut().set_as_windows_menu_for_nsapp()
    }

    /// Set this submenu as the Help menu for the application on macOS.
    /// This will cause macOS to automatically add a search box to the menu.
    ///
    /// Must be called after adding this submenu to [`Menu`](crate::Menu)
    /// and after calling [`Menu::init_for_nsapp`](crate::Menu::init_for_nsapp) on that menu.
    ///
    /// If no menu is set as the Help menu, macOS will automatically use any menu
    /// which has a title matching the localized word "Help".
    ///
    /// # Note
    ///
    /// Because a [`Submenu`] can be added multiple times to the same [`Menu`](crate::Menu)
    /// this method will set the first instance of this submenu as the Help menu for the application.
    ///
    /// It is not recommended to add the same submenu multiple times to the same menu, but if you do, be aware of this behavior.
    #[cfg(target_os = "macos")]
    pub fn set_as_help_menu_for_nsapp(&self) {
        self.platform.borrow_mut().set_as_help_menu_for_nsapp()
    }

    /// Convert this submenu into its menu ID.
    pub fn into_id(mut self) -> MenuId {
        // Note: `Rc::into_inner` is available from Rust 1.70
        if let Some(id) = Rc::get_mut(&mut self.id) {
            mem::take(id)
        } else {
            self.id().clone()
        }
    }

    /// Change this menu item icon or remove it.
    ///
    /// Platform-specific:
    ///
    /// - GTK 4: Unsupported.
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
    /// - **GTK 3**: Known variants map to freedesktop-style icon theme names. Use
    ///   [`NativeIcon::Raw`] or `NativeIcon::from_name` to pass an icon theme name resolved by
    ///   [`GtkIconTheme`].
    /// - **GTK 4**: Unsupported.
    ///
    /// [`NSImage.Name`]: https://developer.apple.com/documentation/appkit/nsimage/name-swift.typealias
    /// [`SHSTOCKICONID`]: https://learn.microsoft.com/en-us/windows/win32/api/shellapi/ne-shellapi-shstockiconid
    /// [`SHGetStockIconInfo`]: https://learn.microsoft.com/en-us/windows/win32/api/shellapi/nf-shellapi-shgetstockiconinfo
    /// [`GtkIconTheme`]: https://docs.gtk.org/gtk3/class.IconTheme.html
    /// [Icon Naming Specification]: https://specifications.freedesktop.org/icon-naming-spec/latest/
    ///
    /// (Note that setting a native icon will override any existing [.set_icon()](Self::set_icon))
    pub fn set_native_icon(&self, icon: Option<NativeIcon>) {
        let icon = icon.map(IconType::Native);
        self.state.borrow_mut().icon = icon;
        let state = self.state.borrow();
        self.platform.borrow_mut().set_icon(state.icon.as_ref())
    }
}

impl ContextMenu for Submenu {
    #[cfg(target_os = "windows")]
    fn hpopupmenu(&self) -> isize {
        self.platform.borrow().hpopupmenu()
    }

    #[cfg(target_os = "windows")]
    unsafe fn show_context_menu_for_hwnd(&self, hwnd: isize, position: Option<Position>) -> bool {
        let selected = self.platform.borrow().show_context_menu(hwnd, position);
        crate::platform_impl::dispatch_selection(hwnd, selected)
    }

    #[cfg(target_os = "windows")]
    unsafe fn attach_menu_subclass_for_hwnd(&self, hwnd: isize) {
        self.platform.borrow().attach_menu_subclass_for_hwnd(hwnd)
    }

    #[cfg(target_os = "windows")]
    unsafe fn detach_menu_subclass_from_hwnd(&self, hwnd: isize) {
        self.platform.borrow().detach_menu_subclass_from_hwnd(hwnd)
    }

    #[cfg(all(
        any(
            target_os = "linux",
            target_os = "dragonfly",
            target_os = "freebsd",
            target_os = "netbsd",
            target_os = "openbsd"
        ),
        any(feature = "gtk", feature = "gtk4")
    ))]
    fn show_context_menu_for_gtk_window(
        &self,
        w: &gtk::Window,
        position: Option<Position>,
    ) -> bool {
        let state = self.state.borrow();
        self.platform
            .borrow_mut()
            .show_context_menu_for_gtk_window(&state.children, w, position)
    }

    #[cfg(all(
        any(
            target_os = "linux",
            target_os = "dragonfly",
            target_os = "freebsd",
            target_os = "netbsd",
            target_os = "openbsd"
        ),
        feature = "gtk"
    ))]
    fn gtk_context_menu(&self) -> gtk::Menu {
        let state = self.state.borrow();
        self.platform.borrow_mut().gtk_context_menu(&state.children)
    }

    #[cfg(all(
        any(
            target_os = "linux",
            target_os = "dragonfly",
            target_os = "freebsd",
            target_os = "netbsd",
            target_os = "openbsd"
        ),
        feature = "gtk4"
    ))]
    fn gtk_context_menu(&self) -> gtk::PopoverMenu {
        self.platform.borrow_mut().gtk_context_menu()
    }

    #[cfg(target_os = "macos")]
    unsafe fn show_context_menu_for_nsview(
        &self,
        view: *const std::ffi::c_void,
        position: Option<Position>,
    ) -> bool {
        self.platform
            .borrow_mut()
            .show_context_menu_for_nsview(view, position)
    }

    #[cfg(target_os = "macos")]
    fn ns_menu(&self) -> *mut std::ffi::c_void {
        self.platform.borrow().ns_menu()
    }

    fn as_submenu(&self) -> Option<&Submenu> {
        Some(self)
    }
}
