// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

#[cfg(target_os = "windows")]
#[path = "windows/mod.rs"]
mod platform;
#[cfg(all(
    any(
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd"
    ),
    feature = "gtk"
))]
#[path = "gtk/mod.rs"]
mod platform;
#[cfg(target_os = "macos")]
#[path = "macos/mod.rs"]
mod platform;

use std::{
    cell::{Ref, RefCell, RefMut},
    rc::Rc,
};

#[cfg(all(feature = "linux-ksni", target_os = "linux"))]
use std::sync::Arc;
#[cfg(all(feature = "linux-ksni", target_os = "linux"))]
use arc_swap::ArcSwap;

use crate::{items::*, IsMenuItem, MenuItemKind, MenuItemType};

#[cfg(all(feature = "linux-ksni", target_os = "linux"))]
use crate::items::{CompatMenuItem, CompatStandardItem, CompatCheckmarkItem, CompatSubMenuItem, strip_mnemonic};

pub(crate) use self::platform::*;

#[cfg(all(feature = "linux-ksni", target_os = "linux"))]
pub use self::platform::AboutDialog;

impl dyn IsMenuItem + '_ {
    fn child(&self) -> Rc<RefCell<MenuChild>> {
        match self.kind() {
            MenuItemKind::MenuItem(i) => i.inner,
            MenuItemKind::Submenu(i) => i.inner,
            MenuItemKind::Predefined(i) => i.inner,
            MenuItemKind::Check(i) => i.inner,
            MenuItemKind::Icon(i) => i.inner,
        }
    }
}

/// Internal utilities
impl MenuChild {
    fn kind(&self, c: Rc<RefCell<MenuChild>>) -> MenuItemKind {
        match self.item_type() {
            MenuItemType::Submenu => {
                let borrowed = c.borrow();
                let id = borrowed.id().clone();
                #[cfg(all(feature = "linux-ksni", target_os = "linux"))]
                let text = borrowed.text();
                #[cfg(all(feature = "linux-ksni", target_os = "linux"))]
                let enabled = borrowed.is_enabled();
                drop(borrowed);
                MenuItemKind::Submenu(Submenu {
                    id: Rc::new(id),
                    inner: c,
                    #[cfg(all(feature = "linux-ksni", target_os = "linux"))]
                    compat: Arc::new(ArcSwap::from_pointee(CompatMenuItem::SubMenu(
                        CompatSubMenuItem {
                            label: strip_mnemonic(&text),
                            enabled,
                            submenu: Vec::new(), // Will be populated by compat_items()
                        },
                    ))),
                })
            }
            MenuItemType::MenuItem => {
                let borrowed = c.borrow();
                let id = borrowed.id().clone();
                #[cfg(all(feature = "linux-ksni", target_os = "linux"))]
                let text = borrowed.text();
                #[cfg(all(feature = "linux-ksni", target_os = "linux"))]
                let enabled = borrowed.is_enabled();
                drop(borrowed);
                MenuItemKind::MenuItem(MenuItem {
                    id: Rc::new(id.clone()),
                    inner: c,
                    #[cfg(all(feature = "linux-ksni", target_os = "linux"))]
                    compat: Arc::new(ArcSwap::from_pointee(CompatMenuItem::Standard(
                        CompatStandardItem {
                            id: id.0.clone(),
                            label: strip_mnemonic(&text),
                            enabled,
                            icon: None,
                            predefined_item_id: None,
                            about_metadata: None,
                        },
                    ))),
                })
            }
            MenuItemType::Predefined => {
                let borrowed = c.borrow();
                let id = borrowed.id().clone();
                #[cfg(all(feature = "linux-ksni", target_os = "linux"))]
                let text = borrowed.text();
                #[cfg(all(feature = "linux-ksni", target_os = "linux"))]
                let enabled = borrowed.is_enabled();
                drop(borrowed);
                MenuItemKind::Predefined(PredefinedMenuItem {
                    id: Rc::new(id.clone()),
                    inner: c,
                    #[cfg(all(feature = "linux-ksni", target_os = "linux"))]
                    compat: Arc::new(ArcSwap::from_pointee(CompatMenuItem::Standard(
                        CompatStandardItem {
                            id: id.0.clone(),
                            label: strip_mnemonic(&text),
                            enabled,
                            icon: None,
                            predefined_item_id: None, // TODO: populate from predefined type
                            about_metadata: None,
                        },
                    ))),
                })
            }
            MenuItemType::Check => {
                let borrowed = c.borrow();
                let id = borrowed.id().clone();
                #[cfg(all(feature = "linux-ksni", target_os = "linux"))]
                let text = borrowed.text();
                #[cfg(all(feature = "linux-ksni", target_os = "linux"))]
                let enabled = borrowed.is_enabled();
                #[cfg(all(feature = "linux-ksni", target_os = "linux"))]
                let checked = borrowed.is_checked();
                drop(borrowed);
                MenuItemKind::Check(CheckMenuItem {
                    id: Rc::new(id.clone()),
                    inner: c,
                    #[cfg(all(feature = "linux-ksni", target_os = "linux"))]
                    compat: Arc::new(ArcSwap::from_pointee(CompatMenuItem::Checkmark(
                        CompatCheckmarkItem {
                            id: id.0.clone(),
                            label: strip_mnemonic(&text),
                            enabled,
                            checked,
                        },
                    ))),
                })
            }
            MenuItemType::Icon => {
                let borrowed = c.borrow();
                let id = borrowed.id().clone();
                #[cfg(all(feature = "linux-ksni", target_os = "linux"))]
                let text = borrowed.text();
                #[cfg(all(feature = "linux-ksni", target_os = "linux"))]
                let enabled = borrowed.is_enabled();
                #[cfg(all(feature = "linux-ksni", target_os = "linux"))]
                let icon_bytes = borrowed.icon().map(|i| i.inner.png_data().to_vec());
                drop(borrowed);
                MenuItemKind::Icon(IconMenuItem {
                    id: Rc::new(id.clone()),
                    inner: c,
                    #[cfg(all(feature = "linux-ksni", target_os = "linux"))]
                    compat: Arc::new(ArcSwap::from_pointee(CompatMenuItem::Standard(
                        CompatStandardItem {
                            id: id.0.clone(),
                            label: strip_mnemonic(&text),
                            enabled,
                            icon: icon_bytes,
                            predefined_item_id: None,
                            about_metadata: None,
                        },
                    ))),
                })
            }
        }
    }
}

#[allow(unused)]
impl MenuItemKind {
    pub(crate) fn as_ref(&self) -> &dyn IsMenuItem {
        match self {
            MenuItemKind::MenuItem(i) => i,
            MenuItemKind::Submenu(i) => i,
            MenuItemKind::Predefined(i) => i,
            MenuItemKind::Check(i) => i,
            MenuItemKind::Icon(i) => i,
        }
    }

    pub(crate) fn child(&self) -> Ref<'_, MenuChild> {
        match self {
            MenuItemKind::MenuItem(i) => i.inner.borrow(),
            MenuItemKind::Submenu(i) => i.inner.borrow(),
            MenuItemKind::Predefined(i) => i.inner.borrow(),
            MenuItemKind::Check(i) => i.inner.borrow(),
            MenuItemKind::Icon(i) => i.inner.borrow(),
        }
    }

    pub(crate) fn child_mut(&self) -> RefMut<'_, MenuChild> {
        match self {
            MenuItemKind::MenuItem(i) => i.inner.borrow_mut(),
            MenuItemKind::Submenu(i) => i.inner.borrow_mut(),
            MenuItemKind::Predefined(i) => i.inner.borrow_mut(),
            MenuItemKind::Check(i) => i.inner.borrow_mut(),
            MenuItemKind::Icon(i) => i.inner.borrow_mut(),
        }
    }
}
