use cocoa::{
    appkit::{NSApp, NSApplication, NSMenu, NSMenuItem},
    base::{id, nil, NO},
    foundation::{NSAutoreleasePool, NSString},
};
use objc::{msg_send, sel, sel_impl};

mod menu_item;
pub use menu_item::TextMenuItem;
use menu_item::*;

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

    pub fn add_submenu(&mut self, label: impl AsRef<str>, enabled: bool) -> Submenu {
        let mut sub_menu = Submenu(Menu::new());
        sub_menu.set_label(label.as_ref());
        sub_menu.set_enabled(enabled);
        let item = TextMenuItem::new(label, enabled, sel!(fireMenubarAction:));

        unsafe {
            item.ns_menu_item.setSubmenu_(sub_menu.0 .0);
            self.0.addItem_(item.ns_menu_item);
        }

        sub_menu
    }

    pub fn init_for_nsapp(&self) {
        unsafe {
            NSApp().setMainMenu_(self.0);
        }
    }
}

#[derive(Debug, Clone)]
pub struct Submenu(Menu);

impl Submenu {
    pub fn label(&self) -> String {
        todo!()
    }

    pub fn set_label(&mut self, label: impl AsRef<str>) {
        unsafe {
            let menu_title = NSString::alloc(nil).init_str(label.as_ref());
            let () = msg_send![self.0 .0, setTitle: menu_title];
        }
    }

    pub fn enabled(&self) -> bool {
        true
    }

    pub fn set_enabled(&mut self, _enabled: bool) {}

    pub fn add_submenu(&mut self, label: impl AsRef<str>, enabled: bool) -> Submenu {
        self.0.add_submenu(label, enabled)
    }

    pub fn add_text_item(&mut self, label: impl AsRef<str>, enabled: bool) -> TextMenuItem {
        let item = TextMenuItem::new(label, enabled, sel!(fireMenubarAction:));
        unsafe {
            self.0 .0.addItem_(item.ns_menu_item);
        }
        item
    }
}
