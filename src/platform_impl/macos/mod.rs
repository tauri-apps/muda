mod accelerator;

use std::{rc::Rc, cell::RefCell};

use cocoa::{
    appkit::{NSApp, NSApplication, NSMenu, NSMenuItem, NSEventModifierFlags},
    base::{nil, id, NO, selector},
    foundation::{NSAutoreleasePool, NSString},
};
use objc::runtime::{Object, Sel};

use crate::{
    accelerator::Accelerator,
    internal::MenuItemType,
    predefined::PredfinedMenuItemType,
    util::{AddOp, Counter},
};

static COUNTER: Counter = Counter::new();

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

    // menu item fields
    id: u32,
    accelerator: Option<Accelerator>,

    // predefined menu item fields
    is_predefined: bool,
    predefined_item_type: PredfinedMenuItemType,

    // check menu item fields
    checked: bool,

    // submenu fields
    submenu: Option<Menu>,
    children: Option<Vec<Rc<RefCell<MenuChild>>>>,
}

impl MenuChild {
}


#[derive(Clone, Debug)]
pub struct Menu {
    pub ns_menu: id,
}

impl Menu {
    pub fn new() -> Self {
        let ns_menu: *mut Object;
        unsafe {
            ns_menu = NSMenu::alloc(nil).autorelease();
            let () = msg_send![ns_menu, setAutoenablesItems: NO];
        }
        Self { ns_menu }
    }

    pub fn append(&self, item: &dyn crate::MenuEntry) {
        self.add_menu_item(item, AddOp::Append)
    }

    pub fn prepend(&self, item: &dyn crate::MenuEntry) {
        self.add_menu_item(item, AddOp::Insert(0))
    }

    pub fn insert(&self, item: &dyn crate::MenuEntry, position: usize) {
        self.add_menu_item(item, AddOp::Insert(position))
    }

    fn add_menu_item(&self, item: &dyn crate::MenuEntry, op: AddOp) {
        let child = match item.type_() {
            MenuItemType::Submenu => {
                let submenu = item.as_any().downcast_ref::<crate::Submenu>().unwrap();
                let child = &submenu.0 .0;
                child
            },
            MenuItemType::Normal => {
                let menuitem = item.as_any().downcast_ref::<crate::MenuItem>().unwrap();
                let child = &menuitem.0 .0;
                child
            },
            MenuItemType::Check => {
                let menuitem = item.as_any().downcast_ref::<crate::CheckMenuItem>().unwrap();
                let child = &menuitem.0 .0;
                child
            },
            MenuItemType::Predefined => {
                let menuitem = item.as_any().downcast_ref::<crate::PredefinedMenuItem>().unwrap();
                let child = &menuitem.0 .0;
                child
            },
        }.borrow();

        unsafe {
            if item.type_() == MenuItemType::Predefined && child.predefined_item_type == PredfinedMenuItemType::Separator {
                self.ns_menu.addItem_(NSMenuItem::separatorItem(nil).autorelease());
                return
            }

            let title = NSString::alloc(nil).init_str(&child.text);
            let selector = if item.type_() == MenuItemType::Predefined {
                child.predefined_item_type.selector()
            } else {
                sel!(fireMenubarAction:)
            };

            let key_equivalent = child.accelerator.clone()
                .map(|accel| { accel.key_equivalent() })
                .unwrap_or_else(|| "".into());
            let key_equivalent = NSString::alloc(nil).init_str(key_equivalent.as_str());

            let modifier_mask = child.accelerator.clone()
                .map(|accel| { accel.key_modifier_mask() })
                .unwrap_or_else(NSEventModifierFlags::empty);

            let ns_menu_item = NSMenuItem::alloc(nil).autorelease()
                .initWithTitle_action_keyEquivalent_(title, selector, key_equivalent);

            ns_menu_item.setKeyEquivalentModifierMask_(modifier_mask);

            if !child.enabled {
                let () = msg_send![ns_menu_item, setEnabled: NO];
            }

            match item.type_() {
                MenuItemType::Normal => {}
                MenuItemType::Check => {
                    if child.checked {
                        let () = msg_send![ns_menu_item, setState: 1_isize];
                    }
                }
                MenuItemType::Submenu => {
                    let submenu = child.submenu.as_ref().unwrap();
                    let ns_submenu = submenu.ns_menu;
                    let () = msg_send![ns_submenu, setTitle: title];
                    let () = msg_send![ns_menu_item, setSubmenu: ns_submenu];
                }
                MenuItemType::Predefined => {

                }
            };

            self.ns_menu.addItem_(ns_menu_item);
        }
    }

    pub fn remove(&self, item: &dyn crate::MenuEntry) {
        todo!()
    }

    pub fn items(&self) -> Vec<Box<dyn crate::MenuEntry>> {
        todo!()
    }

    pub fn init_for_nsapp(&self) {
        unsafe {
            NSApp().setMainMenu_(self.ns_menu)
        }
    }

    pub fn remove_for_nsapp(&self) {
        unsafe {
            NSApp().setMainMenu_(nil)
        }
    }

    pub fn show_context_menu_for_nsview(&self, view: id) {
        unsafe {
            let ns_menu_class = class!(NSMenu);
            let ns_event: &mut Object = msg_send![NSApp(), currentEvent];
            msg_send![ns_menu_class, popUpContextMenu: self.ns_menu withEvent: ns_event forView: view]
        }
    }
}

#[derive(Clone)]
pub(crate) struct Submenu(Rc<RefCell<MenuChild>>);

impl Submenu {
    pub fn new(text: &str, enabled: bool) -> Self {
        Self(Rc::new(RefCell::new(MenuChild {
            text: text.to_string().replace("&", ""),
            enabled,
            children: Some(Vec::new()),
            submenu: Some(Menu::new()),
            ..Default::default()
        })))
    }

    pub fn id(&self) -> u32 {
        todo!()
    }

    pub fn append(&self, item: &dyn crate::MenuEntry) {
        self.add_menu_item(item, AddOp::Append)
    }

    pub fn prepend(&self, item: &dyn crate::MenuEntry) {
        self.add_menu_item(item, AddOp::Insert(0))
    }

    pub fn insert(&self, item: &dyn crate::MenuEntry, position: usize) {
        self.add_menu_item(item, AddOp::Insert(position))
    }

    fn add_menu_item(&self, item: &dyn crate::MenuEntry, op: AddOp) {
        self.0.borrow_mut().submenu.as_ref().unwrap().add_menu_item(item, op)
    }

    pub fn remove(&self, item: &dyn crate::MenuEntry) {
        todo!()
    }

    pub fn items(&self) -> Vec<Box<dyn crate::MenuEntry>> {
        todo!()
    }

    pub fn text(&self) -> String {
        todo!()
    }

    pub fn set_text(&self, text: &str) {
        todo!()
    }

    pub fn is_enabled(&self) -> bool {
        todo!()
    }

    pub fn set_enabled(&self, enabled: bool) {
        todo!()
    }
}


#[derive(Clone, Debug)]
pub(crate) struct MenuItem(Rc<RefCell<MenuChild>>);

impl MenuItem {
    pub fn new(text: &str, enabled: bool, accelerator: Option<Accelerator>) -> Self {
        Self(Rc::new(RefCell::new(MenuChild {
            type_: MenuItemType::Normal,
            text: text.to_string().replace("&", ""),
            enabled,
            id: COUNTER.next(),
            accelerator,
            ..Default::default()
        })))
    }

    pub fn id(&self) -> u32 {
        todo!()
    }

    pub fn text(&self) -> String {
        todo!()
    }

    pub fn set_text(&self, text: &str) {
        todo!()
    }

    pub fn is_enabled(&self) -> bool {
        todo!()
    }

    pub fn set_enabled(&self, enabled: bool) {
        todo!()
    }
}


#[derive(Clone, Debug)]
pub(crate) struct PredefinedMenuItem(Rc<RefCell<MenuChild>>);

impl PredefinedMenuItem {
    pub fn new(item_type: PredfinedMenuItemType, text: Option<String>) -> Self {
        Self(Rc::new(RefCell::new(MenuChild {
            type_: MenuItemType::Predefined,
            text: text.unwrap_or_else(|| item_type.text().to_string()).replace("&", ""),
            enabled: true,
            id: COUNTER.next(),
            accelerator: item_type.accelerator(),
            predefined_item_type: item_type,
            ..Default::default()
        })))
    }

    pub fn id(&self) -> u32 {
        todo!()
    }

    pub fn text(&self) -> String {
        todo!()
    }

    pub fn set_text(&self, text: &str) {
        todo!()
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

    pub fn id(&self) -> u32 {
        todo!()
    }

    pub fn text(&self) -> String {
        todo!()
    }

    pub fn set_text(&self, text: &str) {
        todo!()
    }

    pub fn is_enabled(&self) -> bool {
        todo!()
    }

    pub fn set_enabled(&self, enabled: bool) {
        todo!()
    }

    pub fn is_checked(&self) -> bool {
        todo!()
    }

    pub fn set_checked(&self, checked: bool) {
        todo!()
    }
}

impl PredfinedMenuItemType {
    pub(crate) fn selector(&self) -> Sel {
        match self {
            PredfinedMenuItemType::Copy => selector("copy:"),
            PredfinedMenuItemType::Cut => selector("cut:"),
            PredfinedMenuItemType::Paste =>selector("paste:"),
            PredfinedMenuItemType::SelectAll => selector("selectAll:"),
            PredfinedMenuItemType::Separator => selector(""),
            PredfinedMenuItemType::Minimize => selector("performMiniaturize:"),
            PredfinedMenuItemType::CloseWindow => selector("performClose:"),
            PredfinedMenuItemType::Quit => selector("terminate:"),
            PredfinedMenuItemType::About(_) => selector("orderFrontStandardAboutPanel:"),
            PredfinedMenuItemType::None => selector(""),
        }
    }
}