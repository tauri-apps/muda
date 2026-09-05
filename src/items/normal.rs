use std::{cell::RefCell, mem, rc::Rc};

#[cfg(target_os = "macos")]
use objc2::{rc::Retained, Message};
#[cfg(target_os = "macos")]
use objc2_foundation::NSAttributedString;

use crate::{
    accelerator::{Accelerator, KeyAccelerator, MenuAccelerator},
    platform_impl::PlatformMenuItem,
    sealed::IsMenuItemBase,
    util, ClickAction, IsMenuItem, MenuId, MenuItemBuilder, MenuItemKind, TextStyle,
};

/// A menu item inside a [`Menu`] or [`Submenu`] and contains only text.
///
/// [`Menu`]: crate::Menu
/// [`Submenu`]: crate::Submenu
#[derive(Clone)]
pub struct MenuItem {
    pub(crate) id: Rc<MenuId>,
    pub(crate) state: Rc<RefCell<MenuItemState>>,
    pub(crate) platform: Rc<RefCell<PlatformMenuItem>>,
}

/// Shared state of a [`MenuItem`].
#[derive(Debug, Clone)]
pub(crate) struct MenuItemState {
    pub text: String,
    pub enabled: bool,
    pub accelerator: Option<MenuAccelerator>,
    pub styled_text: Option<Vec<(String, TextStyle)>>,
    /// macOS-only fully custom attributed title set via
    /// [`MenuItem::set_attributed_title`]. When present it takes precedence over
    /// `text`/`styled_text` when the native item is (re)created.
    #[cfg(target_os = "macos")]
    pub attributed_title: Option<Retained<NSAttributedString>>,
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
            styled_text: None,
            #[cfg(target_os = "macos")]
            attributed_title: None,
        };
        let click = ClickAction::Emit(id.clone());
        let platform = PlatformMenuItem::new(click);

        Self {
            id: Rc::new(id),
            state: Rc::new(RefCell::new(state)),
            platform: Rc::new(RefCell::new(platform)),
        }
    }

    /// Returns a unique identifier associated with this menu item.
    pub fn id(&self) -> &MenuId {
        &self.id
    }

    /// Get the text for this menu item.
    pub fn text(&self) -> String {
        self.platform
            .borrow()
            .text()
            .unwrap_or_else(|| self.state.borrow().text.clone())
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
            state.styled_text = None;
            #[cfg(target_os = "macos")]
            {
                state.attributed_title = None;
            }
            state.accelerator.clone()
        };

        self.platform
            .borrow_mut()
            .set_text(text.as_ref(), accelerator.as_ref())
    }

    /// Set the item's label as styled parts. On Windows and Linux the parts render as plain text.
    pub fn set_styled_text<S: AsRef<str>>(&self, parts: impl IntoIterator<Item = (S, TextStyle)>) {
        let parts = parts
            .into_iter()
            .map(|(text, style)| (text.as_ref().to_string(), style))
            .collect::<Vec<_>>();
        let (text, accelerator) = {
            let mut state = self.state.borrow_mut();
            state.text = parts.iter().map(|(text, _)| text.as_str()).collect();
            state.styled_text = Some(parts.clone());
            #[cfg(target_os = "macos")]
            {
                state.attributed_title = None;
            }
            (state.text.clone(), state.accelerator.clone())
        };
        self.platform
            .borrow_mut()
            .set_styled_text(&text, &parts, accelerator.as_ref())
    }

    /// Set the item's label to a fully custom [`NSAttributedString`] (macOS only).
    ///
    /// This is an escape hatch for layouts and colors that the semantic
    /// [`set_styled_text`](Self::set_styled_text) API cannot express — for
    /// example a right-aligned trailing segment (built with an `NSParagraphStyle`
    /// that has a right-aligned `NSTextTab` and a `\t` separator, as the system
    /// battery menu does) or a custom `NSForegroundColorAttributeName` used to
    /// tint a whole row. Because the caller supplies raw attributes, it is the
    /// caller's responsibility to keep the label legible in light and dark modes,
    /// under increased contrast, and when the system menu font changes.
    ///
    /// The attributed title takes precedence over any [`set_text`](Self::set_text)
    /// or [`set_styled_text`](Self::set_styled_text) value until it is cleared.
    /// Pass `None` to clear it and fall back to the plain text.
    #[cfg(target_os = "macos")]
    pub fn set_attributed_title(&self, title: Option<&NSAttributedString>) {
        let title = {
            let mut state = self.state.borrow_mut();
            state.attributed_title = title.map(|t| t.retain());
            state.attributed_title.clone()
        };
        self.platform
            .borrow_mut()
            .set_attributed_title(title.as_deref());
    }

    /// Get whether this menu item is enabled or not.
    pub fn is_enabled(&self) -> bool {
        self.platform
            .borrow()
            .is_enabled()
            .unwrap_or_else(|| self.state.borrow().enabled)
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
        // Note: `Rc::into_inner` is available from Rust 1.70
        if let Some(id) = Rc::get_mut(&mut self.id) {
            mem::take(id)
        } else {
            self.id().clone()
        }
    }
}
