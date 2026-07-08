use std::{cell::RefCell, mem, rc::Rc};

#[cfg(all(feature = "linux-ksni", target_os = "linux"))]
use std::sync::Arc;

#[cfg(all(feature = "linux-ksni", target_os = "linux"))]
use arc_swap::ArcSwap;

use crate::{
    accelerator::{Accelerator, KeyAccelerator},
    sealed::IsMenuItemBase,
    IsMenuItem, MenuId, MenuItemKind,
};

/// A menu item inside a [`Menu`] or [`Submenu`] and contains only text.
///
/// [`Menu`]: crate::Menu
/// [`Submenu`]: crate::Submenu
#[derive(Clone)]
pub struct MenuItem {
    pub(crate) id: Rc<MenuId>,
    pub(crate) inner: Rc<RefCell<crate::platform_impl::MenuChild>>,
    #[cfg(all(feature = "linux-ksni", target_os = "linux"))]
    pub(crate) compat: Arc<ArcSwap<crate::CompatMenuItem>>,
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
    #[cfg(all(feature = "linux-ksni", target_os = "linux"))]
    pub(crate) fn compat_menu_item(
        item: &crate::platform_impl::MenuChild,
    ) -> crate::CompatMenuItem {
        crate::CompatStandardItem {
            id: item.id().0.clone(),
            label: super::strip_mnemonic(item.text()),
            enabled: item.is_enabled(),
            icon: None,
        }
        .into()
    }

    /// Create a new menu item.
    ///
    /// - `text` could optionally contain an `&` before a character to assign this character as the mnemonic
    ///   for this menu item. To display a `&` without assigning a mnemenonic, use `&&`.
    pub fn new<S: AsRef<str>>(text: S, enabled: bool, accelerator: Option<Accelerator>) -> Self {
        let item = crate::platform_impl::MenuChild::new(
            text.as_ref(),
            enabled,
            accelerator.map(KeyAccelerator::from),
            None,
        );
        #[cfg(all(feature = "linux-ksni", target_os = "linux"))]
        let compat = item.compat_child();

        Self {
            id: Rc::new(item.id().clone()),
            inner: Rc::new(RefCell::new(item)),
            #[cfg(all(feature = "linux-ksni", target_os = "linux"))]
            compat,
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
        let item = crate::platform_impl::MenuChild::new(
            text.as_ref(),
            enabled,
            accelerator.map(KeyAccelerator::from),
            Some(id),
        );
        #[cfg(all(feature = "linux-ksni", target_os = "linux"))]
        let compat = item.compat_child();

        Self {
            id: Rc::new(item.id().clone()),
            inner: Rc::new(RefCell::new(item)),
            #[cfg(all(feature = "linux-ksni", target_os = "linux"))]
            compat,
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
        let mut inner = self.inner.borrow_mut();
        inner.set_text(text.as_ref());

        #[cfg(all(feature = "linux-ksni", target_os = "linux"))]
        {
            self.compat.store(Arc::new(Self::compat_menu_item(&inner)));
            crate::send_menu_update();
        }
    }

    /// Get whether this menu item is enabled or not.
    pub fn is_enabled(&self) -> bool {
        self.inner.borrow().is_enabled()
    }

    /// Enable or disable this menu item.
    pub fn set_enabled(&self, enabled: bool) {
        let mut inner = self.inner.borrow_mut();
        inner.set_enabled(enabled);

        #[cfg(all(feature = "linux-ksni", target_os = "linux"))]
        {
            self.compat.store(Arc::new(Self::compat_menu_item(&inner)));
            crate::send_menu_update();
        }
    }

    /// Set this menu item accelerator.
    ///
    /// (Note that setting an accelerator will override any existing [.set_key_accelerator()](Self::set_key_accelerator))
    pub fn set_accelerator(&self, accelerator: Option<Accelerator>) -> crate::Result<()> {
        self.inner
            .borrow_mut()
            .set_key_accelerator(accelerator.map(KeyAccelerator::from))
    }

    /// Set this menu item accelerator using a [`KeyAccelerator`].
    ///
    /// (Note that setting a key_accelerator will override any existing [.set_accelerator()](Self::set_accelerator))
    pub fn set_key_accelerator(&self, accelerator: Option<KeyAccelerator>) -> crate::Result<()> {
        self.inner.borrow_mut().set_key_accelerator(accelerator)
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
