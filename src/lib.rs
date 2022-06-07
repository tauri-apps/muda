//! muda is a Menu Utilities library for Desktop Applications.
//! # Creating root menus
//!
//! Before you can add submenus and menu items, you first need a root or a base menu.
//! ```no_run
//! let mut menu = Menu::new();
//! ```
//!
//! # Adding submens to the root menu
//!
//! Once you have a root menu you can start adding [`Submenu`]s by using [`Menu::add_submenu`].
//! ```no_run
//! let mut menu = Menu::new();
//! let file_menu = menu.add_submenu("File", true);
//! let edit_menu = menu.add_submenu("Edit", true);
//! ```
//!
//! # Aadding menu items and submenus within another submenu
//!
//! Once you have a [`Submenu`] you can star creating more [`Submenu`]s or [`TextMenuItem`]s.
//! ```no_run
//! let mut menu = Menu::new();
//!
//! let file_menu = menu.add_submenu("File", true);
//! let open_item = file_menu.add_text_item("Open", true);
//! let save_item = file_menu.add_text_item("Save", true);
//!
//! let edit_menu = menu.add_submenu("Edit", true);
//! let copy_item = file_menu.add_text_item("Copy", true);
//! let cut_item = file_menu.add_text_item("Cut", true);
//! ```
//!
//! # Add your root menu to a Window (Windows and Linux Only)
//!
//! You can use [`Menu`] to display a top menu in a Window on Windows and Linux.
//! ```no_run
//! let mut menu = Menu::new();
//! // --snip--
//! #[cfg(target_os = "windows")]
//! menu.init_for_hwnd(window.hwnd() as isize);
//! #[cfg(target_os = "linux")]
//! menu.init_for_gtk_window(&gtk_window);
//! #[cfg(target_os = "macos")]
//! menu.init_for_nsapp();
//! ```
//!
//! # Processing menu events
//!
//! You can use [`menu_event_receiver`] to get a reference to the [`MenuEventReceiver`]
//! which you can use to listen to events when a menu item is activated
//! ```no_run
//! if let Ok(event) = menu_event_receiver().try_recv() {
//!     match event.id {
//!         _ if event.id == save_item.id() => {
//!             println!("Save menu item activated");
//!         },
//!         _ => {}
//!     }
//! }
//! ```

use crossbeam_channel::{unbounded, Receiver, Sender};
use once_cell::sync::Lazy;

mod counter;
mod platform_impl;

static MENU_CHANNEL: Lazy<(Sender<MenuEvent>, Receiver<MenuEvent>)> = Lazy::new(|| unbounded());

/// A type alias to the receiver of the menu events channel.
pub type MenuEventReceiver = Receiver<MenuEvent>;

/// Gets a reference to the event channel's [MenuEventReceiver]
/// which can be used to listen for menu events.
pub fn menu_event_receiver<'a>() -> &'a MenuEventReceiver {
    &MENU_CHANNEL.1
}

/// Describes a menu event emitted when a menu item is activated
pub struct MenuEvent {
    /// Id of the menu item which triggered this event
    pub id: u64,
}

/// This is the root menu type to which you can add
/// more submenus and later be add to the top of a window (on Windows and Linux)
/// or used as the menubar menu (on macOS) or displayed as a popup menu.
///
/// # Example
///
/// ```
/// let mut menu = Menu::new();
/// let file_menu = menu.add_submenu("File", true);
/// let edit_menu = menu.add_submenu("Edit", true);
/// ```
#[derive(Clone)]
pub struct Menu(platform_impl::Menu);

impl Menu {
    /// Creates a new root menu.
    pub fn new() -> Self {
        Self(platform_impl::Menu::new())
    }

    /// Creates a new [`Submenu`] whithin this menu.
    ///
    /// ## Platform-specific:
    ///
    /// - **Windows / Linux**: The menu label can containt `&` to indicate which letter should get a generated accelerator.
    /// For example, using `&File` for the File menu would result in the label gets an underline under the `F`,
    /// and the `&` character is not displayed on menu label.
    /// Then the menu can be activated by press `Alt+F`.
    pub fn add_submenu(&mut self, label: impl AsRef<str>, enabled: bool) -> Submenu {
        Submenu(self.0.add_submenu(label, enabled))
    }

    /// Adds this menu to a [`gtk::Window`]
    ///
    /// This method adds a [`gtk::Box`] then adds a [`gtk::MenuBar`] as its first child and returns the [`gtk::Box`].
    /// So if more widgets need to be added, then [`gtk::prelude::BoxExt::pack_start`] or
    /// similiar methods should be used on the returned [`gtk::Box`].
    ///
    /// ## Safety:
    ///
    /// This should be called before anything is added to the window.
    #[cfg(target_os = "linux")]
    pub fn init_for_gtk_window<W>(&self, w: &W) -> std::rc::Rc<gtk::Box>
    where
        W: gtk::prelude::IsA<gtk::Container>,
        W: gtk::prelude::IsA<gtk::Window>,
    {
        self.0.init_for_gtk_window(w)
    }

    /// Adds this menu to a win32 window.
    ///
    /// ##  Note about accelerators:
    ///
    /// For accelerators to work, the event loop needs to call
    /// [`TranslateAcceleratorW`](windows_sys::Win32::UI::WindowsAndMessaging::TranslateAcceleratorW)
    /// with the [`HACCEL`](windows_sys::Win32::UI::WindowsAndMessaging::HACCEL) returned from [`Menu::haccel`]
    ///
    /// #### Example:
    /// ```
    /// # use windows_sys::Win32::UI::WindowsAndMessaging::{MSG, GetMessageW, TranslateMessage, DispatchMessageW };
    /// let menu = Menu::new();
    /// unsafe {
    ///     let msg: MSG = std::mem::zeroed();
    ///     while GetMessageW(&mut msg, 0, 0, 0) == 1 {
    ///         let translated = TranslateAcceleratorW(msg.hwnd, menu.haccel(), msg);
    ///         if !translated {
    ///             TranslateMessage(&msg);
    ///             DispatchMessageW(&msg);
    ///         }
    ///     }
    /// }
    /// ```
    #[cfg(target_os = "windows")]
    pub fn init_for_hwnd(&self, hwnd: isize) {
        self.0.init_for_hwnd(hwnd)
    }

    /// Returns The [`HACCEL`](windows_sys::Win32::UI::WindowsAndMessaging::HACCEL) associated with this menu
    /// It can be used with [`TranslateAcceleratorW`](windows_sys::Win32::UI::WindowsAndMessaging::TranslateAcceleratorW)
    /// in the event loop to enable accelerators
    #[cfg(target_os = "windows")]
    pub fn haccel(&self) -> windows_sys::Win32::UI::WindowsAndMessaging::HACCEL {
        self.0.haccel()
    }

    /// Adds this menu to NSApp.
    #[cfg(target_os = "macos")]
    pub fn init_for_nsapp(&self) {
        self.0.init_for_nsapp()
    }
}

/// This is a submenu within another [`Submenu`] or [`Menu`].
#[derive(Clone)]
pub struct Submenu(platform_impl::Submenu);

impl Submenu {
    /// Gets the submenus's current label.
    pub fn label(&self) -> String {
        self.0.label()
    }

    /// Sets a new label for the submenu.
    pub fn set_label(&mut self, label: impl AsRef<str>) {
        self.0.set_label(label)
    }

    /// Gets the submenu's current state, whether enabled or not.
    pub fn enabled(&self) -> bool {
        self.0.enabled()
    }

    /// Enables or disables the submenu
    pub fn set_enabled(&mut self, enabled: bool) {
        self.0.set_enabled(enabled)
    }

    /// Creates a new [`Submenu`] whithin this submenu.
    ///
    /// ## Platform-specific:
    ///
    /// - **Windows / Linux**: The menu label can containt `&` to indicate which letter should get a generated accelerator.
    /// For example, using `&File` for the File menu would result in the label gets an underline under the `F`,
    /// and the `&` character is not displayed on menu label.
    /// Then the menu can be activated by press `F` when its parent menu is active.
    pub fn add_submenu(&mut self, label: impl AsRef<str>, enabled: bool) -> Submenu {
        Submenu(self.0.add_submenu(label, enabled))
    }

    /// Creates a new [`TextMenuItem`] whithin this submenu.
    ///
    /// ## Platform-specific:
    ///
    /// - **Windows / Linux**: The menu item label can containt `&` to indicate which letter should get a generated accelerator.
    /// For example, using `&Save` for the save menu item would result in the label gets an underline under the `S`,
    /// and the `&` character is not displayed on menu item label.
    /// Then the menu item can be activated by press `S` when its parent menu is active.
    pub fn add_text_item(
        &mut self,
        label: impl AsRef<str>,
        enabled: bool,
        accelerator: Option<&str>,
    ) -> TextMenuItem {
        TextMenuItem(self.0.add_text_item(label, enabled, accelerator))
    }
}

/// This is a Text menu item within a [`Submenu`].
#[derive(Clone)]
pub struct TextMenuItem(platform_impl::TextMenuItem);

impl TextMenuItem {
    /// Gets the menu item's current label.
    pub fn label(&self) -> String {
        self.0.label()
    }

    /// Sets a new label for the menu item.
    pub fn set_label(&mut self, label: impl AsRef<str>) {
        self.0.set_label(label)
    }

    /// Gets the menu item's current state, whether enabled or not.
    pub fn enabled(&self) -> bool {
        self.0.enabled()
    }

    /// Enables or disables the menu item.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.0.set_enabled(enabled)
    }

    /// Gets the unique id for this menu item.
    pub fn id(&self) -> u64 {
        self.0.id()
    }
}
