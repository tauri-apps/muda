use std::{cell::RefCell, mem, rc::Rc, sync::Arc};

use crate::{
    accelerator::{Accelerator, KeyAccelerator, MenuAccelerator},
    platform_impl::PlatformMenuItem,
    util, IsMenuItem, MenuId, MenuItemAction, MenuItemBuilder, MenuItemKind, StateCell,
};

/// A menu item inside a [`Menu`] or [`Submenu`] and contains only text.
///
/// [`Menu`]: crate::Menu
/// [`Submenu`]: crate::Submenu
#[derive(Clone)]
pub struct MenuItem {
    pub(crate) id: Arc<MenuId>,
    pub(crate) state: StateCell<MenuItemState>,
    pub(crate) platform: Rc<RefCell<PlatformMenuItem>>,
}

/// Shared state of a [`MenuItem`].
#[derive(Debug, Clone)]
pub(crate) struct MenuItemState {
    pub text: String,
    pub enabled: bool,
    pub accelerator: Option<MenuAccelerator>,
}

impl crate::sealed::Sealed for MenuItem {}
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
    /// Returns a new [`MenuItemBuilder`].
    pub fn builder() -> MenuItemBuilder {
        MenuItemBuilder::new()
    }

    /// Create a new menu item.
    ///
    /// - `text` could optionally contain an `&` before a character to assign this character as the mnemonic
    ///   for this menu item. To display a `&` without assigning a mnemenonic, use `&&`.
    pub fn new<S: AsRef<str>>(text: S, enabled: bool, accelerator: Option<Accelerator>) -> Self {
        Self::new_inner(
            None,
            text.as_ref(),
            enabled,
            accelerator.map(MenuAccelerator::Physical),
        )
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
        Self::new_inner(
            Some(id.into()),
            text.as_ref(),
            enabled,
            accelerator.map(MenuAccelerator::Physical),
        )
    }

    fn new_inner(
        id: Option<MenuId>,
        text: &str,
        enabled: bool,
        accelerator: Option<MenuAccelerator>,
    ) -> Self {
        let id = util::next_id(id);
        let state = MenuItemState {
            text: text.to_string(),
            enabled,
            accelerator,
        };
        let click = MenuItemAction::Emit(id.clone());
        let platform = PlatformMenuItem::new(click);

        Self {
            id: Arc::new(id),
            state: StateCell::new(state),
            platform: Rc::new(RefCell::new(platform)),
        }
    }

    /// Returns a unique identifier associated with this menu item.
    pub fn id(&self) -> &MenuId {
        &self.id
    }

    /// Get the text for this menu item.
    pub fn text(&self) -> String {
        let text = self.platform.borrow().text();
        text.unwrap_or_else(|| self.state.borrow().text.clone())
    }

    /// Set the text for this menu item. `text` could optionally contain
    /// an `&` before a character to assign this character as the mnemonic
    /// for this menu item. To display a `&` without assigning a mnemenonic, use `&&`.
    pub fn set_text<S: AsRef<str>>(&self, text: S) {
        // Shared state is written first and its guard released before the platform is called:
        // the platform may reach back into state, and holding both at once is what B1 forbids.
        let accelerator = {
            let mut state = self.state.borrow_mut();
            state.text = text.as_ref().to_string();
            state.accelerator.clone()
        };

        self.platform
            .borrow_mut()
            .set_text(text.as_ref(), accelerator.as_ref())
    }

    /// Set the menu item's label to a fully custom
    /// [`NSAttributedString`](objc2_foundation::NSAttributedString) (macOS only).
    ///
    /// This is an escape hatch for layouts and colors that plain
    /// [`set_text`](Self::set_text) cannot express — for example a right-aligned trailing
    /// segment (built with an `NSParagraphStyle` that has a right-aligned `NSTextTab` and
    /// a `\t` separator, as the system battery menu does) or a custom
    /// `NSForegroundColorAttributeName` used to tint a whole row. Because the caller
    /// supplies raw attributes, it is the caller's responsibility to keep the label
    /// legible in light and dark modes, under increased contrast, and when the system
    /// menu font changes.
    ///
    /// This and [`set_text`](Self::set_text) write the same label, so the last one
    /// called wins. Pass `None` to clear the custom title and fall back to the plain text.
    #[cfg(target_os = "macos")]
    pub fn set_attributed_title(&self, title: Option<&objc2_foundation::NSAttributedString>) {
        self.platform.borrow_mut().set_attributed_title(title)
    }

    /// Get whether this menu item is enabled or not.
    pub fn is_enabled(&self) -> bool {
        let enabled = self.platform.borrow().is_enabled();
        enabled.unwrap_or_else(|| self.state.borrow().enabled)
    }

    /// Enable or disable this menu item.
    pub fn set_enabled(&self, enabled: bool) {
        self.state.borrow_mut().enabled = enabled;
        self.platform.borrow_mut().set_enabled(enabled)
    }

    /// Set this menu item accelerator.
    ///
    /// (Note that setting an accelerator will override any existing [.set_key_accelerator()](Self::set_key_accelerator))
    pub fn set_accelerator(&self, accelerator: Option<Accelerator>) -> crate::Result<()> {
        self.set_accelerator_inner(accelerator.map(MenuAccelerator::Physical))
    }

    /// Set this menu item accelerator using a [`KeyAccelerator`].
    ///
    /// (Note that setting a key_accelerator will override any existing [.set_accelerator()](Self::set_accelerator))
    pub fn set_key_accelerator(&self, accelerator: Option<KeyAccelerator>) -> crate::Result<()> {
        self.set_accelerator_inner(accelerator.map(MenuAccelerator::Logical))
    }

    fn set_accelerator_inner(&self, accelerator: Option<MenuAccelerator>) -> crate::Result<()> {
        let text = {
            let mut state = self.state.borrow_mut();
            state.accelerator = accelerator.clone();
            state.text.clone()
        };

        self.platform
            .borrow_mut()
            .set_accelerator(&text, accelerator.as_ref())
    }

    /// Convert this menu item into its menu ID.
    pub fn into_id(mut self) -> MenuId {
        if let Some(id) = Arc::get_mut(&mut self.id) {
            mem::take(id)
        } else {
            self.id().clone()
        }
    }
}
