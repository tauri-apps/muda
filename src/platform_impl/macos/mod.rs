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
        let menu = Menu::new();
        let menu_item = TextMenuItem::new("", enabled, false, sel!(fireMenubarAction:));

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
}

#[derive(Debug, Clone)]
pub struct Submenu {
    pub(crate) menu: Menu,
    pub(crate) menu_item: TextMenuItem,
}

impl Submenu {
    pub fn label(&self) -> String {
        self.menu_item.label()
    }

    pub fn set_label(&mut self, label: impl AsRef<str>) {
        self.menu_item.set_label(label.as_ref().to_string());
        unsafe {
            let menu_title = NSString::alloc(nil).init_str(label.as_ref());
            let () = msg_send![self.menu.0, setTitle: menu_title];
        }
    }

    pub fn enabled(&self) -> bool {
        self.menu_item.enabled()
    }

    pub fn set_enabled(&mut self, _enabled: bool) {
        self.menu_item.set_enabled(_enabled)
    }

    pub fn add_submenu(&mut self, label: impl AsRef<str>, enabled: bool) -> Submenu {
        self.menu.add_submenu(label, enabled)
    }

    pub fn add_text_item(
        &mut self,
        label: impl AsRef<str>,
        enabled: bool,
        selected: bool,
    ) -> TextMenuItem {
        let item = TextMenuItem::new(label, enabled, selected, sel!(fireMenubarAction:));
        unsafe {
            self.menu.0.addItem_(item.ns_menu_item);
        }
        item
    }
}
