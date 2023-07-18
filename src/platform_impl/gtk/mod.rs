// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

mod accelerator;
mod icon;

pub(crate) use icon::PlatformIcon;

use crate::{
    accelerator::Accelerator,
    icon::{Icon, NativeIcon},
    items::*,
    util::{AddOp, Counter},
    MenuEvent, MenuItemType,
};
use accelerator::{from_gtk_mnemonic, parse_accelerator, to_gtk_mnemonic};
use gtk::{prelude::*, Orientation};
use std::{
    cell::RefCell,
    collections::HashMap,
    rc::Rc,
    sync::atomic::{AtomicBool, Ordering},
};

static COUNTER: Counter = Counter::new();

macro_rules! return_if_predefined_item_not_supported {
    ($item:tt) => {
        let child = $item.child();
        let child_ = child.borrow();
        match (&child_.type_, &child_.predefined_item_type) {
            (
                crate::MenuItemType::Predefined,
                PredfinedMenuItemType::Separator
                | PredfinedMenuItemType::Copy
                | PredfinedMenuItemType::Cut
                | PredfinedMenuItemType::Paste
                | PredfinedMenuItemType::SelectAll
                | PredfinedMenuItemType::About(_),
            ) => {}
            (
                crate::MenuItemType::Submenu
                | crate::MenuItemType::Normal
                | crate::MenuItemType::Check
                | crate::MenuItemType::Icon,
                _,
            ) => {}
            _ => return Ok(()),
        }
        drop(child_);
    };
}

pub struct Menu {
    id: u32,
    children: Vec<Rc<RefCell<MenuChild>>>,
    gtk_menubars: HashMap<u32, (Option<gtk::MenuBar>, gtk::Box)>,
    accel_group: Option<gtk::AccelGroup>,
    gtk_menu: (u32, Option<gtk::Menu>), // dedicated menu for tray or context menus
}

impl Menu {
    pub fn new() -> Self {
        Self {
            id: COUNTER.next(),
            children: Vec::new(),
            gtk_menubars: HashMap::new(),
            accel_group: None,
            gtk_menu: (COUNTER.next(), None),
        }
    }

    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn add_menu_item(&mut self, item: &dyn crate::IsMenuItem, op: AddOp) -> crate::Result<()> {
        return_if_predefined_item_not_supported!(item);

        for (menu_id, (menu_bar, _)) in &self.gtk_menubars {
            if let Some(menu_bar) = menu_bar {
                let gtk_item =
                    item.make_gtk_menu_item(*menu_id, self.accel_group.as_ref(), true)?;
                match op {
                    AddOp::Append => menu_bar.append(&gtk_item),
                    AddOp::Insert(position) => menu_bar.insert(&gtk_item, position as i32),
                }
                gtk_item.show();
            }
        }

        {
            let (menu_id, menu) = &self.gtk_menu;
            if let Some(menu) = menu {
                let gtk_item =
                    item.make_gtk_menu_item(*menu_id, self.accel_group.as_ref(), true)?;
                match op {
                    AddOp::Append => menu.append(&gtk_item),
                    AddOp::Insert(position) => menu.insert(&gtk_item, position as i32),
                }
                gtk_item.show();
            }
        }

        match op {
            AddOp::Append => self.children.push(item.child()),
            AddOp::Insert(position) => self.children.insert(position, item.child()),
        }

        Ok(())
    }

    fn add_menu_item_with_id(&self, item: &dyn crate::IsMenuItem, id: u32) -> crate::Result<()> {
        return_if_predefined_item_not_supported!(item);

        for (menu_id, (menu_bar, _)) in self.gtk_menubars.iter().filter(|m| *m.0 == id) {
            if let Some(menu_bar) = menu_bar {
                let gtk_item =
                    item.make_gtk_menu_item(*menu_id, self.accel_group.as_ref(), true)?;
                menu_bar.append(&gtk_item);
                gtk_item.show();
            }
        }

        Ok(())
    }

    fn add_menu_item_to_context_menu(&self, item: &dyn crate::IsMenuItem) -> crate::Result<()> {
        return_if_predefined_item_not_supported!(item);

        let (menu_id, menu) = &self.gtk_menu;
        if let Some(menu) = menu {
            let gtk_item = item.make_gtk_menu_item(*menu_id, self.accel_group.as_ref(), true)?;
            menu.append(&gtk_item);
            gtk_item.show();
        }

        Ok(())
    }

    pub fn remove(&mut self, item: &dyn crate::IsMenuItem) -> crate::Result<()> {
        self.remove_inner(item, true, None)
    }

    pub fn remove_inner(
        &mut self,
        item: &dyn crate::IsMenuItem,
        remove_from_cache: bool,
        id: Option<u32>,
    ) -> crate::Result<()> {
        let child = {
            let index = self
                .children
                .iter()
                .position(|e| e.borrow().id == item.id())
                .ok_or(crate::Error::NotAChildOfThisMenu)?;
            if remove_from_cache {
                self.children.remove(index)
            } else {
                self.children.get(index).cloned().unwrap()
            }
        };

        if item.type_() == crate::MenuItemType::Submenu {
            let submenu = item.as_any().downcast_ref::<crate::Submenu>().unwrap();
            let gtk_menus = submenu.0.borrow().gtk_menus.clone();

            for (menu_id, _) in gtk_menus {
                for item in submenu.items() {
                    submenu
                        .0
                        .borrow_mut()
                        .remove_inner(item.as_ref(), false, Some(menu_id))?;
                }
            }
        }

        for (menu_id, (menu_bar, _)) in &self.gtk_menubars {
            if id.map(|i| i == *menu_id).unwrap_or(true) {
                if let Some(menu_bar) = menu_bar {
                    if let Some(items) = child
                        .borrow_mut()
                        .gtk_menu_items
                        .borrow_mut()
                        .remove(menu_id)
                    {
                        for item in items {
                            menu_bar.remove(&item);
                        }
                    }
                }
            }
        }

        if remove_from_cache {
            let (menu_id, menu) = &self.gtk_menu;
            if let Some(menu) = menu {
                if let Some(items) = child
                    .borrow_mut()
                    .gtk_menu_items
                    .borrow_mut()
                    .remove(menu_id)
                {
                    for item in items {
                        menu.remove(&item);
                    }
                }
            }
        }
        Ok(())
    }

    pub fn items(&self) -> Vec<Box<dyn crate::IsMenuItem>> {
        self.children
            .iter()
            .map(|c| c.borrow().boxed(c.clone()))
            .collect()
    }

    pub fn init_for_gtk_window<W>(&mut self, window: &W) -> crate::Result<gtk::Box>
    where
        W: IsA<gtk::ApplicationWindow>,
        W: IsA<gtk::Container>,
        W: IsA<gtk::Window>,
    {
        let id = window.as_ptr() as u32;

        if self.accel_group.is_none() {
            self.accel_group = Some(gtk::AccelGroup::new());
        }

        // This is the first time this method has been called on this window
        // so we need to create the menubar and its parent box
        if self.gtk_menubars.get(&id).is_none() {
            let menu_bar = gtk::MenuBar::new();
            let vbox = gtk::Box::new(Orientation::Vertical, 0);
            window.add(&vbox);
            vbox.show();
            self.gtk_menubars.insert(id, (Some(menu_bar), vbox));
        } else if let Some((menu_bar, _)) = self.gtk_menubars.get_mut(&id) {
            // This is NOT the first time this method has been called on a window.
            // So it already contains a [`gtk::Box`] but it doesn't have a [`gtk::MenuBar`]
            // because it was probably removed using [`Menu::remove_for_gtk_window`]
            // so we only need to create the menubar
            if menu_bar.is_none() {
                menu_bar.replace(gtk::MenuBar::new());
            } else {
                return Err(crate::Error::AlreadyInitialized);
            }
        }

        // Construct the entries of the menubar
        let (menu_bar, vbox) = self.gtk_menubars.get(&id).cloned().unwrap();
        let menu_bar = menu_bar.as_ref().unwrap();

        window.add_accel_group(self.accel_group.as_ref().unwrap());

        for item in self.items() {
            self.add_menu_item_with_id(item.as_ref(), id)?;
        }

        // Show the menubar on the window
        vbox.pack_start(menu_bar, false, false, 0);
        menu_bar.show();

        Ok(vbox)
    }

    pub fn remove_for_gtk_window<W>(&mut self, window: &W) -> crate::Result<()>
    where
        W: IsA<gtk::ApplicationWindow>,
        W: IsA<gtk::Window>,
    {
        let id = window.as_ptr() as u32;
        let menu_bar = self
            .gtk_menubars
            .remove(&id)
            .ok_or(crate::Error::NotInitialized)?;

        if let (Some(menu_bar), vbox) = menu_bar {
            for item in self.items() {
                let _ = self.remove_inner(item.as_ref(), false, Some(id));
            }

            // Remove the [`gtk::Menubar`] from the widget tree
            unsafe { menu_bar.destroy() };
            // Detach the accelerators from the window
            window.remove_accel_group(self.accel_group.as_ref().unwrap());
            // Remove the removed [`gtk::Menubar`] from our cache
            self.gtk_menubars.insert(id, (None, vbox));
            Ok(())
        } else {
            self.gtk_menubars.insert(id, menu_bar);
            Err(crate::Error::NotInitialized)
        }
    }

    pub fn hide_for_gtk_window<W>(&mut self, window: &W) -> crate::Result<()>
    where
        W: IsA<gtk::ApplicationWindow>,
    {
        if let Some((Some(menu_bar), _)) = self.gtk_menubars.get(&(window.as_ptr() as u32)) {
            menu_bar.hide();
            Ok(())
        } else {
            Err(crate::Error::NotInitialized)
        }
    }

    pub fn show_for_gtk_window<W>(&self, window: &W) -> crate::Result<()>
    where
        W: IsA<gtk::ApplicationWindow>,
    {
        if let Some((Some(menu_bar), _)) = self.gtk_menubars.get(&(window.as_ptr() as u32)) {
            menu_bar.show_all();
            Ok(())
        } else {
            Err(crate::Error::NotInitialized)
        }
    }

    pub fn is_visible_on_gtk_window<W>(&self, window: &W) -> bool
    where
        W: IsA<gtk::ApplicationWindow>,
    {
        self.gtk_menubars
            .get(&(window.as_ptr() as u32))
            .map(|m| m.0.as_ref().map(|m| m.get_visible()).unwrap_or(false))
            .unwrap_or(false)
    }

    pub fn show_context_menu_for_gtk_window(&self, window: &impl IsA<gtk::Widget>, x: f64, y: f64) {
        if let Some(window) = window.window() {
            let gtk_menu = gtk::Menu::new();

            for item in self.items() {
                let gtk_item = item.make_gtk_menu_item(0, None, false).unwrap();
                gtk_menu.append(&gtk_item);
            }
            gtk_menu.show_all();

            gtk_menu.popup_at_rect(
                &window,
                &gdk::Rectangle::new(x as _, y as _, 0, 0),
                gdk::Gravity::NorthWest,
                gdk::Gravity::NorthWest,
                None,
            );
        }
    }

    pub fn gtk_context_menu(&mut self) -> gtk::Menu {
        let mut add_items = false;

        {
            if self.gtk_menu.1.is_none() {
                self.gtk_menu.1 = Some(gtk::Menu::new());
                add_items = true;
            }
        }

        if add_items {
            for item in self.items() {
                self.add_menu_item_to_context_menu(item.as_ref()).unwrap();
            }
        }

        self.gtk_menu.1.as_ref().unwrap().clone()
    }
}

/// A generic child in a menu
#[derive(Debug, Default)]
pub struct MenuChild {
    // shared fields between submenus and menu items
    pub type_: MenuItemType,
    text: String,
    enabled: bool,
    id: u32,

    gtk_menu_items: Rc<RefCell<HashMap<u32, Vec<gtk::MenuItem>>>>,

    // menu item fields
    accelerator: Option<Accelerator>,
    gtk_accelerator: Option<(gdk::ModifierType, u32)>,

    // predefined menu item fields
    predefined_item_type: PredfinedMenuItemType,

    // check menu item fields
    checked: Rc<AtomicBool>,
    is_syncing_checked_state: Rc<AtomicBool>,

    // icon menu item fields
    icon: Option<Icon>,

    // submenu fields
    pub children: Option<Vec<Rc<RefCell<MenuChild>>>>,
    gtk_menus: HashMap<u32, Vec<(u32, gtk::Menu)>>,
    gtk_menu: (u32, Option<gtk::Menu>), // dedicated menu for tray or context menus
    accel_group: Option<gtk::AccelGroup>,
}

/// Constructors
impl MenuChild {
    pub fn new(text: &str, enabled: bool, accelerator: Option<Accelerator>) -> Self {
        Self {
            text: text.to_string(),
            enabled,
            accelerator,
            id: COUNTER.next(),
            type_: MenuItemType::Normal,
            gtk_menu_items: Rc::new(RefCell::new(HashMap::new())),
            ..Default::default()
        }
    }

    pub fn new_submenu(text: &str, enabled: bool) -> Self {
        Self {
            text: text.to_string(),
            enabled,
            id: COUNTER.next(),
            children: Some(Vec::new()),
            type_: MenuItemType::Submenu,
            gtk_menu: (COUNTER.next(), None),
            gtk_menu_items: Rc::new(RefCell::new(HashMap::new())),
            gtk_menus: HashMap::new(),
            ..Default::default()
        }
    }

    pub(crate) fn new_predefined(item_type: PredfinedMenuItemType, text: Option<String>) -> Self {
        Self {
            text: text.unwrap_or_else(|| item_type.text().to_string()),
            enabled: true,
            accelerator: item_type.accelerator(),
            id: COUNTER.next(),
            type_: MenuItemType::Predefined,
            predefined_item_type: item_type,
            gtk_menu_items: Rc::new(RefCell::new(HashMap::new())),
            ..Default::default()
        }
    }

    pub fn new_check(
        text: &str,
        enabled: bool,
        checked: bool,
        accelerator: Option<Accelerator>,
    ) -> Self {
        Self {
            text: text.to_string(),
            enabled,
            checked: Rc::new(AtomicBool::new(checked)),
            accelerator,
            id: COUNTER.next(),
            type_: MenuItemType::Check,
            gtk_menu_items: Rc::new(RefCell::new(HashMap::new())),
            is_syncing_checked_state: Rc::new(AtomicBool::new(false)),
            ..Default::default()
        }
    }

    pub fn new_icon(
        text: &str,
        enabled: bool,
        icon: Option<Icon>,
        accelerator: Option<Accelerator>,
    ) -> Self {
        Self {
            text: text.to_string(),
            enabled,
            icon,
            accelerator,
            id: COUNTER.next(),
            type_: MenuItemType::Icon,
            gtk_menu_items: Rc::new(RefCell::new(HashMap::new())),
            is_syncing_checked_state: Rc::new(AtomicBool::new(false)),
            ..Default::default()
        }
    }

    pub fn new_native_icon(
        text: &str,
        enabled: bool,
        _native_icon: Option<NativeIcon>,
        accelerator: Option<Accelerator>,
    ) -> Self {
        Self {
            text: text.to_string(),
            enabled,
            accelerator,
            id: COUNTER.next(),
            type_: MenuItemType::Icon,
            gtk_menu_items: Rc::new(RefCell::new(HashMap::new())),
            is_syncing_checked_state: Rc::new(AtomicBool::new(false)),
            ..Default::default()
        }
    }
}

/// Shared methods
impl MenuChild {
    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn text(&self) -> String {
        match self
            .gtk_menu_items
            .borrow()
            .values()
            .collect::<Vec<_>>()
            .first()
            .map(|v| v.first())
            .map(|e| e.map(|i| i.label().map(from_gtk_mnemonic)))
        {
            Some(Some(Some(text))) => text,
            _ => self.text.clone(),
        }
    }

    pub fn set_text(&mut self, text: &str) {
        self.text = text.to_string();
        let text = to_gtk_mnemonic(text);
        for items in self.gtk_menu_items.borrow().values() {
            for i in items {
                i.set_label(&text);
            }
        }
    }

    pub fn is_enabled(&self) -> bool {
        match self
            .gtk_menu_items
            .borrow()
            .values()
            .collect::<Vec<_>>()
            .first()
            .map(|v| v.first())
            .map(|e| e.map(|i| i.is_sensitive()))
        {
            Some(Some(enabled)) => enabled,
            _ => self.enabled,
        }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        for items in self.gtk_menu_items.borrow().values() {
            for i in items {
                i.set_sensitive(enabled);
            }
        }
    }

    pub fn set_accelerator(&mut self, accelerator: Option<Accelerator>) -> crate::Result<()> {
        let prev_accel = self.gtk_accelerator.as_ref();
        let new_accel = accelerator.as_ref().map(parse_accelerator).transpose()?;

        for items in self.gtk_menu_items.borrow().values() {
            for i in items {
                if let Some((mods, key)) = prev_accel {
                    i.remove_accelerator(self.accel_group.as_ref().unwrap(), *key, *mods);
                }
                if let Some((mods, key)) = new_accel {
                    i.add_accelerator(
                        "activate",
                        self.accel_group.as_ref().unwrap(),
                        key,
                        mods,
                        gtk::AccelFlags::VISIBLE,
                    )
                }
            }
        }

        self.gtk_accelerator = new_accel;
        self.accelerator = accelerator;

        Ok(())
    }
}

/// CheckMenuItem methods
impl MenuChild {
    pub fn is_checked(&self) -> bool {
        match self
            .gtk_menu_items
            .borrow()
            .values()
            .collect::<Vec<_>>()
            .first()
            .map(|v| v.first())
            .map(|e| e.map(|i| i.downcast_ref::<gtk::CheckMenuItem>().unwrap().is_active()))
        {
            Some(Some(checked)) => checked,
            _ => self.checked.load(Ordering::Relaxed),
        }
    }

    pub fn set_checked(&mut self, checked: bool) {
        self.checked.store(checked, Ordering::Release);
        self.is_syncing_checked_state.store(true, Ordering::Release);
        for items in self.gtk_menu_items.borrow().values() {
            for i in items {
                i.downcast_ref::<gtk::CheckMenuItem>()
                    .unwrap()
                    .set_active(checked);
            }
        }
        self.is_syncing_checked_state
            .store(false, Ordering::Release);
    }
}

/// IconMenuItem methods
impl MenuChild {
    pub fn set_icon(&mut self, icon: Option<Icon>) {
        self.icon = icon.clone();

        let pixbuf = icon.map(|i| i.inner.to_pixbuf_scale(16, 16));
        for items in self.gtk_menu_items.borrow().values() {
            for i in items {
                let box_container = i.child().unwrap().downcast::<gtk::Box>().unwrap();
                box_container.children()[0]
                    .downcast_ref::<gtk::Image>()
                    .unwrap()
                    .set_pixbuf(pixbuf.as_ref())
            }
        }
    }
}

/// Submenu methods
impl MenuChild {
    pub fn add_menu_item(&mut self, item: &dyn crate::IsMenuItem, op: AddOp) -> crate::Result<()> {
        return_if_predefined_item_not_supported!(item);

        for menus in self.gtk_menus.values() {
            for (menu_id, menu) in menus {
                let gtk_item =
                    item.make_gtk_menu_item(*menu_id, self.accel_group.as_ref(), true)?;
                match op {
                    AddOp::Append => menu.append(&gtk_item),
                    AddOp::Insert(position) => menu.insert(&gtk_item, position as i32),
                }
                gtk_item.show();
            }
        }

        {
            let (menu_id, menu) = &self.gtk_menu;
            if let Some(menu) = menu {
                let gtk_item =
                    item.make_gtk_menu_item(*menu_id, self.accel_group.as_ref(), true)?;
                match op {
                    AddOp::Append => menu.append(&gtk_item),
                    AddOp::Insert(position) => menu.insert(&gtk_item, position as i32),
                }
                gtk_item.show();
            }
        }

        match op {
            AddOp::Append => self.children.as_mut().unwrap().push(item.child()),
            AddOp::Insert(position) => self
                .children
                .as_mut()
                .unwrap()
                .insert(position, item.child()),
        }

        Ok(())
    }

    fn add_menu_item_with_id(&self, item: &dyn crate::IsMenuItem, id: u32) -> crate::Result<()> {
        return_if_predefined_item_not_supported!(item);

        for menus in self.gtk_menus.values() {
            for (menu_id, menu) in menus.iter().filter(|m| m.0 == id) {
                let gtk_item =
                    item.make_gtk_menu_item(*menu_id, self.accel_group.as_ref(), true)?;
                menu.append(&gtk_item);
                gtk_item.show();
            }
        }

        Ok(())
    }

    fn add_menu_item_to_context_menu(&self, item: &dyn crate::IsMenuItem) -> crate::Result<()> {
        return_if_predefined_item_not_supported!(item);

        let (menu_id, menu) = &self.gtk_menu;
        if let Some(menu) = menu {
            let gtk_item = item.make_gtk_menu_item(*menu_id, None, true)?;
            menu.append(&gtk_item);
            gtk_item.show();
        }

        Ok(())
    }

    pub fn remove(&mut self, item: &dyn crate::IsMenuItem) -> crate::Result<()> {
        self.remove_inner(item, true, None)
    }

    fn remove_inner(
        &mut self,
        item: &dyn crate::IsMenuItem,
        remove_from_cache: bool,
        id: Option<u32>,
    ) -> crate::Result<()> {
        let child = {
            let index = self
                .children
                .as_ref()
                .unwrap()
                .iter()
                .position(|e| e.borrow().id == item.id())
                .ok_or(crate::Error::NotAChildOfThisMenu)?;
            if remove_from_cache {
                self.children.as_mut().unwrap().remove(index)
            } else {
                self.children.as_ref().unwrap().get(index).cloned().unwrap()
            }
        };

        if item.type_() == crate::MenuItemType::Submenu {
            let submenu = item.as_any().downcast_ref::<crate::Submenu>().unwrap();
            let gtk_menus = submenu.0.borrow().gtk_menus.clone();

            for (menu_id, _) in gtk_menus {
                for item in submenu.items() {
                    submenu
                        .0
                        .borrow_mut()
                        .remove_inner(item.as_ref(), false, Some(menu_id))?;
                }
            }
        }

        for menus in self.gtk_menus.values() {
            for (menu_id, menu) in menus {
                if id.map(|i| i == *menu_id).unwrap_or(true) {
                    if let Some(items) = child
                        .borrow_mut()
                        .gtk_menu_items
                        .borrow_mut()
                        .remove(menu_id)
                    {
                        for item in items {
                            menu.remove(&item);
                        }
                    }
                }
            }
        }

        if remove_from_cache {
            let (menu_id, menu) = &self.gtk_menu;
            if let Some(menu) = menu {
                if let Some(items) = child
                    .borrow_mut()
                    .gtk_menu_items
                    .borrow_mut()
                    .remove(menu_id)
                {
                    for item in items {
                        menu.remove(&item);
                    }
                }
            }
        }

        Ok(())
    }

    pub fn items(&self) -> Vec<Box<dyn crate::IsMenuItem>> {
        self.children
            .as_ref()
            .unwrap()
            .iter()
            .map(|c| c.borrow().boxed(c.clone()))
            .collect()
    }

    pub fn show_context_menu_for_gtk_window(&self, window: &impl IsA<gtk::Widget>, x: f64, y: f64) {
        if let Some(window) = window.window() {
            let gtk_menu = gtk::Menu::new();

            for item in self.items() {
                let gtk_item = item.make_gtk_menu_item(0, None, false).unwrap();
                gtk_menu.append(&gtk_item);
            }
            gtk_menu.show_all();

            gtk_menu.popup_at_rect(
                &window,
                &gdk::Rectangle::new(x as _, y as _, 0, 0),
                gdk::Gravity::NorthWest,
                gdk::Gravity::NorthWest,
                None,
            );
        }
    }

    pub fn gtk_context_menu(&mut self) -> gtk::Menu {
        let mut add_items = false;
        {
            if self.gtk_menu.1.is_none() {
                self.gtk_menu.1 = Some(gtk::Menu::new());
                add_items = true;
            }
        }

        if add_items {
            for item in self.items() {
                self.add_menu_item_to_context_menu(item.as_ref()).unwrap();
            }
        }

        self.gtk_menu.1.as_ref().unwrap().clone()
    }
}

macro_rules! register_accel {
    ($self:ident, $item:ident, $accel_group:ident) => {
        $self.gtk_accelerator = $self
            .accelerator
            .as_ref()
            .map(parse_accelerator)
            .transpose()?;

        if let Some((mods, key)) = &$self.gtk_accelerator {
            if let Some(accel_group) = $accel_group {
                $item.add_accelerator(
                    "activate",
                    accel_group,
                    *key,
                    *mods,
                    gtk::AccelFlags::VISIBLE,
                )
            }
        }
    };
}

/// Gtk menu item creation methods
impl MenuChild {
    fn create_gtk_item_for_submenu(
        &mut self,
        menu_id: u32,
        accel_group: Option<&gtk::AccelGroup>,
        add_to_cache: bool,
    ) -> crate::Result<gtk::MenuItem> {
        let submenu = gtk::Menu::new();
        let item = gtk::MenuItem::builder()
            .label(&to_gtk_mnemonic(&self.text))
            .use_underline(true)
            .submenu(&submenu)
            .sensitive(self.enabled)
            .build();

        item.show();
        item.set_submenu(Some(&submenu));

        self.accel_group = accel_group.cloned();

        let mut id = 0;
        if add_to_cache {
            id = COUNTER.next();

            self.gtk_menu_items
                .borrow_mut()
                .entry(menu_id)
                .or_insert_with(Vec::new)
                .push(item.clone());
            self.gtk_menus
                .entry(menu_id)
                .or_insert_with(Vec::new)
                .push((id, submenu.clone()));
        }

        for item in self.items() {
            if add_to_cache {
                self.add_menu_item_with_id(item.as_ref(), id)?;
            } else {
                let gtk_item = item.make_gtk_menu_item(0, None, false)?;
                submenu.append(&gtk_item);
            }
        }

        Ok(item)
    }

    fn create_gtk_item_for_menu_item(
        &mut self,
        menu_id: u32,
        accel_group: Option<&gtk::AccelGroup>,
        add_to_cache: bool,
    ) -> crate::Result<gtk::MenuItem> {
        let item = gtk::MenuItem::builder()
            .label(&to_gtk_mnemonic(&self.text))
            .use_underline(true)
            .sensitive(self.enabled)
            .build();

        self.accel_group = accel_group.cloned();

        register_accel!(self, item, accel_group);

        let id = self.id;
        item.connect_activate(move |_| {
            MenuEvent::send(crate::MenuEvent { id });
        });

        if add_to_cache {
            self.gtk_menu_items
                .borrow_mut()
                .entry(menu_id)
                .or_insert_with(Vec::new)
                .push(item.clone());
        }

        Ok(item)
    }

    fn create_gtk_item_for_predefined_menu_item(
        &mut self,
        menu_id: u32,
        accel_group: Option<&gtk::AccelGroup>,
        add_to_cache: bool,
    ) -> crate::Result<gtk::MenuItem> {
        let text = self.text.clone();
        self.gtk_accelerator = self
            .accelerator
            .as_ref()
            .map(parse_accelerator)
            .transpose()?;
        let predefined_item_type = self.predefined_item_type.clone();

        let make_item = || {
            gtk::MenuItem::builder()
                .label(&to_gtk_mnemonic(&text))
                .use_underline(true)
                .sensitive(true)
                .build()
        };
        let register_accel = |item: &gtk::MenuItem| {
            if let Some((mods, key)) = &self.gtk_accelerator {
                if let Some(accel_group) = accel_group {
                    item.add_accelerator(
                        "activate",
                        accel_group,
                        *key,
                        *mods,
                        gtk::AccelFlags::VISIBLE,
                    )
                }
            }
        };

        let item = match predefined_item_type {
            PredfinedMenuItemType::Separator => {
                gtk::SeparatorMenuItem::new().upcast::<gtk::MenuItem>()
            }
            PredfinedMenuItemType::Copy
            | PredfinedMenuItemType::Cut
            | PredfinedMenuItemType::Paste
            | PredfinedMenuItemType::SelectAll => {
                let item = make_item();
                let (mods, key) =
                    parse_accelerator(&predefined_item_type.accelerator().unwrap()).unwrap();
                item.child()
                    .unwrap()
                    .downcast::<gtk::AccelLabel>()
                    .unwrap()
                    .set_accel(key, mods);
                item.connect_activate(move |_| {
                    // TODO: wayland
                    #[cfg(feature = "libxdo")]
                    if let Ok(xdo) = libxdo::XDo::new(None) {
                        let _ = xdo.send_keysequence(predefined_item_type.xdo_keys(), 0);
                    }
                });
                item
            }
            PredfinedMenuItemType::About(metadata) => {
                let item = make_item();
                register_accel(&item);
                item.connect_activate(move |_| {
                    if let Some(metadata) = &metadata {
                        let mut builder = gtk::builders::AboutDialogBuilder::new()
                            .modal(true)
                            .resizable(false);

                        if let Some(name) = &metadata.name {
                            builder = builder.program_name(name);
                        }
                        if let Some(version) = &metadata.full_version() {
                            builder = builder.version(version);
                        }
                        if let Some(authors) = &metadata.authors {
                            builder = builder.authors(authors.clone());
                        }
                        if let Some(comments) = &metadata.comments {
                            builder = builder.comments(comments);
                        }
                        if let Some(copyright) = &metadata.copyright {
                            builder = builder.copyright(copyright);
                        }
                        if let Some(license) = &metadata.license {
                            builder = builder.license(license);
                        }
                        if let Some(website) = &metadata.website {
                            builder = builder.website(website);
                        }
                        if let Some(website_label) = &metadata.website_label {
                            builder = builder.website_label(website_label);
                        }
                        if let Some(icon) = &metadata.icon {
                            builder = builder.logo(&icon.inner.to_pixbuf());
                        }

                        let about = builder.build();
                        about.run();
                        unsafe {
                            about.destroy();
                        }
                    }
                });
                item
            }
            _ => unreachable!(),
        };

        if add_to_cache {
            self.gtk_menu_items
                .borrow_mut()
                .entry(menu_id)
                .or_insert_with(Vec::new)
                .push(item.clone());
        }
        Ok(item)
    }

    fn create_gtk_item_for_check_menu_item(
        &mut self,
        menu_id: u32,
        accel_group: Option<&gtk::AccelGroup>,
        add_to_cache: bool,
    ) -> crate::Result<gtk::MenuItem> {
        let item = gtk::CheckMenuItem::builder()
            .label(&to_gtk_mnemonic(&self.text))
            .use_underline(true)
            .sensitive(self.enabled)
            .active(self.checked.load(Ordering::Relaxed))
            .build();

        self.accel_group = accel_group.cloned();

        register_accel!(self, item, accel_group);

        let id = self.id;
        let is_syncing_checked_state = self.is_syncing_checked_state.clone();
        let checked = self.checked.clone();
        let store = self.gtk_menu_items.clone();
        item.connect_toggled(move |i| {
            let should_dispatch = is_syncing_checked_state
                .compare_exchange(false, true, Ordering::Release, Ordering::Relaxed)
                .is_ok();

            if should_dispatch {
                let c = i.is_active();
                checked.store(c, Ordering::Release);

                for items in store.borrow().values() {
                    for i in items {
                        i.downcast_ref::<gtk::CheckMenuItem>()
                            .unwrap()
                            .set_active(c);
                    }
                }

                is_syncing_checked_state.store(false, Ordering::Release);

                MenuEvent::send(crate::MenuEvent { id });
            }
        });

        let item = item.upcast::<gtk::MenuItem>();

        if add_to_cache {
            self.gtk_menu_items
                .borrow_mut()
                .entry(menu_id)
                .or_insert_with(Vec::new)
                .push(item.clone());
        }

        Ok(item)
    }

    fn create_gtk_item_for_icon_menu_item(
        &mut self,
        menu_id: u32,
        accel_group: Option<&gtk::AccelGroup>,
        add_to_cache: bool,
    ) -> crate::Result<gtk::MenuItem> {
        let image = self
            .icon
            .as_ref()
            .map(|i| gtk::Image::from_pixbuf(Some(&i.inner.to_pixbuf_scale(16, 16))))
            .unwrap_or_else(gtk::Image::default);

        self.accel_group = accel_group.cloned();

        let label = gtk::AccelLabel::builder()
            .label(&to_gtk_mnemonic(&self.text))
            .use_underline(true)
            .xalign(0.0)
            .build();

        let box_container = gtk::Box::new(Orientation::Horizontal, 6);
        let style_context = box_container.style_context();
        let css_provider = gtk::CssProvider::new();
        let theme = r#"
            box {
                margin-left: -22px;
            }
          "#;
        let _ = css_provider.load_from_data(theme.as_bytes());
        style_context.add_provider(&css_provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);
        box_container.pack_start(&image, false, false, 0);
        box_container.pack_start(&label, true, true, 0);
        box_container.show_all();

        let item = gtk::MenuItem::builder()
            .child(&box_container)
            .sensitive(self.enabled)
            .build();

        register_accel!(self, item, accel_group);

        let id = self.id;
        item.connect_activate(move |_| {
            MenuEvent::send(crate::MenuEvent { id });
        });

        if add_to_cache {
            self.gtk_menu_items
                .borrow_mut()
                .entry(menu_id)
                .or_insert_with(Vec::new)
                .push(item.clone());
        }

        Ok(item)
    }
}

impl dyn crate::IsMenuItem + '_ {
    fn make_gtk_menu_item(
        &self,
        menu_id: u32,
        accel_group: Option<&gtk::AccelGroup>,
        add_to_cache: bool,
    ) -> crate::Result<gtk::MenuItem> {
        match self.type_() {
            MenuItemType::Submenu => self
                .as_any()
                .downcast_ref::<Submenu>()
                .unwrap()
                .0
                .borrow_mut()
                .create_gtk_item_for_submenu(menu_id, accel_group, add_to_cache),
            MenuItemType::Normal => self
                .as_any()
                .downcast_ref::<MenuItem>()
                .unwrap()
                .0
                .borrow_mut()
                .create_gtk_item_for_menu_item(menu_id, accel_group, add_to_cache),
            MenuItemType::Predefined => self
                .as_any()
                .downcast_ref::<PredefinedMenuItem>()
                .unwrap()
                .0
                .borrow_mut()
                .create_gtk_item_for_predefined_menu_item(menu_id, accel_group, add_to_cache),
            MenuItemType::Check => self
                .as_any()
                .downcast_ref::<CheckMenuItem>()
                .unwrap()
                .0
                .borrow_mut()
                .create_gtk_item_for_check_menu_item(menu_id, accel_group, add_to_cache),
            MenuItemType::Icon => self
                .as_any()
                .downcast_ref::<IconMenuItem>()
                .unwrap()
                .0
                .borrow_mut()
                .create_gtk_item_for_icon_menu_item(menu_id, accel_group, add_to_cache),
        }
    }
}

impl PredfinedMenuItemType {
    fn xdo_keys(&self) -> &str {
        match self {
            PredfinedMenuItemType::Copy => "ctrl+c",
            PredfinedMenuItemType::Cut => "ctrl+X",
            PredfinedMenuItemType::Paste => "ctrl+v",
            PredfinedMenuItemType::SelectAll => "ctrl+a",
            _ => unreachable!(),
        }
    }
}
