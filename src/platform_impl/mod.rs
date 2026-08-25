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
    feature = "gtk4"
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
    feature = "gtk"
))]
#[path = "gtk/mod.rs"]
mod platform;
#[cfg(target_os = "macos")]
#[path = "macos/mod.rs"]
mod platform;

use std::{cell::RefCell, rc::Rc};

use crate::{
    accelerator::MenuAccelerator,
    items::{IconType, PredefinedMenuItemType},
    MenuItemKind,
};

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
    pub item_type: crate::MenuItemType,
    pub icon: Option<IconType>,
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
                    item_type: crate::MenuItemType::MenuItem,
                    icon: None,
                }
            }
            MenuItemKind::Submenu(item) => {
                let state = item.state.borrow();
                PlatformAttachArgs {
                    text: state.text.clone(),
                    enabled: state.enabled,
                    checked: false,
                    accelerator: None,
                    item_type: crate::MenuItemType::Submenu,
                    icon: state.icon.clone(),
                }
            }
            MenuItemKind::Predefined(item) => {
                let state = item.state.borrow();
                PlatformAttachArgs {
                    text: state.text.clone(),
                    enabled: state.enabled,
                    checked: false,
                    accelerator: state.predefined_item_type.accelerator(),
                    item_type: if matches!(
                        state.predefined_item_type,
                        PredefinedMenuItemType::Separator
                    ) {
                        crate::MenuItemType::Separator
                    } else {
                        crate::MenuItemType::Predefined
                    },
                    icon: None,
                }
            }
            MenuItemKind::Check(item) => {
                let state = item.state.borrow();
                PlatformAttachArgs {
                    text: state.text.clone(),
                    enabled: state.enabled,
                    checked: state.checked,
                    accelerator: state.accelerator.clone(),
                    item_type: crate::MenuItemType::Check,
                    icon: None,
                }
            }
            MenuItemKind::Icon(item) => {
                let state = item.state.borrow();
                PlatformAttachArgs {
                    text: state.text.clone(),
                    enabled: state.enabled,
                    checked: false,
                    accelerator: state.accelerator.clone(),
                    item_type: crate::MenuItemType::Icon,
                    icon: state.icon.clone(),
                }
            }
        }
    }

    #[cfg_attr(target_os = "windows", allow(dead_code))]
    pub(crate) fn click_action(&self) -> crate::ClickAction {
        match self {
            Self::MenuItem(item) => crate::ClickAction::Emit((*item.id).clone()),
            Self::Submenu(item) => crate::ClickAction::Emit((*item.id).clone()),
            Self::Predefined(item) => crate::ClickAction::Predefined(Rc::downgrade(&item.state)),
            Self::Check(item) => {
                crate::ClickAction::Toggle((*item.id).clone(), Rc::downgrade(&item.state))
            }
            Self::Icon(item) => crate::ClickAction::Emit((*item.id).clone()),
        }
    }

    #[cfg_attr(target_os = "windows", allow(dead_code))]
    pub(crate) fn children(&self) -> Vec<MenuItemKind> {
        match self {
            Self::Submenu(item) => item.state.borrow().children.clone(),
            _ => Vec::new(),
        }
    }
}

#[cfg(all(
    any(
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd"
    ),
    any(feature = "gtk", feature = "gtk4")
))]
impl crate::NativeIcon {
    pub(crate) fn gtk_icon_name(&self) -> &str {
        match self {
            Self::Add => "list-add-symbolic",
            Self::Advanced => "preferences-system-symbolic",
            Self::Bluetooth => "bluetooth-symbolic",
            Self::Bookmarks => "user-bookmarks-symbolic",
            Self::Caution => "dialog-warning-symbolic",
            Self::ColorPanel => "applications-graphics-symbolic",
            Self::ColumnView => "view-list-symbolic",
            Self::Computer => "computer-symbolic",
            Self::EnterFullScreen => "view-fullscreen-symbolic",
            Self::Everyone => "system-users-symbolic",
            Self::ExitFullScreen => "view-restore-symbolic",
            Self::FlowView => "view-grid-symbolic",
            Self::Folder => "folder-symbolic",
            Self::FolderBurnable => "media-optical-symbolic",
            Self::FolderSmart => "folder-saved-search-symbolic",
            Self::FollowLinkFreestanding => "insert-link-symbolic",
            Self::FontPanel => "preferences-desktop-font-symbolic",
            Self::GoLeft => "go-previous-symbolic",
            Self::GoRight => "go-next-symbolic",
            Self::Home => "user-home-symbolic",
            Self::IChatTheater => "camera-video-symbolic",
            Self::IconView => "view-grid-symbolic",
            Self::Info => "dialog-information-symbolic",
            Self::InvalidDataFreestanding => "dialog-error-symbolic",
            Self::LeftFacingTriangle => "pan-start-symbolic",
            Self::ListView => "view-list-symbolic",
            Self::LockLocked => "changes-prevent-symbolic",
            Self::LockUnlocked => "changes-allow-symbolic",
            Self::MenuMixedState => "list-remove-symbolic",
            Self::MenuOnState => "object-select-symbolic",
            Self::MobileMe => "network-server-symbolic",
            Self::MultipleDocuments => "edit-copy-symbolic",
            Self::Network => "network-workgroup-symbolic",
            Self::Path => "document-open-recent-symbolic",
            Self::PreferencesGeneral => "preferences-system-symbolic",
            Self::QuickLook => "document-preview-symbolic",
            Self::RefreshFreestanding | Self::Refresh => "view-refresh-symbolic",
            Self::Remove => "list-remove-symbolic",
            Self::RevealFreestanding => "folder-open-symbolic",
            Self::RightFacingTriangle => "pan-end-symbolic",
            Self::Share => "emblem-shared-symbolic",
            Self::Slideshow => "view-presentation-symbolic",
            Self::SmartBadge => "emblem-favorite-symbolic",
            Self::StatusAvailable => "user-available-symbolic",
            Self::StatusNone => "user-offline-symbolic",
            Self::StatusPartiallyAvailable => "user-idle-symbolic",
            Self::StatusUnavailable => "user-busy-symbolic",
            Self::StopProgressFreestanding | Self::StopProgress => "process-stop-symbolic",
            Self::TrashEmpty => "user-trash-symbolic",
            Self::TrashFull => "user-trash-full-symbolic",
            Self::User => "avatar-default-symbolic",
            Self::UserAccounts | Self::UserGroup => "system-users-symbolic",
            Self::UserGuest => "avatar-default-symbolic",
            Self::Raw(name) => name,
        }
    }
}
