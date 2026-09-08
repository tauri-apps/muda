// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use objc2::{
    define_class, msg_send,
    rc::Retained,
    runtime::{AnyObject, Sel},
    MainThreadOnly,
};
use objc2_app_kit::{NSEventModifierFlags, NSMenuItem};
use objc2_foundation::{MainThreadMarker, NSString};

use super::{util::strip_mnemonic, PlatformMenuItem};
use crate::accelerator::MenuAccelerator;

define_class!(
    #[unsafe(super(NSMenuItem))]
    #[name = "MudaMenuItem"]
    #[thread_kind = MainThreadOnly]
    #[ivars = Cell<Option<Rc<RefCell<PlatformMenuItem>>>>]
    pub(super) struct NsMenuItem;

    impl NsMenuItem {
        #[unsafe(method(customAction:))]
        fn custom_action(&self, _sender: Option<&AnyObject>) {
            self.action();
        }

        #[unsafe(method(customShowAboutPanel:))]
        fn custom_show_about_panel(&self, _sender: Option<&AnyObject>) {
            self.show_about_panel();
        }
    }
);

impl NsMenuItem {
    fn new(
        mtm: MainThreadMarker,
        title: &NSString,
        action: Option<Sel>,
        key_equivalent: &NSString,
    ) -> Retained<Self> {
        let this = mtm.alloc().set_ivars(Cell::new(None));
        unsafe {
            msg_send![super(this), initWithTitle: title, action: action, keyEquivalent: key_equivalent]
        }
    }

    pub(super) fn create(
        mtm: MainThreadMarker,
        title: &str,
        selector: Option<Sel>,
        accelerator: &Option<MenuAccelerator>,
    ) -> crate::Result<Retained<Self>> {
        let title = NSString::from_str(&strip_mnemonic(title));

        let key_equivalent = accelerator
            .as_ref()
            .map(|accel| accel.key_equivalent())
            .transpose()?
            .unwrap_or_default();
        let key_equivalent = NSString::from_str(&key_equivalent);

        let modifier_mask = accelerator
            .as_ref()
            .map(MenuAccelerator::modifier_mask)
            .unwrap_or_else(NSEventModifierFlags::empty);

        let item = Self::new(mtm, &title, selector, &key_equivalent);
        item.setKeyEquivalentModifierMask(modifier_mask);

        Ok(item)
    }
}
