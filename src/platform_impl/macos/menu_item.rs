use crate::counter::Counter;
use crate::platform_impl::platform_impl::accelerator::{parse_accelerator, remove_mnemonic};
use cocoa::{
    appkit::{NSButton, NSEventModifierFlags, NSMenuItem},
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
use std::rc::Rc;
use std::sync::Once;

static COUNTER: Counter = Counter::new();

#[derive(Debug, Clone)]
pub struct MenuItem {
    pub(crate) id: u64,
    pub(crate) ns_menu_item: id,
    label: Rc<str>,
}

impl MenuItem {
    pub fn new<S: AsRef<str>>(
        label: S,
        enabled: bool,
        selector: Sel,
        accelerator: Option<&str>,
    ) -> Self {
        let (id, ns_menu_item) = make_menu_item(&remove_mnemonic(&label), selector, accelerator);

        unsafe {
            (&mut *ns_menu_item).set_ivar(MENU_IDENTITY, id);
            let () = msg_send![&*ns_menu_item, setTarget:&*ns_menu_item];

            if !enabled {
                let () = msg_send![ns_menu_item, setEnabled: NO];
            }
        }
        Self {
            id,
            ns_menu_item,
            label: Rc::from(label.as_ref()),
        }
    }

    pub fn label(&self) -> String {
        self.label.to_string()
    }

    pub fn set_label<S: AsRef<str>>(&mut self, label: S) {
        unsafe {
            let title = NSString::alloc(nil).init_str(&remove_mnemonic(&label));
            self.ns_menu_item.setTitle_(title);
        }
        self.label = Rc::from(label.as_ref());
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

pub fn make_menu_item(title: &str, selector: Sel, accelerator: Option<&str>) -> (u64, *mut Object) {
    let alloc = make_menu_item_alloc();
    let menu_id = COUNTER.next();

    unsafe {
        let title = NSString::alloc(nil).init_str(title);
        let menu_item = make_menu_item_from_alloc(alloc, title, selector, accelerator);

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
    accelerator: Option<&str>,
) -> *mut Object {
    unsafe {
        let (key_equivalent, masks) = match accelerator {
            Some(accelerator) => {
                let (key, mods) = parse_accelerator(accelerator);
                let key = NSString::alloc(nil).init_str(&key);
                (key, mods)
            }
            None => (
                NSString::alloc(nil).init_str(""),
                NSEventModifierFlags::empty(),
            ),
        };

        // allocate our item to our class
        let item: id =
            msg_send![alloc, initWithTitle: title action: selector keyEquivalent: key_equivalent];
        item.setKeyEquivalentModifierMask_(masks);

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
