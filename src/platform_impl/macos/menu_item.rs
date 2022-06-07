use crate::counter::Counter;
use cocoa::{
    appkit::NSButton,
    base::{id, nil, BOOL, NO, YES},
    foundation::NSString,
};
use objc::{
    class,
    declare::ClassDecl,
    msg_send,
    runtime::{Class, Object, Sel},
    sel, sel_impl,
};
use std::slice;
use std::sync::Once;
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

static COUNTER: Counter = Counter::new();

#[derive(Debug, Clone)]
pub struct TextMenuItem {
    pub(crate) id: u64,
    pub(crate) ns_menu_item: id,
}

impl TextMenuItem {
    pub fn new(label: impl AsRef<str>, enabled: bool, selector: Sel) -> Self {
        let (id, ns_menu_item) = make_menu_item(label.as_ref(), selector);

        unsafe {
            (&mut *ns_menu_item).set_ivar(MENU_IDENTITY, id);
            let () = msg_send![&*ns_menu_item, setTarget:&*ns_menu_item];

            if !enabled {
                let () = msg_send![ns_menu_item, setEnabled: NO];
            }
        }

        Self { id, ns_menu_item }
    }

    pub fn label(&self) -> String {
        unsafe {
            let title: id = msg_send![self.ns_menu_item, title];
            let data = title.UTF8String() as *const u8;
            let len = title.len();

            String::from_utf8_lossy(slice::from_raw_parts(data, len)).to_string()
        }
    }

    pub fn set_label(&mut self, label: impl AsRef<str>) {
        unsafe {
            let title = NSString::alloc(nil).init_str(label.as_ref());
            self.ns_menu_item.setTitle_(title);
        }
    }

    pub fn enabled(&self) -> bool {
        unsafe {
            let enabled: BOOL = msg_send![self.ns_menu_item, isEnabled];
            enabled == YES
        }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        unsafe {
            let status = match enabled {
                true => YES,
                false => NO,
            };
            let () = msg_send![self.ns_menu_item, setEnabled: status];
        }
    }

    pub fn id(&self) -> u64 {
        self.id
    }
}

pub fn make_menu_item(
    title: &str,
    selector: Sel,
    //key_equivalent: Option<key::KeyEquivalent>,
    //menu_type: MenuType,
) -> (MenuId, *mut Object) {
    let alloc = make_menu_item_alloc();
    let menu_id = COUNTER.next();

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
