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

use std::{cell::RefCell, rc::Rc};

use crate::{items::*, IsMenuItem, MenuItemType};

pub(crate) use self::platform::*;

impl dyn IsMenuItem + '_ {
    fn child(&self) -> Rc<RefCell<MenuChild>> {
        match self.type_() {
            MenuItemType::Submenu => self
                .as_any()
                .downcast_ref::<crate::Submenu>()
                .unwrap()
                .0
                .clone(),
            MenuItemType::Normal => self
                .as_any()
                .downcast_ref::<crate::MenuItem>()
                .unwrap()
                .0
                .clone(),
            MenuItemType::Predefined => self
                .as_any()
                .downcast_ref::<crate::PredefinedMenuItem>()
                .unwrap()
                .0
                .clone(),
            MenuItemType::Check => self
                .as_any()
                .downcast_ref::<crate::CheckMenuItem>()
                .unwrap()
                .0
                .clone(),
            MenuItemType::Icon => self
                .as_any()
                .downcast_ref::<crate::IconMenuItem>()
                .unwrap()
                .0
                .clone(),
        }
    }
}

/// Internal utilities
impl MenuChild {
    fn boxed(&self, c: Rc<RefCell<MenuChild>>) -> Box<dyn IsMenuItem> {
        match self.type_ {
            MenuItemType::Submenu => Box::new(Submenu(c)),
            MenuItemType::Normal => Box::new(MenuItem(c)),
            MenuItemType::Predefined => Box::new(PredefinedMenuItem(c)),
            MenuItemType::Check => Box::new(CheckMenuItem(c)),
            MenuItemType::Icon => Box::new(IconMenuItem(c)),
        }
    }
}
