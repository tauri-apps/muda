mod accelerator;
mod util;

use std::{cell::RefCell, collections::HashMap, rc::Rc, sync::Once};

use cocoa::{
    appkit::{CGFloat, NSApp, NSApplication, NSEventModifierFlags, NSMenu, NSMenuItem},
    base::{id, nil, selector, NO, YES},
    foundation::{NSAutoreleasePool, NSInteger, NSPoint, NSRect, NSString},
};
use objc::{
    declare::ClassDecl,
    runtime::{Class, Object, Sel},
};

use self::util::{app_name_string, strip_mnemonic};
use crate::{
    accelerator::Accelerator,
    predefined::PredfinedMenuItemType,
    util::{AddOp, Counter},
    MenuItemExt, MenuItemType,
};

static COUNTER: Counter = Counter::new();
static BLOCK_PTR: &str = "mudaMenuItemBlockPtr";

/// A generic child in a menu
///
/// Be careful when cloning this item and treat it as read-only
#[derive(Debug, Default, PartialEq, Eq)]
#[allow(dead_code)]
struct MenuChild {
    // shared fields between submenus and menu items
    type_: MenuItemType,
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

    // submenu fields
    children: Option<Vec<Rc<RefCell<MenuChild>>>>,
    ns_menus: HashMap<u32, Vec<id>>,
}

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

    pub fn append(&self, item: &dyn crate::MenuItemExt) {
        self.add_menu_item(item, AddOp::Append)
    }

    pub fn prepend(&self, item: &dyn crate::MenuItemExt) {
        self.add_menu_item(item, AddOp::Insert(0))
    }

    pub fn insert(&self, item: &dyn crate::MenuItemExt, position: usize) {
        self.add_menu_item(item, AddOp::Insert(position))
    }

    fn add_menu_item(&self, item: &dyn crate::MenuItemExt, op: AddOp) {
        let ns_menu_item: *mut Object = item.make_ns_item_for_menu(self.id);
        let child: Rc<RefCell<MenuChild>> = item.get_child();

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
    }

    pub fn remove(&self, item: &dyn crate::MenuItemExt) -> crate::Result<()> {
        // get a list of instances of the specified NSMenuItem in this menu
        if let Some(ns_menu_items) = match item.type_() {
            MenuItemType::Submenu => {
                let submenu = item.as_any().downcast_ref::<crate::Submenu>().unwrap();
                submenu.0 .0.borrow_mut()
            }
            MenuItemType::Normal => {
                let menuitem = item.as_any().downcast_ref::<crate::MenuItem>().unwrap();
                menuitem.0 .0.borrow_mut()
            }
            MenuItemType::Check => {
                let menuitem = item
                    .as_any()
                    .downcast_ref::<crate::CheckMenuItem>()
                    .unwrap();
                menuitem.0 .0.borrow_mut()
            }
            MenuItemType::Predefined => {
                let menuitem = item
                    .as_any()
                    .downcast_ref::<crate::PredefinedMenuItem>()
                    .unwrap();
                menuitem.0 .0.borrow_mut()
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

    pub fn items(&self) -> Vec<Box<dyn crate::MenuItemExt>> {
        self.children
            .borrow()
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
                }
            })
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

#[derive(Clone)]
pub(crate) struct Submenu(Rc<RefCell<MenuChild>>);

impl Submenu {
    pub fn new(text: &str, enabled: bool) -> Self {
        Self(Rc::new(RefCell::new(MenuChild {
            type_: MenuItemType::Submenu,
            text: strip_mnemonic(text),
            enabled,
            children: Some(Vec::new()),
            ..Default::default()
        })))
    }

    pub fn id(&self) -> u32 {
        self.0.borrow().id()
    }

    pub fn make_ns_item_for_menu(&self, menu_id: u32) -> id {
        let mut self_ = self.0.borrow_mut();
        let ns_menu_item: *mut Object;
        let ns_submenu: *mut Object;

        unsafe {
            ns_menu_item = NSMenuItem::alloc(nil).autorelease();
            ns_submenu = NSMenu::alloc(nil).autorelease();

            let title = NSString::alloc(nil).init_str(&self_.text).autorelease();
            let () = msg_send![ns_submenu, setTitle: title];
            let () = msg_send![ns_menu_item, setTitle: title];
            let () = msg_send![ns_menu_item, setSubmenu: ns_submenu];

            if !self_.enabled {
                let () = msg_send![ns_menu_item, setEnabled: NO];
            }
        }

        for item in self_.children.as_ref().unwrap() {
            let item_type = &item.borrow().type_.clone();
            let ns_item = match item_type {
                MenuItemType::Submenu => Submenu(item.clone()).make_ns_item_for_menu(menu_id),
                MenuItemType::Normal => MenuItem(item.clone()).make_ns_item_for_menu(menu_id),
                MenuItemType::Predefined => {
                    PredefinedMenuItem(item.clone()).make_ns_item_for_menu(menu_id)
                }
                MenuItemType::Check => CheckMenuItem(item.clone()).make_ns_item_for_menu(menu_id),
            };
            unsafe { ns_submenu.addItem_(ns_item) };
        }

        self_
            .ns_menus
            .entry(menu_id)
            .or_insert_with(Vec::new)
            .push(ns_submenu);

        self_
            .ns_menu_items
            .entry(menu_id)
            .or_insert_with(Vec::new)
            .push(ns_menu_item);

        ns_menu_item
    }

    pub fn append(&self, item: &dyn crate::MenuItemExt) {
        self.add_menu_item(item, AddOp::Append)
    }

    pub fn prepend(&self, item: &dyn crate::MenuItemExt) {
        self.add_menu_item(item, AddOp::Insert(0))
    }

    pub fn insert(&self, item: &dyn crate::MenuItemExt, position: usize) {
        self.add_menu_item(item, AddOp::Insert(position))
    }

    fn add_menu_item(&self, item: &dyn crate::MenuItemExt, op: AddOp) {
        let mut self_ = self.0.borrow_mut();

        let item_child: Rc<RefCell<MenuChild>> = item.get_child();

        unsafe {
            match op {
                AddOp::Append => {
                    for menus in self_.ns_menus.values() {
                        for ns_menu in menus {
                            let ns_menu_item: *mut Object = item.make_ns_item_for_menu(self_.id);
                            ns_menu.addItem_(ns_menu_item);
                        }
                    }
                    self_.children.as_mut().unwrap().push(item_child);
                }
                AddOp::Insert(position) => {
                    for menus in self_.ns_menus.values() {
                        for &ns_menu in menus {
                            let ns_menu_item: *mut Object = item.make_ns_item_for_menu(self_.id);
                            let () = msg_send![ns_menu, insertItem: ns_menu_item atIndex: position as NSInteger];
                        }
                    }
                    self_
                        .children
                        .as_mut()
                        .unwrap()
                        .insert(position, item_child);
                }
            }
        }
    }

    pub fn remove(&self, item: &dyn crate::MenuItemExt) -> crate::Result<()> {
        let mut child = self.0.borrow_mut();

        let item_child: Rc<RefCell<MenuChild>> = item.get_child();

        // get a list of instances of the specified NSMenuItem in this menu
        if let Some(ns_menu_items) = item_child.borrow_mut().ns_menu_items.remove(&child.id) {
            // remove each NSMenuItem from the NSMenu
            unsafe {
                for item in ns_menu_items {
                    for menus in child.ns_menus.values() {
                        for &ns_menu in menus {
                            let () = msg_send![ns_menu, removeItem: item];
                        }
                    }
                }
            }
        }

        // remove the item from our internal list of children
        let children = child.children.as_mut().unwrap();
        let index = children
            .iter()
            .position(|e| e == &item_child)
            .ok_or(crate::Error::NotAChildOfThisMenu)?;
        children.remove(index);

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
                }
            })
            .collect()
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

    pub fn show_context_menu_for_nsview(&self, view: id, x: f64, y: f64) {
        // TODO: this needs to work even if it hasn't already been added to a menu
        if let Some(ns_menus) = self.0.borrow().ns_menus.get(&1) {
            unsafe {
                let window: id = msg_send![view, window];
                let scale_factor: CGFloat = msg_send![window, backingScaleFactor];
                let view_point = NSPoint::new(x / scale_factor, y / scale_factor);
                let view_rect: NSRect = msg_send![view, frame];
                let location = NSPoint::new(view_point.x, view_rect.size.height - view_point.y);
                msg_send![ns_menus[0], popUpMenuPositioningItem: nil atLocation: location inView: view]
            }
        }
    }

    pub fn set_windows_menu_for_nsapp(&self) {
        if let Some(ns_menus) = self.0.borrow().ns_menus.get(&1) {
            unsafe { NSApp().setWindowsMenu_(ns_menus[0]) }
        }
    }

    pub fn set_help_menu_for_nsapp(&self) {
        if let Some(ns_menus) = self.0.borrow().ns_menus.get(&1) {
            unsafe { msg_send![NSApp(), setHelpMenu: ns_menus[0]] }
        }
    }

    pub fn ns_menu(&self) -> *mut std::ffi::c_void {
        *self
            .0
            .borrow()
            .ns_menus
            .values()
            .collect::<Vec<_>>()
            .get(0)
            .unwrap()
            .get(0)
            .unwrap() as _
    }
}

#[derive(Clone, Debug)]
pub(crate) struct MenuItem(Rc<RefCell<MenuChild>>);

impl MenuItem {
    pub fn new(text: &str, enabled: bool, accelerator: Option<Accelerator>) -> Self {
        Self(Rc::new(RefCell::new(MenuChild {
            type_: MenuItemType::Normal,
            text: strip_mnemonic(text),
            enabled,
            id: COUNTER.next(),
            accelerator,
            ..Default::default()
        })))
    }

    pub fn make_ns_item_for_menu(&self, menu_id: u32) -> id {
        let mut child = self.0.borrow_mut();

        let ns_menu_item = create_ns_menu_item(
            &child.text,
            Some(sel!(fireMenuItemAction:)),
            &child.accelerator,
        );

        unsafe {
            let _: () = msg_send![ns_menu_item, setTarget: ns_menu_item];
            let _: () = msg_send![ns_menu_item, setTag:child.id()];

            // Store a raw pointer to the `MenuChild` as an instance variable on the native menu item
            let ptr = Box::into_raw(Box::new(&*child));
            (*ns_menu_item).set_ivar(BLOCK_PTR, ptr as usize);

            if !child.enabled {
                let () = msg_send![ns_menu_item, setEnabled: NO];
            }
        }

        child
            .ns_menu_items
            .entry(menu_id)
            .or_insert_with(Vec::new)
            .push(ns_menu_item);

        ns_menu_item
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

#[derive(Clone, Debug)]
pub(crate) struct PredefinedMenuItem(Rc<RefCell<MenuChild>>);

impl PredefinedMenuItem {
    pub fn new(item_type: PredfinedMenuItemType, text: Option<String>) -> Self {
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

        Self(Rc::new(RefCell::new(MenuChild {
            type_: MenuItemType::Predefined,
            text,
            enabled: true,
            id: COUNTER.next(),
            accelerator,
            predefined_item_type: item_type,
            // ns_menu_item,
            ..Default::default()
        })))
    }

    pub fn make_ns_item_for_menu(&self, menu_id: u32) -> id {
        let mut child = self.0.borrow_mut();

        let item_type = &child.predefined_item_type;
        let ns_menu_item = match item_type {
            PredfinedMenuItemType::Separator => unsafe {
                NSMenuItem::separatorItem(nil).autorelease()
            },
            _ => create_ns_menu_item(&child.text, item_type.selector(), &child.accelerator),
        };

        unsafe {
            if !child.enabled {
                let () = msg_send![ns_menu_item, setEnabled: NO];
            }
            if child.predefined_item_type == PredfinedMenuItemType::Services {
                // we have to assign an empty menu as the app's services menu, and macOS will populate it
                let services_menu = NSMenu::new(nil).autorelease();
                let () = msg_send![NSApp(), setServicesMenu: services_menu];
                let () = msg_send![ns_menu_item, setSubmenu: services_menu];
            }
        }

        child
            .ns_menu_items
            .entry(menu_id)
            .or_insert_with(Vec::new)
            .push(ns_menu_item);

        ns_menu_item
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

#[derive(Clone, Debug)]
pub(crate) struct CheckMenuItem(Rc<RefCell<MenuChild>>);

impl CheckMenuItem {
    pub fn new(text: &str, enabled: bool, checked: bool, accelerator: Option<Accelerator>) -> Self {
        Self(Rc::new(RefCell::new(MenuChild {
            type_: MenuItemType::Check,
            text: text.to_string(),
            enabled,
            id: COUNTER.next(),
            accelerator,
            checked,
            ..Default::default()
        })))
    }

    pub fn make_ns_item_for_menu(&self, menu_id: u32) -> id {
        let mut child = self.0.borrow_mut();

        let ns_menu_item = create_ns_menu_item(
            &child.text,
            Some(sel!(fireMenuItemAction:)),
            &child.accelerator,
        );

        unsafe {
            let _: () = msg_send![ns_menu_item, setTarget: ns_menu_item];
            let _: () = msg_send![ns_menu_item, setTag:child.id()];

            // Store a raw pointer to the `MenuChild` as an instance variable on the native menu item
            let ptr = Box::into_raw(Box::new(&*child));
            (*ns_menu_item).set_ivar(BLOCK_PTR, ptr as usize);

            if !child.enabled {
                let () = msg_send![ns_menu_item, setEnabled: NO];
            }
            if child.checked {
                let () = msg_send![ns_menu_item, setState: 1_isize];
            }
        }

        child
            .ns_menu_items
            .entry(menu_id)
            .or_insert_with(Vec::new)
            .push(ns_menu_item);

        ns_menu_item
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
            PredfinedMenuItemType::About(_) => Some(selector("orderFrontStandardAboutPanel:")),
            PredfinedMenuItemType::Services => None,
            PredfinedMenuItemType::None => None,
        }
    }
}

impl dyn MenuItemExt + '_ {
    fn get_child(&self) -> Rc<RefCell<MenuChild>> {
        match self.type_() {
            MenuItemType::Submenu => {
                let submenu = self.as_any().downcast_ref::<crate::Submenu>().unwrap();
                Rc::clone(&submenu.0 .0)
            }
            MenuItemType::Normal => {
                let menuitem = self.as_any().downcast_ref::<crate::MenuItem>().unwrap();
                Rc::clone(&menuitem.0 .0)
            }
            MenuItemType::Check => {
                let menuitem = self
                    .as_any()
                    .downcast_ref::<crate::CheckMenuItem>()
                    .unwrap();
                Rc::clone(&menuitem.0 .0)
            }
            MenuItemType::Predefined => {
                let menuitem = self
                    .as_any()
                    .downcast_ref::<crate::PredefinedMenuItem>()
                    .unwrap();
                Rc::clone(&menuitem.0 .0)
            }
        }
    }

    fn make_ns_item_for_menu(&self, menu_id: u32) -> *mut Object {
        match self.type_() {
            MenuItemType::Submenu => {
                let submenu = self.as_any().downcast_ref::<crate::Submenu>().unwrap();
                submenu.0.make_ns_item_for_menu(menu_id)
            }
            MenuItemType::Normal => {
                let menuitem = self.as_any().downcast_ref::<crate::MenuItem>().unwrap();
                menuitem.0.make_ns_item_for_menu(menu_id)
            }
            MenuItemType::Check => {
                let menuitem = self
                    .as_any()
                    .downcast_ref::<crate::CheckMenuItem>()
                    .unwrap();
                menuitem.0.make_ns_item_for_menu(menu_id)
            }
            MenuItemType::Predefined => {
                let menuitem = self
                    .as_any()
                    .downcast_ref::<crate::PredefinedMenuItem>()
                    .unwrap();
                menuitem.0.make_ns_item_for_menu(menu_id)
            }
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

        if (*item).type_ == MenuItemType::Check {
            (*item).set_checked(!(*item).is_checked());
        }

        let _ = crate::MENU_CHANNEL.0.send(crate::MenuEvent { id });
    }
}

fn create_ns_menu_item(
    title: &str,
    selector: Option<Sel>,
    accelerator: &Option<Accelerator>,
) -> id {
    unsafe {
        let title = NSString::alloc(nil).init_str(title).autorelease();

        let selector = selector.unwrap_or_else(|| Sel::from_ptr(std::ptr::null()));

        let key_equivalent = accelerator
            .clone()
            .map(|accel| accel.key_equivalent())
            .unwrap_or_default();
        let key_equivalent = NSString::alloc(nil)
            .init_str(key_equivalent.as_str())
            .autorelease();

        let modifier_mask = accelerator
            .clone()
            .map(|accel| accel.key_modifier_mask())
            .unwrap_or_else(NSEventModifierFlags::empty);

        let ns_menu_item: *mut Object = msg_send![make_menu_item_class(), alloc];

        ns_menu_item.initWithTitle_action_keyEquivalent_(title, selector, key_equivalent);
        ns_menu_item.setKeyEquivalentModifierMask_(modifier_mask);

        ns_menu_item.autorelease()
    }
}
