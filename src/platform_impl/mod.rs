// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

#![cfg_attr(
    all(
        any(
            target_os = "linux",
            target_os = "dragonfly",
            target_os = "freebsd",
            target_os = "netbsd",
            target_os = "openbsd"
        ),
        any(
            not(any(feature = "gtk3", feature = "gtk4")),
            all(feature = "gtk3", feature = "gtk4")
        )
    ),
    allow(dead_code)
)]

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
    feature = "gtk4",
    not(feature = "gtk3")
))]
#[path = "gtk4/mod.rs"]
mod platform;
#[cfg(all(
    any(
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd"
    ),
    feature = "gtk3",
    not(feature = "gtk4")
))]
#[path = "gtk/mod.rs"]
mod platform;
#[cfg(all(
    any(
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd"
    ),
    any(
        not(any(feature = "gtk3", feature = "gtk4")),
        all(feature = "gtk3", feature = "gtk4")
    )
))]
#[path = "noop/mod.rs"]
mod platform;
#[cfg(target_os = "macos")]
#[path = "macos/mod.rs"]
mod platform;

use std::{cell::RefCell, rc::Rc};

#[cfg(target_os = "macos")]
use crate::TextStyle;
use crate::{accelerator::MenuAccelerator, items::IconType, MenuItemKind};

pub(crate) use self::platform::*;

impl MenuItemKind {
    pub(crate) fn platform(&self) -> Rc<RefCell<PlatformMenuItem>> {
        match self {
            Self::MenuItem(item) => item.platform.clone(),
            Self::Submenu(item) => item.platform.clone(),
            Self::Predefined(item) => item.platform.clone(),
            Self::Check(item) => item.platform.clone(),
            Self::Icon(item) => item.platform.clone(),
        }
    }
}

#[derive(Clone)]
pub(crate) struct PlatformAttachArgs {
    pub text: String,
    pub enabled: bool,
    pub checked: bool,
    pub accelerator: Option<MenuAccelerator>,
    pub icon: Option<IconType>,
    #[cfg(target_os = "macos")]
    pub styled_text: Option<Vec<(String, TextStyle)>>,
}

impl MenuItemKind {
    pub(crate) fn platform_attach_args(&self) -> PlatformAttachArgs {
        match self {
            MenuItemKind::MenuItem(item) => {
                let state = item.state.borrow();
                PlatformAttachArgs {
                    text: state.text.clone(),
                    enabled: state.enabled,
                    checked: false,
                    accelerator: state.accelerator.clone(),
                    icon: None,
                    #[cfg(target_os = "macos")]
                    styled_text: state.styled_text.clone(),
                }
            }
            MenuItemKind::Submenu(item) => {
                let state = item.state.borrow();
                PlatformAttachArgs {
                    text: state.text.clone(),
                    enabled: state.enabled,
                    checked: false,
                    accelerator: None,
                    icon: state.icon.clone(),
                    #[cfg(target_os = "macos")]
                    styled_text: state.styled_text.clone(),
                }
            }
            MenuItemKind::Predefined(item) => {
                let state = item.state.borrow();
                PlatformAttachArgs {
                    text: state.text.clone(),
                    enabled: state.enabled,
                    checked: false,
                    accelerator: state.predefined_item_type.accelerator(),
                    icon: None,
                    #[cfg(target_os = "macos")]
                    styled_text: None,
                }
            }
            MenuItemKind::Check(item) => {
                let state = item.state.borrow();
                PlatformAttachArgs {
                    text: state.text.clone(),
                    enabled: state.enabled,
                    checked: state.checked,
                    accelerator: state.accelerator.clone(),
                    icon: None,
                    #[cfg(target_os = "macos")]
                    styled_text: state.styled_text.clone(),
                }
            }
            MenuItemKind::Icon(item) => {
                let state = item.state.borrow();
                PlatformAttachArgs {
                    text: state.text.clone(),
                    enabled: state.enabled,
                    checked: false,
                    accelerator: state.accelerator.clone(),
                    icon: state.icon.clone(),
                    #[cfg(target_os = "macos")]
                    styled_text: state.styled_text.clone(),
                }
            }
        }
    }

    #[cfg_attr(any(target_os = "windows", target_os = "macos"), allow(dead_code))]
    pub(crate) fn click_action(&self) -> crate::MenuItemAction {
        match self {
            Self::MenuItem(item) => crate::MenuItemAction::Emit((*item.id).clone()),
            Self::Submenu(item) => crate::MenuItemAction::Emit((*item.id).clone()),
            Self::Predefined(item) => crate::MenuItemAction::Predefined(item.state.downgrade()),
            Self::Check(item) => {
                crate::MenuItemAction::Toggle((*item.id).clone(), item.state.downgrade())
            }
            Self::Icon(item) => crate::MenuItemAction::Emit((*item.id).clone()),
        }
    }

    #[cfg_attr(target_os = "windows", allow(dead_code))]
    pub(crate) fn items(&self) -> Vec<MenuItemKind> {
        match self {
            Self::Submenu(item) => item.items(),
            _ => Vec::new(),
        }
    }
}
