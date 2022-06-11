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
    /// - **Windows / Linux:** The menu label can containt `&` to indicate which letter should get a generated accelerator.
    /// For example, using `&File` for the File menu would result in the label gets an underline under the `F`,
    /// and the `&` character is not displayed on menu label.
    /// Then the menu can be activated by press `Alt+F`.
    pub fn add_submenu(&mut self, label: impl AsRef<str>, enabled: bool) -> Submenu {
        Submenu(self.0.add_submenu(label, enabled))
    }

    /// Adds this menu to a [`gtk::ApplicationWindow`]
    ///
    /// This method adds a [`gtk::Box`] then adds a [`gtk::MenuBar`] as its first child and returns the [`gtk::Box`].
    /// So if more widgets need to be added, then [`gtk::prelude::BoxExt::pack_start`] or
    /// similiar methods should be used on the returned [`gtk::Box`].
    ///
    /// ## Safety:
    ///
    /// This should be called before anything is added to the window.
    ///
    /// ## Panics:
    ///
    /// Panics if the gtk event loop hasn't been initialized on the thread.
    #[cfg(target_os = "linux")]
    pub fn init_for_gtk_window<W>(&self, w: &W) -> std::rc::Rc<gtk::Box>
    where
        W: gtk::prelude::IsA<gtk::ApplicationWindow>,
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

    /// Removes this menu from a [`gtk::ApplicationWindow`]
    #[cfg(target_os = "linux")]
    pub fn remove_for_gtk_window<W>(&self, w: &W)
    where
        W: gtk::prelude::IsA<gtk::ApplicationWindow>,
        W: gtk::prelude::IsA<gtk::Window>,
    {
        self.0.remove_for_gtk_window(w)
    }

    /// Removes this menu from a win32 window
    #[cfg(target_os = "windows")]
    pub fn remove_for_hwnd(&self, hwnd: isize) {
        self.0.remove_for_hwnd(hwnd)
    }

    /// Hides this menu from a [`gtk::ApplicationWindow`]
    #[cfg(target_os = "linux")]
    pub fn hide_for_gtk_window<W>(&self, w: &W)
    where
        W: gtk::prelude::IsA<gtk::ApplicationWindow>,
    {
        self.0.hide_for_gtk_window(w)
    }

    /// Hides this menu from a win32 window
    #[cfg(target_os = "windows")]
    pub fn hide_for_hwnd(&self, hwnd: isize) {
        self.0.hide_for_hwnd(hwnd)
    }

    /// Shows this menu from a [`gtk::ApplicationWindow`]
    #[cfg(target_os = "linux")]
    pub fn show_for_gtk_window<W>(&self, w: &W)
    where
        W: gtk::prelude::IsA<gtk::ApplicationWindow>,
    {
        self.0.show_for_gtk_window(w)
    }

    /// Shows this menu from a win32 window
    #[cfg(target_os = "windows")]
    pub fn show_for_hwnd(&self, hwnd: isize) {
        self.0.show_for_hwnd(hwnd)
    }

    /// Adds this menu to an NSApp.
    #[cfg(target_os = "macos")]
    pub fn init_for_nsapp(&self) {
        self.0.init_for_nsapp()
    }

    /// Removes this menu from an NSApp.
    #[cfg(target_os = "macos")]
    pub fn remove_for_nsapp(&self) {
        self.0.remove_for_nsapp()
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
    /// - **Windows / Linux:** The menu label can containt `&` to indicate which letter should get a generated accelerator.
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
    /// - **Windows / Linux:** The menu item label can containt `&` to indicate which letter should get a generated accelerator.
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

    pub fn add_native_item(&mut self, item: NativeMenuItem) {
        self.0.add_native_item(item)
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

#[non_exhaustive]
#[derive(PartialEq, Eq, Debug, Clone)]
pub enum NativeMenuItem {
    /// A native “About” menu item.
    ///
    /// The first value is the application name, and the second is its metadata.
    ///
    /// ## platform-specific:
    ///
    /// - **macOS:** the metadata is ignore.
    /// - **Windows:** Not implemented.
    About(String, AboutMetadata),
    /// A native “hide the app” menu item.
    ///
    /// ## platform-specific:
    ///
    /// - **Windows / Linux:** Unsupported.
    Hide,
    /// A native “hide all other windows" menu item.
    ///
    /// ## platform-specific:
    ///
    /// - **Windows / Linux:** Unsupported.
    HideOthers,
    /// A native "Show all windows for this app" menu item.
    ///
    /// ## platform-specific:
    ///
    /// - **Windows / Linux:** Unsupported.
    ShowAll,
    /// A native "Services" menu item.
    ///
    /// ## platform-specific:
    ///
    /// - **Windows / Linux:** Unsupported.
    Services,
    /// A native "Close current window" menu item.
    CloseWindow,
    /// A native "Quit///
    Quit,
    /// A native "Copy" menu item.
    ///
    /// ## Platform-specific:
    ///
    /// - **macOS:** macOS require this menu item to enable "Copy" keyboard shortcut for your app.
    /// - **Linux Wayland:** Not implmeneted.
    Copy,
    /// A native "Cut" menu item.
    ///
    /// ## Platform-specific:
    ///
    /// - **macOS:** macOS require this menu item to enable "Cut" keyboard shortcut for your app.
    /// - **Linux Wayland:** Not implmeneted.
    Cut,
    /// A native "Paste" menu item.
    ///
    /// ## Platform-specific:
    ///
    /// - **macOS:** macOS require this menu item to enable "Paste" keyboard shortcut for your app.
    /// - **Linux Wayland:** Not implmeneted.
    Paste,
    /// A native "Undo" menu item.
    ///
    /// ## Platform-specific:
    ///
    /// - **macOS:** macOS require this menu item to enable "Undo" keyboard shortcut for your app.
    /// - **Windows / Linux:** Unsupported.
    Undo,
    /// A native "Redo" menu item.
    ///
    /// ## Platform-specific:
    ///
    /// - **macOS:** macOS require this menu item to enable "Redo" keyboard shortcut for your app.
    /// - **Windows / Linux:** Unsupported.
    Redo,
    /// A native "Select All" menu item.
    ///
    /// ## Platform-specific:
    ///
    /// - **macOS:** macOS require this menu item to enable "Select All" keyboard shortcut for your app.
    /// - **Linux Wayland:** Not implmeneted.
    SelectAll,
    /// A native "Enter fullscreen" menu item.
    ///
    /// ## platform-specific:
    ///
    /// - **Windows / Linux:** Unsupported.
    EnterFullScreen,
    /// A native "Minimize current window" menu item.
    Minimize,
    /// A native "Zoom" menu item.
    ///
    /// ## platform-specific:
    ///
    /// - **Windows / Linux:** Unsupported.
    Zoom,
    /// Represends a Separator in the menu.
    Separator,
}

/// Application metadata for the [`NativeMenuItem::About`].
///
/// ## Platform-specific
///
/// - **macOS:** The metadata is ignored.
#[derive(PartialEq, Eq, Debug, Clone, Default)]
pub struct AboutMetadata {
    /// The application name.
    pub version: Option<String>,
    /// The authors of the application.
    pub authors: Option<Vec<String>>,
    /// Application comments.
    pub comments: Option<String>,
    /// The copyright of the application.
    pub copyright: Option<String>,
    /// The license of the application.
    pub license: Option<String>,
    /// The application website.
    pub website: Option<String>,
    /// The website label.
    pub website_label: Option<String>,
}
