mod accelerator;
mod menu_item;

use crate::NativeMenuItem;
use cocoa::{
    appkit::{NSApp, NSApplication, NSMenu, NSMenuItem},
    base::{id, nil, NO},
    foundation::{NSAutoreleasePool, NSString},
};
use objc::{msg_send, sel, sel_impl};

pub use menu_item::MenuItem;

use self::accelerator::remove_mnemonic;

#[derive(Debug, Clone)]
pub struct Menu(id);

impl Menu {
    pub fn new() -> Self {
        unsafe {
            let ns_menu = NSMenu::alloc(nil).autorelease();
            let () = msg_send![ns_menu, setAutoenablesItems: NO];
            Self(ns_menu)
        }
    }

    pub fn add_submenu<S: AsRef<str>>(&mut self, label: S, enabled: bool) -> Submenu {
        let menu = Menu::new();
        let menu_item = MenuItem::new("", enabled, sel!(fireMenubarAction:), None);

        unsafe {
            menu_item.ns_menu_item.setSubmenu_(menu.0);
            self.0.addItem_(menu_item.ns_menu_item);
        }

        let mut sub_menu = Submenu { menu, menu_item };
        sub_menu.set_label(label);

        sub_menu
    }

    pub fn init_for_nsapp(&self) {
        unsafe {
            NSApp().setMainMenu_(self.0);
        }
    }

    pub fn remove_for_nsapp(&self) {
        unsafe {
            NSApp().setMainMenu_(std::ptr::null_mut());
        }
    }
}

#[derive(Debug, Clone)]
pub struct Submenu {
    pub(crate) menu: Menu,
    pub(crate) menu_item: MenuItem,
}

impl Submenu {
    pub fn label(&self) -> String {
        self.menu_item.label()
    }

    pub fn set_label<S: AsRef<str>>(&mut self, label: S) {
        let label = remove_mnemonic(label);
        self.menu_item.set_label(&label);
        unsafe {
            let menu_title = NSString::alloc(nil).init_str(&label);
            let () = msg_send![self.menu.0, setTitle: menu_title];
        }
    }

    pub fn enabled(&self) -> bool {
        self.menu_item.enabled()
    }

    pub fn set_enabled(&mut self, _enabled: bool) {
        self.menu_item.set_enabled(_enabled)
    }

    pub fn add_submenu<S: AsRef<str>>(&mut self, label: S, enabled: bool) -> Submenu {
        self.menu.add_submenu(label, enabled)
    }

    pub fn add_item<S: AsRef<str>>(
        &mut self,
        label: S,
        enabled: bool,
        accelerator: Option<&str>,
    ) -> MenuItem {
        let item = MenuItem::new(label, enabled, sel!(fireMenubarAction:), accelerator);
        unsafe {
            self.menu.0.addItem_(item.ns_menu_item);
        }
        item
    }

    pub fn add_native_item(&mut self, _item: NativeMenuItem) {
        // TODO
        return;
    }
}
