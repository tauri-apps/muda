use std::{cell::RefCell, rc::Rc};

use crate::{accelerator::Accelerator, IsMenuItem, MenuId, MenuItemKind};

/// A menu item inside a [`Menu`] or [`Submenu`] and contains only text.
///
/// [`Menu`]: crate::Menu
/// [`Submenu`]: crate::Submenu
#[derive(Clone)]
pub struct MenuItem(pub(crate) Rc<RefCell<crate::platform_impl::MenuChild>>);

unsafe impl IsMenuItem for MenuItem {
    fn kind(&self) -> MenuItemKind {
        MenuItemKind::MenuItem(self.clone())
    }
}

impl MenuItem {
    /// Create a new menu item.
    ///
    /// - `text` could optionally contain an `&` before a character to assign this character as the mnemonic
    /// for this menu item. To display a `&` without assigning a mnemenonic, use `&&`.
    pub fn new<S: AsRef<str>>(text: S, enabled: bool, acccelerator: Option<Accelerator>) -> Self {
        Self(Rc::new(RefCell::new(crate::platform_impl::MenuChild::new(
            text.as_ref(),
            enabled,
            acccelerator,
            None,
        ))))
    }

    /// Create a new menu item with the specified id.
    ///
    /// - `text` could optionally contain an `&` before a character to assign this character as the mnemonic
    /// for this menu item. To display a `&` without assigning a mnemenonic, use `&&`.
    pub fn with_id<S: AsRef<str>>(
        id: MenuId,
        text: S,
        enabled: bool,
        acccelerator: Option<Accelerator>,
    ) -> Self {
        Self(Rc::new(RefCell::new(crate::platform_impl::MenuChild::new(
            text.as_ref(),
            enabled,
            acccelerator,
            Some(id),
        ))))
    }

    /// Returns a unique identifier associated with this menu item.
    pub fn id(&self) -> MenuId {
        self.0.borrow().id()
    }

    /// Set the text for this menu item.
    pub fn text(&self) -> String {
        self.0.borrow().text()
    }

    /// Set the text for this menu item. `text` could optionally contain
    /// an `&` before a character to assign this character as the mnemonic
    /// for this menu item. To display a `&` without assigning a mnemenonic, use `&&`.
    pub fn set_text<S: AsRef<str>>(&self, text: S) {
        self.0.borrow_mut().set_text(text.as_ref())
    }

    /// Get whether this menu item is enabled or not.
    pub fn is_enabled(&self) -> bool {
        self.0.borrow().is_enabled()
    }

    /// Enable or disable this menu item.
    pub fn set_enabled(&self, enabled: bool) {
        self.0.borrow_mut().set_enabled(enabled)
    }

    /// Set this menu item accelerator.
    pub fn set_accelerator(&self, acccelerator: Option<Accelerator>) -> crate::Result<()> {
        self.0.borrow_mut().set_accelerator(acccelerator)
    }
}
