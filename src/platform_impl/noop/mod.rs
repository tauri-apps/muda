// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use crate::{
    accelerator::MenuAccelerator,
    icon::NoIcon,
    items::{ClickAction, IconType},
    util::AddOp,
    MenuItemKind, TextStyle,
};

pub(crate) type PlatformIcon = NoIcon;

#[cfg(feature = "snapshot")]
pub(crate) fn dispatch_on_main_thread<F>(f: F)
where
    F: FnOnce() + Send + 'static,
{
    f();
}

pub(crate) struct PlatformMenu;

impl PlatformMenu {
    pub(crate) fn new() -> Self {
        Self
    }

    pub(crate) fn attach(&mut self, _child: &MenuItemKind, _op: AddOp) -> crate::Result<()> {
        Ok(())
    }

    pub(crate) fn remove_at(&mut self, _index: usize, _child: &MenuItemKind) {}
}

pub(crate) struct PlatformMenuItem;

impl PlatformMenuItem {
    pub(crate) fn new(_click: ClickAction) -> Self {
        Self
    }

    pub(crate) fn new_submenu(_click: ClickAction) -> Self {
        Self
    }

    pub(crate) fn text(&self) -> Option<String> {
        None
    }

    pub(crate) fn set_text(&mut self, _text: &str, _accelerator: Option<&MenuAccelerator>) {}

    pub(crate) fn set_styled_text(
        &mut self,
        _text: &str,
        _parts: &[(String, TextStyle)],
        _accelerator: Option<&MenuAccelerator>,
    ) {
    }

    pub(crate) fn is_enabled(&self) -> Option<bool> {
        None
    }

    pub(crate) fn set_enabled(&mut self, _enabled: bool) {}

    pub(crate) fn set_accelerator(
        &mut self,
        _text: &str,
        _accelerator: Option<&MenuAccelerator>,
    ) -> crate::Result<()> {
        Ok(())
    }

    pub(crate) fn is_checked(&self) -> Option<bool> {
        None
    }

    pub(crate) fn set_checked(&mut self, _checked: bool) {}

    pub(crate) fn set_icon(&mut self, _icon: Option<&IconType>) {}

    pub(crate) fn attach(&mut self, _child: &MenuItemKind, _op: AddOp) -> crate::Result<()> {
        Ok(())
    }

    pub(crate) fn remove_at(&mut self, _index: usize, _child: &MenuItemKind) {}
}
