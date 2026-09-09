// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use crate::{dpi, sealed, Menu, Submenu};

#[cfg(feature = "snapshot")]
use crate::MenuSnapshotHandle;

/// A trait that defines the behavior and interface for context menus across different platforms.
pub trait ContextMenu: sealed::Sealed {
    /// Get the popup [`HMENU`] for this menu.
    ///
    /// The returned [`HMENU`] is valid as long as the `ContextMenu` is.
    ///
    /// [`HMENU`]: windows_sys::Win32::UI::WindowsAndMessaging::HMENU
    #[cfg(target_os = "windows")]
    fn hpopupmenu(&self) -> isize;

    /// Shows this menu as a context menu inside a win32 window.
    ///
    /// - `position` is relative to the window top-left corner, if `None`, the cursor position is used.
    ///
    /// Returns `true` if menu tracking ended because an item was selected, and `false` if menu tracking was cancelled for any reason.
    ///
    /// # Safety
    ///
    /// The `hwnd` must be a valid window HWND.
    #[cfg(target_os = "windows")]
    unsafe fn show_context_menu_for_hwnd(
        &self,
        hwnd: isize,
        position: Option<dpi::Position>,
    ) -> bool;

    /// Attach the menu subclass handler to the given hwnd
    /// so you can receive events from that window using [`crate::MenuEvent::receiver`]
    ///
    /// This can be used along with [`ContextMenu::hpopupmenu`] when implementing a tray icon menu.
    ///
    /// # Safety
    ///
    /// The `hwnd` must be a valid window HWND.
    #[cfg(target_os = "windows")]
    unsafe fn attach_menu_subclass_for_hwnd(&self, hwnd: isize);

    /// Remove the menu subclass handler from the given hwnd
    ///
    /// The view must be a pointer to a valid `NSView`.
    ///
    /// # Safety
    ///
    /// The `hwnd` must be a valid window HWND.
    #[cfg(target_os = "windows")]
    unsafe fn detach_menu_subclass_from_hwnd(&self, hwnd: isize);

    /// Shows this menu as a context menu inside a [`gtk::Window`].
    ///
    /// - `position` is relative to the window top-left corner, if `None`, the cursor position is used.
    ///
    /// Returns `true` if menu tracking ended because an item was selected or clicked outside the menu to dismiss it.
    ///
    /// Returns `false` if menu tracking was cancelled for any reason.
    #[cfg(all(
        any(
            target_os = "linux",
            target_os = "dragonfly",
            target_os = "freebsd",
            target_os = "netbsd",
            target_os = "openbsd"
        ),
        any(all(feature = "gtk3", not(feature = "gtk4")), feature = "gtk4")
    ))]
    fn show_context_menu_for_gtk_window(
        &self,
        w: &gtk::Window,
        position: Option<dpi::Position>,
    ) -> bool;

    /// Get the underlying GTK 3 menu reserved for context menus.
    ///
    /// The returned [`gtk::Menu`] is valid as long as the `ContextMenu` is.
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
    fn gtk_context_menu(&self) -> gtk::Menu;

    /// Get the underlying GTK 4 popover menu reserved for context menus.
    ///
    /// The returned [`gtk::PopoverMenu`] is valid as long as the `ContextMenu` is.
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
    fn gtk_context_menu(&self) -> gtk::PopoverMenu;

    /// Shows this menu as a context menu for the specified `NSView`.
    ///
    /// - `position` is relative to the window top-left corner, if `None`, the cursor position is used.
    ///
    /// Returns `true` if menu tracking ended because an item was selected, and `false` if menu tracking was cancelled for any reason.
    ///
    /// # Safety
    ///
    /// The view must be a pointer to a valid `NSView`.
    #[cfg(target_os = "macos")]
    unsafe fn show_context_menu_for_nsview(
        &self,
        view: *const std::ffi::c_void,
        position: Option<dpi::Position>,
    ) -> bool;

    /// Get the underlying NSMenu reserved for context menus.
    ///
    /// The returned pointer is valid for as long as the `ContextMenu` is. If
    /// you need it to be alive for longer, retain it.
    #[cfg(target_os = "macos")]
    fn ns_menu(&self) -> *mut std::ffi::c_void;

    /// Cast this context menu to a [`Menu`], and returns `None` if it wasn't.
    fn as_menu(&self) -> Option<&Menu> {
        None
    }

    /// Casts this context menu to a [`Menu`], and panics if it wasn't.
    fn as_menu_unchecked(&self) -> &Menu {
        self.as_menu().expect("Not a Menu")
    }

    /// Cast this context menu to a [`Submenu`], and returns `None` if it wasn't.
    fn as_submenu(&self) -> Option<&Submenu> {
        None
    }

    /// Casts this context menu to a [`Submenu`], and panics if it wasn't.
    fn as_submenu_unchecked(&self) -> &Menu {
        self.as_menu().expect("Not a Submenu")
    }

    /// Returns a thread-safe snapshot handle for this menu tree.
    #[cfg(feature = "snapshot")]
    fn snapshot_handle(&self) -> MenuSnapshotHandle;
}
