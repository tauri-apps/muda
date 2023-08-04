// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

#[cfg(target_os = "windows")]
#[path = "windows/mod.rs"]
mod platform;
#[cfg(target_os = "linux")]
#[path = "gtk/mod.rs"]
mod platform;
#[cfg(target_os = "macos")]
#[path = "macos/mod.rs"]
mod platform;

use std::{
    cell::{Ref, RefCell, RefMut},
    rc::Rc,
};

use crate::{items::*, IsMenuItem, MenuItemKind, MenuItemType};

pub(crate) use self::platform::*;

impl dyn IsMenuItem + '_ {
    fn child(&self) -> Rc<RefCell<MenuChild>> {
        match self.kind() {
            MenuItemKind::MenuItem(i) => i.0,
            MenuItemKind::Submenu(i) => i.0,
            MenuItemKind::Predefined(i) => i.0,
            MenuItemKind::Check(i) => i.0,
            MenuItemKind::Icon(i) => i.0,
        }
    }
}

/// Internal utilities
impl MenuChild {
    fn kind(&self, c: Rc<RefCell<MenuChild>>) -> MenuItemKind {
        match self.item_type() {
            MenuItemType::Submenu => MenuItemKind::Submenu(Submenu(c)),
            MenuItemType::MenuItem => MenuItemKind::MenuItem(MenuItem(c)),
            MenuItemType::Predefined => MenuItemKind::Predefined(PredefinedMenuItem(c)),
            MenuItemType::Check => MenuItemKind::Check(CheckMenuItem(c)),
            MenuItemType::Icon => MenuItemKind::Icon(IconMenuItem(c)),
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

    pub(crate) fn child(&self) -> Ref<MenuChild> {
        match self {
            MenuItemKind::MenuItem(i) => i.0.borrow(),
            MenuItemKind::Submenu(i) => i.0.borrow(),
            MenuItemKind::Predefined(i) => i.0.borrow(),
            MenuItemKind::Check(i) => i.0.borrow(),
            MenuItemKind::Icon(i) => i.0.borrow(),
        }
    }

    pub(crate) fn child_mut(&self) -> RefMut<MenuChild> {
        match self {
            MenuItemKind::MenuItem(i) => i.0.borrow_mut(),
            MenuItemKind::Submenu(i) => i.0.borrow_mut(),
            MenuItemKind::Predefined(i) => i.0.borrow_mut(),
            MenuItemKind::Check(i) => i.0.borrow_mut(),
            MenuItemKind::Icon(i) => i.0.borrow_mut(),
        }
    }
}
