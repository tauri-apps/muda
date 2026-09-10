// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use crate::{sealed, MenuKind};

#[cfg(feature = "snapshot")]
use crate::MenuSnapshotHandle;

/// A trait that defines the behavior and interface for context menus across different platforms.
pub trait ContextMenu: sealed::Sealed {
    /// Returns the concrete kind of this context menu.
    fn kind(&self) -> MenuKind;

    /// Returns a thread-safe snapshot handle for this menu tree.
    #[cfg(feature = "snapshot")]
    fn snapshot_handle(&self) -> MenuSnapshotHandle;
}

/// Windows-specific operations for context menus.
#[cfg(target_os = "windows")]
pub trait ContextMenuExtWindows {
    /// Get the popup [`HMENU`] for this menu.
    ///
    /// The returned [`HMENU`] is valid as long as the context menu is.
    ///
    /// [`HMENU`]: windows_sys::Win32::UI::WindowsAndMessaging::HMENU
    fn hpopupmenu(&self) -> isize;

    /// Shows this menu as a context menu inside a Win32 window.
    ///
    /// `position` is relative to the window's top-left corner. If it is `None`, the cursor
    /// position is used.
    ///
    /// Returns `true` if menu tracking ended because an item was selected, and `false` if it
    /// was cancelled.
    ///
    /// # Safety
    ///
    /// `hwnd` must be a valid window handle.
    unsafe fn show_context_menu_for_hwnd(
        &self,
        hwnd: isize,
        position: Option<crate::dpi::Position>,
    ) -> bool;

    /// Attaches the menu subclass handler to the window so menu events can be received.
    ///
    /// # Safety
    ///
    /// `hwnd` must be a valid window handle.
    unsafe fn attach_menu_subclass_for_hwnd(&self, hwnd: isize);

    /// Detaches the menu subclass handler from the window.
    ///
    /// # Safety
    ///
    /// `hwnd` must be a valid window handle.
    unsafe fn detach_menu_subclass_from_hwnd(&self, hwnd: isize);
}

/// macOS-specific operations for context menus.
#[cfg(target_os = "macos")]
pub trait ContextMenuExtMacOS {
    /// Shows this menu as a context menu for the specified `NSView`.
    ///
    /// `position` is relative to the view's top-left corner. If it is `None`, the cursor
    /// position is used.
    ///
    /// # Safety
    ///
    /// `view` must point to a valid `NSView`.
    unsafe fn show_context_menu_for_nsview(
        &self,
        view: *const std::ffi::c_void,
        position: Option<crate::dpi::Position>,
    ) -> bool;

    /// Gets the underlying `NSMenu` reserved for context menus.
    ///
    /// The returned pointer is valid for as long as the context menu. Retain it if it must
    /// remain alive for longer.
    fn ns_menu(&self) -> *mut std::ffi::c_void;
}

#[cfg(target_os = "windows")]
impl ContextMenuExtWindows for dyn ContextMenu + '_ {
    fn hpopupmenu(&self) -> isize {
        match self.kind() {
            MenuKind::Menu(menu) => ContextMenuExtWindows::hpopupmenu(&menu),
            MenuKind::Submenu(submenu) => ContextMenuExtWindows::hpopupmenu(&submenu),
        }
    }

    unsafe fn show_context_menu_for_hwnd(
        &self,
        hwnd: isize,
        position: Option<crate::dpi::Position>,
    ) -> bool {
        match self.kind() {
            MenuKind::Menu(menu) => unsafe {
                ContextMenuExtWindows::show_context_menu_for_hwnd(&menu, hwnd, position)
            },
            MenuKind::Submenu(submenu) => unsafe {
                ContextMenuExtWindows::show_context_menu_for_hwnd(&submenu, hwnd, position)
            },
        }
    }

    unsafe fn attach_menu_subclass_for_hwnd(&self, hwnd: isize) {
        match self.kind() {
            MenuKind::Menu(menu) => unsafe {
                ContextMenuExtWindows::attach_menu_subclass_for_hwnd(&menu, hwnd)
            },
            MenuKind::Submenu(submenu) => unsafe {
                ContextMenuExtWindows::attach_menu_subclass_for_hwnd(&submenu, hwnd)
            },
        }
    }

    unsafe fn detach_menu_subclass_from_hwnd(&self, hwnd: isize) {
        match self.kind() {
            MenuKind::Menu(menu) => unsafe {
                ContextMenuExtWindows::detach_menu_subclass_from_hwnd(&menu, hwnd)
            },
            MenuKind::Submenu(submenu) => unsafe {
                ContextMenuExtWindows::detach_menu_subclass_from_hwnd(&submenu, hwnd)
            },
        }
    }
}

#[cfg(target_os = "macos")]
impl ContextMenuExtMacOS for dyn ContextMenu + '_ {
    unsafe fn show_context_menu_for_nsview(
        &self,
        view: *const std::ffi::c_void,
        position: Option<crate::dpi::Position>,
    ) -> bool {
        match self.kind() {
            MenuKind::Menu(menu) => unsafe {
                ContextMenuExtMacOS::show_context_menu_for_nsview(&menu, view, position)
            },
            MenuKind::Submenu(submenu) => unsafe {
                ContextMenuExtMacOS::show_context_menu_for_nsview(&submenu, view, position)
            },
        }
    }

    fn ns_menu(&self) -> *mut std::ffi::c_void {
        match self.kind() {
            MenuKind::Menu(menu) => ContextMenuExtMacOS::ns_menu(&menu),
            MenuKind::Submenu(submenu) => ContextMenuExtMacOS::ns_menu(&submenu),
        }
    }
}

/// GTK 3-specific operations for context menus.
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
pub trait ContextMenuGtkExt {
    /// Shows this menu as a context menu inside a GTK 3 window.
    ///
    /// `position` is relative to the window's top-left corner. If it is `None`, the cursor
    /// position is used.
    fn show_context_menu_for_gtk_window(
        &self,
        window: &gtk::Window,
        position: Option<crate::dpi::Position>,
    ) -> bool;

    /// Returns the underlying GTK 3 menu reserved for context menus.
    fn gtk_menu(&self) -> gtk::Menu;
}

/// GTK 4-specific operations for context menus.
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
pub trait ContextMenuGtk4Ext {
    /// Shows this menu as a context menu inside a GTK 4 window.
    ///
    /// `position` is relative to the window's top-left corner. If it is `None`, the cursor
    /// position is used.
    fn show_context_menu_for_gtk_window(
        &self,
        window: &gtk4::Window,
        position: Option<crate::dpi::Position>,
    ) -> bool;

    /// Returns the underlying GTK 4 popover menu reserved for context menus.
    fn gtk_popover_menu(&self) -> gtk4::PopoverMenu;
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
impl ContextMenuGtkExt for dyn ContextMenu + '_ {
    fn show_context_menu_for_gtk_window(
        &self,
        window: &gtk::Window,
        position: Option<crate::dpi::Position>,
    ) -> bool {
        match self.kind() {
            MenuKind::Menu(menu) => {
                ContextMenuGtkExt::show_context_menu_for_gtk_window(&menu, window, position)
            }
            MenuKind::Submenu(submenu) => {
                ContextMenuGtkExt::show_context_menu_for_gtk_window(&submenu, window, position)
            }
        }
    }

    fn gtk_menu(&self) -> gtk::Menu {
        match self.kind() {
            MenuKind::Menu(menu) => ContextMenuGtkExt::gtk_menu(&menu),
            MenuKind::Submenu(submenu) => ContextMenuGtkExt::gtk_menu(&submenu),
        }
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
impl ContextMenuGtk4Ext for dyn ContextMenu + '_ {
    fn show_context_menu_for_gtk_window(
        &self,
        window: &gtk4::Window,
        position: Option<crate::dpi::Position>,
    ) -> bool {
        match self.kind() {
            MenuKind::Menu(menu) => {
                ContextMenuGtk4Ext::show_context_menu_for_gtk_window(&menu, window, position)
            }
            MenuKind::Submenu(submenu) => {
                ContextMenuGtk4Ext::show_context_menu_for_gtk_window(&submenu, window, position)
            }
        }
    }

    fn gtk_popover_menu(&self) -> gtk4::PopoverMenu {
        match self.kind() {
            MenuKind::Menu(menu) => ContextMenuGtk4Ext::gtk_popover_menu(&menu),
            MenuKind::Submenu(submenu) => ContextMenuGtk4Ext::gtk_popover_menu(&submenu),
        }
    }
}
