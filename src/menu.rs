use crate::{ContextMenu, MenuItemExt};

/// A root menu that can be added to a Window on Windows and Linux
/// and used as the app global menu on macOS.
#[derive(Clone)]
pub struct Menu(crate::platform_impl::Menu);

impl Default for Menu {
    fn default() -> Self {
        Self::new()
    }
}

impl ContextMenu for Menu {
    #[cfg(target_os = "windows")]
    fn hpopupmenu(&self) -> windows_sys::Win32::UI::WindowsAndMessaging::HMENU {
        self.0.hpopupmenu()
    }

    #[cfg(target_os = "windows")]
    fn show_context_menu_for_hwnd(&self, hwnd: isize, x: f64, y: f64) {
        self.0.show_context_menu_for_hwnd(hwnd, x, y)
    }

    #[cfg(target_os = "windows")]
    fn attach_menu_subclass_for_hwnd(&self, hwnd: isize) {
        self.0.attach_menu_subclass_for_hwnd(hwnd)
    }

    #[cfg(target_os = "windows")]
    fn detach_menu_subclass_from_hwnd(&self, hwnd: isize) {
        self.0.detach_menu_subclass_from_hwnd(hwnd)
    }

    #[cfg(target_os = "linux")]
    fn show_context_menu_for_gtk_window(&self, w: &gtk::ApplicationWindow, x: f64, y: f64) {
        self.0.show_context_menu_for_gtk_window(w, x, y)
    }

    #[cfg(target_os = "linux")]
    fn gtk_context_menu(&self) -> gtk::Menu {
        self.0.gtk_context_menu()
    }
}

impl Menu {
    /// Creates a new menu.
    pub fn new() -> Self {
        Self(crate::platform_impl::Menu::new())
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

    /// Add menu items to the end of this menu. It calls [`Menu::append`] in a loop internally.
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

    /// Add menu items to the beginning of this menu. It calls [`Menu::insert_items`] with position of `0` internally.
    ///
    /// ## Platform-spcific:
    ///
    /// - **macOS:** Only [`Submenu`] can be added to the menu
    pub fn prepend_items(&self, items: &[&dyn MenuItemExt]) {
        self.insert_items(items, 0);
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
