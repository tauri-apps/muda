use std::sync::Arc;

use crate::{
    items::{IconType, PredefinedMenuItemType},
    CheckMenuItemState, IconMenuItemState, MenuEvent, MenuItemKind, MenuItemState, NativeIcon,
    PredefinedMenuItemState, StateCell, SubmenuState, UnsafeMenuItemKind,
};

/// A thread-safe handle to a menu tree.
#[derive(Clone)]
pub struct MenuSnapshot {
    pub items: Vec<MenuItemKindSnapshot>,
}

/// A thread-safe read projection and activation callback for a [`crate::MenuItem`].
#[derive(Clone)]
pub struct MenuItemSnapshot {
    pub(crate) state: StateCell<MenuItemState>,
    /// Activates the item and emits its [`crate::MenuEvent`].
    pub activate: Arc<dyn Fn() + Send + Sync>,
}

/// A thread-safe handle to a read-only projection of a [`crate::Submenu`].
#[derive(Clone)]
pub struct SubmenuSnapshot {
    pub(crate) state: StateCell<SubmenuState>,
}

/// A thread-safe handle to a read-only projection of a [`crate::PredefinedMenuItem`].
#[derive(Clone)]
pub struct PredefinedMenuItemSnapshot {
    pub(crate) state: StateCell<PredefinedMenuItemState>,
}

/// A thread-safe read projection and activation callback for a [`crate::CheckMenuItem`].
#[derive(Clone)]
pub struct CheckMenuItemSnapshot {
    pub(crate) state: StateCell<CheckMenuItemState>,
    /// Toggles the item and emits its [`crate::MenuEvent`].
    pub activate: Arc<dyn Fn() + Send + Sync>,
}

/// A thread-safe read projection and activation callback for an [`crate::IconMenuItem`].
#[derive(Clone)]
pub struct IconMenuItemSnapshot {
    pub(crate) state: StateCell<IconMenuItemState>,
    /// Activates the item and emits its [`crate::MenuEvent`].
    pub activate: Arc<dyn Fn() + Send + Sync>,
}

/// A thread-safe snapshot handle for any menu item kind.
#[derive(Clone)]
pub enum MenuItemKindSnapshot {
    MenuItem(MenuItemSnapshot),
    Submenu(SubmenuSnapshot),
    Predefined(PredefinedMenuItemSnapshot),
    Check(CheckMenuItemSnapshot),
    Icon(IconMenuItemSnapshot),
}

/// An icon value that can be read from a menu snapshot on any thread.
#[derive(Clone, Debug)]
pub enum SnapshotIcon {
    /// Raw RGBA icon data.
    Rgba {
        rgba: Vec<u8>,
        width: u32,
        height: u32,
    },
    /// A platform-native icon.
    Native(NativeIcon),
}

impl MenuItemSnapshot {
    /// Returns the menu item's text.
    pub fn text(&self) -> String {
        self.state.borrow().text.clone()
    }

    /// Returns whether the menu item is enabled.
    pub fn is_enabled(&self) -> bool {
        self.state.borrow().enabled
    }
}

impl SubmenuSnapshot {
    /// Returns the submenu's text.
    pub fn text(&self) -> String {
        self.state.borrow().text.clone()
    }

    /// Returns whether the submenu is enabled.
    pub fn is_enabled(&self) -> bool {
        self.state.borrow().enabled
    }

    /// Returns the submenu's icon.
    pub fn icon(&self) -> Option<SnapshotIcon> {
        self.state.borrow().icon.as_ref().and_then(Into::into)
    }

    /// Returns thread-safe snapshot handles for the submenu's items.
    pub fn items(&self) -> Vec<MenuItemKindSnapshot> {
        self.state
            .borrow()
            .children
            .iter()
            .map(UnsafeMenuItemKind::snapshot)
            .collect()
    }
}

impl PredefinedMenuItemSnapshot {
    /// Returns the predefined menu item's text.
    pub fn text(&self) -> String {
        self.state.borrow().text.clone()
    }

    /// Returns whether the predefined menu item is enabled.
    pub fn is_enabled(&self) -> bool {
        self.state.borrow().enabled
    }

    /// Returns whether this item is a separator.
    pub fn is_separator(&self) -> bool {
        matches!(
            self.state.borrow().predefined_item_type,
            PredefinedMenuItemType::Separator
        )
    }
}

impl CheckMenuItemSnapshot {
    /// Returns the check menu item's text.
    pub fn text(&self) -> String {
        self.state.borrow().text.clone()
    }

    /// Returns whether the check menu item is enabled.
    pub fn is_enabled(&self) -> bool {
        self.state.borrow().enabled
    }

    /// Returns whether the check menu item is checked.
    pub fn is_checked(&self) -> bool {
        self.state.borrow().checked
    }
}

impl IconMenuItemSnapshot {
    /// Returns the icon menu item's text.
    pub fn text(&self) -> String {
        self.state.borrow().text.clone()
    }

    /// Returns whether the icon menu item is enabled.
    pub fn is_enabled(&self) -> bool {
        self.state.borrow().enabled
    }

    /// Returns the menu item's icon.
    pub fn icon(&self) -> Option<SnapshotIcon> {
        self.state.borrow().icon.as_ref().and_then(Into::into)
    }
}

impl From<&IconType> for Option<SnapshotIcon> {
    fn from(icon: &IconType) -> Self {
        match icon {
            IconType::Custom(icon) => icon.rgba.as_ref().map(|icon| SnapshotIcon::Rgba {
                rgba: icon.rgba.clone(),
                width: icon.width,
                height: icon.height,
            }),
            IconType::Native(icon) => Some(SnapshotIcon::Native({
                #[cfg(windows)]
                {
                    *icon
                }
                #[cfg(not(windows))]
                {
                    icon.clone()
                }
            })),
        }
    }
}

impl From<&MenuItemKind> for MenuItemKindSnapshot {
    fn from(item: &MenuItemKind) -> Self {
        match item {
            MenuItemKind::MenuItem(item) => {
                let id = Arc::clone(&item.id);
                Self::MenuItem(MenuItemSnapshot {
                    state: item.state.clone(),
                    activate: Arc::new(move || {
                        MenuEvent::send(MenuEvent { id: (*id).clone() });
                    }),
                })
            }
            MenuItemKind::Submenu(item) => Self::Submenu(SubmenuSnapshot {
                state: item.state.clone(),
            }),
            MenuItemKind::Predefined(item) => Self::Predefined(PredefinedMenuItemSnapshot {
                state: item.state.clone(),
            }),
            MenuItemKind::Check(item) => {
                let id = Arc::clone(&item.id);
                let state = item.state.clone();
                Self::Check(CheckMenuItemSnapshot {
                    state: state.clone(),
                    activate: Arc::new(move || {
                        {
                            let mut state = state.borrow_mut();
                            state.checked = !state.checked;
                        }
                        MenuEvent::send(MenuEvent { id: (*id).clone() });
                    }),
                })
            }
            MenuItemKind::Icon(item) => {
                let id = Arc::clone(&item.id);
                Self::Icon(IconMenuItemSnapshot {
                    state: item.state.clone(),
                    activate: Arc::new(move || {
                        MenuEvent::send(MenuEvent { id: (*id).clone() });
                    }),
                })
            }
        }
    }
}

impl UnsafeMenuItemKind {
    /// Creates a snapshot from thread-safe ID and state fields without touching platform fields.
    pub(crate) fn snapshot(&self) -> MenuItemKindSnapshot {
        // The `Send` implementation relies on this method reading only the immutable discriminant
        // and the matching thread-safe ID and state fields. It must not clone, dereference, or
        // otherwise access an `Rc` or platform field.
        MenuItemKindSnapshot::from(&*self.0)
    }
}

#[cfg(test)]
mod tests {
    use std::thread;

    use crate::{IsMenuItem, MenuItem, UnsafeMenuItemKind};

    use super::MenuItemKindSnapshot;

    fn assert_send<T: Send>() {}
    fn assert_send_sync<T: Send + Sync>() {}

    #[test]
    fn unsafe_menu_item_kind_is_send() {
        assert_send::<UnsafeMenuItemKind>();
        assert_send_sync::<crate::StateCell<crate::menu::MenuState>>();
        assert_send_sync::<MenuItemKindSnapshot>();
    }

    #[test]
    fn shared_state_can_be_projected_off_thread_and_local_value_recovered() {
        let item = MenuItem::with_id("item", "Item", true, None);
        let wrapped = UnsafeMenuItemKind::new(item.kind());

        let wrapped = thread::spawn(move || {
            assert!(matches!(
                wrapped.snapshot(),
                MenuItemKindSnapshot::MenuItem(_)
            ));
            wrapped
        })
        .join()
        .unwrap();

        // SAFETY: the wrapper has returned to the thread where its `MenuItemKind` was created.
        let recovered = unsafe { wrapped.unwrap() };
        assert_eq!(recovered.id(), item.id());
    }
}
