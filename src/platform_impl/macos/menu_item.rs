use cocoa::{
    appkit::{NSButton, NSEventModifierFlags, NSMenuItem},
    base::{id, nil, NO, YES},
    foundation::NSString,
};
use objc::{
    class,
    declare::ClassDecl,
    msg_send,
    runtime::{Class, Object, Sel},
    sel, sel_impl,
};
use std::sync::Once;
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

/// Identifier of a custom menu item.
///
/// Whenever you receive an event arising from a particular menu, this event contains a `MenuId` which
/// identifies its origin.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct MenuId(pub u64);

impl From<MenuId> for u64 {
    fn from(s: MenuId) -> u64 {
        s.0
    }
}

impl MenuId {
    /// Return an empty `MenuId`.
    pub const EMPTY: MenuId = MenuId(0);

    /// Create new `MenuId` from a String.
    pub fn new(unique_string: &str) -> MenuId {
        MenuId(hash_string_to_u64(unique_string))
    }

    /// Whenever this menu is empty.
    pub fn is_empty(self) -> bool {
        Self::EMPTY == self
    }
}

fn hash_string_to_u64(title: &str) -> u64 {
    let mut s = DefaultHasher::new();
    title.to_uppercase().hash(&mut s);
    s.finish() as u64
}

#[derive(Debug, Clone)]
pub struct TextMenuItem {
    pub(crate) id: MenuId,
    pub(crate) ns_menu_item: id,
}

impl TextMenuItem {
    pub fn new(label: impl AsRef<str>, enabled: bool, selector: Sel) -> Self {
        let (id, ns_menu_item) = make_menu_item(label.as_ref(), selector);

        unsafe {
            (&mut *ns_menu_item).set_ivar(MENU_IDENTITY, id.0);
            let () = msg_send![&*ns_menu_item, setTarget:&*ns_menu_item];

            if !enabled {
                let () = msg_send![ns_menu_item, setEnabled: NO];
            }
        }

        Self { id, ns_menu_item }
    }

    pub fn label(&self) -> String {
        todo!()
    }

    pub fn set_label(&mut self, label: impl AsRef<str>) {
        todo!()
    }

    pub fn enabled(&self) -> bool {
        todo!()
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        todo!()
    }

    pub fn id(&self) -> u64 {
        self.id.0
    }
}

pub fn make_menu_item(
    title: &str,
    selector: Sel,
    //key_equivalent: Option<key::KeyEquivalent>,
    //menu_type: MenuType,
) -> (MenuId, *mut Object) {
    let alloc = make_menu_item_alloc();
    let menu_id = MenuId::new(title);

    unsafe {
        let title = NSString::alloc(nil).init_str(title);
        let menu_item = make_menu_item_from_alloc(alloc, title, selector); //, key_equivalent, menu_type);

        (menu_id, menu_item)
    }
}

fn make_menu_item_alloc() -> *mut Object {
    unsafe { msg_send![make_menu_item_class(), alloc] }
}

static MENU_IDENTITY: &str = "MenuItemIdentity";

fn make_menu_item_class() -> *const Class {
    static mut APP_CLASS: *const Class = 0 as *const Class;
    static INIT: Once = Once::new();

    INIT.call_once(|| unsafe {
        let superclass = class!(NSMenuItem);
        let mut decl = ClassDecl::new("MenuItem", superclass).unwrap();
        decl.add_ivar::<u64>(MENU_IDENTITY);

        decl.add_method(
            sel!(dealloc),
            dealloc_custom_menuitem as extern "C" fn(&Object, _),
        );

        decl.add_method(
            sel!(fireMenubarAction:),
            fire_menu_bar_click as extern "C" fn(&Object, _, id),
        );

        decl.add_method(
            sel!(fireStatusbarAction:),
            fire_status_bar_click as extern "C" fn(&Object, _, id),
        );

        APP_CLASS = decl.register();
    });

    unsafe { APP_CLASS }
}

fn make_menu_item_from_alloc(
    alloc: *mut Object,
    title: *mut Object,
    selector: Sel,
    //key_equivalent: Option<key::KeyEquivalent>,
    //menu_type: MenuType,
) -> *mut Object {
    unsafe {
        // let (key, masks) = match key_equivalent {
        //   Some(ke) => (
        //     NSString::alloc(nil).init_str(ke.key),
        //     ke.masks.unwrap_or_else(NSEventModifierFlags::empty),
        //   ),
        //   None => (
        //     NSString::alloc(nil).init_str(""),
        //     NSEventModifierFlags::empty(),
        //   ),
        // };
        let key = NSString::alloc(nil).init_str("");

        // allocate our item to our class
        let item: id = msg_send![alloc, initWithTitle: title action: selector keyEquivalent: key];

        // item.setKeyEquivalentModifierMask_(masks);
        item
    }
}

extern "C" fn fire_menu_bar_click(this: &Object, _: Sel, _item: id) {
    send_event(this);
}

extern "C" fn fire_status_bar_click(this: &Object, _: Sel, _item: id) {
    send_event(this);
}

extern "C" fn dealloc_custom_menuitem(this: &Object, _: Sel) {
    unsafe {
        let _: () = msg_send![super(this, class!(NSMenuItem)), dealloc];
    }
}

fn send_event(this: &Object) {
    let id: u64 = unsafe { *this.get_ivar(MENU_IDENTITY) };
    let _ = crate::MENU_CHANNEL.0.send(crate::MenuEvent { id: id as _ });
}
