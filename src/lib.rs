//! muda is a Menu Utilities library for Desktop Applications.

use accelerator::Accelerator;
use crossbeam_channel::{unbounded, Receiver, Sender};
use internal::{MenuEntry, MenuItemType};
use once_cell::sync::Lazy;
use predefined::PredfinedMenuItemType;

pub mod accelerator;
mod platform_impl;
mod predefined;
mod util;

static MENU_CHANNEL: Lazy<(Sender<MenuEvent>, Receiver<MenuEvent>)> = Lazy::new(unbounded);

/// Gets a reference to the event channel's [Receiver<MenuEvent>]
/// which can be used to listen for menu events.
pub fn menu_event_receiver<'a>() -> &'a Receiver<MenuEvent> {
    &MENU_CHANNEL.1
}

/// Describes a menu event emitted when a menu item is activated
#[derive(Debug)]
pub struct MenuEvent {
    /// Id of the menu item which triggered this event
    pub id: u32,
}

#[derive(Clone)]
pub struct Menu(platform_impl::Menu);

impl Default for Menu {
    fn default() -> Self {
        Self::new()
    }
}

impl Menu {
    /// Creates a new root menu.
    pub fn new() -> Self {
        Self(platform_impl::Menu::new())
    }

    pub fn with_items(items: &[&dyn MenuEntry]) -> Self {
        let menu = Self::new();
        menu.append_items(items);
        menu
    }

    pub fn append(&self, item: &dyn MenuEntry) {
        self.0.append(item)
    }

    pub fn append_items(&self, items: &[&dyn MenuEntry]) {
        for item in items {
            self.append(*item);
        }
    }
    pub fn prepend(&self, item: &dyn MenuEntry) {
        self.0.prepend(item)
    }

    pub fn prepend_items(&self, items: &[&dyn MenuEntry]) {
        for item in items {
            self.prepend(*item);
        }
    }

    pub fn insert(&self, item: &dyn MenuEntry, position: usize) {
        self.0.insert(item, position)
    }

    pub fn insert_items(&self, items: &[&dyn MenuEntry], position: usize) {
        for (i, item) in items.iter().enumerate() {
            self.insert(*item, position + i)
        }
    }

    /// ## Panics
    ///
    /// - If `item` has already been removed
    /// - If `item` wasn't previously [append](Menu::append)ed to this menu
    pub fn remove(&self, item: &dyn MenuEntry) {
        self.0.remove(item)
    }

    pub fn items(&self) -> Vec<Box<dyn MenuEntry>> {
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

    #[cfg(target_os = "windows")]
    pub fn show_context_menu_for_hwnd(&self, hwnd: isize, x: f64, y: f64) {
        self.0.show_context_menu_for_hwnd(hwnd, x, y)
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

#[derive(Clone)]
pub struct Submenu(platform_impl::Submenu);

unsafe impl MenuEntry for Submenu {
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
    pub fn new<S: AsRef<str>>(text: S, enabled: bool) -> Self {
        Self(platform_impl::Submenu::new(text.as_ref(), enabled))
    }

    pub fn with_items<S: AsRef<str>>(text: S, enabled: bool, items: &[&dyn MenuEntry]) -> Self {
        let menu = Self::new(text, enabled);
        menu.append_items(items);
        menu
    }

    pub fn id(&self) -> u32 {
        self.0.id()
    }

    pub fn append(&self, item: &dyn MenuEntry) {
        self.0.append(item)
    }

    pub fn append_items(&self, items: &[&dyn MenuEntry]) {
        for item in items {
            self.append(*item);
        }
    }
    pub fn prepend(&self, item: &dyn MenuEntry) {
        self.0.prepend(item)
    }

    pub fn prepend_items(&self, items: &[&dyn MenuEntry]) {
        for item in items {
            self.prepend(*item);
        }
    }

    pub fn insert(&self, item: &dyn MenuEntry, position: usize) {
        self.0.insert(item, position)
    }

    pub fn insert_items(&self, items: &[&dyn MenuEntry], position: usize) {
        for (i, item) in items.iter().enumerate() {
            self.insert(*item, position + i)
        }
    }

    pub fn remove(&self, item: &dyn MenuEntry) {
        self.0.remove(item)
    }

    pub fn items(&self) -> Vec<Box<dyn MenuEntry>> {
        self.0.items()
    }

    pub fn text(&self) -> String {
        self.0.text()
    }

    pub fn set_text<S: AsRef<str>>(&self, text: S) {
        self.0.set_text(text.as_ref())
    }

    pub fn is_enabled(&self) -> bool {
        self.0.is_enabled()
    }

    pub fn set_enabled(&self, enabled: bool) {
        self.0.set_enabled(enabled)
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

    #[cfg(target_os = "windows")]
    pub fn show_context_menu_for_hwnd(&self, hwnd: isize, x: f64, y: f64) {
        self.0.show_context_menu_for_hwnd(hwnd, x, y)
    }
}

#[derive(Clone)]
pub struct MenuItem(platform_impl::MenuItem);

unsafe impl MenuEntry for MenuItem {
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
    pub fn new<S: AsRef<str>>(text: S, enabled: bool, acccelerator: Option<Accelerator>) -> Self {
        Self(platform_impl::MenuItem::new(
            text.as_ref(),
            enabled,
            acccelerator,
        ))
    }

    pub fn id(&self) -> u32 {
        self.0.id()
    }

    pub fn text(&self) -> String {
        self.0.text()
    }

    pub fn set_text<S: AsRef<str>>(&self, text: S) {
        self.0.set_text(text.as_ref())
    }

    pub fn is_enabled(&self) -> bool {
        self.0.is_enabled()
    }

    pub fn set_enabled(&self, enabled: bool) {
        self.0.set_enabled(enabled)
    }
}

pub struct PredefinedMenuItem(platform_impl::PredefinedMenuItem);

unsafe impl MenuEntry for PredefinedMenuItem {
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

    /// A Separator in a menu
    ///
    /// ## Platform-specific:
    ///
    /// - **Windows**: Doesn't work when added in the [menu bar](crate::Menu)
    pub fn separator() -> PredefinedMenuItem {
        PredefinedMenuItem::new::<&str>(PredfinedMenuItemType::Separator, None)
    }

    pub fn minimize(text: Option<&str>) -> PredefinedMenuItem {
        PredefinedMenuItem::new(PredfinedMenuItemType::Minimize, text)
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

    fn new<S: AsRef<str>>(item: PredfinedMenuItemType, text: Option<S>) -> Self {
        Self(platform_impl::PredefinedMenuItem::new(
            item,
            text.map(|t| t.as_ref().to_string()),
        ))
    }

    fn id(&self) -> u32 {
        self.0.id()
    }

    pub fn text(&self) -> String {
        self.0.text()
    }

    pub fn set_text<S: AsRef<str>>(&self, text: S) {
        self.0.set_text(text.as_ref())
    }
}

#[derive(Clone)]
pub struct CheckMenuItem(platform_impl::CheckMenuItem);

unsafe impl MenuEntry for CheckMenuItem {
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

    pub fn id(&self) -> u32 {
        self.0.id()
    }

    pub fn text(&self) -> String {
        self.0.text()
    }

    pub fn set_text<S: AsRef<str>>(&self, text: S) {
        self.0.set_text(text.as_ref())
    }

    pub fn is_enabled(&self) -> bool {
        self.0.is_enabled()
    }

    pub fn set_enabled(&self, enabled: bool) {
        self.0.set_enabled(enabled)
    }

    pub fn is_checked(&self) -> bool {
        self.0.is_checked()
    }

    pub fn set_checked(&self, checked: bool) {
        self.0.set_checked(checked)
    }
}

/// Application metadata for the [`NativeMenuItem::About`].
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

mod internal {
    //!  **DO NOT USE:**. This module is ONLY meant to be used internally.

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

    /// # Safety
    ///
    /// **DO NOT IMPLEMENT:** This trait is ONLY meant to be implemented internally.
    pub unsafe trait MenuEntry {
        fn type_(&self) -> MenuItemType;

        fn as_any(&self) -> &(dyn std::any::Any + 'static);

        fn id(&self) -> u32;
    }
}
