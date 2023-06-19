// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

mod accelerator;
mod icon;

pub(crate) use icon::PlatformIcon;

use crate::{
    accelerator::Accelerator,
    icon::Icon,
    predefined::PredfinedMenuItemType,
    util::{AddOp, Counter},
    MenuEvent, MenuItemType,
};
use accelerator::{
    from_gtk_mnemonic, parse_accelerator, register_accelerator, remove_accelerator, to_gtk_mnemonic,
};
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
        let child = $item.get_child();
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
            _ => return,
        }
        drop(child_);
    };
}

/// A generic child in a menu
///
/// Be careful when cloning this item and treat it as read-only
#[derive(Debug, Default)]
#[allow(dead_code)]
struct MenuChild {
    // shared fields between submenus and menu items
    type_: MenuItemType,
    text: String,
    enabled: bool,
    id: u32,

    gtk_menu_items: HashMap<u32, Vec<gtk::MenuItem>>,

    // menu item fields
    accelerator: Option<Accelerator>,

    // predefined menu item fields
    predefined_item_type: PredfinedMenuItemType,

    // check menu item fields
    checked: bool,
    is_syncing_checked_state: Rc<AtomicBool>,

    // icon menu item fields
    icon: Option<Icon>,

    // submenu fields
    children: Option<Vec<Rc<RefCell<MenuChild>>>>,
    gtk_menus: HashMap<u32, Vec<(u32, gtk::Menu)>>,
    gtk_menu: (u32, Option<gtk::Menu>), // dedicated menu for tray or context menus
    accel_group: Option<gtk::AccelGroup>,
}

impl MenuChild {
    fn id(&self) -> u32 {
        self.id
    }

    fn text(&self) -> String {
        match self
            .gtk_menu_items
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

    fn set_text(&mut self, text: &str) {
        self.text = text.to_string();
        let text = to_gtk_mnemonic(text);
        for items in self.gtk_menu_items.values() {
            for i in items {
                i.set_label(&text);
            }
        }
    }

    fn is_enabled(&self) -> bool {
        match self
            .gtk_menu_items
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

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        for items in self.gtk_menu_items.values() {
            for i in items {
                i.set_sensitive(enabled);
            }
        }
    }

    fn is_checked(&self) -> bool {
        match self
            .gtk_menu_items
            .values()
            .collect::<Vec<_>>()
            .first()
            .map(|v| v.first())
            .map(|e| e.map(|i| i.downcast_ref::<gtk::CheckMenuItem>().unwrap().is_active()))
        {
            Some(Some(checked)) => checked,
            _ => self.checked,
        }
    }

    fn set_checked(&mut self, checked: bool) {
        self.checked = checked;
        self.is_syncing_checked_state.store(true, Ordering::Release);
        for items in self.gtk_menu_items.values() {
            for i in items {
                i.downcast_ref::<gtk::CheckMenuItem>()
                    .unwrap()
                    .set_active(checked);
            }
        }
        self.is_syncing_checked_state
            .store(false, Ordering::Release);
    }

    fn set_icon(&mut self, icon: Option<Icon>) {
        self.icon = icon.clone();

        let pixbuf = icon.map(|i| i.inner.to_pixbuf_scale(16, 16));
        for items in self.gtk_menu_items.values() {
            for i in items {
                let box_container = i.child().unwrap().downcast::<gtk::Box>().unwrap();
                box_container.children()[0]
                    .downcast_ref::<gtk::Image>()
                    .unwrap()
                    .set_pixbuf(pixbuf.as_ref())
            }
        }
    }

    fn set_accelerator(&mut self, accelerator: Option<Accelerator>) {
        for items in self.gtk_menu_items.values() {
            for i in items {
                if let Some(accel) = self.accelerator {
                    remove_accelerator(i, self.accel_group.as_ref().unwrap(), &accel);
                }
                if let Some(accel) = accelerator.as_ref() {
                    register_accelerator(i, self.accel_group.as_ref().unwrap(), accel);
                }
            }
        }
        self.accelerator = accelerator;
    }
}

struct InnerMenu {
    children: Vec<Rc<RefCell<MenuChild>>>,
    gtk_menubars: HashMap<u32, (Option<gtk::MenuBar>, gtk::Box)>,
    accel_group: Option<gtk::AccelGroup>,
    gtk_menu: (u32, Option<gtk::Menu>), // dedicated menu for tray or context menus
}

#[derive(Clone)]
pub struct Menu(Rc<RefCell<InnerMenu>>);

impl Menu {
    pub fn new() -> Self {
        Self(Rc::new(RefCell::new(InnerMenu {
            children: Vec::new(),
            gtk_menubars: HashMap::new(),
            accel_group: None,
            gtk_menu: (COUNTER.next(), None),
        })))
    }

    pub fn add_menu_item(&self, item: &dyn crate::MenuItemExt, op: AddOp) {
        return_if_predefined_item_not_supported!(item);

        let mut self_ = self.0.borrow_mut();

        for (menu_id, (menu_bar, _)) in &self_.gtk_menubars {
            if let Some(menu_bar) = menu_bar {
                let gtk_item = item.make_gtk_menu_item(*menu_id, self_.accel_group.as_ref(), true);
                match op {
                    AddOp::Append => menu_bar.append(&gtk_item),
                    AddOp::Insert(position) => menu_bar.insert(&gtk_item, position as i32),
                }
                gtk_item.show();
            }
        }

        {
            let (menu_id, menu) = &self_.gtk_menu;
            if let Some(menu) = menu {
                let gtk_item = item.make_gtk_menu_item(*menu_id, self_.accel_group.as_ref(), true);
                match op {
                    AddOp::Append => menu.append(&gtk_item),
                    AddOp::Insert(position) => menu.insert(&gtk_item, position as i32),
                }
                gtk_item.show();
            }
        }

        match op {
            AddOp::Append => self_.children.push(item.get_child()),
            AddOp::Insert(position) => self_.children.insert(position, item.get_child()),
        }
    }

    fn add_menu_item_with_id(&self, item: &dyn crate::MenuItemExt, id: u32) {
        return_if_predefined_item_not_supported!(item);

        let self_ = self.0.borrow();

        for (menu_id, (menu_bar, _)) in self_.gtk_menubars.iter().filter(|m| *m.0 == id) {
            if let Some(menu_bar) = menu_bar {
                let gtk_item = item.make_gtk_menu_item(*menu_id, self_.accel_group.as_ref(), true);
                menu_bar.append(&gtk_item);
                gtk_item.show();
            }
        }
    }

    fn add_menu_item_to_context_menu(&self, item: &dyn crate::MenuItemExt) {
        return_if_predefined_item_not_supported!(item);

        let self_ = self.0.borrow();

        let (menu_id, menu) = &self_.gtk_menu;
        if let Some(menu) = menu {
            let gtk_item = item.make_gtk_menu_item(*menu_id, self_.accel_group.as_ref(), true);
            menu.append(&gtk_item);
            gtk_item.show();
        }
    }

    pub fn remove(&self, item: &dyn crate::MenuItemExt) -> crate::Result<()> {
        self.remove_inner(item, true, None)
    }
    pub fn remove_inner(
        &self,
        item: &dyn crate::MenuItemExt,
        remove_from_cache: bool,
        id: Option<u32>,
    ) -> crate::Result<()> {
        let child = {
            let mut self_ = self.0.borrow_mut();
            let index = self_
                .children
                .iter()
                .position(|e| e.borrow().id == item.id())
                .ok_or(crate::Error::NotAChildOfThisMenu)?;
            if remove_from_cache {
                self_.children.remove(index)
            } else {
                self_.children.get(index).cloned().unwrap()
            }
        };

        if item.type_() == crate::MenuItemType::Submenu {
            let submenu = item.as_any().downcast_ref::<crate::Submenu>().unwrap();
            let gtk_menus = submenu.0 .0.borrow().gtk_menus.clone();

            for (menu_id, _) in gtk_menus {
                for item in submenu.items() {
                    submenu
                        .0
                        .remove_inner(item.as_ref(), false, Some(menu_id))?;
                }
            }
        }

        let self_ = self.0.borrow();
        for (menu_id, (menu_bar, _)) in &self_.gtk_menubars {
            if id.map(|i| i == *menu_id).unwrap_or(true) {
                if let Some(menu_bar) = menu_bar {
                    if let Some(items) = child.borrow_mut().gtk_menu_items.remove(menu_id) {
                        for item in items {
                            menu_bar.remove(&item);
                        }
                    }
                }
            }
        }

        if remove_from_cache {
            let (menu_id, menu) = &self_.gtk_menu;
            if let Some(menu) = menu {
                if let Some(items) = child.borrow_mut().gtk_menu_items.remove(menu_id) {
                    for item in items {
                        menu.remove(&item);
                    }
                }
            }
        }
        Ok(())
    }

    pub fn items(&self) -> Vec<Box<dyn crate::MenuItemExt>> {
        self.0
            .borrow()
            .children
            .iter()
            .map(|c| -> Box<dyn crate::MenuItemExt> {
                let child = c.borrow();
                match child.type_ {
                    MenuItemType::Submenu => Box::new(crate::Submenu(Submenu(c.clone()))),
                    MenuItemType::Normal => Box::new(crate::MenuItem(MenuItem(c.clone()))),
                    MenuItemType::Predefined => {
                        Box::new(crate::PredefinedMenuItem(PredefinedMenuItem(c.clone())))
                    }
                    MenuItemType::Check => Box::new(crate::CheckMenuItem(CheckMenuItem(c.clone()))),
                    MenuItemType::Icon => Box::new(crate::IconMenuItem(IconMenuItem(c.clone()))),
                }
            })
            .collect()
    }

    pub fn init_for_gtk_window<W>(&self, window: &W) -> crate::Result<gtk::Box>
    where
        W: IsA<gtk::ApplicationWindow>,
        W: IsA<gtk::Container>,
        W: IsA<gtk::Window>,
    {
        let mut self_ = self.0.borrow_mut();
        let id = window.as_ptr() as u32;

        if self_.accel_group.is_none() {
            self_.accel_group = Some(gtk::AccelGroup::new());
        }

        // This is the first time this method has been called on this window
        // so we need to create the menubar and its parent box
        if self_.gtk_menubars.get(&id).is_none() {
            let menu_bar = gtk::MenuBar::new();
            let vbox = gtk::Box::new(Orientation::Vertical, 0);
            window.add(&vbox);
            vbox.show();
            self_.gtk_menubars.insert(id, (Some(menu_bar), vbox));
        } else if let Some((menu_bar, _)) = self_.gtk_menubars.get_mut(&id) {
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
        let (menu_bar, vbox) = self_.gtk_menubars.get(&id).cloned().unwrap();
        let menu_bar = menu_bar.as_ref().unwrap();

        window.add_accel_group(self_.accel_group.as_ref().unwrap());

        drop(self_);

        for item in self.items() {
            self.add_menu_item_with_id(item.as_ref(), id);
        }

        // Show the menubar on the window
        vbox.pack_start(menu_bar, false, false, 0);
        menu_bar.show();

        Ok(vbox)
    }

    pub fn remove_for_gtk_window<W>(&self, window: &W) -> crate::Result<()>
    where
        W: IsA<gtk::ApplicationWindow>,
        W: IsA<gtk::Window>,
    {
        let id = window.as_ptr() as u32;
        let menu_bar = {
            let mut self_ = self.0.borrow_mut();
            self_
                .gtk_menubars
                .remove(&id)
                .ok_or(crate::Error::NotInitialized)?
        };

        if let (Some(menu_bar), vbox) = menu_bar {
            for item in self.items() {
                let _ = self.remove_inner(item.as_ref(), false, Some(id));
            }

            let mut self_ = self.0.borrow_mut();
            // Remove the [`gtk::Menubar`] from the widget tree
            unsafe { menu_bar.destroy() };
            // Detach the accelerators from the window
            window.remove_accel_group(self_.accel_group.as_ref().unwrap());
            // Remove the removed [`gtk::Menubar`] from our cache
            self_.gtk_menubars.insert(id, (None, vbox));
            Ok(())
        } else {
            self.0.borrow_mut().gtk_menubars.insert(id, menu_bar);
            Err(crate::Error::NotInitialized)
        }
    }

    pub fn hide_for_gtk_window<W>(&self, window: &W) -> crate::Result<()>
    where
        W: IsA<gtk::ApplicationWindow>,
    {
        if let Some((Some(menu_bar), _)) =
            self.0.borrow().gtk_menubars.get(&(window.as_ptr() as u32))
        {
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
        if let Some((Some(menu_bar), _)) =
            self.0.borrow().gtk_menubars.get(&(window.as_ptr() as u32))
        {
            menu_bar.show_all();
            Ok(())
        } else {
            Err(crate::Error::NotInitialized)
        }
    }

    pub fn show_context_menu_for_gtk_window(&self, window: &impl IsA<gtk::Widget>, x: f64, y: f64) {
        if let Some(window) = window.window() {
            let gtk_menu = gtk::Menu::new();

            for item in self.items() {
                let gtk_item = item.make_gtk_menu_item(0, None, false);
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

    pub fn gtk_context_menu(&self) -> gtk::Menu {
        let mut add_items = false;

        {
            let mut self_ = self.0.borrow_mut();
            if self_.gtk_menu.1.is_none() {
                self_.gtk_menu.1 = Some(gtk::Menu::new());
                add_items = true;
            }
        }

        if add_items {
            for item in self.items() {
                self.add_menu_item_to_context_menu(item.as_ref());
            }
        }

        self.0.borrow().gtk_menu.1.as_ref().unwrap().clone()
    }
}

#[derive(Clone)]
pub struct Submenu(Rc<RefCell<MenuChild>>);

impl Submenu {
    pub fn new(text: &str, enabled: bool) -> Self {
        let child = Rc::new(RefCell::new(MenuChild {
            text: text.to_string(),
            enabled,
            id: COUNTER.next(),
            children: Some(Vec::new()),
            type_: MenuItemType::Submenu,
            gtk_menu: (COUNTER.next(), None),
            gtk_menu_items: HashMap::new(),
            gtk_menus: HashMap::new(),
            ..Default::default()
        }));

        Self(child)
    }

    pub fn add_menu_item(&self, item: &dyn crate::MenuItemExt, op: AddOp) {
        return_if_predefined_item_not_supported!(item);

        let mut self_ = self.0.borrow_mut();

        for menus in self_.gtk_menus.values() {
            for (menu_id, menu) in menus {
                let gtk_item = item.make_gtk_menu_item(*menu_id, self_.accel_group.as_ref(), true);
                match op {
                    AddOp::Append => menu.append(&gtk_item),
                    AddOp::Insert(position) => menu.insert(&gtk_item, position as i32),
                }
                gtk_item.show();
            }
        }

        {
            let (menu_id, menu) = &self_.gtk_menu;
            if let Some(menu) = menu {
                let gtk_item = item.make_gtk_menu_item(*menu_id, self_.accel_group.as_ref(), true);
                match op {
                    AddOp::Append => menu.append(&gtk_item),
                    AddOp::Insert(position) => menu.insert(&gtk_item, position as i32),
                }
                gtk_item.show();
            }
        }

        match op {
            AddOp::Append => self_.children.as_mut().unwrap().push(item.get_child()),
            AddOp::Insert(position) => self_
                .children
                .as_mut()
                .unwrap()
                .insert(position, item.get_child()),
        }
    }

    fn add_menu_item_with_id(&self, item: &dyn crate::MenuItemExt, id: u32) {
        return_if_predefined_item_not_supported!(item);

        let self_ = self.0.borrow();

        for menus in self_.gtk_menus.values() {
            for (menu_id, menu) in menus.iter().filter(|m| m.0 == id) {
                let gtk_item = item.make_gtk_menu_item(*menu_id, self_.accel_group.as_ref(), true);
                menu.append(&gtk_item);
                gtk_item.show();
            }
        }
    }

    fn add_menu_item_to_context_menu(&self, item: &dyn crate::MenuItemExt) {
        return_if_predefined_item_not_supported!(item);

        let self_ = self.0.borrow();

        let (menu_id, menu) = &self_.gtk_menu;
        if let Some(menu) = menu {
            let gtk_item = item.make_gtk_menu_item(*menu_id, None, true);
            menu.append(&gtk_item);
            gtk_item.show();
        }
    }

    pub fn remove(&self, item: &dyn crate::MenuItemExt) -> crate::Result<()> {
        self.remove_inner(item, true, None)
    }

    fn remove_inner(
        &self,
        item: &dyn crate::MenuItemExt,
        remove_from_cache: bool,
        id: Option<u32>,
    ) -> crate::Result<()> {
        let child = {
            let mut self_ = self.0.borrow_mut();
            let index = self_
                .children
                .as_ref()
                .unwrap()
                .iter()
                .position(|e| e.borrow().id == item.id())
                .ok_or(crate::Error::NotAChildOfThisMenu)?;
            if remove_from_cache {
                self_.children.as_mut().unwrap().remove(index)
            } else {
                self_
                    .children
                    .as_ref()
                    .unwrap()
                    .get(index)
                    .cloned()
                    .unwrap()
            }
        };

        if item.type_() == crate::MenuItemType::Submenu {
            let submenu = item.as_any().downcast_ref::<crate::Submenu>().unwrap();
            let gtk_menus = submenu.0 .0.borrow().gtk_menus.clone();

            for (menu_id, _) in gtk_menus {
                for item in submenu.items() {
                    submenu
                        .0
                        .remove_inner(item.as_ref(), false, Some(menu_id))?;
                }
            }
        }

        let self_ = self.0.borrow();
        for menus in self_.gtk_menus.values() {
            for (menu_id, menu) in menus {
                if id.map(|i| i == *menu_id).unwrap_or(true) {
                    if let Some(items) = child.borrow_mut().gtk_menu_items.remove(menu_id) {
                        for item in items {
                            menu.remove(&item);
                        }
                    }
                }
            }
        }

        if remove_from_cache {
            let (menu_id, menu) = &self_.gtk_menu;
            if let Some(menu) = menu {
                if let Some(items) = child.borrow_mut().gtk_menu_items.remove(menu_id) {
                    for item in items {
                        menu.remove(&item);
                    }
                }
            }
        }

        Ok(())
    }

    pub fn items(&self) -> Vec<Box<dyn crate::MenuItemExt>> {
        self.0
            .borrow()
            .children
            .as_ref()
            .unwrap()
            .iter()
            .map(|c| -> Box<dyn crate::MenuItemExt> {
                let child = c.borrow();
                match child.type_ {
                    MenuItemType::Submenu => Box::new(crate::Submenu(Submenu(c.clone()))),
                    MenuItemType::Normal => Box::new(crate::MenuItem(MenuItem(c.clone()))),
                    MenuItemType::Predefined => {
                        Box::new(crate::PredefinedMenuItem(PredefinedMenuItem(c.clone())))
                    }
                    MenuItemType::Check => Box::new(crate::CheckMenuItem(CheckMenuItem(c.clone()))),
                    MenuItemType::Icon => Box::new(crate::IconMenuItem(IconMenuItem(c.clone()))),
                }
            })
            .collect()
    }

    pub fn show_context_menu_for_gtk_window(&self, window: &impl IsA<gtk::Widget>, x: f64, y: f64) {
        if let Some(window) = window.window() {
            let gtk_menu = gtk::Menu::new();

            for item in self.items() {
                let gtk_item = item.make_gtk_menu_item(0, None, false);
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

    pub fn gtk_context_menu(&self) -> gtk::Menu {
        let mut add_items = false;
        {
            let mut self_ = self.0.borrow_mut();
            if self_.gtk_menu.1.is_none() {
                self_.gtk_menu.1 = Some(gtk::Menu::new());
                add_items = true;
            }
        }

        if add_items {
            for item in self.items() {
                self.add_menu_item_to_context_menu(item.as_ref());
            }
        }

        self.0.borrow().gtk_menu.1.as_ref().unwrap().clone()
    }

    fn make_gtk_menu_item(
        &self,
        menu_id: u32,
        accel_group: Option<&gtk::AccelGroup>,
        add_to_cache: bool,
    ) -> gtk::MenuItem {
        let mut self_ = self.0.borrow_mut();
        let submenu = gtk::Menu::new();
        let item = gtk::MenuItem::builder()
            .label(&to_gtk_mnemonic(&self_.text))
            .use_underline(true)
            .submenu(&submenu)
            .sensitive(self_.enabled)
            .build();

        item.show();
        item.set_submenu(Some(&submenu));

        self_.accel_group = accel_group.cloned();

        let mut id = 0;
        if add_to_cache {
            id = COUNTER.next();

            self_
                .gtk_menu_items
                .entry(menu_id)
                .or_insert_with(Vec::new)
                .push(item.clone());
            self_
                .gtk_menus
                .entry(menu_id)
                .or_insert_with(Vec::new)
                .push((id, submenu.clone()));
        }

        drop(self_);

        for item in self.items() {
            if add_to_cache {
                self.add_menu_item_with_id(item.as_ref(), id);
            } else {
                let gtk_item = item.make_gtk_menu_item(0, None, false);
                submenu.append(&gtk_item);
            }
        }

        item
    }

    pub fn id(&self) -> u32 {
        self.0.borrow().id()
    }
    pub fn text(&self) -> String {
        self.0.borrow().text()
    }

    pub fn set_text(&self, text: &str) {
        self.0.borrow_mut().set_text(text)
    }

    pub fn is_enabled(&self) -> bool {
        self.0.borrow().is_enabled()
    }

    pub fn set_enabled(&self, enabled: bool) {
        self.0.borrow_mut().set_enabled(enabled)
    }
}

#[derive(Clone)]
pub struct MenuItem(Rc<RefCell<MenuChild>>);

impl MenuItem {
    pub fn new(text: &str, enabled: bool, accelerator: Option<Accelerator>) -> Self {
        let child = Rc::new(RefCell::new(MenuChild {
            text: text.to_string(),
            enabled,
            accelerator,
            id: COUNTER.next(),
            type_: MenuItemType::Normal,
            gtk_menu_items: HashMap::new(),
            ..Default::default()
        }));

        Self(child)
    }

    fn make_gtk_menu_item(
        &self,
        menu_id: u32,
        accel_group: Option<&gtk::AccelGroup>,
        add_to_cache: bool,
    ) -> gtk::MenuItem {
        let mut self_ = self.0.borrow_mut();
        let item = gtk::MenuItem::builder()
            .label(&to_gtk_mnemonic(&self_.text))
            .use_underline(true)
            .sensitive(self_.enabled)
            .build();

        self_.accel_group = accel_group.cloned();

        if let Some(accelerator) = &self_.accelerator {
            if let Some(accel_group) = accel_group {
                register_accelerator(&item, accel_group, accelerator);
            }
        }

        let id = self_.id;
        item.connect_activate(move |_| {
            MenuEvent::send(crate::MenuEvent { id });
        });

        if add_to_cache {
            self_
                .gtk_menu_items
                .entry(menu_id)
                .or_insert_with(Vec::new)
                .push(item.clone());
        }

        item
    }

    pub fn id(&self) -> u32 {
        self.0.borrow().id()
    }
    pub fn text(&self) -> String {
        self.0.borrow().text()
    }

    pub fn set_text(&self, text: &str) {
        self.0.borrow_mut().set_text(text)
    }

    pub fn is_enabled(&self) -> bool {
        self.0.borrow().is_enabled()
    }

    pub fn set_enabled(&self, enabled: bool) {
        self.0.borrow_mut().set_enabled(enabled)
    }

    pub fn set_accelerator(&self, acccelerator: Option<Accelerator>) {
        self.0.borrow_mut().set_accelerator(acccelerator)
    }
}

#[derive(Clone)]
pub struct PredefinedMenuItem(Rc<RefCell<MenuChild>>);

impl PredefinedMenuItem {
    pub(crate) fn new(item: PredfinedMenuItemType, text: Option<String>) -> Self {
        let child = Rc::new(RefCell::new(MenuChild {
            text: text.unwrap_or_else(|| item.text().to_string()),
            enabled: true,
            accelerator: item.accelerator(),
            id: COUNTER.next(),
            type_: MenuItemType::Predefined,
            predefined_item_type: item,
            gtk_menu_items: HashMap::new(),
            ..Default::default()
        }));

        Self(child)
    }

    fn make_gtk_menu_item(
        &self,
        menu_id: u32,
        accel_group: Option<&gtk::AccelGroup>,
        add_to_cache: bool,
    ) -> gtk::MenuItem {
        let self_ = self.0.borrow();
        let text = self_.text.clone();
        let accelerator = self_.accelerator;
        let predefined_item_type = self_.predefined_item_type.clone();
        drop(self_);

        let make_item = || {
            gtk::MenuItem::builder()
                .label(&to_gtk_mnemonic(&text))
                .use_underline(true)
                .sensitive(true)
                .build()
        };
        let register_accel = |item: &gtk::MenuItem| {
            if let Some(accelerator) = accelerator {
                if let Some(accel_group) = accel_group {
                    register_accelerator(item, accel_group, &accelerator);
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
            let mut self_ = self.0.borrow_mut();
            self_
                .gtk_menu_items
                .entry(menu_id)
                .or_insert_with(Vec::new)
                .push(item.clone());
        }
        item
    }

    pub fn id(&self) -> u32 {
        self.0.borrow().id()
    }
    pub fn text(&self) -> String {
        self.0.borrow().text()
    }

    pub fn set_text(&self, text: &str) {
        self.0.borrow_mut().set_text(text)
    }
}

#[derive(Clone)]
pub struct CheckMenuItem(Rc<RefCell<MenuChild>>);

impl CheckMenuItem {
    pub fn new(text: &str, enabled: bool, checked: bool, accelerator: Option<Accelerator>) -> Self {
        let child = Rc::new(RefCell::new(MenuChild {
            text: text.to_string(),
            enabled,
            checked,
            accelerator,
            id: COUNTER.next(),
            type_: MenuItemType::Check,
            gtk_menu_items: HashMap::new(),
            is_syncing_checked_state: Rc::new(AtomicBool::new(false)),
            ..Default::default()
        }));

        Self(child)
    }

    fn make_gtk_menu_item(
        &self,
        menu_id: u32,
        accel_group: Option<&gtk::AccelGroup>,
        add_to_cache: bool,
    ) -> gtk::MenuItem {
        let mut self_ = self.0.borrow_mut();
        let item = gtk::CheckMenuItem::builder()
            .label(&to_gtk_mnemonic(&self_.text))
            .use_underline(true)
            .sensitive(self_.enabled)
            .active(self_.checked)
            .build();

        self_.accel_group = accel_group.cloned();

        if let Some(accelerator) = &self_.accelerator {
            if let Some(accel_group) = accel_group {
                register_accelerator(&item, accel_group, accelerator);
            }
        }

        let id = self_.id;
        let self_c = self.0.clone();
        let is_syncing_checked_state = self_.is_syncing_checked_state.clone();
        item.connect_toggled(move |i| {
            let should_dispatch = is_syncing_checked_state
                .compare_exchange(false, true, Ordering::Release, Ordering::Relaxed)
                .is_ok();

            if should_dispatch {
                let checked = i.is_active();
                let (is_syncing_checked_state_c, store) = {
                    let mut self_ = self_c.borrow_mut();
                    self_.checked = checked;
                    (
                        Rc::clone(&self_.is_syncing_checked_state),
                        self_.gtk_menu_items.clone(),
                    )
                };

                for items in store.values() {
                    for i in items {
                        i.downcast_ref::<gtk::CheckMenuItem>()
                            .unwrap()
                            .set_active(checked);
                    }
                }

                is_syncing_checked_state_c.store(false, Ordering::Release);

                MenuEvent::send(crate::MenuEvent { id });
            }
        });

        let item = item.upcast::<gtk::MenuItem>();

        if add_to_cache {
            self_
                .gtk_menu_items
                .entry(menu_id)
                .or_insert_with(Vec::new)
                .push(item.clone());
        }

        item
    }

    pub fn id(&self) -> u32 {
        self.0.borrow().id()
    }
    pub fn text(&self) -> String {
        self.0.borrow().text()
    }

    pub fn set_text(&self, text: &str) {
        self.0.borrow_mut().set_text(text)
    }

    pub fn is_enabled(&self) -> bool {
        self.0.borrow().is_enabled()
    }

    pub fn set_enabled(&self, enabled: bool) {
        self.0.borrow_mut().set_enabled(enabled)
    }

    pub fn is_checked(&self) -> bool {
        self.0.borrow().is_checked()
    }

    pub fn set_checked(&self, checked: bool) {
        self.0.borrow_mut().set_checked(checked)
    }

    pub fn set_accelerator(&self, acccelerator: Option<Accelerator>) {
        self.0.borrow_mut().set_accelerator(acccelerator)
    }
}

#[derive(Clone)]
pub struct IconMenuItem(Rc<RefCell<MenuChild>>);

impl IconMenuItem {
    pub fn new(
        text: &str,
        enabled: bool,
        icon: Option<Icon>,
        accelerator: Option<Accelerator>,
    ) -> Self {
        let child = Rc::new(RefCell::new(MenuChild {
            text: text.to_string(),
            enabled,
            icon,
            accelerator,
            id: COUNTER.next(),
            type_: MenuItemType::Icon,
            gtk_menu_items: HashMap::new(),
            is_syncing_checked_state: Rc::new(AtomicBool::new(false)),
            ..Default::default()
        }));

        Self(child)
    }

    fn make_gtk_menu_item(
        &self,
        menu_id: u32,
        accel_group: Option<&gtk::AccelGroup>,
        add_to_cache: bool,
    ) -> gtk::MenuItem {
        let mut self_ = self.0.borrow_mut();

        let image = self_
            .icon
            .as_ref()
            .map(|i| gtk::Image::from_pixbuf(Some(&i.inner.to_pixbuf_scale(16, 16))))
            .unwrap_or_else(gtk::Image::default);

        self_.accel_group = accel_group.cloned();

        let label = gtk::AccelLabel::builder()
            .label(&to_gtk_mnemonic(&self_.text))
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
            .sensitive(self_.enabled)
            .build();

        if let Some(accelerator) = &self_.accelerator {
            if let Some(accel_group) = accel_group {
                if let Some((mods, key)) = register_accelerator(&item, accel_group, accelerator) {
                    label.set_accel(key, mods);
                }
            }
        }

        let id = self_.id;
        item.connect_activate(move |_| {
            MenuEvent::send(crate::MenuEvent { id });
        });

        if add_to_cache {
            self_
                .gtk_menu_items
                .entry(menu_id)
                .or_insert_with(Vec::new)
                .push(item.clone());
        }

        item
    }

    pub fn id(&self) -> u32 {
        self.0.borrow().id()
    }
    pub fn text(&self) -> String {
        self.0.borrow().text()
    }

    pub fn set_text(&self, text: &str) {
        self.0.borrow_mut().set_text(text)
    }

    pub fn is_enabled(&self) -> bool {
        self.0.borrow().is_enabled()
    }

    pub fn set_enabled(&self, enabled: bool) {
        self.0.borrow_mut().set_enabled(enabled)
    }

    pub fn set_icon(&self, icon: Option<Icon>) {
        self.0.borrow_mut().set_icon(icon)
    }

    pub fn set_accelerator(&self, acccelerator: Option<Accelerator>) {
        self.0.borrow_mut().set_accelerator(acccelerator)
    }
}

impl dyn crate::MenuItemExt + '_ {
    fn get_child(&self) -> Rc<RefCell<MenuChild>> {
        match self.type_() {
            MenuItemType::Submenu => self
                .as_any()
                .downcast_ref::<crate::Submenu>()
                .unwrap()
                .0
                 .0
                .clone(),
            MenuItemType::Normal => self
                .as_any()
                .downcast_ref::<crate::MenuItem>()
                .unwrap()
                .0
                 .0
                .clone(),

            MenuItemType::Predefined => self
                .as_any()
                .downcast_ref::<crate::PredefinedMenuItem>()
                .unwrap()
                .0
                 .0
                .clone(),
            MenuItemType::Check => self
                .as_any()
                .downcast_ref::<crate::CheckMenuItem>()
                .unwrap()
                .0
                 .0
                .clone(),
            MenuItemType::Icon => self
                .as_any()
                .downcast_ref::<crate::IconMenuItem>()
                .unwrap()
                .0
                 .0
                .clone(),
        }
    }

    fn make_gtk_menu_item(
        &self,
        menu_id: u32,
        accel_group: Option<&gtk::AccelGroup>,
        add_to_cache: bool,
    ) -> gtk::MenuItem {
        match self.type_() {
            MenuItemType::Submenu => self
                .as_any()
                .downcast_ref::<crate::Submenu>()
                .unwrap()
                .0
                .make_gtk_menu_item(menu_id, accel_group, add_to_cache),
            MenuItemType::Normal => self
                .as_any()
                .downcast_ref::<crate::MenuItem>()
                .unwrap()
                .0
                .make_gtk_menu_item(menu_id, accel_group, add_to_cache),
            MenuItemType::Predefined => self
                .as_any()
                .downcast_ref::<crate::PredefinedMenuItem>()
                .unwrap()
                .0
                .make_gtk_menu_item(menu_id, accel_group, add_to_cache),
            MenuItemType::Check => self
                .as_any()
                .downcast_ref::<crate::CheckMenuItem>()
                .unwrap()
                .0
                .make_gtk_menu_item(menu_id, accel_group, add_to_cache),
            MenuItemType::Icon => self
                .as_any()
                .downcast_ref::<crate::IconMenuItem>()
                .unwrap()
                .0
                .make_gtk_menu_item(menu_id, accel_group, add_to_cache),
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
            _ => "",
        }
    }
}
