// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

#![allow(clippy::uninlined_format_args)]

//! muda is a Menu Utilities library for Desktop Applications.
//!
//! # Platforms supported:
//!
//! - Windows
//! - macOS
//! - Linux/BSD with GTK 3 or GTK 4
//!
//! # Platform-specific notes:
//!
//! - On macOS, menus can only be used from the main thread, and most
//!   functionality will panic if you try to use it from any other thread.
//!
//! - On Windows, accelerators don't work unless the win32 message loop calls
//!   [`TranslateAcceleratorW`](https://docs.rs/windows-sys/latest/windows_sys/Win32/UI/WindowsAndMessaging/fn.TranslateAcceleratorW.html).
//!   See [`MenuExtWindows::init_for_hwnd`](https://docs.rs/muda/latest/x86_64-pc-windows-msvc/muda/trait.MenuExtWindows.html#tymethod.init_for_hwnd) for more details.
//!
//! # Cargo features
//!
//! The Win32 and AppKit backends are always enabled on Windows and macOS, respectively.
//!
//! - `gtk3`: Enables the GTK 3 backend on Linux and BSD platforms. This is enabled by default.
//! - `gtk4`: Enables the GTK 4 backend on Linux and BSD platforms. Disable default features when
//!   enabling this feature because the defaults include `gtk3`.
//! - `libxdo`: Enables linking to `libxdo` for the GTK 3 backend. This is used by the predefined
//!   `Copy`, `Cut`, `Paste` and `SelectAll` menu items, and is enabled by default. It is not used
//!   by GTK 4.
//! - `snapshot`: Enables thread-safe menu snapshot types and methods, switching shared menu state
//!   to thread-safe synchronization.
//!
//! The `gtk3` and `gtk4` features are mutually exclusive.
//!
//! # Dependencies (Linux/BSD)
//!
//! The `gtk3` feature uses GTK 3 for menus. The `gtk4` feature uses GTK 4 for menus. `libxdo` is
//! only used by the GTK 3 backend to make the predefined `Copy`, `Cut`, `Paste` and `SelectAll`
//! menu items work when the `libxdo` feature is enabled. Be sure to install the packages for the
//! GTK backend you enabled before building:
//!
//! #### Arch Linux / Manjaro:
//!
//! ```sh
//! # GTK 3 backend
//! pacman -S gtk3 xdotool
//!
//! # GTK 4 backend
//! pacman -S gtk4
//! ```
//!
//! #### Debian / Ubuntu:
//!
//! ```sh
//! # GTK 3 backend
//! sudo apt install libgtk-3-dev libxdo-dev
//!
//! # GTK 4 backend
//! sudo apt install libgtk-4-dev
//! ```
//!
//! #### FreeBSD:
//!
//! ```sh
//! # GTK 3 backend
//! pkg install -y rust glib pkgconf gtk3 xdotool
//!
//! # GTK 4 backend
//! pkg install -y rust glib pkgconf gtk4
//! ```
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
//!             Some(Accelerator::new(Modifiers::ALT, Code::KeyD)),
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
//!         ).unwrap(),
//!     ],
//! );
//! ```
//!
//! Then add your root menu to a window on Windows or GTK, or use it as your global app menu on
//! macOS. The GTK examples below use GTK 3.
//!
//! ```no_run
//! # #[cfg(target_os = "windows")]
//! # use muda::MenuExtWindows;
//! # #[cfg(target_os = "macos")]
//! # use muda::MenuExtMacOS;
//! # #[cfg(all(
//!     feature = "gtk3",
//!     any(
//!         target_os = "linux",
//!         target_os = "dragonfly",
//!         target_os = "freebsd",
//!         target_os = "netbsd",
//!         target_os = "openbsd"
//!     )
//! ))]
//! # use muda::MenuGtkExt;
//! # let menu = muda::Menu::new();
//! # let window_hwnd = 0;
//! # #[cfg(all(
//!     feature = "gtk3",
//!     any(
//!         target_os = "linux",
//!         target_os = "dragonfly",
//!         target_os = "freebsd",
//!         target_os = "netbsd",
//!         target_os = "openbsd"
//! )))]
//! # let gtk_window = gtk::Window::builder().build();
//! # #[cfg(all(
//!     feature = "gtk3",
//!     any(
//!         target_os = "linux",
//!         target_os = "dragonfly",
//!         target_os = "freebsd",
//!         target_os = "netbsd",
//!         target_os = "openbsd"
//! )))]
//! # let vertical_gtk_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
//! // --snip--
//! #[cfg(target_os = "windows")]
//! unsafe { menu.init_for_hwnd(window_hwnd) };
//! #[cfg(all(
//!     any(
//!         target_os = "linux",
//!         target_os = "dragonfly",
//!         target_os = "freebsd",
//!         target_os = "netbsd",
//!         target_os = "openbsd"
//!     ),
//!     feature = "gtk3"
//! ))]
//! menu.init_for_gtk_window(&gtk_window, Some(&vertical_gtk_box));
//! #[cfg(target_os = "macos")]
//! menu.init_for_nsapp();
//! ```
//!
//! # Context menus (Popup menus)
//!
//! You can also use a [`Menu`] or a [`Submenu`] to show a context menu.
//!
//! ```no_run
//! # #[cfg(target_os = "windows")]
//! # use muda::ContextMenuExtWindows;
//! # #[cfg(target_os = "macos")]
//! # use muda::ContextMenuExtMacOS;
//! # #[cfg(all(
//!     feature = "gtk3",
//!     any(
//!         target_os = "linux",
//!         target_os = "dragonfly",
//!         target_os = "freebsd",
//!         target_os = "netbsd",
//!         target_os = "openbsd"
//!     )
//! ))]
//! # use muda::ContextMenuGtkExt;
//! # let menu = muda::Menu::new();
//! # let window_hwnd = 0;
//! # #[cfg(all(
//!     feature = "gtk3",
//!     any(
//!         target_os = "linux",
//!         target_os = "dragonfly",
//!         target_os = "freebsd",
//!         target_os = "netbsd",
//!         target_os = "openbsd"
//! )))]
//! # let gtk_window = gtk::Window::builder().build();
//! # #[cfg(target_os = "macos")]
//! # let nsview = std::ptr::null();
//! // --snip--
//! let position = muda::dpi::PhysicalPosition { x: 100., y: 120. };
//! #[cfg(target_os = "windows")]
//! unsafe { menu.show_context_menu_for_hwnd(window_hwnd, Some(position.into())) };
//! #[cfg(all(
//!     any(
//!         target_os = "linux",
//!         target_os = "dragonfly",
//!         target_os = "freebsd",
//!         target_os = "netbsd",
//!         target_os = "openbsd"
//!     ),
//!     feature = "gtk3"
//! ))]
//! menu.show_context_menu_for_gtk_window(&gtk_window, Some(position.into()));
//! #[cfg(target_os = "macos")]
//! unsafe { menu.show_context_menu_for_nsview(nsview, Some(position.into())) };
//! ```
//! # Processing menu events
//!
//! You can use [`MenuEvent::receiver`] to get a reference to the [`MenuEventReceiver`]
//! which you can use to listen to events when a menu item is activated
//! ```no_run
//! # use muda::MenuEvent;
//! #
//! # let save_item: muda::MenuItem = unsafe { std::mem::zeroed() };
//! if let Ok(event) = MenuEvent::receiver().try_recv() {
//!     match event.id {
//!         id if id == save_item.id() => {
//!             println!("Save menu item activated");
//!         },
//!         _ => {}
//!     }
//! }
//! ```
//!
//! ### Note for [winit] or [tao] users:
//!
//! You should use [`MenuEvent::set_event_handler`] and forward
//! the menu events to the event loop by using [`EventLoopProxy`]
//! so that the event loop is awakened on each menu event.
//!
//! ```no_run
//! # use tao::event_loop::EventLoopBuilder;
//! enum UserEvent {
//!   MenuEvent(muda::MenuEvent)
//! }
//!
//! let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
//!
//! let proxy = event_loop.create_proxy();
//! muda::MenuEvent::set_event_handler(Some(move |event| {
//!     proxy.send_event(UserEvent::MenuEvent(event));
//! }));
//! ```
//!
//! [`EventLoopProxy`]: https://docs.rs/winit/latest/winit/event_loop/struct.EventLoopProxy.html
//! [winit]: https://docs.rs/winit
//! [tao]: https://docs.rs/tao

#[cfg(all(feature = "gtk3", feature = "gtk4"))]
compile_error!("features `gtk3` and `gtk4` cannot be enabled together");

pub mod about_metadata;
pub mod accelerator;
mod builders;
mod context_menu;
mod error;
mod icon;
mod items;
mod menu_event;
mod menu_id;
mod platform_impl;
mod sealed;
#[cfg(feature = "snapshot")]
mod snapshot;
mod state_cell;
mod util;

pub use about_metadata::AboutMetadata;
pub use builders::*;
pub use context_menu::*;
pub use dpi;
pub use error::*;
pub use icon::{BadIcon, Icon, NativeIcon};
pub use items::*;
pub use menu_event::{MenuEvent, MenuEventReceiver};
pub use menu_id::MenuId;
#[cfg(feature = "snapshot")]
pub use snapshot::*;
pub(crate) use state_cell::{StateCell, WeakStateCell};
