use std::{cell::RefCell, mem, rc::Rc};

use crate::{
    accelerator::{Accelerator, KeyAccelerator, MenuAccelerator},
    sealed::IsMenuItemBase,
    IsMenuItem, MenuId, MenuItemKind, TextStyle,
};

/// A menu item inside a [`Menu`] or [`Submenu`] and contains only text.
///
/// [`Menu`]: crate::Menu
/// [`Submenu`]: crate::Submenu
#[derive(Clone)]
pub struct MenuItem {
    pub(crate) id: Rc<MenuId>,
    pub(crate) inner: Rc<RefCell<crate::platform_impl::MenuChild>>,
}

impl IsMenuItemBase for MenuItem {}
impl IsMenuItem for MenuItem {
    fn kind(&self) -> MenuItemKind {
        MenuItemKind::MenuItem(self.clone())
    }

    fn id(&self) -> &MenuId {
        self.id()
    }

    fn into_id(self) -> MenuId {
        self.into_id()
    }
}

impl MenuItem {
    /// Create a new menu item.
    ///
    /// - `text` could optionally contain an `&` before a character to assign this character as the mnemonic
    ///   for this menu item. To display a `&` without assigning a mnemenonic, use `&&`.
    pub fn new<S: AsRef<str>>(text: S, enabled: bool, accelerator: Option<Accelerator>) -> Self {
        let item = crate::platform_impl::MenuChild::new(
            text.as_ref(),
            enabled,
            accelerator.map(MenuAccelerator::Physical),
            None,
        );
        Self {
            id: Rc::new(item.id().clone()),
            inner: Rc::new(RefCell::new(item)),
        }
    }

    /// Create a new menu item with the specified id.
    ///
    /// - `text` could optionally contain an `&` before a character to assign this character as the mnemonic
    ///   for this menu item. To display a `&` without assigning a mnemenonic, use `&&`.
    pub fn with_id<I: Into<MenuId>, S: AsRef<str>>(
        id: I,
        text: S,
        enabled: bool,
        accelerator: Option<Accelerator>,
    ) -> Self {
        let id = id.into();
        Self {
            id: Rc::new(id.clone()),
            inner: Rc::new(RefCell::new(crate::platform_impl::MenuChild::new(
                text.as_ref(),
                enabled,
                accelerator.map(MenuAccelerator::Physical),
                Some(id),
            ))),
        }
    }

    /// Returns a unique identifier associated with this menu item.
    pub fn id(&self) -> &MenuId {
        &self.id
    }

    /// Set the text for this menu item.
    pub fn text(&self) -> String {
        self.inner.borrow().text()
    }

    /// Set the text for this menu item. `text` could optionally contain
    /// an `&` before a character to assign this character as the mnemonic
    /// for this menu item. To display a `&` without assigning a mnemenonic, use `&&`.
    pub fn set_text<S: AsRef<str>>(&self, text: S) {
        self.inner.borrow_mut().set_text(text.as_ref())
    }

    /// Set the item's label as a sequence of styled runs, so one part of the label can be
    /// de-emphasized relative to the rest. Finder uses this for entries like
    /// `Preview (default)` in its "Open with" submenu.
    ///
    /// ```
    /// # use muda::{MenuItem, TextStyle};
    /// # fn example(item: &MenuItem) {
    /// item.set_styled_text([
    ///     ("Preview", TextStyle::Normal),
    ///     (" (default)", TextStyle::Secondary),
    /// ]);
    /// # }
    /// ```
    ///
    /// [`MenuItem::text`] returns the concatenation of the runs, which is what the item
    /// actually draws. [`MenuItem::set_text`] clears the styling.
    ///
    /// ## Platform-specific:
    ///
    /// - **Windows / Linux**: the runs are concatenated and set as plain text; styles are
    ///   not visually distinguished.
    pub fn set_styled_text<S: AsRef<str>>(&self, runs: impl IntoIterator<Item = (S, TextStyle)>) {
        #[cfg(target_os = "macos")]
        {
            let runs: Vec<(String, TextStyle)> = runs
                .into_iter()
                .map(|(text, style)| (text.as_ref().to_string(), style))
                .collect();
            self.inner.borrow_mut().set_styled_text(&runs);
        }
        #[cfg(not(target_os = "macos"))]
        {
            let combined: String = runs
                .into_iter()
                .map(|(text, _)| text.as_ref().to_string())
                .collect();
            self.inner.borrow_mut().set_text(&combined);
        }
    }

    /// Get whether this menu item is enabled or not.
    pub fn is_enabled(&self) -> bool {
        self.inner.borrow().is_enabled()
    }

    /// Enable or disable this menu item.
    pub fn set_enabled(&self, enabled: bool) {
        self.inner.borrow_mut().set_enabled(enabled)
    }

    /// Set this menu item accelerator.
    ///
    /// (Note that setting an accelerator will override any existing [.set_key_accelerator()](Self::set_key_accelerator))
    pub fn set_accelerator(&self, accelerator: Option<Accelerator>) -> crate::Result<()> {
        self.inner
            .borrow_mut()
            .set_accelerator(accelerator.map(MenuAccelerator::Physical))
    }

    /// Set this menu item accelerator using a [`KeyAccelerator`].
    ///
    /// (Note that setting a key_accelerator will override any existing [.set_accelerator()](Self::set_accelerator))
    pub fn set_key_accelerator(&self, accelerator: Option<KeyAccelerator>) -> crate::Result<()> {
        self.inner
            .borrow_mut()
            .set_accelerator(accelerator.map(MenuAccelerator::Logical))
    }

    /// Convert this menu item into its menu ID.
    pub fn into_id(mut self) -> MenuId {
        // Note: `Rc::into_inner` is available from Rust 1.70
        if let Some(id) = Rc::get_mut(&mut self.id) {
            mem::take(id)
        } else {
            self.id().clone()
        }
    }
}
