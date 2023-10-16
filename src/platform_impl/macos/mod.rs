// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

mod accelerator;
mod icon;
mod util;

pub(crate) use icon::PlatformIcon;

use std::{cell::RefCell, collections::HashMap, rc::Rc, sync::Once};

use cocoa::{
    appkit::{self, CGFloat, NSApp, NSApplication, NSEventModifierFlags, NSMenu, NSMenuItem},
    base::{id, nil, selector, NO, YES},
    foundation::{
        NSArray, NSAutoreleasePool, NSDictionary, NSInteger, NSPoint, NSRect, NSSize, NSString,
    },
};
use objc::{
    declare::ClassDecl,
    runtime::{Class, Object, Sel},
};

use self::util::{app_name_string, strip_mnemonic};
use crate::{
    accelerator::Accelerator,
    icon::{Icon, NativeIcon},
    items::*,
    util::{AddOp, Counter},
    IsMenuItem, LogicalPosition, MenuEvent, MenuId, MenuItemKind, MenuItemType, Position,
};

static COUNTER: Counter = Counter::new();
static BLOCK_PTR: &str = "mudaMenuItemBlockPtr";

#[link(name = "AppKit", kind = "framework")]
extern "C" {
    static NSAboutPanelOptionApplicationName: id;
    static NSAboutPanelOptionApplicationIcon: id;
    static NSAboutPanelOptionApplicationVersion: id;
    static NSAboutPanelOptionCredits: id;
    static NSAboutPanelOptionVersion: id;
}

/// https://developer.apple.com/documentation/appkit/nsapplication/1428479-orderfrontstandardaboutpanelwith#discussion
#[allow(non_upper_case_globals)]
const NSAboutPanelOptionCopyright: &str = "Copyright";

#[derive(Debug, Clone)]
struct NsMenuRef(u32, id);

impl Drop for NsMenuRef {
    fn drop(&mut self) {
        unsafe {
            let _: () = msg_send![self.1, removeAllItems];
            let _: () = msg_send![self.1, release];
        }
    }
}

#[derive(Debug, Clone)]
struct NsMenuItemRef(id);

impl Drop for NsMenuItemRef {
    fn drop(&mut self) {
        unsafe {
            let _: () = msg_send![self.0, release];
        }
    }
}

#[derive(Debug)]
pub struct Menu {
    id: MenuId,
    ns_menu: NsMenuRef,
    children: Vec<Rc<RefCell<MenuChild>>>,
}

impl Drop for Menu {
    fn drop(&mut self) {
        for child in &self.children {
            let mut child_ = child.borrow_mut();
            child_.ns_menu_items.remove(&self.ns_menu.0);
            if child_.item_type == MenuItemType::Submenu {
                child_.ns_menus.as_mut().unwrap().remove(&self.ns_menu.0);
            }
        }
    }
}

impl Menu {
    pub fn new(id: Option<MenuId>) -> Self {
        Self {
            id: id.unwrap_or_else(|| MenuId(COUNTER.next().to_string())),
            ns_menu: NsMenuRef(COUNTER.next(), unsafe {
                let ns_menu = NSMenu::new(nil);
                ns_menu.setAutoenablesItems(NO);
                let _: () = msg_send![ns_menu, retain];
                ns_menu
            }),
            children: Vec::new(),
        }
    }

    pub fn id(&self) -> &MenuId {
        &self.id
    }

    pub fn add_menu_item(&mut self, item: &dyn crate::IsMenuItem, op: AddOp) -> crate::Result<()> {
        let ns_menu_item: id = item.make_ns_item_for_menu(self.ns_menu.0)?;
        let child = item.child();

        unsafe {
            match op {
                AddOp::Append => {
                    self.ns_menu.1.addItem_(ns_menu_item);
                    self.children.push(child);
                }
                AddOp::Insert(position) => {
                    let () = msg_send![self.ns_menu.1, insertItem: ns_menu_item atIndex: position as NSInteger];
                    self.children.insert(position, child);
                }
            }
        }

        Ok(())
    }

    pub fn remove(&mut self, item: &dyn crate::IsMenuItem) -> crate::Result<()> {
        // get child
        let child = {
            let index = self
                .children
                .iter()
                .position(|e| e.borrow().id == item.id())
                .ok_or(crate::Error::NotAChildOfThisMenu)?;
            self.children.remove(index)
        };

        let mut child_ = child.borrow_mut();

        if child_.item_type == MenuItemType::Submenu {
            let menu_id = &self.ns_menu.0;
            let menus = child_.ns_menus.as_ref().unwrap().get(menu_id).cloned();
            if let Some(menus) = menus {
                for menu in menus {
                    for item in child_.items() {
                        child_.remove_inner(item.as_ref(), false, Some(menu.0))?;
                    }
                }
            }
            child_.ns_menus.as_mut().unwrap().remove(menu_id);
        }

        // remove each NSMenuItem from the NSMenu
        if let Some(ns_menu_items) = child_.ns_menu_items.remove(&self.ns_menu.0) {
            for item in ns_menu_items {
                let () = unsafe { msg_send![self.ns_menu.1, removeItem: item] };
            }
        }

        Ok(())
    }

    pub fn items(&self) -> Vec<MenuItemKind> {
        self.children
            .iter()
            .map(|c| c.borrow().kind(c.clone()))
            .collect()
    }

    pub fn init_for_nsapp(&self) {
        unsafe { NSApp().setMainMenu_(self.ns_menu.1) }
    }

    pub fn remove_for_nsapp(&self) {
        unsafe { NSApp().setMainMenu_(NSMenu::new(nil) as _) }
    }

    pub fn show_context_menu_for_nsview(&self, view: id, position: Option<Position>) {
        show_context_menu(self.ns_menu.1, view, position)
    }

    pub fn ns_menu(&self) -> *mut std::ffi::c_void {
        self.ns_menu.1 as _
    }
}

/// A generic child in a menu
#[derive(Debug, Default)]
pub struct MenuChild {
    // shared fields between submenus and menu items
    item_type: MenuItemType,
    id: MenuId,
    text: String,
    enabled: bool,

    ns_menu_items: HashMap<u32, Vec<NsMenuItemRef>>,

    // menu item fields
    accelerator: Option<Accelerator>,

    // predefined menu item fields
    predefined_item_type: Option<PredefinedMenuItemType>,

    // check menu item fields
    checked: bool,

    // icon menu item fields
    icon: Option<Icon>,
    native_icon: Option<NativeIcon>,

    // submenu fields
    pub children: Option<Vec<Rc<RefCell<MenuChild>>>>,
    ns_menus: Option<HashMap<u32, Vec<NsMenuRef>>>,
    ns_menu: Option<NsMenuRef>,
}

impl Drop for MenuChild {
    fn drop(&mut self) {
        fn drop_children(id: u32, children: &Vec<Rc<RefCell<MenuChild>>>) {
            for child in children {
                let mut child_ = child.borrow_mut();
                child_.ns_menu_items.remove(&id);

                if child_.item_type == MenuItemType::Submenu {
                    if let Some(menus) = child_.ns_menus.as_mut().unwrap().remove(&id) {
                        for menu in menus {
                            drop_children(menu.0, child_.children.as_ref().unwrap());
                        }
                    }
                }
            }
        }

        if self.item_type == MenuItemType::Submenu {
            for menus in self.ns_menus.as_ref().unwrap().values() {
                for menu in menus {
                    drop_children(menu.0, self.children.as_ref().unwrap())
                }
            }

            if let Some(menu) = &self.ns_menu {
                drop_children(menu.0, self.children.as_ref().unwrap());
            }
        }
    }
}

/// Constructors
impl MenuChild {
    pub fn new(
        text: &str,
        enabled: bool,
        accelerator: Option<Accelerator>,
        id: Option<MenuId>,
    ) -> Self {
        Self {
            item_type: MenuItemType::MenuItem,
            text: strip_mnemonic(text),
            enabled,
            id: id.unwrap_or_else(|| MenuId(COUNTER.next().to_string())),
            accelerator,
            checked: false,
            children: None,
            icon: None,
            native_icon: None,
            ns_menu: None,
            ns_menu_items: HashMap::new(),
            ns_menus: None,
            predefined_item_type: None,
        }
    }

    pub fn new_submenu(text: &str, enabled: bool, id: Option<MenuId>) -> Self {
        Self {
            item_type: MenuItemType::Submenu,
            text: strip_mnemonic(text),
            id: id.unwrap_or_else(|| MenuId(COUNTER.next().to_string())),
            enabled,
            children: Some(Vec::new()),
            ns_menu: Some(NsMenuRef(COUNTER.next(), unsafe {
                let menu = NSMenu::new(nil);
                let _: () = msg_send![menu, retain];
                menu
            })),
            accelerator: None,
            checked: false,
            icon: None,
            native_icon: None,
            ns_menu_items: HashMap::new(),
            ns_menus: Some(HashMap::new()),
            predefined_item_type: None,
        }
    }

    pub(crate) fn new_predefined(item_type: PredefinedMenuItemType, text: Option<String>) -> Self {
        let text = strip_mnemonic(text.unwrap_or_else(|| {
            match item_type {
                PredefinedMenuItemType::About(_) => {
                    format!("About {}", unsafe { app_name_string() }.unwrap_or_default())
                        .trim()
                        .to_string()
                }
                PredefinedMenuItemType::Hide => {
                    format!("Hide {}", unsafe { app_name_string() }.unwrap_or_default())
                        .trim()
                        .to_string()
                }
                PredefinedMenuItemType::Quit => {
                    format!("Quit {}", unsafe { app_name_string() }.unwrap_or_default())
                        .trim()
                        .to_string()
                }
                _ => item_type.text().to_string(),
            }
        }));
        let accelerator = item_type.accelerator();

        Self {
            item_type: MenuItemType::Predefined,
            text,
            enabled: true,
            id: MenuId(COUNTER.next().to_string()),
            accelerator,
            predefined_item_type: Some(item_type),
            checked: false,
            children: None,
            icon: None,
            native_icon: None,
            ns_menu: None,
            ns_menu_items: HashMap::new(),
            ns_menus: None,
        }
    }

    pub fn new_check(
        text: &str,
        enabled: bool,
        checked: bool,
        accelerator: Option<Accelerator>,
        id: Option<MenuId>,
    ) -> Self {
        Self {
            item_type: MenuItemType::Check,
            text: text.to_string(),
            enabled,
            id: id.unwrap_or_else(|| MenuId(COUNTER.next().to_string())),
            accelerator,
            checked,
            children: None,
            icon: None,
            native_icon: None,
            ns_menu: None,
            ns_menu_items: HashMap::new(),
            ns_menus: None,
            predefined_item_type: None,
        }
    }

    pub fn new_icon(
        text: &str,
        enabled: bool,
        icon: Option<Icon>,
        accelerator: Option<Accelerator>,
        id: Option<MenuId>,
    ) -> Self {
        Self {
            item_type: MenuItemType::Icon,
            text: text.to_string(),
            enabled,
            id: id.unwrap_or_else(|| MenuId(COUNTER.next().to_string())),
            icon,
            accelerator,
            checked: false,
            children: None,
            native_icon: None,
            ns_menu: None,
            ns_menu_items: HashMap::new(),
            ns_menus: None,
            predefined_item_type: None,
        }
    }

    pub fn new_native_icon(
        text: &str,
        enabled: bool,
        native_icon: Option<NativeIcon>,
        accelerator: Option<Accelerator>,
        id: Option<MenuId>,
    ) -> Self {
        Self {
            item_type: MenuItemType::Icon,
            text: text.to_string(),
            enabled,
            id: id.unwrap_or_else(|| MenuId(COUNTER.next().to_string())),
            native_icon,
            accelerator,
            checked: false,
            children: None,
            icon: None,
            ns_menu: None,
            ns_menu_items: HashMap::new(),
            ns_menus: None,
            predefined_item_type: None,
        }
    }
}

/// Shared methods
impl MenuChild {
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
        self.text = strip_mnemonic(text);
        unsafe {
            let title = NSString::alloc(nil).init_str(&self.text).autorelease();
            for ns_items in self.ns_menu_items.values() {
                for ns_item in ns_items {
                    let () = msg_send![ns_item.0, setTitle: title];
                    let ns_submenu: id = msg_send![ns_item.0, submenu];
                    if ns_submenu != nil {
                        let () = msg_send![ns_submenu, setTitle: title];
                    }
                }
            }
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        for ns_items in self.ns_menu_items.values() {
            for ns_item in ns_items {
                unsafe {
                    let () = msg_send![ns_item.0, setEnabled: if enabled { YES } else { NO }];
                }
            }
        }
    }

    pub fn set_accelerator(&mut self, accelerator: Option<Accelerator>) -> crate::Result<()> {
        let key_equivalent = (accelerator)
            .as_ref()
            .map(|accel| accel.key_equivalent())
            .transpose()?;

        if let Some(key_equivalent) = key_equivalent {
            let key_equivalent = unsafe {
                NSString::alloc(nil)
                    .init_str(key_equivalent.as_str())
                    .autorelease()
            };

            let modifier_mask = (accelerator)
                .as_ref()
                .map(|accel| accel.key_modifier_mask())
                .unwrap_or_else(NSEventModifierFlags::empty);

            for ns_items in self.ns_menu_items.values() {
                for ns_item in ns_items {
                    unsafe {
                        let _: () = msg_send![ns_item.0, setKeyEquivalent: key_equivalent];
                        ns_item.0.setKeyEquivalentModifierMask_(modifier_mask);
                    }
                }
            }
        }

        self.accelerator = accelerator;

        Ok(())
    }
}

/// CheckMenuItem methods
impl MenuChild {
    pub fn is_checked(&self) -> bool {
        self.checked
    }

    pub fn set_checked(&mut self, checked: bool) {
        self.checked = checked;
        for ns_items in self.ns_menu_items.values() {
            for ns_item in ns_items {
                unsafe {
                    let () = msg_send![ns_item.0, setState: checked as u32];
                }
            }
        }
    }
}

/// IconMenuItem methods
impl MenuChild {
    pub fn set_icon(&mut self, icon: Option<Icon>) {
        self.icon = icon.clone();
        self.native_icon = None;
        for ns_items in self.ns_menu_items.values() {
            for ns_item in ns_items {
                menuitem_set_icon(ns_item.0, icon.as_ref());
            }
        }
    }

    pub fn set_native_icon(&mut self, icon: Option<NativeIcon>) {
        self.native_icon = icon;
        self.icon = None;
        for ns_items in self.ns_menu_items.values() {
            for ns_item in ns_items {
                menuitem_set_native_icon(ns_item.0, icon);
            }
        }
    }
}

/// Submenu methods
impl MenuChild {
    pub fn add_menu_item(&mut self, item: &dyn crate::IsMenuItem, op: AddOp) -> crate::Result<()> {
        let child = item.child();

        unsafe {
            match op {
                AddOp::Append => {
                    for menus in self.ns_menus.as_ref().unwrap().values() {
                        for ns_menu in menus {
                            let ns_menu_item: id =
                                item.make_ns_item_for_menu(self.ns_menu.as_ref().unwrap().0)?;
                            ns_menu.1.addItem_(ns_menu_item);
                        }
                    }

                    let ns_menu_item: id =
                        item.make_ns_item_for_menu(self.ns_menu.as_ref().unwrap().0)?;
                    self.ns_menu.as_ref().unwrap().1.addItem_(ns_menu_item);

                    self.children.as_mut().unwrap().push(child);
                }
                AddOp::Insert(position) => {
                    for menus in self.ns_menus.as_ref().unwrap().values() {
                        for ns_menu in menus {
                            let ns_menu_item: id =
                                item.make_ns_item_for_menu(self.ns_menu.as_ref().unwrap().0)?;
                            let () = msg_send![ns_menu.1, insertItem: ns_menu_item atIndex: position as NSInteger];
                        }
                    }

                    let ns_menu_item: id =
                        item.make_ns_item_for_menu(self.ns_menu.as_ref().unwrap().0)?;
                    let () = msg_send![ self.ns_menu.as_ref().unwrap().1, insertItem: ns_menu_item atIndex: position as NSInteger];

                    self.children.as_mut().unwrap().insert(position, child);
                }
            }
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
        // get child
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

        for menus in self.ns_menus.as_ref().unwrap().values() {
            for menu in menus {
                // check if we are removing this item from all ns_menus
                //      which is usually when this is the item the user is actaully removing
                // or if we are removing from a specific menu (id)
                //      which is when the actual item being removed is a submenu
                //      and we are iterating through its children and removing
                //      each child ns menu item that are related to this submenu.
                if id.map(|i| i == menu.0).unwrap_or(true) {
                    let mut child_ = child.borrow_mut();

                    if child_.item_type == MenuItemType::Submenu {
                        let menus = child_.ns_menus.as_ref().unwrap().get(&menu.0).cloned();
                        if let Some(menus) = menus {
                            for menu in menus {
                                // iterate through children and only remove the ns menu items
                                // related to this submenu
                                for item in child_.items() {
                                    child_.remove_inner(item.as_ref(), false, Some(menu.0))?;
                                }
                            }
                        }
                        child_.ns_menus.as_mut().unwrap().remove(&menu.0);
                    }

                    if let Some(items) = child_.ns_menu_items.remove(&menu.0) {
                        for item in items {
                            let () = unsafe { msg_send![menu.1, removeItem: item] };
                        }
                    }
                }
            }
        }

        if remove_from_cache {
            if let Some(ns_menu_items) = child
                .borrow_mut()
                .ns_menu_items
                .remove(&self.ns_menu.as_ref().unwrap().0)
            {
                for item in ns_menu_items {
                    let () =
                        unsafe { msg_send![self.ns_menu.as_ref().unwrap().1, removeItem: item] };
                }
            }
        }

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

    pub fn show_context_menu_for_nsview(&self, view: id, position: Option<Position>) {
        show_context_menu(self.ns_menu.as_ref().unwrap().1, view, position)
    }

    pub fn set_as_windows_menu_for_nsapp(&self) {
        unsafe { NSApp().setWindowsMenu_(self.ns_menu.as_ref().unwrap().1) }
    }

    pub fn set_as_help_menu_for_nsapp(&self) {
        unsafe { msg_send![NSApp(), setHelpMenu: self.ns_menu.as_ref().unwrap().1] }
    }

    pub fn ns_menu(&self) -> *mut std::ffi::c_void {
        self.ns_menu.as_ref().unwrap().1 as _
    }
}

/// NSMenuItem item creation methods
impl MenuChild {
    pub fn create_ns_item_for_submenu(&mut self, menu_id: u32) -> crate::Result<id> {
        let ns_menu_item: id;
        let ns_submenu: id;

        unsafe {
            ns_menu_item = NSMenuItem::alloc(nil);
            ns_submenu = NSMenu::alloc(nil);

            let title = NSString::alloc(nil).init_str(&self.text).autorelease();
            let () = msg_send![ns_submenu, setTitle: title];
            let () = msg_send![ns_menu_item, setTitle: title];
            let () = msg_send![ns_menu_item, setSubmenu: ns_submenu];

            if !self.enabled {
                let () = msg_send![ns_menu_item, setEnabled: NO];
            }
        }

        let id = COUNTER.next();

        for item in self.children.as_ref().unwrap() {
            let ns_item = item.borrow_mut().make_ns_item_for_menu(id)?;
            unsafe { ns_submenu.addItem_(ns_item) };
        }

        self.ns_menus
            .as_mut()
            .unwrap()
            .entry(menu_id)
            .or_insert_with(Vec::new)
            .push(NsMenuRef(id, ns_submenu));

        self.ns_menu_items
            .entry(menu_id)
            .or_insert_with(Vec::new)
            .push(NsMenuItemRef(ns_menu_item));

        Ok(ns_menu_item)
    }

    pub fn create_ns_item_for_menu_item(&mut self, menu_id: u32) -> crate::Result<id> {
        let ns_menu_item = create_ns_menu_item(
            &self.text,
            Some(sel!(fireMenuItemAction:)),
            &self.accelerator,
        )?;

        unsafe {
            let _: () = msg_send![ns_menu_item, setTarget: ns_menu_item];

            // Store a raw pointer to the `MenuChild` as an instance variable on the native menu item
            let ptr = Box::into_raw(Box::new(&*self));
            (*ns_menu_item).set_ivar(BLOCK_PTR, ptr as usize);

            if !self.enabled {
                let () = msg_send![ns_menu_item, setEnabled: NO];
            }
        }

        self.ns_menu_items
            .entry(menu_id)
            .or_insert_with(Vec::new)
            .push(NsMenuItemRef(ns_menu_item));

        Ok(ns_menu_item)
    }

    pub fn create_ns_item_for_predefined_menu_item(&mut self, menu_id: u32) -> crate::Result<id> {
        let item_type = self.predefined_item_type.as_ref().unwrap();
        let ns_menu_item = match item_type {
            PredefinedMenuItemType::Separator => unsafe { NSMenuItem::separatorItem(nil) },
            _ => create_ns_menu_item(&self.text, item_type.selector(), &self.accelerator)?,
        };

        if let PredefinedMenuItemType::About(_) = item_type {
            unsafe {
                let _: () = msg_send![ns_menu_item, setTarget: ns_menu_item];

                // Store a raw pointer to the `MenuChild` as an instance variable on the native menu item
                let ptr = Box::into_raw(Box::new(&*self));
                (*ns_menu_item).set_ivar(BLOCK_PTR, ptr as usize);
            }
        }

        unsafe {
            if !self.enabled {
                let () = msg_send![ns_menu_item, setEnabled: NO];
            }
            if let PredefinedMenuItemType::Services = item_type {
                // we have to assign an empty menu as the app's services menu, and macOS will populate it
                let services_menu = NSMenu::new(nil).autorelease();
                let () = msg_send![NSApp(), setServicesMenu: services_menu];
                let () = msg_send![ns_menu_item, setSubmenu: services_menu];
            }
        }

        self.ns_menu_items
            .entry(menu_id)
            .or_insert_with(Vec::new)
            .push(NsMenuItemRef(ns_menu_item));

        Ok(ns_menu_item)
    }

    pub fn create_ns_item_for_check_menu_item(&mut self, menu_id: u32) -> crate::Result<id> {
        let ns_menu_item = create_ns_menu_item(
            &self.text,
            Some(sel!(fireMenuItemAction:)),
            &self.accelerator,
        )?;

        unsafe {
            let _: () = msg_send![ns_menu_item, setTarget: ns_menu_item];

            // Store a raw pointer to the `MenuChild` as an instance variable on the native menu item
            let ptr = Box::into_raw(Box::new(&*self));
            (*ns_menu_item).set_ivar(BLOCK_PTR, ptr as usize);

            if !self.enabled {
                let () = msg_send![ns_menu_item, setEnabled: NO];
            }
            if self.checked {
                let () = msg_send![ns_menu_item, setState: 1_isize];
            }
        }

        self.ns_menu_items
            .entry(menu_id)
            .or_insert_with(Vec::new)
            .push(NsMenuItemRef(ns_menu_item));

        Ok(ns_menu_item)
    }

    pub fn create_ns_item_for_icon_menu_item(&mut self, menu_id: u32) -> crate::Result<id> {
        let ns_menu_item = create_ns_menu_item(
            &self.text,
            Some(sel!(fireMenuItemAction:)),
            &self.accelerator,
        )?;

        unsafe {
            let _: () = msg_send![ns_menu_item, setTarget: ns_menu_item];

            // Store a raw pointer to the `MenuChild` as an instance variable on the native menu item
            let ptr = Box::into_raw(Box::new(&*self));
            (*ns_menu_item).set_ivar(BLOCK_PTR, ptr as usize);

            if !self.enabled {
                let () = msg_send![ns_menu_item, setEnabled: NO];
            }

            if self.icon.is_some() {
                menuitem_set_icon(ns_menu_item, self.icon.as_ref());
            } else if self.native_icon.is_some() {
                menuitem_set_native_icon(ns_menu_item, self.native_icon);
            }
        }

        self.ns_menu_items
            .entry(menu_id)
            .or_insert_with(Vec::new)
            .push(NsMenuItemRef(ns_menu_item));

        Ok(ns_menu_item)
    }

    fn make_ns_item_for_menu(&mut self, menu_id: u32) -> crate::Result<id> {
        match self.item_type {
            MenuItemType::Submenu => self.create_ns_item_for_submenu(menu_id),
            MenuItemType::MenuItem => self.create_ns_item_for_menu_item(menu_id),
            MenuItemType::Predefined => self.create_ns_item_for_predefined_menu_item(menu_id),
            MenuItemType::Check => self.create_ns_item_for_check_menu_item(menu_id),
            MenuItemType::Icon => self.create_ns_item_for_icon_menu_item(menu_id),
        }
    }
}

impl PredefinedMenuItemType {
    pub(crate) fn selector(&self) -> Option<Sel> {
        match self {
            PredefinedMenuItemType::Separator => None,
            PredefinedMenuItemType::Copy => Some(selector("copy:")),
            PredefinedMenuItemType::Cut => Some(selector("cut:")),
            PredefinedMenuItemType::Paste => Some(selector("paste:")),
            PredefinedMenuItemType::SelectAll => Some(selector("selectAll:")),
            PredefinedMenuItemType::Undo => Some(selector("undo:")),
            PredefinedMenuItemType::Redo => Some(selector("redo:")),
            PredefinedMenuItemType::Minimize => Some(selector("performMiniaturize:")),
            PredefinedMenuItemType::Maximize => Some(selector("performZoom:")),
            PredefinedMenuItemType::Fullscreen => Some(selector("toggleFullScreen:")),
            PredefinedMenuItemType::Hide => Some(selector("hide:")),
            PredefinedMenuItemType::HideOthers => Some(selector("hideOtherApplications:")),
            PredefinedMenuItemType::ShowAll => Some(selector("unhideAllApplications:")),
            PredefinedMenuItemType::CloseWindow => Some(selector("performClose:")),
            PredefinedMenuItemType::Quit => Some(selector("terminate:")),
            // manual implementation in `fire_menu_item_click`
            PredefinedMenuItemType::About(_) => Some(selector("fireMenuItemAction:")),
            PredefinedMenuItemType::Services => None,
            PredefinedMenuItemType::BringAllToFront => Some(selector("arrangeInFront:")),
            PredefinedMenuItemType::None => None,
        }
    }
}

impl dyn IsMenuItem + '_ {
    fn make_ns_item_for_menu(&self, menu_id: u32) -> crate::Result<id> {
        match self.kind() {
            MenuItemKind::Submenu(i) => i.inner.borrow_mut().create_ns_item_for_submenu(menu_id),
            MenuItemKind::MenuItem(i) => i.inner.borrow_mut().create_ns_item_for_menu_item(menu_id),
            MenuItemKind::Predefined(i) => i
                .inner
                .borrow_mut()
                .create_ns_item_for_predefined_menu_item(menu_id),
            MenuItemKind::Check(i) => i
                .inner
                .borrow_mut()
                .create_ns_item_for_check_menu_item(menu_id),
            MenuItemKind::Icon(i) => i
                .inner
                .borrow_mut()
                .create_ns_item_for_icon_menu_item(menu_id),
        }
    }
}

fn make_menu_item_class() -> *const Class {
    static mut APP_CLASS: *const Class = 0 as *const Class;
    static INIT: Once = Once::new();

    // The first time the function is called,
    INIT.call_once(|| unsafe {
        let superclass = class!(NSMenuItem);
        let mut decl = ClassDecl::new("MudaMenuItem", superclass).unwrap();

        // An instance variable which will hold a pointer to the `MenuChild`
        decl.add_ivar::<usize>(BLOCK_PTR);

        decl.add_method(
            sel!(dealloc),
            dealloc_custom_menuitem as extern "C" fn(&Object, _),
        );

        decl.add_method(
            sel!(fireMenuItemAction:),
            fire_menu_item_click as extern "C" fn(&Object, _, id),
        );

        APP_CLASS = decl.register();
    });

    unsafe { APP_CLASS }
}

extern "C" fn dealloc_custom_menuitem(this: &Object, _: Sel) {
    unsafe {
        let ptr: usize = *this.get_ivar(BLOCK_PTR);
        let obj = ptr as *mut &mut MenuChild;
        drop(Box::from_raw(obj));
        let _: () = msg_send![super(this, class!(NSMenuItem)), dealloc];
    }
}

extern "C" fn fire_menu_item_click(this: &Object, _: Sel, _item: id) {
    unsafe {
        // Create a reference to the `MenuChild` from the raw pointer
        // stored as an instance variable on the native menu item
        let ptr: usize = *this.get_ivar(BLOCK_PTR);
        let item = ptr as *mut &mut MenuChild;

        if let Some(PredefinedMenuItemType::About(about_meta)) = &(*item).predefined_item_type {
            match about_meta {
                Some(about_meta) => {
                    unsafe fn mkstr(s: &str) -> id {
                        NSString::alloc(nil).init_str(s)
                    }

                    let mut keys: Vec<id> = Default::default();
                    let mut objects: Vec<id> = Default::default();

                    if let Some(name) = &about_meta.name {
                        keys.push(NSAboutPanelOptionApplicationName);
                        objects.push(mkstr(name));
                    }

                    if let Some(version) = &about_meta.version {
                        keys.push(NSAboutPanelOptionApplicationVersion);
                        objects.push(mkstr(version));
                    }

                    if let Some(short_version) = &about_meta.short_version {
                        keys.push(NSAboutPanelOptionVersion);
                        objects.push(mkstr(short_version));
                    }

                    if let Some(copyright) = &about_meta.copyright {
                        keys.push(mkstr(NSAboutPanelOptionCopyright));
                        objects.push(mkstr(copyright));
                    }

                    if let Some(icon) = &about_meta.icon {
                        keys.push(NSAboutPanelOptionApplicationIcon);
                        objects.push(icon.inner.to_nsimage(None));
                    }

                    if let Some(credits) = &about_meta.credits {
                        keys.push(NSAboutPanelOptionCredits);
                        let attributed_str: id = msg_send![class!(NSAttributedString), alloc];
                        let _: () = msg_send![attributed_str, initWithString: mkstr(credits)];
                        objects.push(attributed_str);
                    }

                    let keys_array = NSArray::arrayWithObjects(nil, &keys);
                    let objs_array = NSArray::arrayWithObjects(nil, &objects);

                    let dict =
                        NSDictionary::dictionaryWithObjects_forKeys_(nil, objs_array, keys_array);

                    let _: () = msg_send![NSApp(), orderFrontStandardAboutPanelWithOptions: dict];
                }

                None => {
                    let _: () = msg_send![NSApp(), orderFrontStandardAboutPanel: this];
                }
            }
        }

        if (*item).item_type == MenuItemType::Check {
            (*item).set_checked(!(*item).is_checked());
        }

        let id = (*item).id().clone();
        MenuEvent::send(crate::MenuEvent { id });
    }
}

fn create_ns_menu_item(
    title: &str,
    selector: Option<Sel>,
    accelerator: &Option<Accelerator>,
) -> crate::Result<id> {
    unsafe {
        let title = NSString::alloc(nil).init_str(title).autorelease();

        let selector = selector.unwrap_or_else(|| Sel::from_ptr(std::ptr::null()));

        let key_equivalent = (*accelerator)
            .map(|accel| accel.key_equivalent())
            .transpose()?
            .unwrap_or_default();
        let key_equivalent = NSString::alloc(nil)
            .init_str(key_equivalent.as_str())
            .autorelease();

        let modifier_mask = (*accelerator)
            .map(|accel| accel.key_modifier_mask())
            .unwrap_or_else(NSEventModifierFlags::empty);

        let ns_menu_item: id = msg_send![make_menu_item_class(), alloc];

        ns_menu_item.initWithTitle_action_keyEquivalent_(title, selector, key_equivalent);
        ns_menu_item.setKeyEquivalentModifierMask_(modifier_mask);

        Ok(ns_menu_item)
    }
}

fn menuitem_set_icon(menuitem: id, icon: Option<&Icon>) {
    if let Some(icon) = icon {
        unsafe {
            let nsimage = icon.inner.to_nsimage(Some(18.));
            let _: () = msg_send![menuitem, setImage: nsimage];
        }
    } else {
        unsafe {
            let _: () = msg_send![menuitem, setImage: nil];
        }
    }
}

fn menuitem_set_native_icon(menuitem: id, icon: Option<NativeIcon>) {
    if let Some(icon) = icon {
        unsafe {
            let named_img: id = icon.named_img();
            let nsimage: id = msg_send![class!(NSImage), imageNamed: named_img];
            let size = NSSize::new(18.0, 18.0);
            let _: () = msg_send![nsimage, setSize: size];
            let _: () = msg_send![menuitem, setImage: nsimage];
        }
    } else {
        unsafe {
            let _: () = msg_send![menuitem, setImage: nil];
        }
    }
}

fn show_context_menu(ns_menu: id, view: id, position: Option<Position>) {
    unsafe {
        let window: id = msg_send![view, window];
        let scale_factor: CGFloat = msg_send![window, backingScaleFactor];
        let (location, in_view) = if let Some(pos) = position.map(|p| p.to_logical(scale_factor)) {
            let view_rect: NSRect = msg_send![view, frame];
            let location = NSPoint::new(pos.x, view_rect.size.height - pos.y);
            (location, view)
        } else {
            let mouse_location: NSPoint = msg_send![class!(NSEvent), mouseLocation];
            let pos = Position::Logical(LogicalPosition {
                x: mouse_location.x,
                y: mouse_location.y,
            });
            let pos = pos.to_logical(scale_factor);
            let location = NSPoint::new(pos.x, pos.y);
            (location, nil)
        };

        msg_send![ns_menu, popUpMenuPositioningItem: nil atLocation: location inView: in_view]
    }
}

impl NativeIcon {
    unsafe fn named_img(self) -> id {
        match self {
            NativeIcon::Add => appkit::NSImageNameAddTemplate,
            NativeIcon::StatusAvailable => appkit::NSImageNameStatusAvailable,
            NativeIcon::StatusUnavailable => appkit::NSImageNameStatusUnavailable,
            NativeIcon::StatusPartiallyAvailable => appkit::NSImageNameStatusPartiallyAvailable,
            NativeIcon::Advanced => appkit::NSImageNameAdvanced,
            NativeIcon::Bluetooth => appkit::NSImageNameBluetoothTemplate,
            NativeIcon::Bookmarks => appkit::NSImageNameBookmarksTemplate,
            NativeIcon::Caution => appkit::NSImageNameCaution,
            NativeIcon::ColorPanel => appkit::NSImageNameColorPanel,
            NativeIcon::ColumnView => appkit::NSImageNameColumnViewTemplate,
            NativeIcon::Computer => appkit::NSImageNameComputer,
            NativeIcon::EnterFullScreen => appkit::NSImageNameEnterFullScreenTemplate,
            NativeIcon::Everyone => appkit::NSImageNameEveryone,
            NativeIcon::ExitFullScreen => appkit::NSImageNameExitFullScreenTemplate,
            NativeIcon::FlowView => appkit::NSImageNameFlowViewTemplate,
            NativeIcon::Folder => appkit::NSImageNameFolder,
            NativeIcon::FolderBurnable => appkit::NSImageNameFolderBurnable,
            NativeIcon::FolderSmart => appkit::NSImageNameFolderSmart,
            NativeIcon::FollowLinkFreestanding => appkit::NSImageNameFollowLinkFreestandingTemplate,
            NativeIcon::FontPanel => appkit::NSImageNameFontPanel,
            NativeIcon::GoLeft => appkit::NSImageNameGoLeftTemplate,
            NativeIcon::GoRight => appkit::NSImageNameGoRightTemplate,
            NativeIcon::Home => appkit::NSImageNameHomeTemplate,
            NativeIcon::IChatTheater => appkit::NSImageNameIChatTheaterTemplate,
            NativeIcon::IconView => appkit::NSImageNameIconViewTemplate,
            NativeIcon::Info => appkit::NSImageNameInfo,
            NativeIcon::InvalidDataFreestanding => {
                appkit::NSImageNameInvalidDataFreestandingTemplate
            }
            NativeIcon::LeftFacingTriangle => appkit::NSImageNameLeftFacingTriangleTemplate,
            NativeIcon::ListView => appkit::NSImageNameListViewTemplate,
            NativeIcon::LockLocked => appkit::NSImageNameLockLockedTemplate,
            NativeIcon::LockUnlocked => appkit::NSImageNameLockUnlockedTemplate,
            NativeIcon::MenuMixedState => appkit::NSImageNameMenuMixedStateTemplate,
            NativeIcon::MenuOnState => appkit::NSImageNameMenuOnStateTemplate,
            NativeIcon::MobileMe => appkit::NSImageNameMobileMe,
            NativeIcon::MultipleDocuments => appkit::NSImageNameMultipleDocuments,
            NativeIcon::Network => appkit::NSImageNameNetwork,
            NativeIcon::Path => appkit::NSImageNamePathTemplate,
            NativeIcon::PreferencesGeneral => appkit::NSImageNamePreferencesGeneral,
            NativeIcon::QuickLook => appkit::NSImageNameQuickLookTemplate,
            NativeIcon::RefreshFreestanding => appkit::NSImageNameRefreshFreestandingTemplate,
            NativeIcon::Refresh => appkit::NSImageNameRefreshTemplate,
            NativeIcon::Remove => appkit::NSImageNameRemoveTemplate,
            NativeIcon::RevealFreestanding => appkit::NSImageNameRevealFreestandingTemplate,
            NativeIcon::RightFacingTriangle => appkit::NSImageNameRightFacingTriangleTemplate,
            NativeIcon::Share => appkit::NSImageNameShareTemplate,
            NativeIcon::Slideshow => appkit::NSImageNameSlideshowTemplate,
            NativeIcon::SmartBadge => appkit::NSImageNameSmartBadgeTemplate,
            NativeIcon::StatusNone => appkit::NSImageNameStatusNone,
            NativeIcon::StopProgressFreestanding => {
                appkit::NSImageNameStopProgressFreestandingTemplate
            }
            NativeIcon::StopProgress => appkit::NSImageNameStopProgressTemplate,
            NativeIcon::TrashEmpty => appkit::NSImageNameTrashEmpty,
            NativeIcon::TrashFull => appkit::NSImageNameTrashFull,
            NativeIcon::User => appkit::NSImageNameUser,
            NativeIcon::UserAccounts => appkit::NSImageNameUserAccounts,
            NativeIcon::UserGroup => appkit::NSImageNameUserGroup,
            NativeIcon::UserGuest => appkit::NSImageNameUserGuest,
        }
    }
}
