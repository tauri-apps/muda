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
#[cfg(all(target_os = "linux", feature = "linux-ksni", not(feature = "gtk")))]
#[path = "linux/mod.rs"]
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

pub(crate) use self::platform::*;

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
                let id = c.borrow().id().clone();
                #[cfg(all(feature = "linux-ksni", target_os = "linux"))]
                let compat = c.borrow().compat_child();
                MenuItemKind::Submenu(Submenu {
                    id: Rc::new(id),
                    inner: c,
                    #[cfg(all(feature = "linux-ksni", target_os = "linux"))]
                    compat,
                })
            }
            MenuItemType::MenuItem => {
                let id = c.borrow().id().clone();
                #[cfg(all(feature = "linux-ksni", target_os = "linux"))]
                let compat = c.borrow().compat_child();
                MenuItemKind::MenuItem(MenuItem {
                    id: Rc::new(id),
                    inner: c,
                    #[cfg(all(feature = "linux-ksni", target_os = "linux"))]
                    compat,
                })
            }
            MenuItemType::Predefined => {
                let id = c.borrow().id().clone();
                #[cfg(all(feature = "linux-ksni", target_os = "linux"))]
                let compat = c.borrow().compat_child();
                MenuItemKind::Predefined(PredefinedMenuItem {
                    id: Rc::new(id),
                    inner: c,
                    #[cfg(all(feature = "linux-ksni", target_os = "linux"))]
                    compat,
                })
            }
            MenuItemType::Check => {
                let id = c.borrow().id().clone();
                #[cfg(all(feature = "linux-ksni", target_os = "linux"))]
                let compat = c.borrow().compat_child();
                MenuItemKind::Check(CheckMenuItem {
                    id: Rc::new(id),
                    inner: c,
                    #[cfg(all(feature = "linux-ksni", target_os = "linux"))]
                    compat,
                })
            }
            MenuItemType::Icon => {
                let id = c.borrow().id().clone();
                #[cfg(all(feature = "linux-ksni", target_os = "linux"))]
                let compat = c.borrow().compat_child();
                MenuItemKind::Icon(IconMenuItem {
                    id: Rc::new(id),
                    inner: c,
                    #[cfg(all(feature = "linux-ksni", target_os = "linux"))]
                    compat,
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

    #[cfg(all(feature = "linux-ksni", target_os = "linux"))]
    pub(crate) fn compat_child(&self) -> Arc<ArcSwap<crate::CompatMenuItem>> {
        match self {
            MenuItemKind::MenuItem(i) => i.compat.clone(),
            MenuItemKind::Submenu(i) => i.compat.clone(),
            MenuItemKind::Predefined(i) => i.compat.clone(),
            MenuItemKind::Check(i) => i.compat.clone(),
            MenuItemKind::Icon(i) => i.compat.clone(),
        }
    }
}
