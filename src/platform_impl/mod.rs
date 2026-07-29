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

use std::{
    cell::{Ref, RefCell, RefMut},
    rc::Rc,
};

use crate::{items::*, IsMenuItem, MenuItemKind, MenuItemType, NativeIcon};

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
                let id = self.id().clone();
                MenuItemKind::Submenu(Submenu {
                    id: Rc::new(id),
                    inner: c,
                })
            }
            MenuItemType::MenuItem => {
                let id = self.id().clone();
                MenuItemKind::MenuItem(MenuItem {
                    id: Rc::new(id),
                    inner: c,
                })
            }
            MenuItemType::Predefined => {
                let id = self.id().clone();
                MenuItemKind::Predefined(PredefinedMenuItem {
                    id: Rc::new(id),
                    inner: c,
                })
            }
            MenuItemType::Check => {
                let id = self.id().clone();
                MenuItemKind::Check(CheckMenuItem {
                    id: Rc::new(id),
                    inner: c,
                })
            }
            MenuItemType::Icon => {
                let id = self.id().clone();
                MenuItemKind::Icon(IconMenuItem {
                    id: Rc::new(id),
                    inner: c,
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
impl NativeIcon {
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
