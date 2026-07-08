// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use std::{cell::RefCell, rc::Rc, sync::Arc};

use arc_swap::ArcSwap;

use crate::{
    accelerator::KeyAccelerator,
    icon::{BadIcon, Icon, NativeIcon},
    items::PredefinedMenuItemType,
    util::{AddOp, Counter},
    IsMenuItem, MenuId, MenuItemKind, MenuItemType,
};

fn compat_placeholder() -> Arc<ArcSwap<crate::CompatMenuItem>> {
    Arc::new(ArcSwap::from_pointee(crate::CompatMenuItem::Separator))
}

static COUNTER: Counter = Counter::new();

#[derive(Debug, Clone)]
pub struct PlatformIcon {
    rgba: Vec<u8>,
    width: u32,
    height: u32,
}

impl PlatformIcon {
    pub fn from_rgba(rgba: Vec<u8>, width: u32, height: u32) -> Result<Self, BadIcon> {
        Ok(Self {
            rgba,
            width,
            height,
        })
    }

    pub(crate) fn to_png(&self) -> Vec<u8> {
        let mut png = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut png, self.width, self.height);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);

            let mut writer = encoder
                .write_header()
                .expect("writing an in-memory PNG header should not fail");
            writer
                .write_image_data(&self.rgba)
                .expect("writing in-memory PNG data should not fail");
        }
        png
    }
}

pub struct Menu {
    id: MenuId,
    children: Vec<Rc<RefCell<MenuChild>>>,
}

impl Menu {
    pub fn new(id: Option<MenuId>) -> Self {
        Self {
            id: id.unwrap_or_else(|| MenuId(COUNTER.next().to_string())),
            children: Vec::new(),
        }
    }

    pub fn id(&self) -> &MenuId {
        &self.id
    }

    pub fn add_menu_item(&mut self, item: &dyn IsMenuItem, op: AddOp) -> crate::Result<()> {
        match op {
            AddOp::Append => self.children.push(item.child()),
            AddOp::Insert(position) => self.children.insert(position, item.child()),
        }

        Ok(())
    }

    pub fn remove(&mut self, item: &dyn IsMenuItem) -> crate::Result<()> {
        let index = self
            .children
            .iter()
            .position(|e| e.borrow().id == item.id())
            .ok_or(crate::Error::NotAChildOfThisMenu)?;
        self.children.remove(index);
        Ok(())
    }

    pub fn items(&self) -> Vec<MenuItemKind> {
        self.children
            .iter()
            .map(|c| c.borrow().kind(c.clone()))
            .collect()
    }

    pub fn compat_items(&self) -> Vec<std::sync::Arc<arc_swap::ArcSwap<crate::CompatMenuItem>>> {
        self.items()
            .into_iter()
            .map(|item| item.compat_child())
            .collect()
    }
}

/// A generic child in a menu
#[derive(Debug, Default)]
pub struct MenuChild {
    item_type: MenuItemType,
    text: String,
    enabled: bool,
    id: MenuId,
    accelerator: Option<KeyAccelerator>,
    predefined_item_type: Option<PredefinedMenuItemType>,
    checked: bool,
    icon: Option<Icon>,
    children: Option<Vec<Rc<RefCell<MenuChild>>>>,
    compat: Arc<ArcSwap<crate::CompatMenuItem>>,
}

impl MenuChild {
    pub fn new(
        text: &str,
        enabled: bool,
        key_accelerator: Option<KeyAccelerator>,
        id: Option<MenuId>,
    ) -> Self {
        Self {
            item_type: MenuItemType::MenuItem,
            text: text.to_string(),
            enabled,
            id: id.unwrap_or_else(|| MenuId(COUNTER.next().to_string())),
            accelerator: key_accelerator,
            predefined_item_type: None,
            checked: false,
            icon: None,
            children: None,
            compat: compat_placeholder(),
        }
        .with_compat()
    }

    pub fn new_submenu(text: &str, enabled: bool, id: Option<MenuId>) -> Self {
        Self {
            item_type: MenuItemType::Submenu,
            text: text.to_string(),
            enabled,
            id: id.unwrap_or_else(|| MenuId(COUNTER.next().to_string())),
            accelerator: None,
            predefined_item_type: None,
            checked: false,
            icon: None,
            children: Some(Vec::new()),
            compat: compat_placeholder(),
        }
        .with_compat()
    }

    pub(crate) fn new_predefined(item_type: PredefinedMenuItemType, text: Option<String>) -> Self {
        Self {
            item_type: MenuItemType::Predefined,
            text: text.unwrap_or_else(|| item_type.text().to_string()),
            enabled: true,
            id: MenuId(COUNTER.next().to_string()),
            accelerator: item_type.accelerator().map(Into::into),
            predefined_item_type: Some(item_type),
            checked: false,
            icon: None,
            children: None,
            compat: compat_placeholder(),
        }
        .with_compat()
    }

    pub fn new_check(
        text: &str,
        enabled: bool,
        checked: bool,
        key_accelerator: Option<KeyAccelerator>,
        id: Option<MenuId>,
    ) -> Self {
        Self {
            item_type: MenuItemType::Check,
            text: text.to_string(),
            enabled,
            id: id.unwrap_or_else(|| MenuId(COUNTER.next().to_string())),
            accelerator: key_accelerator,
            predefined_item_type: None,
            checked,
            icon: None,
            children: None,
            compat: compat_placeholder(),
        }
        .with_compat()
    }

    pub fn new_icon(
        text: &str,
        enabled: bool,
        icon: Option<Icon>,
        key_accelerator: Option<KeyAccelerator>,
        id: Option<MenuId>,
    ) -> Self {
        Self {
            item_type: MenuItemType::Icon,
            text: text.to_string(),
            enabled,
            id: id.unwrap_or_else(|| MenuId(COUNTER.next().to_string())),
            accelerator: key_accelerator,
            predefined_item_type: None,
            checked: false,
            icon,
            children: None,
            compat: compat_placeholder(),
        }
        .with_compat()
    }

    pub fn new_native_icon(
        text: &str,
        enabled: bool,
        _native_icon: Option<NativeIcon>,
        key_accelerator: Option<KeyAccelerator>,
        id: Option<MenuId>,
    ) -> Self {
        Self {
            item_type: MenuItemType::Icon,
            text: text.to_string(),
            enabled,
            id: id.unwrap_or_else(|| MenuId(COUNTER.next().to_string())),
            accelerator: key_accelerator,
            predefined_item_type: None,
            checked: false,
            icon: None,
            children: None,
            compat: compat_placeholder(),
        }
        .with_compat()
    }
}

impl MenuChild {
    fn with_compat(mut self) -> Self {
        self.refresh_compat();
        self
    }

    pub(crate) fn compat_child(&self) -> Arc<ArcSwap<crate::CompatMenuItem>> {
        self.compat.clone()
    }

    pub(crate) fn refresh_compat(&mut self) {
        self.compat.store(Arc::new(self.compat_menu_item()));
    }

    fn compat_menu_item(&self) -> crate::CompatMenuItem {
        match self.item_type {
            MenuItemType::Submenu => crate::CompatSubMenuItem {
                label: crate::strip_mnemonic(self.text()),
                enabled: self.is_enabled(),
                submenu: self.compat_items(),
            }
            .into(),
            MenuItemType::Check => crate::CompatCheckmarkItem {
                id: self.id.0.clone(),
                label: crate::strip_mnemonic(self.text()),
                enabled: self.is_enabled(),
                checked: self.is_checked(),
            }
            .into(),
            MenuItemType::Predefined if self.is_separator() => crate::CompatMenuItem::Separator,
            MenuItemType::MenuItem | MenuItemType::Predefined | MenuItemType::Icon => {
                crate::CompatStandardItem {
                    id: self.id.0.clone(),
                    label: crate::strip_mnemonic(self.text()),
                    enabled: self.is_enabled(),
                    icon: self.icon_png(),
                }
                .into()
            }
        }
    }

    pub(crate) fn item_type(&self) -> MenuItemType {
        self.item_type
    }

    pub fn id(&self) -> &MenuId {
        &self.id
    }

    pub fn text(&self) -> String {
        self.text.clone()
    }

    pub fn set_text(&mut self, text: &str) {
        self.text = text.to_string();
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn set_key_accelerator(
        &mut self,
        accelerator: Option<KeyAccelerator>,
    ) -> crate::Result<()> {
        self.accelerator = accelerator;
        Ok(())
    }
}

impl MenuChild {
    pub fn is_checked(&self) -> bool {
        self.checked
    }

    pub fn set_checked(&mut self, checked: bool) {
        self.checked = checked;
    }
}

impl MenuChild {
    pub fn icon_png(&self) -> Option<Vec<u8>> {
        self.icon.as_ref().map(|icon| icon.inner.to_png())
    }

    pub fn set_icon(&mut self, icon: Option<Icon>) {
        self.icon = icon;
    }
}

impl MenuChild {
    pub fn is_separator(&self) -> bool {
        matches!(
            self.predefined_item_type,
            Some(PredefinedMenuItemType::Separator)
        )
    }

    pub fn add_menu_item(&mut self, item: &dyn IsMenuItem, op: AddOp) -> crate::Result<()> {
        let children = self.children.as_mut().unwrap();
        match op {
            AddOp::Append => children.push(item.child()),
            AddOp::Insert(position) => children.insert(position, item.child()),
        }

        Ok(())
    }

    pub fn remove(&mut self, item: &dyn IsMenuItem) -> crate::Result<()> {
        let children = self.children.as_mut().unwrap();
        let index = children
            .iter()
            .position(|e| e.borrow().id == item.id())
            .ok_or(crate::Error::NotAChildOfThisMenu)?;
        children.remove(index);
        Ok(())
    }

    pub fn items(&self) -> Vec<MenuItemKind> {
        self.children
            .as_ref()
            .unwrap()
            .iter()
            .map(|c| c.borrow().kind(c.clone()))
            .collect()
    }

    pub fn compat_items(&self) -> Vec<std::sync::Arc<arc_swap::ArcSwap<crate::CompatMenuItem>>> {
        self.items()
            .into_iter()
            .map(|item| item.compat_child())
            .collect()
    }
}
