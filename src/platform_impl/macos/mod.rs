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
    IsMenuItem, MenuEvent, MenuItemType,
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

#[derive(Clone, Debug)]
pub struct Menu {
    id: u32,
    ns_menu: id,
    children: Rc<RefCell<Vec<Rc<RefCell<MenuChild>>>>>,
}

impl Menu {
    pub fn new() -> Self {
        Self {
            id: COUNTER.next(),
            ns_menu: unsafe {
                let ns_menu = NSMenu::alloc(nil).autorelease();
                ns_menu.setAutoenablesItems(NO);
                ns_menu
            },
            children: Rc::new(RefCell::new(Vec::new())),
        }
    }

    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn add_menu_item(&mut self, item: &dyn crate::IsMenuItem, op: AddOp) -> crate::Result<()> {
        let ns_menu_item: *mut Object = item.make_ns_item_for_menu(self.id)?;
        let child = item.child();

        unsafe {
            match op {
                AddOp::Append => {
                    self.ns_menu.addItem_(ns_menu_item);
                    self.children.borrow_mut().push(child);
                }
                AddOp::Insert(position) => {
                    let () = msg_send![self.ns_menu, insertItem: ns_menu_item atIndex: position as NSInteger];
                    self.children.borrow_mut().insert(position, child);
                }
            }
        }

        Ok(())
    }

    pub fn remove(&self, item: &dyn crate::IsMenuItem) -> crate::Result<()> {
        // get a list of instances of the specified NSMenuItem in this menu
        if let Some(ns_menu_items) = match item.type_() {
            MenuItemType::Submenu => {
                let submenu = item.as_any().downcast_ref::<Submenu>().unwrap();
                submenu.0.borrow_mut()
            }
            MenuItemType::Normal => {
                let menuitem = item.as_any().downcast_ref::<MenuItem>().unwrap();
                menuitem.0.borrow_mut()
            }
            MenuItemType::Predefined => {
                let menuitem = item.as_any().downcast_ref::<PredefinedMenuItem>().unwrap();
                menuitem.0.borrow_mut()
            }
            MenuItemType::Check => {
                let menuitem = item.as_any().downcast_ref::<CheckMenuItem>().unwrap();
                menuitem.0.borrow_mut()
            }
            MenuItemType::Icon => {
                let menuitem = item.as_any().downcast_ref::<IconMenuItem>().unwrap();
                menuitem.0.borrow_mut()
            }
        }
        .ns_menu_items
        .remove(&self.id)
        {
            // remove each NSMenuItem from the NSMenu
            unsafe {
                for item in ns_menu_items {
                    let () = msg_send![self.ns_menu, removeItem: item];
                }
            }
        }

        // remove the item from our internal list of children
        let mut children = self.children.borrow_mut();
        let index = children
            .iter()
            .position(|e| e.borrow().id() == item.id())
            .ok_or(crate::Error::NotAChildOfThisMenu)?;
        children.remove(index);

        Ok(())
    }

    pub fn items(&self) -> Vec<Box<dyn crate::IsMenuItem>> {
        self.children
            .borrow()
            .iter()
            .map(|c| c.borrow().boxed(c.clone()))
            .collect()
    }

    pub fn init_for_nsapp(&self) {
        unsafe { NSApp().setMainMenu_(self.ns_menu) }
    }

    pub fn remove_for_nsapp(&self) {
        unsafe { NSApp().setMainMenu_(nil) }
    }

    pub fn show_context_menu_for_nsview(&self, view: id, x: f64, y: f64) {
        unsafe {
            let window: id = msg_send![view, window];
            let scale_factor: CGFloat = msg_send![window, backingScaleFactor];
            let view_point = NSPoint::new(x / scale_factor, y / scale_factor);
            let view_rect: NSRect = msg_send![view, frame];
            let location = NSPoint::new(view_point.x, view_rect.size.height - view_point.y);
            msg_send![self.ns_menu, popUpMenuPositioningItem: nil atLocation: location inView: view]
        }
    }

    pub fn ns_menu(&self) -> *mut std::ffi::c_void {
        self.ns_menu as _
    }
}

/// A generic child in a menu
#[derive(Debug)]
pub struct MenuChild {
    // shared fields between submenus and menu items
    pub type_: MenuItemType,
    id: u32,
    text: String,
    enabled: bool,

    ns_menu_items: HashMap<u32, Vec<id>>,

    // menu item fields
    accelerator: Option<Accelerator>,

    // predefined menu item fields
    predefined_item_type: PredfinedMenuItemType,

    // check menu item fields
    checked: bool,

    // icon menu item fields
    icon: Option<Icon>,
    native_icon: Option<NativeIcon>,

    // submenu fields
    pub children: Option<Vec<Rc<RefCell<MenuChild>>>>,
    ns_menus: HashMap<u32, Vec<id>>,
    ns_menu: (u32, id),
}

impl Default for MenuChild {
    fn default() -> Self {
        Self {
            type_: Default::default(),
            id: Default::default(),
            text: Default::default(),
            enabled: Default::default(),
            ns_menu_items: Default::default(),
            accelerator: Default::default(),
            predefined_item_type: Default::default(),
            checked: Default::default(),
            icon: Default::default(),
            native_icon: Default::default(),
            children: Default::default(),
            ns_menus: Default::default(),
            ns_menu: (0, 0 as _),
        }
    }
}

/// Constructors
impl MenuChild {
    pub fn new(text: &str, enabled: bool, accelerator: Option<Accelerator>) -> Self {
        Self {
            type_: MenuItemType::Normal,
            text: strip_mnemonic(text),
            enabled,
            id: COUNTER.next(),
            accelerator,
            ..Default::default()
        }
    }

    pub fn new_submenu(text: &str, enabled: bool) -> Self {
        Self {
            type_: MenuItemType::Submenu,
            text: strip_mnemonic(text),
            enabled,
            children: Some(Vec::new()),
            ns_menu: (COUNTER.next(), unsafe { NSMenu::alloc(nil).autorelease() }),
            ..Default::default()
        }
    }

    pub(crate) fn new_predefined(item_type: PredfinedMenuItemType, text: Option<String>) -> Self {
        let text = strip_mnemonic(text.unwrap_or_else(|| {
            match item_type {
                PredfinedMenuItemType::About(_) => {
                    format!("About {}", unsafe { app_name_string() }.unwrap_or_default())
                        .trim()
                        .to_string()
                }
                PredfinedMenuItemType::Hide => {
                    format!("Hide {}", unsafe { app_name_string() }.unwrap_or_default())
                        .trim()
                        .to_string()
                }
                PredfinedMenuItemType::Quit => {
                    format!("Quit {}", unsafe { app_name_string() }.unwrap_or_default())
                        .trim()
                        .to_string()
                }
                _ => item_type.text().to_string(),
            }
        }));
        let accelerator = item_type.accelerator();

        Self {
            type_: MenuItemType::Predefined,
            text,
            enabled: true,
            id: COUNTER.next(),
            accelerator,
            predefined_item_type: item_type,
            // ns_menu_item,
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
            type_: MenuItemType::Check,
            text: text.to_string(),
            enabled,
            id: COUNTER.next(),
            accelerator,
            checked,
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
            type_: MenuItemType::Icon,
            text: text.to_string(),
            enabled,
            id: COUNTER.next(),
            icon,
            accelerator,
            ..Default::default()
        }
    }

    pub fn new_native_icon(
        text: &str,
        enabled: bool,
        native_icon: Option<NativeIcon>,
        accelerator: Option<Accelerator>,
    ) -> Self {
        Self {
            type_: MenuItemType::Icon,
            text: text.to_string(),
            enabled,
            id: COUNTER.next(),
            native_icon,
            accelerator,
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
        self.text.clone()
    }

    pub fn set_text(&mut self, text: &str) {
        self.text = strip_mnemonic(text);
        unsafe {
            let title = NSString::alloc(nil).init_str(&self.text).autorelease();
            for ns_items in self.ns_menu_items.values() {
                for &ns_item in ns_items {
                    let () = msg_send![ns_item, setTitle: title];
                    let ns_submenu: *mut Object = msg_send![ns_item, submenu];
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
            for &ns_item in ns_items {
                unsafe {
                    let () = msg_send![ns_item, setEnabled: if enabled { YES } else { NO }];
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
                for &ns_item in ns_items {
                    unsafe {
                        let _: () = msg_send![ns_item, setKeyEquivalent: key_equivalent];
                        ns_item.setKeyEquivalentModifierMask_(modifier_mask);
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
            for &ns_item in ns_items {
                unsafe {
                    let () = msg_send![ns_item, setState: checked as u32];
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
            for &ns_item in ns_items {
                menuitem_set_icon(ns_item, icon.as_ref());
            }
        }
    }

    pub fn set_native_icon(&mut self, icon: Option<NativeIcon>) {
        self.native_icon = icon;
        self.icon = None;
        for ns_items in self.ns_menu_items.values() {
            for &ns_item in ns_items {
                menuitem_set_native_icon(ns_item, icon);
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
                    for menus in self.ns_menus.values() {
                        for ns_menu in menus {
                            let ns_menu_item: *mut Object = item.make_ns_item_for_menu(self.id)?;
                            ns_menu.addItem_(ns_menu_item);
                        }
                    }

                    let ns_menu_item: *mut Object = item.make_ns_item_for_menu(self.ns_menu.0)?;
                    self.ns_menu.1.addItem_(ns_menu_item);

                    self.children.as_mut().unwrap().push(child);
                }
                AddOp::Insert(position) => {
                    for menus in self.ns_menus.values() {
                        for &ns_menu in menus {
                            let ns_menu_item: *mut Object = item.make_ns_item_for_menu(self.id)?;
                            let () = msg_send![ns_menu, insertItem: ns_menu_item atIndex: position as NSInteger];
                        }
                    }

                    let ns_menu_item: *mut Object = item.make_ns_item_for_menu(self.ns_menu.0)?;
                    let () = msg_send![ self.ns_menu.1, insertItem: ns_menu_item atIndex: position as NSInteger];

                    self.children.as_mut().unwrap().insert(position, child);
                }
            }
        }

        Ok(())
    }

    pub fn remove(&mut self, item: &dyn crate::IsMenuItem) -> crate::Result<()> {
        let child = item.child();

        // get a list of instances of the specified NSMenuItem in this menu
        if let Some(ns_menu_items) = child.borrow_mut().ns_menu_items.remove(&self.id) {
            // remove each NSMenuItem from the NSMenu
            unsafe {
                for item in ns_menu_items {
                    for menus in self.ns_menus.values() {
                        for &ns_menu in menus {
                            let () = msg_send![ns_menu, removeItem: item];
                        }
                    }

                    let () = msg_send![self.ns_menu.1, removeItem: item];
                }
            }
        }

        if let Some(ns_menu_items) = child.borrow_mut().ns_menu_items.remove(&self.ns_menu.0) {
            unsafe {
                for item in ns_menu_items {
                    let () = msg_send![self.ns_menu.1, removeItem: item];
                }
            }
        }

        // remove the item from our internal list of children
        let children = self.children.as_mut().unwrap();
        let index = children
            .iter()
            .position(|e| e.borrow().id == item.id())
            .ok_or(crate::Error::NotAChildOfThisMenu)?;
        children.remove(index);

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

    pub fn show_context_menu_for_nsview(&self, view: id, x: f64, y: f64) {
        unsafe {
            let window: id = msg_send![view, window];
            let scale_factor: CGFloat = msg_send![window, backingScaleFactor];
            let view_point = NSPoint::new(x / scale_factor, y / scale_factor);
            let view_rect: NSRect = msg_send![view, frame];
            let location = NSPoint::new(view_point.x, view_rect.size.height - view_point.y);
            msg_send![self.ns_menu.1, popUpMenuPositioningItem: nil atLocation: location inView: view]
        }
    }

    pub fn set_windows_menu_for_nsapp(&self) {
        unsafe { NSApp().setWindowsMenu_(self.ns_menu.1) }
    }

    pub fn set_help_menu_for_nsapp(&self) {
        unsafe { msg_send![NSApp(), setHelpMenu: self.ns_menu.1] }
    }

    pub fn ns_menu(&self) -> *mut std::ffi::c_void {
        self.ns_menu.1 as _
    }
}

/// NSMenuItem item creation methods
impl MenuChild {
    pub fn create_ns_item_for_submenu(&mut self, menu_id: u32) -> crate::Result<id> {
        let ns_menu_item: *mut Object;
        let ns_submenu: *mut Object;

        unsafe {
            ns_menu_item = NSMenuItem::alloc(nil).autorelease();
            ns_submenu = NSMenu::alloc(nil).autorelease();

            let title = NSString::alloc(nil).init_str(&self.text).autorelease();
            let () = msg_send![ns_submenu, setTitle: title];
            let () = msg_send![ns_menu_item, setTitle: title];
            let () = msg_send![ns_menu_item, setSubmenu: ns_submenu];

            if !self.enabled {
                let () = msg_send![ns_menu_item, setEnabled: NO];
            }
        }

        for item in self.children.as_ref().unwrap() {
            let ns_item = item.borrow_mut().make_ns_item_for_menu(menu_id)?;
            unsafe { ns_submenu.addItem_(ns_item) };
        }

        self.ns_menus
            .entry(menu_id)
            .or_insert_with(Vec::new)
            .push(ns_submenu);

        self.ns_menu_items
            .entry(menu_id)
            .or_insert_with(Vec::new)
            .push(ns_menu_item);

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
            let _: () = msg_send![ns_menu_item, setTag:self.id()];

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
            .push(ns_menu_item);

        Ok(ns_menu_item)
    }

    pub fn create_ns_item_for_predefined_menu_item(&mut self, menu_id: u32) -> crate::Result<id> {
        let item_type = &self.predefined_item_type;
        let ns_menu_item = match item_type {
            PredfinedMenuItemType::Separator => unsafe {
                NSMenuItem::separatorItem(nil).autorelease()
            },
            _ => create_ns_menu_item(&self.text, item_type.selector(), &self.accelerator)?,
        };

        if let PredfinedMenuItemType::About(_) = self.predefined_item_type {
            unsafe {
                let _: () = msg_send![ns_menu_item, setTarget: ns_menu_item];
                let _: () = msg_send![ns_menu_item, setTag:self.id()];

                // Store a raw pointer to the `MenuChild` as an instance variable on the native menu item
                let ptr = Box::into_raw(Box::new(&*self));
                (*ns_menu_item).set_ivar(BLOCK_PTR, ptr as usize);
            }
        }

        unsafe {
            if !self.enabled {
                let () = msg_send![ns_menu_item, setEnabled: NO];
            }
            if let PredfinedMenuItemType::Services = self.predefined_item_type {
                // we have to assign an empty menu as the app's services menu, and macOS will populate it
                let services_menu = NSMenu::new(nil).autorelease();
                let () = msg_send![NSApp(), setServicesMenu: services_menu];
                let () = msg_send![ns_menu_item, setSubmenu: services_menu];
            }
        }

        self.ns_menu_items
            .entry(menu_id)
            .or_insert_with(Vec::new)
            .push(ns_menu_item);

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
            let _: () = msg_send![ns_menu_item, setTag:self.id()];

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
            .push(ns_menu_item);

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
            let _: () = msg_send![ns_menu_item, setTag:self.id()];

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
            .push(ns_menu_item);

        Ok(ns_menu_item)
    }

    fn make_ns_item_for_menu(&mut self, menu_id: u32) -> crate::Result<*mut Object> {
        match self.type_ {
            MenuItemType::Submenu => self.create_ns_item_for_submenu(menu_id),
            MenuItemType::Normal => self.create_ns_item_for_menu_item(menu_id),
            MenuItemType::Predefined => self.create_ns_item_for_predefined_menu_item(menu_id),
            MenuItemType::Check => self.create_ns_item_for_check_menu_item(menu_id),
            MenuItemType::Icon => self.create_ns_item_for_icon_menu_item(menu_id),
        }
    }
}

impl PredfinedMenuItemType {
    pub(crate) fn selector(&self) -> Option<Sel> {
        match self {
            PredfinedMenuItemType::Separator => None,
            PredfinedMenuItemType::Copy => Some(selector("copy:")),
            PredfinedMenuItemType::Cut => Some(selector("cut:")),
            PredfinedMenuItemType::Paste => Some(selector("paste:")),
            PredfinedMenuItemType::SelectAll => Some(selector("selectAll:")),
            PredfinedMenuItemType::Undo => Some(selector("undo:")),
            PredfinedMenuItemType::Redo => Some(selector("redo:")),
            PredfinedMenuItemType::Minimize => Some(selector("performMiniaturize:")),
            PredfinedMenuItemType::Maximize => Some(selector("performZoom:")),
            PredfinedMenuItemType::Fullscreen => Some(selector("toggleFullScreen:")),
            PredfinedMenuItemType::Hide => Some(selector("hide:")),
            PredfinedMenuItemType::HideOthers => Some(selector("hideOtherApplications:")),
            PredfinedMenuItemType::ShowAll => Some(selector("unhideAllApplications:")),
            PredfinedMenuItemType::CloseWindow => Some(selector("performClose:")),
            PredfinedMenuItemType::Quit => Some(selector("terminate:")),
            // manual implementation in `fire_menu_item_click`
            PredfinedMenuItemType::About(_) => Some(selector("fireMenuItemAction:")),
            PredfinedMenuItemType::Services => None,
            PredfinedMenuItemType::None => None,
        }
    }
}

impl dyn IsMenuItem + '_ {
    fn make_ns_item_for_menu(&self, menu_id: u32) -> crate::Result<*mut Object> {
        match self.type_() {
            MenuItemType::Submenu => self
                .as_any()
                .downcast_ref::<Submenu>()
                .unwrap()
                .0
                .borrow_mut()
                .create_ns_item_for_submenu(menu_id),
            MenuItemType::Normal => self
                .as_any()
                .downcast_ref::<MenuItem>()
                .unwrap()
                .0
                .borrow_mut()
                .create_ns_item_for_menu_item(menu_id),
            MenuItemType::Predefined => self
                .as_any()
                .downcast_ref::<PredefinedMenuItem>()
                .unwrap()
                .0
                .borrow_mut()
                .create_ns_item_for_predefined_menu_item(menu_id),
            MenuItemType::Check => self
                .as_any()
                .downcast_ref::<CheckMenuItem>()
                .unwrap()
                .0
                .borrow_mut()
                .create_ns_item_for_check_menu_item(menu_id),
            MenuItemType::Icon => self
                .as_any()
                .downcast_ref::<IconMenuItem>()
                .unwrap()
                .0
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
        let id: u32 = msg_send![this, tag];

        // Create a reference to the `MenuChild` from the raw pointer
        // stored as an instance variable on the native menu item
        let ptr: usize = *this.get_ivar(BLOCK_PTR);
        let item = ptr as *mut &mut MenuChild;

        if let PredfinedMenuItemType::About(about_meta) = &(*item).predefined_item_type {
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

        if (*item).type_ == MenuItemType::Check {
            (*item).set_checked(!(*item).is_checked());
        }

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

        let ns_menu_item: *mut Object = msg_send![make_menu_item_class(), alloc];

        ns_menu_item.initWithTitle_action_keyEquivalent_(title, selector, key_equivalent);
        ns_menu_item.setKeyEquivalentModifierMask_(modifier_mask);

        Ok(ns_menu_item.autorelease())
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
