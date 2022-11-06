use crate::{accelerator::Accelerator, MenuItemExt, MenuItemType};

/// A check menu item inside a [`Menu`] or [`Submenu`]
/// and usually contains a text and a check mark or a similar toggle
/// that corresponds to a checked and unchecked states.
///
/// [`Menu`]: crate::Menu
/// [`Submenu`]: crate::Submenu
#[derive(Clone)]
pub struct CheckMenuItem(pub(crate) crate::platform_impl::CheckMenuItem);

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
        Self(crate::platform_impl::CheckMenuItem::new(
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
