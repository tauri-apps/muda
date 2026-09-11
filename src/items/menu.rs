// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use std::{cell::RefCell, rc::Rc};

use crate::{
    platform_impl::PlatformMenu,
    util::{self, AddOp},
    ContextMenu, IsMenuItem, MenuId, MenuItemKind, StateCell, UnsafeMenuItemKind,
};

#[cfg(feature = "snapshot")]
use crate::MenuSnapshotHandle;

#[cfg(any(
    target_os = "windows",
    target_os = "macos",
    all(
        any(
            target_os = "linux",
            target_os = "dragonfly",
            target_os = "freebsd",
            target_os = "netbsd",
            target_os = "openbsd"
        ),
        any(all(feature = "gtk3", not(feature = "gtk4")), feature = "gtk4")
    )
))]
use crate::dpi::Position;

/// The window menu bar theme
#[cfg(windows)]
#[repr(usize)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum MenuTheme {
    Dark = 0,
    Light = 1,
    Auto = 2,
}

/// A root menu that can be added to a window on Windows, GTK 3, or GTK 4
/// and used as the app global menu on macOS.
#[derive(Clone)]
pub struct Menu {
    id: Rc<MenuId>,
    state: StateCell<MenuState>,
    platform: Rc<RefCell<PlatformMenu>>,
}

/// Shared state of a root [`Menu`].
pub(crate) struct MenuState {
    pub children: Vec<UnsafeMenuItemKind>,
}

impl Drop for Menu {
    fn drop(&mut self) {
        if Rc::strong_count(&self.id) == 1 {
            let children: Vec<MenuItemKind> = std::mem::take(&mut self.state.borrow_mut().children)
                .into_iter()
                .map(|child| {
                    // SAFETY: the last thread-bound `Menu` is being dropped on the thread where
                    // its children were wrapped, so they can be recovered and destroyed here.
                    unsafe { child.unwrap() }
                })
                .collect();

            #[cfg(any(
                target_os = "macos",
                all(
                    any(
                        target_os = "linux",
                        target_os = "dragonfly",
                        target_os = "freebsd",
                        target_os = "netbsd",
                        target_os = "openbsd"
                    ),
                    any(all(feature = "gtk3", not(feature = "gtk4")), feature = "gtk4")
                )
            ))]
            self.platform.borrow_mut().destroy(&children);

            drop(children);
        }
    }
}

impl Default for Menu {
    fn default() -> Self {
        Self::new()
    }
}

impl Menu {
    /// Creates a new menu.
    pub fn new() -> Self {
        Self::new_inner(None)
    }

    /// Creates a new menu with the specified id.
    pub fn with_id<I: Into<MenuId>>(id: I) -> Self {
        Self::new_inner(Some(id.into()))
    }

    fn new_inner(id: Option<MenuId>) -> Self {
        Self {
            id: Rc::new(util::next_id(id)),
            state: StateCell::new(MenuState {
                children: Vec::new(),
            }),
            platform: Rc::new(RefCell::new(PlatformMenu::new())),
        }
    }

    /// Creates a new menu with given `items`. It calls [`Menu::new`] and [`Menu::append_items`] internally.
    pub fn with_items(items: &[&dyn IsMenuItem]) -> crate::Result<Self> {
        let menu = Self::new();
        menu.append_items(items)?;
        Ok(menu)
    }

    /// Creates a new menu with the specified id and given `items`. It calls [`Menu::new`] and [`Menu::append_items`] internally.
    pub fn with_id_and_items<I: Into<MenuId>>(
        id: I,
        items: &[&dyn IsMenuItem],
    ) -> crate::Result<Self> {
        let menu = Self::with_id(id);
        menu.append_items(items)?;
        Ok(menu)
    }

    /// Returns a unique identifier associated with this menu.
    pub fn id(&self) -> &MenuId {
        &self.id
    }

    /// Add a menu item to the end of this menu.
    ///
    /// ## Platform-specific:
    ///
    /// - **macOS:** Only [`Submenu`] can be added to the menu
    ///
    /// [`Submenu`]: crate::Submenu
    pub fn append(&self, item: &dyn IsMenuItem) -> crate::Result<()> {
        self.add_menu_item(item, AddOp::Append)
    }

    /// Add menu items to the end of this menu. It calls [`Menu::append`] in a loop internally.
    ///
    /// ## Platform-specific:
    ///
    /// - **macOS:** Only [`Submenu`] can be added to the menu
    ///
    /// [`Submenu`]: crate::Submenu
    pub fn append_items(&self, items: &[&dyn IsMenuItem]) -> crate::Result<()> {
        for item in items {
            self.append(*item)?
        }

        Ok(())
    }

    /// Add a menu item to the beginning of this menu.
    ///
    /// ## Platform-specific:
    ///
    /// - **macOS:** Only [`Submenu`] can be added to the menu
    ///
    /// [`Submenu`]: crate::Submenu
    pub fn prepend(&self, item: &dyn IsMenuItem) -> crate::Result<()> {
        self.add_menu_item(item, AddOp::Insert(0))
    }

    /// Add menu items to the beginning of this menu. It calls [`Menu::insert_items`] with position of `0` internally.
    ///
    /// ## Platform-specific:
    ///
    /// - **macOS:** Only [`Submenu`] can be added to the menu
    ///
    /// [`Submenu`]: crate::Submenu
    pub fn prepend_items(&self, items: &[&dyn IsMenuItem]) -> crate::Result<()> {
        self.insert_items(items, 0)
    }

    /// Insert a menu item at the specified `position` in the menu.
    ///
    /// ## Platform-specific:
    ///
    /// - **macOS:** Only [`Submenu`] can be added to the menu
    ///
    /// [`Submenu`]: crate::Submenu
    pub fn insert(&self, item: &dyn IsMenuItem, position: usize) -> crate::Result<()> {
        self.add_menu_item(item, AddOp::Insert(position))
    }

    /// Insert menu items at the specified `position` in the menu.
    ///
    /// ## Platform-specific:
    ///
    /// - **macOS:** Only [`Submenu`] can be added to the menu
    ///
    /// [`Submenu`]: crate::Submenu
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

        let kind = UnsafeMenuItemKind::new(kind);
        let mut state = self.state.borrow_mut();
        match op {
            AddOp::Append => state.children.push(kind),
            AddOp::Insert(position) => state.children.insert(position, kind),
        }

        Ok(())
    }

    /// Remove a menu item from this menu.
    /// Remove all occurrences of a menu item from this menu.
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

    /// Remove the menu item at the specified position from this menu and returns it.
    pub fn remove_at(&self, position: usize) -> Option<MenuItemKind> {
        let kind = {
            let mut state = self.state.borrow_mut();
            if position >= state.children.len() {
                return None;
            }
            state.children.remove(position)
        };

        // SAFETY: this thread-bound `Menu` can mutate its children only on the thread where their
        // `MenuItemKind` values were wrapped.
        let kind = unsafe { kind.unwrap() };

        self.platform.borrow_mut().remove_at(position, &kind);

        Some(kind)
    }

    /// Returns a list of menu items that has been added to this menu.
    pub fn items(&self) -> Vec<MenuItemKind> {
        self.state
            .borrow()
            .children
            .iter()
            .map(|child| {
                // SAFETY: the thread-bound `Menu` remains on the thread where its children were
                // wrapped, and the returned clones remain on that thread.
                unsafe { child.clone() }
            })
            .collect()
    }
}

/// Windows-specific operations for a [`Menu`].
#[cfg(target_os = "windows")]
pub trait MenuExtWindows {
    /// Adds this menu to a Win32 window.
    ///
    /// # Safety
    ///
    /// `hwnd` must be a valid window handle.
    unsafe fn init_for_hwnd(&self, hwnd: isize) -> crate::Result<()>;

    /// Adds this menu to a Win32 window using the specified theme.
    ///
    /// The theme affects the menu bar itself, but not submenus or context menus.
    ///
    /// # Safety
    ///
    /// `hwnd` must be a valid window handle.
    unsafe fn init_for_hwnd_with_theme(&self, hwnd: isize, theme: MenuTheme) -> crate::Result<()>;

    /// Sets the menu bar theme for a Win32 window.
    ///
    /// # Safety
    ///
    /// `hwnd` must be a valid window handle.
    unsafe fn set_theme_for_hwnd(&self, hwnd: isize, theme: MenuTheme) -> crate::Result<()>;

    /// Gets the [`HACCEL`](windows_sys::Win32::UI::WindowsAndMessaging::HACCEL) associated with
    /// this menu.
    fn haccel(&self) -> isize;

    /// Removes this menu from a Win32 window.
    ///
    /// # Safety
    ///
    /// `hwnd` must be a valid window handle.
    unsafe fn remove_for_hwnd(&self, hwnd: isize) -> crate::Result<()>;

    /// Hides this menu on a Win32 window.
    ///
    /// # Safety
    ///
    /// `hwnd` must be a valid window handle.
    unsafe fn hide_for_hwnd(&self, hwnd: isize) -> crate::Result<()>;

    /// Shows this menu on a Win32 window.
    ///
    /// # Safety
    ///
    /// `hwnd` must be a valid window handle.
    unsafe fn show_for_hwnd(&self, hwnd: isize) -> crate::Result<()>;

    /// Returns whether this menu is visible on a Win32 window.
    ///
    /// # Safety
    ///
    /// `hwnd` must be a valid window handle.
    unsafe fn is_visible_on_hwnd(&self, hwnd: isize) -> bool;
}

#[cfg(target_os = "windows")]
impl MenuExtWindows for Menu {
    unsafe fn init_for_hwnd(&self, hwnd: isize) -> crate::Result<()> {
        unsafe { self.platform.borrow_mut().init_for_hwnd(hwnd) }
    }

    unsafe fn init_for_hwnd_with_theme(&self, hwnd: isize, theme: MenuTheme) -> crate::Result<()> {
        unsafe {
            self.platform
                .borrow_mut()
                .init_for_hwnd_with_theme(hwnd, theme)
        }
    }

    unsafe fn set_theme_for_hwnd(&self, hwnd: isize, theme: MenuTheme) -> crate::Result<()> {
        unsafe { self.platform.borrow().set_theme_for_hwnd(hwnd, theme) }
    }

    fn haccel(&self) -> isize {
        self.platform.borrow().haccel()
    }

    unsafe fn remove_for_hwnd(&self, hwnd: isize) -> crate::Result<()> {
        unsafe { self.platform.borrow_mut().remove_for_hwnd(hwnd) }
    }

    unsafe fn hide_for_hwnd(&self, hwnd: isize) -> crate::Result<()> {
        unsafe { self.platform.borrow().hide_for_hwnd(hwnd) }
    }

    unsafe fn show_for_hwnd(&self, hwnd: isize) -> crate::Result<()> {
        unsafe { self.platform.borrow().show_for_hwnd(hwnd) }
    }

    unsafe fn is_visible_on_hwnd(&self, hwnd: isize) -> bool {
        unsafe { self.platform.borrow().is_visible_on_hwnd(hwnd) }
    }
}

/// macOS-specific operations for a [`Menu`].
#[cfg(target_os = "macos")]
pub trait MenuExtMacOS {
    /// Adds this menu to `NSApp` as its main menu.
    fn init_for_nsapp(&self);

    /// Removes this menu from `NSApp`.
    fn remove_for_nsapp(&self);
}

#[cfg(target_os = "macos")]
impl MenuExtMacOS for Menu {
    fn init_for_nsapp(&self) {
        self.platform.borrow_mut().init_for_nsapp()
    }

    fn remove_for_nsapp(&self) {
        self.platform.borrow_mut().remove_for_nsapp()
    }
}

/// GTK 3-specific operations for a [`Menu`].
#[cfg(all(
    any(
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd"
    ),
    feature = "gtk3"
))]
pub trait MenuGtkExt {
    /// Adds this menu to a GTK 3 window.
    ///
    /// `container` receives the menu bar and should normally be provided. Supported containers
    /// are [`gtk::Box`], [`gtk::Fixed`], and [`gtk::Stack`].
    fn init_for_gtk_window<W, C>(&self, window: &W, container: Option<&C>) -> crate::Result<()>
    where
        W: gtk::prelude::IsA<gtk::Window> + gtk::prelude::IsA<gtk::Widget>,
        C: gtk::prelude::IsA<gtk::Widget>;

    /// Removes this menu from a GTK 3 window.
    fn remove_for_gtk_window<W>(&self, window: &W) -> crate::Result<()>
    where
        W: gtk::prelude::IsA<gtk::Window> + gtk::prelude::IsA<gtk::Widget>;

    /// Hides this menu on a GTK 3 window.
    fn hide_for_gtk_window<W>(&self, window: &W) -> crate::Result<()>
    where
        W: gtk::prelude::IsA<gtk::Window>;

    /// Shows this menu on a GTK 3 window.
    fn show_for_gtk_window<W>(&self, window: &W) -> crate::Result<()>
    where
        W: gtk::prelude::IsA<gtk::Window>;

    /// Returns whether this menu is visible on a GTK 3 window.
    fn is_visible_on_gtk_window<W>(&self, window: &W) -> bool
    where
        W: gtk::prelude::IsA<gtk::Window>;

    /// Returns the GTK 3 menubar associated with this window, if one exists.
    fn gtk_menubar_for_gtk_window<W>(&self, window: &W) -> Option<gtk::MenuBar>
    where
        W: gtk::prelude::IsA<gtk::Window>;
}

#[cfg(all(
    any(
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd"
    ),
    feature = "gtk3",
    not(feature = "gtk4")
))]
impl MenuGtkExt for Menu {
    fn init_for_gtk_window<W, C>(&self, window: &W, container: Option<&C>) -> crate::Result<()>
    where
        W: gtk::prelude::IsA<gtk::Window> + gtk::prelude::IsA<gtk::Widget>,
        C: gtk::prelude::IsA<gtk::Widget>,
    {
        let children = self.items();
        self.platform
            .borrow_mut()
            .init_for_gtk_window(&children, window, container)
    }

    fn remove_for_gtk_window<W>(&self, window: &W) -> crate::Result<()>
    where
        W: gtk::prelude::IsA<gtk::Window> + gtk::prelude::IsA<gtk::Widget>,
    {
        let children = self.items();
        self.platform
            .borrow_mut()
            .remove_for_gtk_window(&children, window)
    }

    fn hide_for_gtk_window<W>(&self, window: &W) -> crate::Result<()>
    where
        W: gtk::prelude::IsA<gtk::Window>,
    {
        self.platform.borrow_mut().hide_for_gtk_window(window)
    }

    fn show_for_gtk_window<W>(&self, window: &W) -> crate::Result<()>
    where
        W: gtk::prelude::IsA<gtk::Window>,
    {
        self.platform.borrow_mut().show_for_gtk_window(window)
    }

    fn is_visible_on_gtk_window<W>(&self, window: &W) -> bool
    where
        W: gtk::prelude::IsA<gtk::Window>,
    {
        self.platform.borrow().is_visible_on_gtk_window(window)
    }

    fn gtk_menubar_for_gtk_window<W>(&self, window: &W) -> Option<gtk::MenuBar>
    where
        W: gtk::prelude::IsA<gtk::Window>,
    {
        self.platform.borrow().gtk_menubar_for_gtk_window(window)
    }
}

// TODO: Remove once Tauri migrates to GTK 4 exclusively.
/// GTK 3-specific operations for a [`Menu`] when both GTK 3 and GTK 4 features are enabled.
#[cfg(all(
    any(
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd"
    ),
    feature = "gtk3",
    feature = "gtk4"
))]
impl MenuGtkExt for Menu {
    fn init_for_gtk_window<W, C>(&self, _window: &W, _container: Option<&C>) -> crate::Result<()>
    where
        W: gtk::prelude::IsA<gtk::Window> + gtk::prelude::IsA<gtk::Widget>,
        C: gtk::prelude::IsA<gtk::Widget>,
    {
        panic!("GTK 3 menu is unavailable when both GTK 3 and GTK 4 features are enabled")
    }

    fn remove_for_gtk_window<W>(&self, _window: &W) -> crate::Result<()>
    where
        W: gtk::prelude::IsA<gtk::Window> + gtk::prelude::IsA<gtk::Widget>,
    {
        panic!("GTK 3 menu is unavailable when both GTK 3 and GTK 4 features are enabled")
    }

    fn hide_for_gtk_window<W>(&self, _window: &W) -> crate::Result<()>
    where
        W: gtk::prelude::IsA<gtk::Window>,
    {
        panic!("GTK 3 menu is unavailable when both GTK 3 and GTK 4 features are enabled")
    }

    fn show_for_gtk_window<W>(&self, _window: &W) -> crate::Result<()>
    where
        W: gtk::prelude::IsA<gtk::Window>,
    {
        panic!("GTK 3 menu is unavailable when both GTK 3 and GTK 4 features are enabled")
    }

    fn is_visible_on_gtk_window<W>(&self, _window: &W) -> bool
    where
        W: gtk::prelude::IsA<gtk::Window>,
    {
        panic!("GTK 3 menu is unavailable when both GTK 3 and GTK 4 features are enabled")
    }

    fn gtk_menubar_for_gtk_window<W>(&self, _window: &W) -> Option<gtk::MenuBar>
    where
        W: gtk::prelude::IsA<gtk::Window>,
    {
        panic!("GTK 3 menu is unavailable when both GTK 3 and GTK 4 features are enabled")
    }
}

/// GTK 4-specific operations for a [`Menu`].
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
pub trait MenuGtk4Ext {
    /// Adds this menu to a GTK 4 window.
    ///
    /// `container` receives the menu bar and should normally be provided. Supported containers
    /// are [`gtk4::Box`], [`gtk4::Fixed`], and [`gtk4::Stack`]. The window must belong to a
    /// [`gtk4::Application`].
    fn init_for_gtk_window<W, C>(&self, window: &W, container: Option<&C>) -> crate::Result<()>
    where
        W: gtk4::prelude::IsA<gtk4::Window> + gtk4::prelude::IsA<gtk4::Widget>,
        C: gtk4::prelude::IsA<gtk4::Widget>;

    /// Removes this menu from a GTK 4 window.
    fn remove_for_gtk_window<W>(&self, window: &W) -> crate::Result<()>
    where
        W: gtk4::prelude::IsA<gtk4::Window> + gtk4::prelude::IsA<gtk4::Widget>;

    /// Hides this menu on a GTK 4 window.
    fn hide_for_gtk_window<W>(&self, window: &W) -> crate::Result<()>
    where
        W: gtk4::prelude::IsA<gtk4::Window>;

    /// Shows this menu on a GTK 4 window.
    fn show_for_gtk_window<W>(&self, window: &W) -> crate::Result<()>
    where
        W: gtk4::prelude::IsA<gtk4::Window>;

    /// Returns whether this menu is visible on a GTK 4 window.
    fn is_visible_on_gtk_window<W>(&self, window: &W) -> bool
    where
        W: gtk4::prelude::IsA<gtk4::Window>;

    /// Returns the GTK 4 menubar associated with this window, if one exists.
    fn gtk_popover_menubar_for_gtk_window<W>(&self, window: &W) -> Option<gtk4::PopoverMenuBar>
    where
        W: gtk4::prelude::IsA<gtk4::Window>;
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
impl MenuGtk4Ext for Menu {
    fn init_for_gtk_window<W, C>(&self, window: &W, container: Option<&C>) -> crate::Result<()>
    where
        W: gtk4::prelude::IsA<gtk4::Window> + gtk4::prelude::IsA<gtk4::Widget>,
        C: gtk4::prelude::IsA<gtk4::Widget>,
    {
        let children = self.items();
        self.platform
            .borrow_mut()
            .init_for_gtk_window(&children, window, container)
    }

    fn remove_for_gtk_window<W>(&self, window: &W) -> crate::Result<()>
    where
        W: gtk4::prelude::IsA<gtk4::Window> + gtk4::prelude::IsA<gtk4::Widget>,
    {
        let children = self.items();
        self.platform
            .borrow_mut()
            .remove_for_gtk_window(&children, window)
    }

    fn hide_for_gtk_window<W>(&self, window: &W) -> crate::Result<()>
    where
        W: gtk4::prelude::IsA<gtk4::Window>,
    {
        self.platform.borrow_mut().hide_for_gtk_window(window)
    }

    fn show_for_gtk_window<W>(&self, window: &W) -> crate::Result<()>
    where
        W: gtk4::prelude::IsA<gtk4::Window>,
    {
        self.platform.borrow_mut().show_for_gtk_window(window)
    }

    fn is_visible_on_gtk_window<W>(&self, window: &W) -> bool
    where
        W: gtk4::prelude::IsA<gtk4::Window>,
    {
        self.platform.borrow().is_visible_on_gtk_window(window)
    }

    fn gtk_popover_menubar_for_gtk_window<W>(&self, window: &W) -> Option<gtk4::PopoverMenuBar>
    where
        W: gtk4::prelude::IsA<gtk4::Window>,
    {
        self.platform
            .borrow()
            .gtk_popover_menubar_for_gtk_window(window)
    }
}

#[cfg(all(
    any(
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd"
    ),
    feature = "gtk3",
    not(feature = "gtk4")
))]
impl crate::ContextMenuGtkExt for Menu {
    fn show_context_menu_for_gtk_window(
        &self,
        window: &gtk::Window,
        position: Option<Position>,
    ) -> bool {
        let children = self.items();
        self.platform
            .borrow_mut()
            .show_context_menu_for_gtk_window(&children, window, position)
    }

    fn gtk_menu(&self) -> gtk::Menu {
        let children = self.items();
        self.platform.borrow_mut().gtk_menu(&children)
    }
}

// TODO: Remove once Tauri migrates to GTK 4 exclusively.
/// GTK 3-specific operations for a [`Menu`] when both GTK 3 and GTK 4 features are enabled.
#[cfg(all(
    any(
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd"
    ),
    feature = "gtk3",
    feature = "gtk4"
))]
impl crate::ContextMenuGtkExt for Menu {
    fn show_context_menu_for_gtk_window(
        &self,
        _window: &gtk::Window,
        _position: Option<Position>,
    ) -> bool {
        panic!("GTK 3 menu is unavailable when both GTK 3 and GTK 4 features are enabled")
    }

    fn gtk_menu(&self) -> gtk::Menu {
        panic!("GTK 3 menu is unavailable when both GTK 3 and GTK 4 features are enabled");
    }
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
impl crate::ContextMenuGtk4Ext for Menu {
    fn show_context_menu_for_gtk_window(
        &self,
        window: &gtk4::Window,
        position: Option<Position>,
    ) -> bool {
        let children = self.items();
        self.platform
            .borrow_mut()
            .show_context_menu_for_gtk_window(&children, window, position)
    }

    fn gtk_popover_menu(&self) -> gtk4::PopoverMenu {
        let children = self.items();
        self.platform.borrow_mut().gtk_popover_menu(&children)
    }
}

#[cfg(target_os = "windows")]
impl crate::ContextMenuExtWindows for Menu {
    fn hpopupmenu(&self) -> isize {
        self.platform.borrow().hpopupmenu()
    }

    unsafe fn show_context_menu_for_hwnd(&self, hwnd: isize, position: Option<Position>) -> bool {
        let selected = unsafe { self.platform.borrow().show_context_menu(hwnd, position) };
        crate::platform_impl::dispatch_selection(hwnd, selected)
    }

    unsafe fn attach_menu_subclass_for_hwnd(&self, hwnd: isize) {
        unsafe { self.platform.borrow().attach_menu_subclass_for_hwnd(hwnd) }
    }

    unsafe fn detach_menu_subclass_from_hwnd(&self, hwnd: isize) {
        unsafe { self.platform.borrow().detach_menu_subclass_from_hwnd(hwnd) }
    }
}

#[cfg(target_os = "macos")]
impl crate::ContextMenuExtMacOS for Menu {
    unsafe fn show_context_menu_for_nsview(
        &self,
        view: *const std::ffi::c_void,
        position: Option<Position>,
    ) -> bool {
        unsafe {
            self.platform
                .borrow_mut()
                .show_context_menu_for_nsview(view, position)
        }
    }

    fn ns_menu(&self) -> *mut std::ffi::c_void {
        self.platform.borrow().ns_menu()
    }
}

impl crate::sealed::Sealed for Menu {}
impl ContextMenu for Menu {
    fn kind(&self) -> crate::MenuKind {
        crate::MenuKind::Menu(self.clone())
    }

    #[cfg(feature = "snapshot")]
    fn snapshot_handle(&self) -> MenuSnapshotHandle {
        MenuSnapshotHandle::from_menu(self.state.clone())
    }
}

/// Returns the positions of the menu items with the specified `id` within the given `children` slice.
pub(crate) fn positions_of(children: &[UnsafeMenuItemKind], id: &MenuId) -> Vec<usize> {
    children
        .iter()
        .enumerate()
        .filter_map(|(index, child)| {
            // SAFETY: callers hold a thread-bound `Menu` or `Submenu`, so this runs on the thread
            // where the child was wrapped.
            (unsafe { child.borrow() }.id() == id).then_some(index)
        })
        .collect()
}
