// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

mod icon;
mod mnemonic;

pub(crate) use icon::PlatformIcon;

use crate::{
    accelerator::MenuAccelerator,
    dpi::Position,
    items::*,
    platform_impl::PlatformAttachArgs,
    util::{AddOp, Counter},
    ClickAction, MenuEvent, MenuItemKind,
};
use glib::translate::ToGlibPtr;
use gtk::{gdk, glib, prelude::*, AboutDialog, Container, Orientation};
use mnemonic::{from_gtk_mnemonic, to_gtk_mnemonic};
use std::{
    cell::RefCell,
    collections::{hash_map::Entry, HashMap},
    rc::Rc,
    sync::atomic::{AtomicBool, Ordering},
};

static COUNTER: Counter = Counter::new();

pub struct PlatformMenu {
    gtk_menubars: HashMap<u32, gtk::MenuBar>,
    gtk_windows: HashMap<u32, glib::WeakRef<gtk::Window>>,
    accel_group: Option<gtk::AccelGroup>,
    /// dedicated menu for tray or context menus
    gtk_menu: (u32, Option<gtk::Menu>),
}

impl PlatformMenu {
    pub fn new() -> Self {
        Self {
            gtk_menubars: HashMap::new(),
            gtk_windows: HashMap::new(),
            accel_group: None,
            gtk_menu: (COUNTER.next(), None),
        }
    }

    pub fn destroy(&mut self, children: &[MenuItemKind]) {
        if let Some(accel_group) = &self.accel_group {
            for (_, window) in self.gtk_windows.drain() {
                if let Some(window) = window.upgrade() {
                    window.remove_accel_group(accel_group);
                }
            }
        }

        for (id, menu) in self.gtk_menubars.drain() {
            drop_children_from_menu_and_destroy(id, &menu, children);
            unsafe { menu.destroy() };
        }

        if let Some(menu) = self.gtk_menu.1.take() {
            drop_children_from_menu_and_destroy(self.gtk_menu.0, &menu, children);
            unsafe { menu.destroy() };
        }
    }

    pub fn attach(&mut self, child: &MenuItemKind, op: AddOp) -> crate::Result<()> {
        for (menu_id, menu_bar) in &self.gtk_menubars {
            let gtk_item = child.create_gtk(*menu_id, self.accel_group.as_ref(), true, true)?;
            match op {
                AddOp::Append => menu_bar.append(&gtk_item),
                AddOp::Insert(position) => menu_bar.insert(&gtk_item, position as i32),
            }
            gtk_item.show();
        }

        if let (menu_id, Some(menu)) = &self.gtk_menu {
            let gtk_item = child.create_gtk(*menu_id, self.accel_group.as_ref(), true, false)?;
            match op {
                AddOp::Append => menu.append(&gtk_item),
                AddOp::Insert(position) => menu.insert(&gtk_item, position as i32),
            }
            gtk_item.show();
        }

        Ok(())
    }

    pub fn remove_at(&mut self, index: usize, child: &MenuItemKind) {
        for (menu_id, menu_bar) in &self.gtk_menubars {
            child
                .platform()
                .borrow_mut()
                .remove_instance_for_parent_at_position(child, *menu_id, menu_bar, index);
        }

        if let (menu_id, Some(menu)) = &self.gtk_menu {
            child
                .platform()
                .borrow_mut()
                .remove_instance_for_parent_at_position(child, *menu_id, menu, index);
        }
    }

    pub fn init_for_gtk_window<W, C>(
        &mut self,
        children: &[MenuItemKind],
        window: &W,
        container: Option<&C>,
    ) -> crate::Result<()>
    where
        W: IsA<gtk::Window>,
        W: IsA<gtk::Widget>,
        C: IsA<gtk::Widget>,
    {
        let id = window.as_ptr() as u32;

        if self.accel_group.is_none() {
            self.accel_group = Some(gtk::AccelGroup::new());
        }

        // This is the first time this method has been called on this window
        // so we need to create the menubar and its parent box
        if let Entry::Vacant(e) = self.gtk_menubars.entry(id) {
            let menu_bar = gtk::MenuBar::new();
            e.insert(menu_bar);
            self.gtk_windows
                .insert(id, window.upcast_ref::<gtk::Window>().downgrade());
        } else {
            return Err(crate::Error::AlreadyInitialized);
        }

        // Construct the entries of the menubar
        let menu_bar = &self.gtk_menubars[&id];

        window.add_accel_group(self.accel_group.as_ref().unwrap());

        for child in children {
            let gtk_item = child.create_gtk(id, self.accel_group.as_ref(), true, true)?;
            menu_bar.append(&gtk_item);
            gtk_item.show();
        }

        Self::attach_menubar_to_window(window, container, menu_bar)?;

        // Show the menubar
        menu_bar.show();

        Ok(())
    }

    fn attach_menubar_to_window<W, C>(
        window: &W,
        container: Option<&C>,
        menu_bar: &gtk::MenuBar,
    ) -> crate::Result<()>
    where
        W: gtk::prelude::IsA<gtk::Window>,
        C: gtk::prelude::IsA<gtk::Widget>,
    {
        if let Some(container) = container {
            if let Some(gtk_box) = container.dynamic_cast_ref::<gtk::Box>() {
                gtk_box.pack_start(menu_bar, false, false, 0);
                gtk_box.reorder_child(menu_bar, 0);
            } else if let Some(gtk_fixed) = container.dynamic_cast_ref::<gtk::Fixed>() {
                gtk_fixed.add(menu_bar);
            } else if let Some(gtk_stack) = container.dynamic_cast_ref::<gtk::Stack>() {
                gtk_stack.add(menu_bar);
            } else {
                return Err(crate::Error::UnsupportedGtkContainer);
            }
        } else {
            if let Some(w) = window.dynamic_cast_ref::<gtk::Window>() {
                w.set_child(Some(menu_bar));
            } else {
                return Err(crate::Error::UnsupportedGtkContainer);
            }
        }

        Ok(())
    }

    pub fn remove_for_gtk_window<W>(
        &mut self,
        children: &[MenuItemKind],
        window: &W,
    ) -> crate::Result<()>
    where
        W: IsA<gtk::Window>,
        W: IsA<gtk::Widget>,
    {
        let id = window.as_ptr() as u32;

        // Remove from our cache
        let menu_bar = self
            .gtk_menubars
            .remove(&id)
            .ok_or(crate::Error::NotInitialized)?;
        self.gtk_windows.remove(&id);

        drop_children_from_menu_and_destroy(id, &menu_bar, children);

        // Remove the [`gtk::Menubar`] from the widget tree
        unsafe { menu_bar.destroy() };
        // Detach the accelerators from the window
        window.remove_accel_group(self.accel_group.as_ref().unwrap());
        Ok(())
    }

    pub fn hide_for_gtk_window<W>(&mut self, window: &W) -> crate::Result<()>
    where
        W: IsA<gtk::Window>,
    {
        self.gtk_menubars
            .get(&(window.as_ptr() as u32))
            .ok_or(crate::Error::NotInitialized)?
            .hide();
        Ok(())
    }

    pub fn show_for_gtk_window<W>(&self, window: &W) -> crate::Result<()>
    where
        W: IsA<gtk::Window>,
    {
        self.gtk_menubars
            .get(&(window.as_ptr() as u32))
            .ok_or(crate::Error::NotInitialized)?
            .show_all();
        Ok(())
    }

    pub fn is_visible_on_gtk_window<W>(&self, window: &W) -> bool
    where
        W: IsA<gtk::Window>,
    {
        self.gtk_menubars
            .get(&(window.as_ptr() as u32))
            .map(|m| m.get_visible())
            .unwrap_or(false)
    }

    pub fn gtk_menubar_for_gtk_window<W>(&self, window: &W) -> Option<gtk::MenuBar>
    where
        W: gtk::prelude::IsA<gtk::Window>,
    {
        self.gtk_menubars.get(&(window.as_ptr() as u32)).cloned()
    }

    pub fn show_context_menu_for_gtk_window(
        &mut self,
        children: &[MenuItemKind],
        widget: &impl IsA<gtk::Widget>,
        position: Option<Position>,
    ) -> bool {
        let menu = self.gtk_context_menu(children);
        show_context_menu(menu, widget, position)
    }

    pub fn gtk_context_menu(&mut self, children: &[MenuItemKind]) -> gtk::Menu {
        let mut add_items = false;

        if self.gtk_menu.1.is_none() {
            self.gtk_menu.1 = Some(gtk::Menu::new());
            add_items = true;
        }

        if add_items {
            for child in children {
                let gtk_item = child
                    .create_gtk(self.gtk_menu.0, self.accel_group.as_ref(), true, false)
                    .unwrap();
                self.gtk_menu.1.as_ref().unwrap().append(&gtk_item);
                gtk_item.show();
            }
        }

        self.gtk_menu.1.as_ref().unwrap().clone()
    }
}

/// A generic child in a menu
pub struct PlatformMenuItem {
    gtk_menu_items: Rc<RefCell<HashMap<u32, Vec<gtk::MenuItem>>>>,

    // menu item fields
    gtk_accelerator: Option<(gdk::ModifierType, u32)>,
    is_syncing: Option<Rc<AtomicBool>>,

    // submenu fields
    gtk_menus: Option<HashMap<u32, Vec<(u32, gtk::Menu)>>>,
    gtk_menu: Option<(u32, Option<gtk::Menu>)>, // dedicated menu for tray or context menus
    accel_groups: HashMap<u32, gtk::AccelGroup>,
}

fn drop_children_from_menu_and_destroy(
    id: u32,
    menu: &impl IsA<Container>,
    children: &[MenuItemKind],
) {
    for child in children {
        let descendants = child.children();
        let child_platform = child.platform();
        let mut child_ = child_platform.borrow_mut();
        {
            let mut menu_items = child_.gtk_menu_items.borrow_mut();
            if let Some(items) = menu_items.remove(&id) {
                for item in items {
                    menu.remove(&item);
                    if let Some(accel_group) = child_.accel_groups.get(&id) {
                        if let Some((mods, key)) = child_.gtk_accelerator {
                            item.remove_accelerator(accel_group, key, mods);
                        }
                    }
                    unsafe { item.destroy() }
                }
            }
        }

        child_.accel_groups.remove(&id);

        if child_.gtk_menus.is_some() {
            if let Some(menus) = child_.gtk_menus.as_mut().unwrap().remove(&id) {
                for (menu_id, menu) in menus {
                    drop_children_from_menu_and_destroy(menu_id, &menu, &descendants);
                    child_.accel_groups.remove(&menu_id);
                    unsafe { menu.destroy() }
                }
            }
        }
    }
}

/// Constructors
impl PlatformMenuItem {
    pub fn new(click: ClickAction) -> Self {
        let needs_syncing = matches!(click, ClickAction::Toggle(..));
        let is_syncing = needs_syncing.then(|| Rc::new(AtomicBool::new(false)));

        Self {
            gtk_menu_items: Rc::new(RefCell::new(HashMap::new())),
            accel_groups: HashMap::new(),
            gtk_accelerator: None,
            gtk_menu: None,
            gtk_menus: None,
            is_syncing,
        }
    }

    pub fn new_submenu(click: ClickAction) -> Self {
        let mut item = Self::new(click);
        item.gtk_menu = Some((COUNTER.next(), None));
        item.gtk_menus = Some(HashMap::new());
        item
    }

    pub fn destroy(&mut self, children: &[MenuItemKind]) {
        if let Some(gtk_menus) = &mut self.gtk_menus {
            for (_, menus) in gtk_menus.drain() {
                for (id, menu) in menus {
                    drop_children_from_menu_and_destroy(id, &menu, children);
                    self.accel_groups.remove(&id);
                    unsafe { menu.destroy() };
                }
            }
        }

        if let Some((id, gtk_menu)) = &mut self.gtk_menu {
            if let Some(menu) = gtk_menu.take() {
                drop_children_from_menu_and_destroy(*id, &menu, children);
                unsafe { menu.destroy() };
            }
        }
    }
}

/// Shared methods
impl PlatformMenuItem {
    fn register_accelerator(
        &mut self,
        args: &PlatformAttachArgs,
        item: &impl IsA<gtk::Widget>,
        menu_id: u32,
        accel_group: Option<&gtk::AccelGroup>,
        add_to_cache: bool,
    ) -> crate::Result<()> {
        self.gtk_accelerator = args
            .accelerator
            .as_ref()
            .map(MenuAccelerator::to_gtk)
            .transpose()?;

        if let (Some((mods, key)), Some(accel_group)) = (&self.gtk_accelerator, accel_group) {
            item.add_accelerator(
                "activate",
                accel_group,
                *key,
                *mods,
                gtk::AccelFlags::VISIBLE,
            );
        }

        if add_to_cache {
            if let Some(accel_group) = accel_group {
                self.accel_groups.insert(menu_id, accel_group.clone());
            }
        }

        Ok(())
    }

    pub fn text(&self) -> Option<String> {
        match self
            .gtk_menu_items
            .borrow()
            .values()
            .collect::<Vec<_>>()
            .first()
            .map(|v| v.first())
            .map(|e| e.map(|i| i.label().map(from_gtk_mnemonic)))
        {
            Some(Some(Some(text))) => Some(text),
            _ => None,
        }
    }

    pub fn set_text(&mut self, text: &str, _accelerator: Option<&MenuAccelerator>) {
        let text = to_gtk_mnemonic(text);
        for items in self.gtk_menu_items.borrow().values() {
            for i in items {
                i.set_label(&text);
            }
        }
    }

    pub fn is_enabled(&self) -> Option<bool> {
        match self
            .gtk_menu_items
            .borrow()
            .values()
            .collect::<Vec<_>>()
            .first()
            .map(|v| v.first())
            .map(|e| e.map(|i| i.is_sensitive()))
        {
            Some(Some(enabled)) => Some(enabled),
            _ => None,
        }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        for items in self.gtk_menu_items.borrow().values() {
            for i in items {
                i.set_sensitive(enabled);
            }
        }
    }

    pub fn set_accelerator(
        &mut self,
        _text: &str,
        accelerator: Option<&MenuAccelerator>,
    ) -> crate::Result<()> {
        let prev_accel = self.gtk_accelerator;
        let new_accel = accelerator.map(MenuAccelerator::to_gtk).transpose()?;
        self.gtk_accelerator = new_accel;

        for (parent_id, items) in self.gtk_menu_items.borrow().iter() {
            let Some(accel_group) = self.accel_groups.get(parent_id) else {
                continue;
            };
            for i in items {
                if let Some((mods, key)) = prev_accel {
                    i.remove_accelerator(accel_group, key, mods);
                }

                if let Some((mods, key)) = new_accel {
                    i.add_accelerator("activate", accel_group, key, mods, gtk::AccelFlags::VISIBLE)
                }
            }
        }

        Ok(())
    }
}

/// CheckMenuItem methods
impl PlatformMenuItem {
    pub fn is_checked(&self) -> Option<bool> {
        match self
            .gtk_menu_items
            .borrow()
            .values()
            .collect::<Vec<_>>()
            .first()
            .map(|v| v.first())
            .map(|e| e.map(|i| i.downcast_ref::<gtk::CheckMenuItem>().unwrap().is_active()))
        {
            Some(Some(checked)) => Some(checked),
            _ => None,
        }
    }

    pub fn set_checked(&mut self, checked: bool) {
        let is_syncing = self
            .is_syncing
            .as_ref()
            .expect("checked state can only be set on a check menu item");

        is_syncing.store(true, Ordering::Release);
        for items in self.gtk_menu_items.borrow().values() {
            for i in items {
                let i = i.downcast_ref::<gtk::CheckMenuItem>().unwrap();
                i.set_active(checked);
            }
        }
        is_syncing.store(false, Ordering::Release);
    }
}

/// IconMenuItem methods
impl PlatformMenuItem {
    fn icon_image(icon: Option<&IconType>) -> Option<gtk::Image> {
        if let Some(IconType::Custom(icon)) = icon {
            let icon = icon.inner.to_pixbuf_scale(16, 16);
            Some(gtk::Image::from_pixbuf(Some(&icon)))
        } else {
            let IconType::Native(icon) = icon? else {
                return None;
            };
            let image = gtk::Image::from_icon_name(Some(icon.gtk_icon_name()), gtk::IconSize::Menu);
            Some(image)
        }
    }

    pub fn set_icon(&mut self, icon: Option<&IconType>) {
        let pixbuf = icon.and_then(|icon| match icon {
            IconType::Custom(icon) => Some(icon.inner.to_pixbuf_scale(16, 16)),
            _ => None,
        });

        for items in self.gtk_menu_items.borrow().values() {
            for i in items {
                let box_container = i.child().unwrap().downcast::<gtk::Box>().unwrap();
                let children = box_container.children();

                // Check if the first child is an image (it might not be if the item
                // was created for menu bar without an icon)
                if let Some(image) = children
                    .first()
                    .and_then(|c| c.downcast_ref::<gtk::Image>())
                {
                    if let Some(pixbuf) = pixbuf.as_ref() {
                        image.set_pixbuf(Some(pixbuf));
                    } else if let Some(IconType::Native(native_icon)) = icon {
                        let native_icon = native_icon.gtk_icon_name();
                        image.set_from_icon_name(Some(native_icon), gtk::IconSize::Menu);
                    } else {
                        box_container.remove(image);
                    }
                } else if let Some(image) = Self::icon_image(icon) {
                    // No image widget exists yet, but we're setting an icon, so create one
                    box_container.pack_start(&image, false, false, 0);
                    box_container.reorder_child(&image, 0);
                    image.show();
                }
            }
        }
    }
}

/// Submenu methods
impl PlatformMenuItem {
    pub fn attach(&mut self, child: &MenuItemKind, op: AddOp) -> crate::Result<()> {
        for menus in self.gtk_menus.as_ref().unwrap().values() {
            for (menu_id, menu) in menus {
                let accel_group = self.accel_groups.get(menu_id);
                let gtk_item = child.create_gtk(*menu_id, accel_group, true, false)?;
                match op {
                    AddOp::Append => menu.append(&gtk_item),
                    AddOp::Insert(position) => menu.insert(&gtk_item, position as i32),
                }
                gtk_item.show();
            }
        }

        if let Some((menu_id, Some(menu))) = &self.gtk_menu {
            let accel_group = self.accel_groups.get(menu_id);
            let gtk_item = child.create_gtk(*menu_id, accel_group, true, false)?;
            match op {
                AddOp::Append => menu.append(&gtk_item),
                AddOp::Insert(position) => menu.insert(&gtk_item, position as i32),
            }
            gtk_item.show();
        }

        Ok(())
    }

    pub fn remove_at(&mut self, index: usize, child: &MenuItemKind) {
        for menus in self.gtk_menus.as_ref().unwrap().values() {
            for (menu_id, menu) in menus {
                child
                    .platform()
                    .borrow_mut()
                    .remove_instance_for_parent_at_position(child, *menu_id, menu, index);
            }
        }

        if let (id, Some(menu)) = self.gtk_menu.as_ref().unwrap() {
            child
                .platform()
                .borrow_mut()
                .remove_instance_for_parent_at_position(child, *id, menu, index);
        }
    }

    pub fn show_context_menu_for_gtk_window(
        &mut self,
        children: &[MenuItemKind],
        widget: &impl IsA<gtk::Widget>,
        position: Option<Position>,
    ) -> bool {
        let menu = self.gtk_context_menu(children);
        show_context_menu(menu, widget, position)
    }

    pub fn gtk_context_menu(&mut self, children: &[MenuItemKind]) -> gtk::Menu {
        let mut add_items = false;
        {
            let gtk_menu = self.gtk_menu.as_mut().unwrap();
            if gtk_menu.1.is_none() {
                gtk_menu.1 = Some(gtk::Menu::new());
                add_items = true;
            }
        }

        if add_items {
            let (menu_id, menu) = self.gtk_menu.as_ref().unwrap();
            for child in children {
                let accel_group = self.accel_groups.get(menu_id);
                let gtk_item = child
                    .create_gtk(*menu_id, accel_group, true, false)
                    .unwrap();
                menu.as_ref().unwrap().append(&gtk_item);
                gtk_item.show();
            }
        }

        self.gtk_menu.as_ref().unwrap().1.as_ref().unwrap().clone()
    }

    fn remove_instance_for_parent_at_position(
        &mut self,
        child: &MenuItemKind,
        parent_id: u32,
        parent_menu: &impl IsA<Container>,
        position: usize,
    ) {
        let item = parent_menu
            .children()
            .get(position)
            .and_then(|item| item.clone().downcast::<gtk::MenuItem>().ok());

        let Some(item) = item else {
            return;
        };

        let Some(occurrence_index) = self.remove_gtk_menu_item(parent_id, &item) else {
            return;
        };

        if let Some(gtk_menus) = self.gtk_menus.as_mut() {
            let removed_menu = {
                let mut remove_parent_entry = false;
                let removed_menu = gtk_menus.get_mut(&parent_id).and_then(|menus| {
                    let removed_menu =
                        (occurrence_index < menus.len()).then(|| menus.remove(occurrence_index));
                    remove_parent_entry = menus.is_empty();
                    removed_menu
                });

                if remove_parent_entry {
                    gtk_menus.remove(&parent_id);
                }
                removed_menu
            };

            if let Some((menu_id, menu)) = removed_menu {
                let descendants = child.children();
                drop_children_from_menu_and_destroy(menu_id, &menu, &descendants);
                self.accel_groups.remove(&menu_id);
                unsafe { menu.destroy() };
            }
        }

        parent_menu.remove(&item);
        if let Some(accel_group) = self.accel_groups.get(&parent_id) {
            if let Some((mods, key)) = self.gtk_accelerator {
                item.remove_accelerator(accel_group, key, mods);
            }
        }
        unsafe { item.destroy() };

        if !self.gtk_menu_items.borrow().contains_key(&parent_id) {
            self.accel_groups.remove(&parent_id);
        }
    }

    fn remove_gtk_menu_item(&mut self, parent_id: u32, item: &gtk::MenuItem) -> Option<usize> {
        let mut removed = None;
        let mut gtk_menu_items = self.gtk_menu_items.borrow_mut();
        if let Some(items) = gtk_menu_items.get_mut(&parent_id) {
            if let Some(occurrence_index) = items.iter().position(|current| current == item) {
                items.remove(occurrence_index);
                removed = Some(occurrence_index);

                if items.is_empty() {
                    gtk_menu_items.remove(&parent_id);
                }
            }
        }

        removed
    }
}

/// Gtk menu item creation methods
impl PlatformMenuItem {
    fn create_gtk_submenu(
        &mut self,
        args: &PlatformAttachArgs,
        children: &[MenuItemKind],
        menu_id: u32,
        accel_group: Option<&gtk::AccelGroup>,
        add_to_cache: bool,
        for_menu_bar: bool,
    ) -> crate::Result<gtk::MenuItem> {
        let submenu = gtk::Menu::new();

        let image = Self::icon_image(args.icon.as_ref()).unwrap_or_default();

        let label = gtk::AccelLabel::builder()
            .label(to_gtk_mnemonic(&args.text))
            .use_underline(true)
            .xalign(0.0)
            .build();

        let box_container = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        if !for_menu_bar {
            let style_context = box_container.style_context();
            let css_provider = gtk::CssProvider::new();
            let theme = r#"
            box {
                margin-left: -22px;
                }
                "#;
            let _ = css_provider.load_from_data(theme.as_bytes());
            style_context.add_provider(&css_provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);
        }
        if !for_menu_bar || args.icon.is_some() {
            box_container.pack_start(&image, false, false, 0);
        }
        box_container.pack_start(&label, true, true, 0);
        box_container.show_all();

        let item = gtk::MenuItem::builder()
            .child(&box_container)
            .sensitive(args.enabled)
            .build();

        item.set_submenu(Some(&submenu));
        item.show();

        let mut id = 0;
        if add_to_cache {
            id = COUNTER.next();

            if let Some(accel_group) = accel_group {
                self.accel_groups.insert(id, accel_group.clone());
            }

            self.gtk_menu_items
                .borrow_mut()
                .entry(menu_id)
                .or_default()
                .push(item.clone());
            self.gtk_menus
                .as_mut()
                .unwrap()
                .entry(menu_id)
                .or_default()
                .push((id, submenu.clone()));
        }

        for child in children {
            let gtk_item = child.create_gtk(id, accel_group, add_to_cache, false)?;
            submenu.append(&gtk_item);
            gtk_item.show();
        }

        Ok(item)
    }

    fn create_gtk_item(
        &mut self,
        args: &PlatformAttachArgs,
        click: &ClickAction,
        menu_id: u32,
        accel_group: Option<&gtk::AccelGroup>,
        add_to_cache: bool,
    ) -> crate::Result<gtk::MenuItem> {
        let item = gtk::MenuItem::builder()
            .label(to_gtk_mnemonic(&args.text))
            .use_underline(true)
            .sensitive(args.enabled)
            .build();

        self.register_accelerator(args, &item, menu_id, accel_group, add_to_cache)?;

        let id = match click {
            ClickAction::Emit(id) => id.clone(),
            _ => unreachable!("regular item without emit action"),
        };
        item.connect_activate(move |_| {
            MenuEvent::send(crate::MenuEvent { id: id.clone() });
        });

        if add_to_cache {
            self.gtk_menu_items
                .borrow_mut()
                .entry(menu_id)
                .or_default()
                .push(item.clone());
        }

        Ok(item)
    }

    fn create_gtk_predefined_item(
        &mut self,
        args: &PlatformAttachArgs,
        click: &ClickAction,
        menu_id: u32,
        accel_group: Option<&gtk::AccelGroup>,
        add_to_cache: bool,
    ) -> crate::Result<gtk::MenuItem> {
        let predefined_item_type = match click {
            ClickAction::Predefined(state) => state
                .upgrade()
                .map(|state| state.borrow().predefined_item_type.clone())
                .expect("predefined menu item state was dropped"),
            _ => unreachable!("predefined item without predefined action"),
        };

        let make_item = || {
            gtk::MenuItem::builder()
                .label(to_gtk_mnemonic(&args.text))
                .use_underline(true)
                .sensitive(true)
                .build()
        };

        let item = if matches!(&predefined_item_type, PredefinedMenuItemType::Separator) {
            gtk::SeparatorMenuItem::new().upcast::<gtk::MenuItem>()
        } else if predefined_item_type.is_supported_on_gtk() {
            let item = make_item();

            if matches!(
                &predefined_item_type,
                PredefinedMenuItemType::Copy
                    | PredefinedMenuItemType::Cut
                    | PredefinedMenuItemType::Paste
                    | PredefinedMenuItemType::SelectAll
            ) {
                // These items do not need an accelerator as GTK automatically have them,
                // but we need to set the accelerator label so that it is displayed in the menu
                let (mods, key) = predefined_item_type
                    .accelerator()
                    .unwrap()
                    .to_gtk()
                    .unwrap();
                item.child()
                    .unwrap()
                    .downcast::<gtk::AccelLabel>()
                    .unwrap()
                    .set_accel(key, mods);
            } else {
                self.register_accelerator(args, &item, menu_id, accel_group, add_to_cache)?;
            }

            item.connect_activate(move |_| run_predefined(&predefined_item_type));
            item
        } else {
            // Render unsupported predefined menu items as disabled menu items
            let item = make_item();
            self.register_accelerator(args, &item, menu_id, accel_group, add_to_cache)?;
            item.set_sensitive(false);
            item
        };

        if add_to_cache {
            self.gtk_menu_items
                .borrow_mut()
                .entry(menu_id)
                .or_default()
                .push(item.clone());
        }
        Ok(item)
    }

    fn create_gtk_check_item(
        &mut self,
        args: &PlatformAttachArgs,
        click: &ClickAction,
        menu_id: u32,
        accel_group: Option<&gtk::AccelGroup>,
        add_to_cache: bool,
    ) -> crate::Result<gtk::MenuItem> {
        let item = gtk::CheckMenuItem::builder()
            .label(to_gtk_mnemonic(&args.text))
            .use_underline(true)
            .sensitive(args.enabled)
            .active(args.checked)
            .build();

        self.register_accelerator(args, &item, menu_id, accel_group, add_to_cache)?;

        let (id, state) = match click {
            ClickAction::Toggle(id, state) => (id.clone(), state.clone()),
            _ => unreachable!("check item without toggle action"),
        };

        let is_syncing = self
            .is_syncing
            .clone()
            .expect("check menu item is missing its synchronization state");

        let store = self.gtk_menu_items.clone();
        item.connect_toggled(move |i| {
            let should_dispatch = is_syncing
                .compare_exchange(false, true, Ordering::Release, Ordering::Relaxed)
                .is_ok();

            if should_dispatch {
                let c = i.is_active();
                if let Some(state) = state.upgrade() {
                    state.borrow_mut().checked = c;
                }

                for items in store.borrow().values() {
                    for i in items {
                        i.downcast_ref::<gtk::CheckMenuItem>()
                            .unwrap()
                            .set_active(c);
                    }
                }

                is_syncing.store(false, Ordering::Release);

                MenuEvent::send(crate::MenuEvent { id: id.clone() });
            }
        });

        let item = item.upcast::<gtk::MenuItem>();

        if add_to_cache {
            self.gtk_menu_items
                .borrow_mut()
                .entry(menu_id)
                .or_default()
                .push(item.clone());
        }

        Ok(item)
    }

    fn create_gtk_icon_item(
        &mut self,
        args: &PlatformAttachArgs,
        click: &ClickAction,
        menu_id: u32,
        accel_group: Option<&gtk::AccelGroup>,
        add_to_cache: bool,
        for_menu_bar: bool,
    ) -> crate::Result<gtk::MenuItem> {
        let image = Self::icon_image(args.icon.as_ref()).unwrap_or_default();

        let label = gtk::AccelLabel::builder()
            .label(to_gtk_mnemonic(&args.text))
            .use_underline(true)
            .xalign(0.0)
            .build();

        let box_container = gtk::Box::new(Orientation::Horizontal, 6);
        if !for_menu_bar {
            let style_context = box_container.style_context();
            let css_provider = gtk::CssProvider::new();
            let theme = r#"
            box {
                margin-left: -22px;
                }
                "#;
            let _ = css_provider.load_from_data(theme.as_bytes());
            style_context.add_provider(&css_provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);
        }
        if !for_menu_bar || args.icon.is_some() {
            box_container.pack_start(&image, false, false, 0);
        }
        box_container.pack_start(&label, true, true, 0);
        box_container.show_all();

        let item = gtk::MenuItem::builder()
            .child(&box_container)
            .sensitive(args.enabled)
            .build();

        self.register_accelerator(args, &item, menu_id, accel_group, add_to_cache)?;

        let id = match click {
            ClickAction::Emit(id) => id.clone(),
            _ => unreachable!("icon item without emit action"),
        };
        item.connect_activate(move |_| {
            MenuEvent::send(crate::MenuEvent { id: id.clone() });
        });

        if add_to_cache {
            self.gtk_menu_items
                .borrow_mut()
                .entry(menu_id)
                .or_default()
                .push(item.clone());
        }

        Ok(item)
    }
}

impl MenuItemKind {
    fn create_gtk(
        &self,
        menu_id: u32,
        accel_group: Option<&gtk::AccelGroup>,
        add_to_cache: bool,
        for_menu_bar: bool,
    ) -> crate::Result<gtk::MenuItem> {
        let args = self.platform_attach_args();
        let click = self.click_action();
        let platform = self.platform();
        let mut item = platform.borrow_mut();

        match self {
            Self::Submenu(_) => item.create_gtk_submenu(
                &args,
                &self.children(),
                menu_id,
                accel_group,
                add_to_cache,
                for_menu_bar,
            ),
            Self::MenuItem(_) => {
                item.create_gtk_item(&args, &click, menu_id, accel_group, add_to_cache)
            }
            Self::Predefined(_) => {
                item.create_gtk_predefined_item(&args, &click, menu_id, accel_group, add_to_cache)
            }
            Self::Check(_) => {
                item.create_gtk_check_item(&args, &click, menu_id, accel_group, add_to_cache)
            }
            Self::Icon(_) => item.create_gtk_icon_item(
                &args,
                &click,
                menu_id,
                accel_group,
                add_to_cache,
                for_menu_bar,
            ),
        }
    }
}

fn show_context_menu(
    gtk_menu: gtk::Menu,
    widget: &impl IsA<gtk::Widget>,
    position: Option<Position>,
) -> bool {
    let (pos, window) = if let Some(pos) = position {
        let window = widget.window();
        let scale = window.as_ref().map(|w| w.scale_factor()).unwrap_or(1) as _;
        (pos.to_logical::<i32>(scale).into(), window)
    } else {
        let root_window = widget.screen().and_then(|s| s.root_window());

        let seat = root_window
            .as_ref()
            .and_then(|w| w.display().default_seat());
        let pointer = seat.and_then(|s| s.pointer());

        let pos = pointer.map(|p| p.position());
        let pos = pos.map(|p| (p.1, p.2)).unwrap_or_default();

        (pos, root_window)
    };

    let Some(window) = window else {
        return false;
    };

    let mut event = gdk::Event::new(gdk::EventType::ButtonPress);
    event.set_device(
        window
            .display()
            .default_seat()
            .and_then(|d| d.pointer())
            .as_ref(),
    );

    // Set the time of the event otherwise GTK will close the menu
    // when right click is released
    let event_ffi: *mut gdk::ffi::GdkEvent = event.to_glib_none().0;
    if !event_ffi.is_null() {
        let time = glib::monotonic_time() / 1000;
        unsafe {
            (*event_ffi).button.time = time as _;
        }
    }

    let (tx, rx) = crossbeam_channel::unbounded();
    let tx_clone = tx.clone();
    let id = gtk_menu.connect_cancel(move |_| tx_clone.send(false).unwrap_or(()));
    let id2 = gtk_menu.connect_selection_done(move |_| tx.send(true).unwrap_or(()));
    gtk_menu.popup_at_rect(
        &window,
        &gdk::Rectangle::new(pos.0, pos.1, 0, 0),
        gdk::Gravity::NorthWest,
        gdk::Gravity::NorthWest,
        Some(&event),
    );

    loop {
        gtk::main_iteration();

        match rx.try_recv() {
            Ok(result) => {
                gtk_menu.disconnect(id);
                gtk_menu.disconnect(id2);
                break result;
            }
            Err(err) => {
                if err.is_disconnected() {
                    gtk_menu.disconnect(id);
                    gtk_menu.disconnect(id2);
                    break false;
                }
            }
        }
    }
}

impl PredefinedMenuItemType {
    fn is_supported_on_gtk(&self) -> bool {
        matches!(
            self,
            PredefinedMenuItemType::Separator
                | PredefinedMenuItemType::Copy
                | PredefinedMenuItemType::Cut
                | PredefinedMenuItemType::Paste
                | PredefinedMenuItemType::SelectAll
                | PredefinedMenuItemType::About(_)
        )
    }

    #[cfg(feature = "libxdo")]
    fn xdo_keys(&self) -> &str {
        match self {
            PredefinedMenuItemType::Copy => "ctrl+c",
            PredefinedMenuItemType::Cut => "ctrl+X",
            PredefinedMenuItemType::Paste => "ctrl+v",
            PredefinedMenuItemType::SelectAll => "ctrl+a",
            _ => unreachable!(),
        }
    }
}

fn run_predefined(predefined_item_type: &PredefinedMenuItemType) {
    match predefined_item_type {
        PredefinedMenuItemType::Copy
        | PredefinedMenuItemType::Cut
        | PredefinedMenuItemType::Paste
        | PredefinedMenuItemType::SelectAll => {
            // TODO: wayland
            #[cfg(feature = "libxdo")]
            if let Ok(xdo) = libxdo::XDo::new(None) {
                let _ = xdo.send_keysequence(predefined_item_type.xdo_keys(), 0);
            }
        }
        PredefinedMenuItemType::About(Some(metadata)) => show_about_dialog(metadata),
        PredefinedMenuItemType::About(None) => {}
        _ => unreachable!("unsupported predefined item activated"),
    }
}

fn show_about_dialog(metadata: &crate::AboutMetadata) {
    let mut builder = AboutDialog::builder().modal(true).resizable(false);

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
