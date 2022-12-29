// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

//! muda is a Menu Utilities library for Desktop Applications.
//!
//! # Example
//!
//! Create the menu and add your items
//!
//! ```no_run
//! # use muda::{Menu, Submenu, MenuItem, accelerator::{Code, Modifiers, Accelerator}, PredefinedMenuItem};
//! let menu = Menu::new();
//! let menu_item2 = MenuItem::new("Menu item #2", false, None);
//! let submenu = Submenu::with_items(
//!     "Submenu Outer",
//!     true,
//!     &[
//!         &MenuItem::new(
//!             "Menu item #1",
//!             true,
//!             Some(Accelerator::new(Some(Modifiers::ALT), Code::KeyD)),
//!         ),
//!         &PredefinedMenuItem::separator(),
//!         &menu_item2,
//!         &MenuItem::new("Menu item #3", true, None),
//!         &PredefinedMenuItem::separator(),
//!         &Submenu::with_items(
//!             "Submenu Inner",
//!             true,
//!             &[
//!                 &MenuItem::new("Submenu item #1", true, None),
//!                 &PredefinedMenuItem::separator(),
//!                 &menu_item2,
//!             ],
//!         ),
//!     ],
//! );
//! ```
//!
//! Then Add your root menu to a Window on Windows and Linux Only or use it
//! as your global app menu on macOS
//!
//! ```no_run
//! # let menu = muda::Menu::new();
//! # let window_hwnd = 0;
//! # #[cfg(target_os = "linux")]
//! # let gtk_window = gtk::ApplicationWindow::builder().build();
//! // --snip--
//! #[cfg(target_os = "windows")]
//! menu.init_for_hwnd(window_hwnd);
//! #[cfg(target_os = "linux")]
//! menu.init_for_gtk_window(&gtk_window);
//! #[cfg(target_os = "macos")]
//! menu.init_for_nsapp();
//! ```
//!
//! # Context menus (Popup menus)
//!
//! You can also use a [`Menu`] or a [`Submenu`] show a context menu.
//!
//! ```no_run
//! use muda::ContextMenu;
//! # let menu = muda::Menu::new();
//! # let window_hwnd = 0;
//! # #[cfg(target_os = "linux")]
//! # let gtk_window = gtk::ApplicationWindow::builder().build();
//! # #[cfg(target_os = "macos")]
//! # let nsview = 0 as *mut objc::runtime::Object;
//! // --snip--
//! let x = 100.0;
//! let y = 120.0;
//! #[cfg(target_os = "windows")]
//! menu.show_context_menu_for_hwnd(window_hwnd, x, y);
//! #[cfg(target_os = "linux")]
//! menu.show_context_menu_for_gtk_window(&gtk_window, x, y);
//! #[cfg(target_os = "macos")]
//! menu.show_context_menu_for_nsview(nsview, x, y);
//! ```
//! # Processing menu events
//!
//! You can use [`menu_event_receiver`] to get a reference to the [`MenuEventReceiver`]
//! which you can use to listen to events when a menu item is activated
//! ```no_run
//! # use muda::menu_event_receiver;
//! #
//! # let save_item: muda::MenuItem = unsafe { std::mem::zeroed() };
//! if let Ok(event) = menu_event_receiver().try_recv() {
//!     match event.id {
//!         id if id == save_item.id() => {
//!             println!("Save menu item activated");
//!         },
//!         _ => {}
//!     }
//! }
//! ```
//!
//! # Accelerators on Windows
//!
//! Accelerators don't work unless the win32 message loop calls
//! [`TranslateAcceleratorW`](windows_sys::Win32::UI::WindowsAndMessaging::TranslateAcceleratorW)
//!
//! See [`Menu::init_for_hwnd`] for more details

use crossbeam_channel::{unbounded, Receiver, Sender};
use once_cell::sync::Lazy;

pub mod accelerator;
mod check_menu_item;
mod error;
mod icon_menu_item;
mod menu;
mod menu_item;
mod platform_impl;
mod predefined;
mod submenu;
mod util;

#[cfg(target_os = "macos")]
#[macro_use]
extern crate objc;

pub use self::error::*;
pub use check_menu_item::CheckMenuItem;
pub use icon_menu_item::IconMenuItem;
pub use menu::Menu;
pub mod icon;
pub use menu_item::MenuItem;
pub use predefined::{AboutMetadata, PredefinedMenuItem};
pub use submenu::Submenu;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MenuItemType {
    Submenu,
    Normal,
    Predefined,
    Check,
    Icon,
}

impl Default for MenuItemType {
    fn default() -> Self {
        Self::Normal
    }
}

/// A trait that defines a generic item in a menu, which may be one of [MenuItemType]
///
/// # Safety
///
/// This trait is ONLY meant to be implemented internally.
// TODO(amrbashir): first person to replace this trait with an enum while keeping `Menu.append_items`
// taking mix of types (`MenuItem`, `CheckMenuItem`, `Submenu`...etc) in the same call, gets a cookie.
pub unsafe trait MenuItemExt {
    /// Get the type of this menu entry
    fn type_(&self) -> MenuItemType;

    /// Casts this menu entry to [`Any`](std::any::Any).
    ///
    /// You can use this to get the concrete underlying type
    /// when calling [`Menu::items`] or [`Submenu::items`] by calling [`downcast_ref`](https://doc.rust-lang.org/std/any/trait.Any.html#method.downcast_ref-1)
    ///
    /// ## Example
    ///
    /// ```no_run
    /// # use muda::{Submenu, MenuItem};
    /// let submenu = Submenu::new("Submenu", true);
    /// let item = MenuItem::new("Text", true, None);
    /// submenu.append(&item);
    /// // --snip--
    /// let item = &submenu.items()[0];
    /// let item = item.as_any().downcast_ref::<MenuItem>().unwrap();
    /// item.set_text("New text")
    /// ````
    fn as_any(&self) -> &(dyn std::any::Any + 'static);

    /// Returns the id associated with this menu entry
    fn id(&self) -> u32;
}

pub trait ContextMenu {
    /// Get the popup [`HMENU`] for this menu.
    ///
    /// [`HMENU`]: windows_sys::Win32::UI::WindowsAndMessaging::HMENU
    #[cfg(target_os = "windows")]
    fn hpopupmenu(&self) -> windows_sys::Win32::UI::WindowsAndMessaging::HMENU;

    /// Shows this menu as a context menu inside a win32 window.
    ///
    /// `x` and `y` are relative to the window's top-left corner.
    #[cfg(target_os = "windows")]
    fn show_context_menu_for_hwnd(&self, hwnd: isize, x: f64, y: f64);

    /// Attach the menu subclass handler to the given hwnd
    /// so you can recieve events from that window using [menu_event_receiver]
    ///
    /// This can be used along with [`ContextMenu::hpopupmenu`] when implementing a tray icon menu.
    #[cfg(target_os = "windows")]
    fn attach_menu_subclass_for_hwnd(&self, hwnd: isize);

    /// Remove the menu subclass handler from the given hwnd
    #[cfg(target_os = "windows")]
    fn detach_menu_subclass_from_hwnd(&self, hwnd: isize);

    /// Shows this menu as a context menu inside a [`gtk::ApplicationWindow`]
    ///
    /// `x` and `y` are relative to the window's top-left corner.
    #[cfg(target_os = "linux")]
    fn show_context_menu_for_gtk_window(&self, w: &gtk::ApplicationWindow, x: f64, y: f64);
    /// Get the underlying gtk menu reserved for context menus.
    #[cfg(target_os = "linux")]
    fn gtk_context_menu(&self) -> gtk::Menu;

    /// Shows this menu as a context menu for the specified `NSView`.
    ///
    /// The menu will be shown at the coordinates of the current event
    /// (the click which triggered the menu to be shown).
    #[cfg(target_os = "macos")]
    fn show_context_menu_for_nsview(&self, view: cocoa::base::id, x: f64, y: f64);

    #[cfg(target_os = "macos")]
    fn ns_menu(&self) -> *mut std::ffi::c_void;
}

/// Describes a menu event emitted when a menu item is activated
#[derive(Debug)]
pub struct MenuEvent {
    /// Id of the menu item which triggered this event
    pub id: u32,
}

/// A reciever that could be used to listen to menu events.
pub type MenuEventReceiver = Receiver<MenuEvent>;

static MENU_CHANNEL: Lazy<(Sender<MenuEvent>, MenuEventReceiver)> = Lazy::new(unbounded);

/// Gets a reference to the event channel's [MenuEventReceiver]
/// which can be used to listen for menu events.
pub fn menu_event_receiver<'a>() -> &'a MenuEventReceiver {
    &MENU_CHANNEL.1
}
