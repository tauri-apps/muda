use crate::{accelerator::Accelerator, MenuItemExt, MenuItemType};

/// A menu item inside a [`Menu`] or [`Submenu`] and contains only text.
///
/// [`Menu`]: crate::Menu
/// [`Submenu`]: crate::Submenu
#[derive(Clone)]
pub struct MenuItem(pub(crate) crate::platform_impl::MenuItem);

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
        Self(crate::platform_impl::MenuItem::new(
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
