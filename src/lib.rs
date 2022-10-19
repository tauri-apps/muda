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
//! let submenu = Submenu::with_items("Submenu Outer", true,&[
//!   &MenuItem::new("Menu item #1", true, Some(Accelerator::new(Some(Modifiers::ALT), Code::KeyD))),
//!   &PredefinedMenuItem::separator(),
//!   &menu_item2,
//!   &MenuItem::new("Menu item #3", true, None),
//!   &PredefinedMenuItem::separator(),
//!   &Submenu::with_items("Submenu Inner", true,&[
//!     &MenuItem::new("Submenu item #1", true, None),
//!     &PredefinedMenuItem::separator(),
//!     &menu_item2,
//!   ])
//! ]);
//!
//! ```
//!
//! Then Add your root menu to a Window on Windows and Linux Only or use it
//! as your global app menu on macOS
//!
//! ```no_run
//! // --snip--
//! #[cfg(target_os = "windows")]
//! menu.init_for_hwnd(window.hwnd() as isize);
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
//! // --snip--
//! let x = 100;
//! let y = 120;
//! #[cfg(target_os = "windows")]
//! menu.show_context_menu_for_hwnd(window.hwnd() as isize, x, y);
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
//!         _ if event.id == save_item.id() => {
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

use accelerator::Accelerator;
use crossbeam_channel::{unbounded, Receiver, Sender};
use once_cell::sync::Lazy;
use predefined::PredfinedMenuItemType;

pub mod accelerator;
mod platform_impl;
mod predefined;
mod util;

#[cfg(target_os = "macos")]
#[macro_use]
extern crate objc;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MenuItemType {
    Submenu,
    Normal,
    Check,
    Predefined,
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
    /// ```
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

/// A reciever that could be used to listen to menu events.
pub type MenuEventReceiver = Receiver<MenuEvent>;

static MENU_CHANNEL: Lazy<(Sender<MenuEvent>, MenuEventReceiver)> = Lazy::new(unbounded);

/// Gets a reference to the event channel's [MenuEventReceiver]
/// which can be used to listen for menu events.
pub fn menu_event_receiver<'a>() -> &'a MenuEventReceiver {
    &MENU_CHANNEL.1
}

/// Describes a menu event emitted when a menu item is activated
#[derive(Debug)]
pub struct MenuEvent {
    /// Id of the menu item which triggered this event
    pub id: u32,
}

/// A root menu that can be added to a Window on Windows and Linux
/// and used as the app global menu on macOS.
#[derive(Clone)]
pub struct Menu(platform_impl::Menu);

impl Default for Menu {
    fn default() -> Self {
        Self::new()
    }
}

impl Menu {
    /// Creates a new menu.
    pub fn new() -> Self {
        Self(platform_impl::Menu::new())
    }

    /// Creates a new menu with given `items`. It calls [`Menu::new`] and [`Menu::append_items`] internally.
    pub fn with_items(items: &[&dyn MenuItemExt]) -> Self {
        let menu = Self::new();
        menu.append_items(items);
        menu
    }

    /// Add a menu item to the end of this menu.
    ///
    /// ## Platform-spcific:
    ///
    /// - **macOS:** Only [`Submenu`] can be added to the menu
    pub fn append(&self, item: &dyn MenuItemExt) {
        self.0.append(item)
    }

    /// Add menu items to the end of this menu. It calls [`Menu::append`] in a loop.
    ///
    /// ## Platform-spcific:
    ///
    /// - **macOS:** Only [`Submenu`] can be added to the menu
    pub fn append_items(&self, items: &[&dyn MenuItemExt]) {
        for item in items {
            self.append(*item);
        }
    }

    /// Add a menu item to the beginning of this menu.
    ///
    /// ## Platform-spcific:
    ///
    /// - **macOS:** Only [`Submenu`] can be added to the menu
    pub fn prepend(&self, item: &dyn MenuItemExt) {
        self.0.prepend(item)
    }

    /// Add menu items to the beginning of this menu.
    /// It calls [`Menu::prepend`] on the first element and
    /// passes the rest to [`Menu::insert_items`] with position of `1`.
    ///
    /// ## Platform-spcific:
    ///
    /// - **macOS:** Only [`Submenu`] can be added to the menu
    pub fn prepend_items(&self, items: &[&dyn MenuItemExt]) {
        self.prepend(items[0]);
        self.insert_items(&items[1..], 1);
    }

    /// Insert a menu item at the specified `postion` in the menu.
    ///
    /// ## Platform-spcific:
    ///
    /// - **macOS:** Only [`Submenu`] can be added to the menu
    pub fn insert(&self, item: &dyn MenuItemExt, position: usize) {
        self.0.insert(item, position)
    }

    /// Insert menu items at the specified `postion` in the menu.
    ///
    /// ## Platform-spcific:
    ///
    /// - **macOS:** Only [`Submenu`] can be added to the menu
    pub fn insert_items(&self, items: &[&dyn MenuItemExt], position: usize) {
        for (i, item) in items.iter().enumerate() {
            self.insert(*item, position + i)
        }
    }

    /// Remove a menu item from this menu.
    ///
    /// ## Panics
    ///
    /// - If `item` has already been removed
    /// - If `item` wasn't previously [append](Menu::append)ed to this menu
    pub fn remove(&self, item: &dyn MenuItemExt) {
        self.0.remove(item)
    }

    /// Returns a list of menu items that has been added to this menu.
    pub fn items(&self) -> Vec<Box<dyn MenuItemExt>> {
        self.0.items()
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
    /// # use muda::Menu;
    /// # use windows_sys::Win32::UI::WindowsAndMessaging::{MSG, GetMessageW, TranslateMessage, DispatchMessageW, TranslateAcceleratorW};
    /// let menu = Menu::new();
    /// unsafe {
    ///     let mut msg: MSG = std::mem::zeroed();
    ///     while GetMessageW(&mut msg, 0, 0, 0) == 1 {
    ///         let translated = TranslateAcceleratorW(msg.hwnd, menu.haccel(), &msg as *const _);
    ///         if translated != 1{
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

    /// Shows this menu on a [`gtk::ApplicationWindow`]
    #[cfg(target_os = "linux")]
    pub fn show_for_gtk_window<W>(&self, w: &W)
    where
        W: gtk::prelude::IsA<gtk::ApplicationWindow>,
    {
        self.0.show_for_gtk_window(w)
    }

    /// Shows this menu on a win32 window
    #[cfg(target_os = "windows")]
    pub fn show_for_hwnd(&self, hwnd: isize) {
        self.0.show_for_hwnd(hwnd)
    }

    /// Shows this menu as a context menu inside a [`gtk::ApplicationWindow`]
    ///
    /// `x` and `y` is relatvie to the window top-left corner
    #[cfg(target_os = "linux")]
    pub fn show_context_menu_for_gtk_window<W>(&self, w: &W, x: f64, y: f64)
    where
        W: gtk::prelude::IsA<gtk::ApplicationWindow>,
        W: gtk::prelude::IsA<gtk::Widget>,
    {
        self.0.show_context_menu_for_gtk_window(w, x, y)
    }

    /// Shows this menu as a context menu inside a win32 window.
    ///
    /// `x` and `y` is relatvie to the window top-left corner
    #[cfg(target_os = "windows")]
    pub fn show_context_menu_for_hwnd(&self, hwnd: isize, x: f64, y: f64) {
        self.0.show_context_menu_for_hwnd(hwnd, x, y)
    }

    #[cfg(target_os = "macos")]
    pub fn show_context_menu_for_nsview(&self, view: cocoa::base::id) {
        self.0.show_context_menu_for_nsview(view)
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

/// A menu that can be added to a [`Menu`] or another [`Submenu`].
#[derive(Clone)]
pub struct Submenu(platform_impl::Submenu);

unsafe impl MenuItemExt for Submenu {
    fn type_(&self) -> MenuItemType {
        MenuItemType::Submenu
    }
    fn as_any(&self) -> &(dyn std::any::Any + 'static) {
        self
    }

    fn id(&self) -> u32 {
        self.id()
    }
}

impl Submenu {
    /// Create a new submenu.
    ///
    /// - `text` could optionally contain an `&` before a character to assign this character as the mnemonic
    /// for this submenu. To display a `&` without assigning a mnemenonic, use `&&`
    pub fn new<S: AsRef<str>>(text: S, enabled: bool) -> Self {
        Self(platform_impl::Submenu::new(text.as_ref(), enabled))
    }

    /// Creates a new submenu with given `items`. It calls [`Submenu::new`] and [`Submenu::append_items`] internally.
    pub fn with_items<S: AsRef<str>>(text: S, enabled: bool, items: &[&dyn MenuItemExt]) -> Self {
        let menu = Self::new(text, enabled);
        menu.append_items(items);
        menu
    }

    /// Returns a unique identifier associated with this submenu.
    pub fn id(&self) -> u32 {
        self.0.id()
    }

    /// Add a menu item to the end of this menu.
    pub fn append(&self, item: &dyn MenuItemExt) {
        self.0.append(item)
    }

    /// Add menu items to the end of this submenu. It calls [`Submenu::append`] in a loop.
    pub fn append_items(&self, items: &[&dyn MenuItemExt]) {
        for item in items {
            self.append(*item);
        }
    }

    /// Add a menu item to the beginning of this submenu.
    pub fn prepend(&self, item: &dyn MenuItemExt) {
        self.0.prepend(item)
    }

    /// Add menu items to the beginning of this submenu.
    /// It calls [`Menu::prepend`] on the first element and
    /// passes the rest to [`Menu::insert_items`] with position of `1`.
    pub fn prepend_items(&self, items: &[&dyn MenuItemExt]) {
        self.prepend(items[0]);
        self.insert_items(&items[1..], 1);
    }

    /// Insert a menu item at the specified `postion` in the submenu.
    pub fn insert(&self, item: &dyn MenuItemExt, position: usize) {
        self.0.insert(item, position)
    }

    /// Insert menu items at the specified `postion` in the submenu.
    pub fn insert_items(&self, items: &[&dyn MenuItemExt], position: usize) {
        for (i, item) in items.iter().enumerate() {
            self.insert(*item, position + i)
        }
    }

    /// Remove a menu item from this submenu.
    pub fn remove(&self, item: &dyn MenuItemExt) {
        self.0.remove(item)
    }

    /// Returns a list of menu items that has been added to this submenu.
    pub fn items(&self) -> Vec<Box<dyn MenuItemExt>> {
        self.0.items()
    }

    /// Get the text for this submenu.
    pub fn text(&self) -> String {
        self.0.text()
    }

    /// Set the text for this submenu. `text` could optionally contain
    /// an `&` before a character to assign this character as the mnemonic
    /// for this submenu. To display a `&` without assigning a mnemenonic, use `&&`
    pub fn set_text<S: AsRef<str>>(&self, text: S) {
        self.0.set_text(text.as_ref())
    }

    /// Get whether this submenu is enabled or not.
    pub fn is_enabled(&self) -> bool {
        self.0.is_enabled()
    }

    /// Enable or disable this submenu.
    pub fn set_enabled(&self, enabled: bool) {
        self.0.set_enabled(enabled)
    }

    /// Shows this submenu as a context menu inside a [`gtk::ApplicationWindow`]
    ///
    /// `x` and `y` is relatvie to the window top-left corner
    #[cfg(target_os = "linux")]
    pub fn show_context_menu_for_gtk_window<W>(&self, w: &W, x: f64, y: f64)
    where
        W: gtk::prelude::IsA<gtk::ApplicationWindow>,
        W: gtk::prelude::IsA<gtk::Widget>,
    {
        self.0.show_context_menu_for_gtk_window(w, x, y)
    }

    /// Shows this submenu as a context menu inside a win32 window.
    ///
    /// `x` and `y` is relatvie to the window top-left corner
    #[cfg(target_os = "windows")]
    pub fn show_context_menu_for_hwnd(&self, hwnd: isize, x: f64, y: f64) {
        self.0.show_context_menu_for_hwnd(hwnd, x, y)
    }

    #[cfg(target_os = "macos")]
    pub fn show_context_menu_for_nsview(&self, view: cocoa::base::id) {
        self.0.show_context_menu_for_nsview(view)
    }
}

/// A menu item inside a [`Menu`] or [`Submenu`] and contains only text.
#[derive(Clone)]
pub struct MenuItem(platform_impl::MenuItem);

unsafe impl MenuItemExt for MenuItem {
    fn type_(&self) -> MenuItemType {
        MenuItemType::Normal
    }
    fn as_any(&self) -> &(dyn std::any::Any + 'static) {
        self
    }

    fn id(&self) -> u32 {
        self.id()
    }
}

impl MenuItem {
    /// Create a new menu item.
    ///
    /// - `text` could optionally contain an `&` before a character to assign this character as the mnemonic
    /// for this menu item. To display a `&` without assigning a mnemenonic, use `&&`
    pub fn new<S: AsRef<str>>(text: S, enabled: bool, acccelerator: Option<Accelerator>) -> Self {
        Self(platform_impl::MenuItem::new(
            text.as_ref(),
            enabled,
            acccelerator,
        ))
    }

    /// Returns a unique identifier associated with this menu item.
    pub fn id(&self) -> u32 {
        self.0.id()
    }

    /// Get the text for this menu item.
    pub fn text(&self) -> String {
        self.0.text()
    }

    /// Set the text for this menu item. `text` could optionally contain
    /// an `&` before a character to assign this character as the mnemonic
    /// for this menu item. To display a `&` without assigning a mnemenonic, use `&&`
    pub fn set_text<S: AsRef<str>>(&self, text: S) {
        self.0.set_text(text.as_ref())
    }

    /// Get whether this menu item is enabled or not.
    pub fn is_enabled(&self) -> bool {
        self.0.is_enabled()
    }

    /// Enable or disable this menu item.
    pub fn set_enabled(&self, enabled: bool) {
        self.0.set_enabled(enabled)
    }
}

/// A predefined (native) menu item which has a predfined behavior by the OS or by this crate.
pub struct PredefinedMenuItem(platform_impl::PredefinedMenuItem);

unsafe impl MenuItemExt for PredefinedMenuItem {
    fn type_(&self) -> MenuItemType {
        MenuItemType::Predefined
    }
    fn as_any(&self) -> &(dyn std::any::Any + 'static) {
        self
    }

    fn id(&self) -> u32 {
        self.id()
    }
}

impl PredefinedMenuItem {
    pub fn separator() -> PredefinedMenuItem {
        PredefinedMenuItem::new::<&str>(PredfinedMenuItemType::Separator, None)
    }

    pub fn copy(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredfinedMenuItemType::Copy, text)
    }

    pub fn cut(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredfinedMenuItemType::Cut, text)
    }

    pub fn paste(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredfinedMenuItemType::Paste, text)
    }

    pub fn select_all(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredfinedMenuItemType::SelectAll, text)
    }

    pub fn undo(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredfinedMenuItemType::Undo, text)
    }

    pub fn redo(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredfinedMenuItemType::Redo, text)
    }

    pub fn minimize(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredfinedMenuItemType::Minimize, text)
    }

    pub fn maximize(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredfinedMenuItemType::Maximize, text)
    }

    pub fn fullscreen(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredfinedMenuItemType::Fullscreen, text)
    }

    pub fn hide(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredfinedMenuItemType::Hide, text)
    }

    pub fn hide_others(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredfinedMenuItemType::HideOthers, text)
    }

    pub fn show_all(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredfinedMenuItemType::ShowAll, text)
    }

    pub fn close_window(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredfinedMenuItemType::CloseWindow, text)
    }

    pub fn quit(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredfinedMenuItemType::Quit, text)
    }

    pub fn about(text: Option<&str>, metadata: Option<AboutMetadata>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredfinedMenuItemType::About(metadata), text)
    }

    pub fn services(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredfinedMenuItemType::Services, text)
    }

    fn new<S: AsRef<str>>(item: PredfinedMenuItemType, text: Option<S>) -> Self {
        Self(platform_impl::PredefinedMenuItem::new(
            item,
            text.map(|t| t.as_ref().to_string()),
        ))
    }

    fn id(&self) -> u32 {
        self.0.id()
    }

    /// Get the text for this predefined menu item.
    pub fn text(&self) -> String {
        self.0.text()
    }

    /// Set the text for this predefined menu item.
    pub fn set_text<S: AsRef<str>>(&self, text: S) {
        self.0.set_text(text.as_ref())
    }
}

/// A check menu item inside a [`Menu`] or [`Submenu`]
/// and usually contains a text and a check mark or a similar toggle
/// that corresponds to a checked and unchecked states.
#[derive(Clone)]
pub struct CheckMenuItem(platform_impl::CheckMenuItem);

unsafe impl MenuItemExt for CheckMenuItem {
    fn type_(&self) -> MenuItemType {
        MenuItemType::Check
    }
    fn as_any(&self) -> &(dyn std::any::Any + 'static) {
        self
    }

    fn id(&self) -> u32 {
        self.id()
    }
}

impl CheckMenuItem {
    /// Create a new check menu item.
    ///
    /// - `text` could optionally contain an `&` before a character to assign this character as the mnemonic
    /// for this check menu item. To display a `&` without assigning a mnemenonic, use `&&`
    pub fn new<S: AsRef<str>>(
        text: S,
        enabled: bool,
        checked: bool,
        acccelerator: Option<Accelerator>,
    ) -> Self {
        Self(platform_impl::CheckMenuItem::new(
            text.as_ref(),
            enabled,
            checked,
            acccelerator,
        ))
    }

    /// Returns a unique identifier associated with this submenu.
    pub fn id(&self) -> u32 {
        self.0.id()
    }

    /// Get the text for this check menu item.
    pub fn text(&self) -> String {
        self.0.text()
    }

    /// Get the text for this check menu item. `text` could optionally contain
    /// an `&` before a character to assign this character as the mnemonic
    /// for this check menu item. To display a `&` without assigning a mnemenonic, use `&&`
    pub fn set_text<S: AsRef<str>>(&self, text: S) {
        self.0.set_text(text.as_ref())
    }

    /// Get whether this check menu item is enabled or not.
    pub fn is_enabled(&self) -> bool {
        self.0.is_enabled()
    }

    /// Enable or disable this check menu item.
    pub fn set_enabled(&self, enabled: bool) {
        self.0.set_enabled(enabled)
    }

    /// Get whether this check menu item is checked or not.
    pub fn is_checked(&self) -> bool {
        self.0.is_checked()
    }

    /// Check or Uncheck this check menu item.
    pub fn set_checked(&self, checked: bool) {
        self.0.set_checked(checked)
    }
}

/// Application metadata for the [`PredefinedMenuItem::about`].
///
/// ## Platform-specific
///
/// - **macOS:** The metadata is ignored.
#[derive(PartialEq, Eq, Debug, Clone, Default)]
pub struct AboutMetadata {
    /// The application name.
    pub name: Option<String>,
    /// The application version.
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
