// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

mod icon;
mod ns_menu_item;
mod util;

pub(crate) use icon::PlatformIcon;
pub(crate) use util::app_name;

use std::{cell::RefCell, collections::HashMap, ffi::c_void, rc::Rc};

use objc2::{
    define_class, msg_send,
    rc::Retained,
    runtime::{AnyObject, NSObjectProtocol, ProtocolObject, Sel},
    sel, DeclaredClass, MainThreadOnly, Message,
};
use objc2_app_kit::{
    NSAboutPanelOptionApplicationIcon, NSAboutPanelOptionApplicationName,
    NSAboutPanelOptionApplicationVersion, NSAboutPanelOptionCredits, NSAboutPanelOptionVersion,
    NSApplication, NSControlStateValueOff, NSControlStateValueOn, NSEvent, NSEventModifierFlags,
    NSImage, NSImageName, NSMenu, NSMenuDelegate, NSMenuItem, NSView,
};
use objc2_foundation::{
    ns_string, MainThreadMarker, NSAttributedString, NSDictionary, NSInteger, NSObject, NSPoint,
    NSSize, NSString,
};

use self::{ns_menu_item::NsMenuItem, util::strip_mnemonic};
use crate::{
    accelerator::MenuAccelerator,
    dpi::{LogicalPosition, Position},
    icon::Icon,
    items::*,
    platform_impl::PlatformAttachArgs,
    util::{AddOp, Counter},
    ClickAction, MenuEvent, MenuItemKind, NativeIcon,
};

static COUNTER: Counter = Counter::new();

/// https://developer.apple.com/documentation/appkit/nsapplication/1428479-orderfrontstandardaboutpanelwith#discussion
#[allow(non_upper_case_globals)]
const NSAboutPanelOptionCopyright: &str = "Copyright";

define_class!(
    /// A delegate for NSMenu that stores the menu id as an instance variable,
    /// so that we can identify it later. Like when calling `set_as_windows_menu_for_nsapp`.
    #[unsafe(super(NSObject))]
    #[name = "MudaMenuDelegate"]
    #[thread_kind = MainThreadOnly]
    #[ivars = u32]
    struct MudaMenuDelegate;

    unsafe impl NSObjectProtocol for MudaMenuDelegate {}
    unsafe impl NSMenuDelegate for MudaMenuDelegate {}
);

impl MudaMenuDelegate {
    fn new(mtm: MainThreadMarker, menu_id: u32) -> Retained<Self> {
        let this = mtm.alloc().set_ivars(menu_id);
        unsafe { msg_send![super(this), init] }
    }

    fn menu_id(&self) -> u32 {
        *self.ivars()
    }
}

#[derive(Clone)]
struct NsMenuRef(
    u32,
    Retained<NSMenu>,
    /// Prevent deallocation — NSMenu's delegate is a weak reference.
    #[allow(dead_code)]
    Retained<MudaMenuDelegate>,
);

impl NsMenuRef {
    fn new(mtm: MainThreadMarker, id: u32, ns_menu: Retained<NSMenu>) -> Self {
        let delegate = MudaMenuDelegate::new(mtm, id);
        ns_menu.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
        Self(id, ns_menu, delegate)
    }
}

impl std::fmt::Debug for NsMenuRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("NsMenuRef")
            .field(&self.0)
            .field(&self.1)
            .finish()
    }
}

impl Drop for NsMenuRef {
    fn drop(&mut self) {
        self.1.cancelTrackingWithoutAnimation();
    }
}

pub struct PlatformMenu {
    ns_menu: NsMenuRef,
}

impl PlatformMenu {
    pub fn new() -> Self {
        let mtm =
            MainThreadMarker::new().expect("`muda::Menu` can only be created on the main thread");
        let ns_menu = NSMenu::new(mtm);
        ns_menu.setAutoenablesItems(false);
        Self {
            ns_menu: NsMenuRef::new(mtm, COUNTER.next(), ns_menu),
        }
    }

    pub fn attach(&mut self, item: &MenuItemKind, op: AddOp) -> crate::Result<()> {
        let ns_menu_item = item.create_ns(self.ns_menu.0)?;

        match op {
            AddOp::Append => self.ns_menu.1.addItem(&ns_menu_item),
            AddOp::Insert(position) => {
                self.ns_menu
                    .1
                    .insertItem_atIndex(&ns_menu_item, position as NSInteger);
            }
        }

        Ok(())
    }

    pub fn remove_at(&mut self, position: usize, item: &MenuItemKind) {
        let children = item.children();
        item.platform()
            .borrow_mut()
            .remove_instance_for_parent_at_position(&self.ns_menu, position, &children);
    }

    pub fn destroy(&mut self, children: &[MenuItemKind]) {
        remove_children_instances_for_parent(self.ns_menu.0, children);
    }

    pub fn init_for_nsapp(&self) {
        let mtm = MainThreadMarker::from(&*self.ns_menu.1);
        let app = NSApplication::sharedApplication(mtm);
        app.setMainMenu(Some(&self.ns_menu.1));
    }

    pub fn remove_for_nsapp(&self) {
        let mtm = MainThreadMarker::from(&*self.ns_menu.1);
        let app = NSApplication::sharedApplication(mtm);
        app.setMainMenu(None);
    }

    pub unsafe fn show_context_menu_for_nsview(
        &self,
        view: *const c_void,
        position: Option<Position>,
    ) -> bool {
        // SAFETY: Upheld by caller
        show_context_menu(&self.ns_menu.1, view, position)
    }

    pub fn ns_menu(&self) -> *mut std::ffi::c_void {
        Retained::as_ptr(&self.ns_menu.1) as _
    }
}

/// A generic child in a menu
pub struct PlatformMenuItem {
    click: ClickAction,
    ns_menu_items: HashMap<u32, Vec<Retained<NSMenuItem>>>,
    ns_menus: Option<HashMap<u32, Vec<NsMenuRef>>>,
    ns_menu: Option<NsMenuRef>,
}

/// Constructors
impl PlatformMenuItem {
    pub fn new(click: ClickAction) -> Self {
        Self {
            click,
            ns_menu: None,
            ns_menu_items: HashMap::new(),
            ns_menus: None,
        }
    }

    pub fn new_submenu(click: ClickAction) -> Self {
        let mtm = if cfg!(test) {
            unsafe { MainThreadMarker::new_unchecked() }
        } else {
            MainThreadMarker::new()
                .expect("`muda::PlatformMenuItem` can only be created on the main thread")
        };
        Self {
            click,
            ns_menu: Some({
                let menu = NSMenu::new(mtm);
                menu.setAutoenablesItems(false);
                NsMenuRef::new(mtm, COUNTER.next(), menu)
            }),
            ns_menu_items: HashMap::new(),
            ns_menus: Some(HashMap::new()),
        }
    }

    fn is_submenu(&self) -> bool {
        self.ns_menu.is_some()
    }

    pub fn destroy(&mut self, children: &[MenuItemKind]) {
        if !self.is_submenu() {
            return;
        }

        let menu_ids = self
            .ns_menus
            .as_ref()
            .unwrap()
            .values()
            .flatten()
            .map(|menu| menu.0)
            .chain(self.ns_menu.iter().map(|menu| menu.0))
            .collect::<Vec<_>>();

        for menu_id in menu_ids {
            remove_children_instances_for_parent(menu_id, children);
        }
    }
}

/// Shared methods
impl PlatformMenuItem {
    pub fn text(&self) -> Option<String> {
        self.ns_menu_items
            .values()
            .flat_map(|items| items.iter())
            .next()
            .map(|item| item.title().to_string())
    }

    pub fn set_text(&mut self, text: &str, _accelerator: Option<&MenuAccelerator>) {
        let title = NSString::from_str(&strip_mnemonic(text));
        for ns_items in self.ns_menu_items.values() {
            for ns_item in ns_items {
                ns_item.setTitle(&title);
                if let Some(submenu) = ns_item.submenu() {
                    submenu.setTitle(&title);
                }
            }
        }
    }

    pub fn is_enabled(&self) -> Option<bool> {
        self.ns_menu_items
            .values()
            .flat_map(|items| items.iter())
            .next()
            .map(|item| item.isEnabled())
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        for ns_items in self.ns_menu_items.values() {
            for ns_item in ns_items {
                ns_item.setEnabled(enabled);
            }
        }
    }

    pub fn set_accelerator(
        &mut self,
        _text: &str,
        accelerator: Option<&MenuAccelerator>,
    ) -> crate::Result<()> {
        let key_equivalent = accelerator
            .map(MenuAccelerator::key_equivalent)
            .transpose()?;

        if let Some(key_equivalent) = key_equivalent {
            let key_equivalent = NSString::from_str(key_equivalent.as_str());
            let modifier_mask = accelerator
                .map(MenuAccelerator::modifier_mask)
                .unwrap_or_else(NSEventModifierFlags::empty);

            for ns_items in self.ns_menu_items.values() {
                for ns_item in ns_items {
                    ns_item.setKeyEquivalent(&key_equivalent);
                    ns_item.setKeyEquivalentModifierMask(modifier_mask);
                }
            }
        }

        Ok(())
    }
}

/// CheckMenuItem methods
impl PlatformMenuItem {
    pub fn is_checked(&self) -> Option<bool> {
        self.ns_menu_items
            .values()
            .flat_map(|items| items.iter())
            .next()
            .map(|item| item.state() == NSControlStateValueOn)
    }

    pub fn set_checked(&mut self, checked: bool) {
        let state = if checked {
            NSControlStateValueOn
        } else {
            NSControlStateValueOff
        };
        for ns_items in self.ns_menu_items.values() {
            for ns_item in ns_items {
                ns_item.setState(state);
            }
        }
    }
}

/// IconMenuItem methods
impl PlatformMenuItem {
    pub fn set_icon(&mut self, icon: Option<&IconType>) {
        for ns_items in self.ns_menu_items.values() {
            for ns_item in ns_items {
                menuitem_set_icon_type(ns_item, icon);
            }
        }
    }
}

fn remove_children_instances_for_parent(parent_id: u32, children: &[MenuItemKind]) {
    for child in children {
        let descendants = child.children();
        child
            .platform()
            .borrow_mut()
            .remove_instances_for_parent(parent_id, &descendants);
    }
}

/// Submenu methods
impl PlatformMenuItem {
    pub fn attach(&mut self, item: &MenuItemKind, op: AddOp) -> crate::Result<()> {
        match op {
            AddOp::Append => {
                for menus in self.ns_menus.as_ref().unwrap().values() {
                    for ns_menu in menus {
                        let ns_menu_item = item.create_ns(ns_menu.0)?;
                        ns_menu.1.addItem(&ns_menu_item);
                    }
                }

                let ns_menu_item = item.create_ns(self.ns_menu.as_ref().unwrap().0)?;
                self.ns_menu.as_ref().unwrap().1.addItem(&ns_menu_item);
            }
            AddOp::Insert(position) => {
                for menus in self.ns_menus.as_ref().unwrap().values() {
                    for ns_menu in menus {
                        let ns_menu_item = item.create_ns(ns_menu.0)?;
                        ns_menu
                            .1
                            .insertItem_atIndex(&ns_menu_item, position as NSInteger);
                    }
                }

                let ns_menu_item = item.create_ns(self.ns_menu.as_ref().unwrap().0)?;
                self.ns_menu
                    .as_ref()
                    .unwrap()
                    .1
                    .insertItem_atIndex(&ns_menu_item, position as NSInteger);
            }
        }

        Ok(())
    }

    pub fn remove_at(&mut self, position: usize, item: &MenuItemKind) {
        let children = item.children();
        let child = item.platform();

        //  Join the ns_menus and ns_menu into a single iterator of parent menus to remove the child from
        let ns_menus = self.ns_menus.as_ref().unwrap();
        let ns_menus = ns_menus.values().flatten().cloned();
        let parent_menus = ns_menus.chain(self.ns_menu.iter().cloned());

        for parent_menu in parent_menus {
            let mut child = child.borrow_mut();
            child.remove_instance_for_parent_at_position(&parent_menu, position, &children);
        }
    }

    fn remove_instance_for_parent_at_position(
        &mut self,
        parent_menu: &NsMenuRef,
        position: usize,
        children: &[MenuItemKind],
    ) {
        let Some(ns_item) = parent_menu.1.itemAtIndex(position as NSInteger) else {
            return;
        };

        if self.is_submenu() {
            self.remove_ns_menu_for_parent_item(parent_menu.0, &ns_item, children);
        }

        self.remove_ns_menu_item_for_parent(parent_menu.0, &ns_item);
        parent_menu.1.removeItemAtIndex(position as NSInteger);
    }

    fn remove_ns_menu_for_parent_item(
        &mut self,
        parent_id: u32,
        ns_item: &NSMenuItem,
        children: &[MenuItemKind],
    ) {
        let Some(ns_submenu) = ns_item.submenu() else {
            return;
        };
        let Some(menus) = self.ns_menus.as_mut().unwrap().get_mut(&parent_id) else {
            return;
        };
        let Some(index) = menus.iter().position(|menu| {
            std::ptr::eq(Retained::as_ptr(&menu.1), Retained::as_ptr(&ns_submenu))
        }) else {
            return;
        };

        let removed = menus.remove(index);

        if menus.is_empty() {
            self.ns_menus.as_mut().unwrap().remove(&parent_id);
        }

        remove_children_instances_for_parent(removed.0, children);
    }

    fn remove_instances_for_parent(&mut self, parent_id: u32, children: &[MenuItemKind]) {
        self.ns_menu_items.remove(&parent_id);

        if !self.is_submenu() {
            return;
        }

        if let Some(menus) = self.ns_menus.as_mut().unwrap().remove(&parent_id) {
            for menu in menus {
                remove_children_instances_for_parent(menu.0, children);
            }
        }
    }

    fn remove_ns_menu_item_for_parent(&mut self, parent_id: u32, ns_item: &NSMenuItem) {
        let Some(items) = self.ns_menu_items.get_mut(&parent_id) else {
            return;
        };

        if let Some(index) = items
            .iter()
            .position(|item| std::ptr::eq(Retained::as_ptr(item), ns_item))
        {
            items.remove(index);
        }

        if items.is_empty() {
            self.ns_menu_items.remove(&parent_id);
        }
    }

    pub unsafe fn show_context_menu_for_nsview(
        &self,
        view: *const c_void,
        position: Option<Position>,
    ) -> bool {
        show_context_menu(&self.ns_menu.as_ref().unwrap().1, view, position)
    }

    pub fn set_as_windows_menu_for_nsapp(&self) {
        let Some(menu) = self.resolve_ns_menu_for_nsapp() else {
            return;
        };

        let mtm = MainThreadMarker::from(&*menu);
        let app = NSApplication::sharedApplication(mtm);
        app.setWindowsMenu(Some(&menu))
    }

    pub fn set_as_help_menu_for_nsapp(&self) {
        let Some(menu) = self.resolve_ns_menu_for_nsapp() else {
            return;
        };

        let mtm = MainThreadMarker::from(&*menu);
        let app = NSApplication::sharedApplication(mtm);
        app.setHelpMenu(Some(&menu))
    }

    /// Finds the NSMenu instance for this submenu that is attached to the
    /// current NSApp main menu, by reading the menu id stored in the
    /// main menu's delegate.
    fn resolve_ns_menu_for_nsapp(&self) -> Option<Retained<NSMenu>> {
        let ns_menu = &self.ns_menu.as_ref().unwrap().1;
        let mtm = MainThreadMarker::from(&**ns_menu);
        let app = NSApplication::sharedApplication(mtm);
        let main_menu = app.mainMenu()?;
        let delegate = main_menu.delegate()?;

        // Downcast the delegate to our MudaMenuDelegate to get the menu id
        let delegate_obj: &AnyObject = ProtocolObject::as_ref(&*delegate);
        let muda_delegate: &MudaMenuDelegate = delegate_obj.downcast_ref()?;
        let parent_id = muda_delegate.menu_id();

        // Look up the NSMenu in ns_menus for this parent id
        self.ns_menus
            .as_ref()
            .unwrap()
            .get(&parent_id)
            // A submenu can be added multiple times to the same parent menu
            // lets just take the first one we find
            .and_then(|menus| menus.first())
            .map(|menu_ref| menu_ref.1.clone())
    }

    pub fn ns_menu(&self) -> *mut std::ffi::c_void {
        Retained::as_ptr(&self.ns_menu.as_ref().unwrap().1) as *mut _
    }
}

/// NSMenuItem item creation methods
impl PlatformMenuItem {
    fn create_ns_submenu(
        &mut self,
        args: &PlatformAttachArgs,
        children: &[MenuItemKind],
        menu_id: u32,
    ) -> crate::Result<Retained<NSMenuItem>> {
        let mtm = MainThreadMarker::new().expect("can only create menu item on the main thread");
        let ns_menu_item;
        let ns_submenu;

        let title = NSString::from_str(&strip_mnemonic(&args.text));
        unsafe {
            ns_menu_item = NSMenuItem::initWithTitle_action_keyEquivalent(
                mtm.alloc(),
                &title,
                None,
                &NSString::new(),
            );
            ns_submenu = NSMenu::new(mtm);
            ns_submenu.setTitle(&title);

            ns_menu_item.setSubmenu(Some(&ns_submenu));
            ns_submenu.setAutoenablesItems(false);

            ns_menu_item.setEnabled(args.enabled);
            menuitem_set_icon_type(&ns_menu_item, args.icon.as_ref());
        }

        let id = COUNTER.next();

        for item in children {
            let ns_item = item.create_ns(id)?;
            ns_submenu.addItem(&ns_item);
        }

        self.ns_menus
            .as_mut()
            .unwrap()
            .entry(menu_id)
            .or_default()
            .push(NsMenuRef::new(mtm, id, ns_submenu));

        self.ns_menu_items
            .entry(menu_id)
            .or_default()
            .push(ns_menu_item.retain());

        Ok(ns_menu_item)
    }

    fn create_ns_item(
        &mut self,
        args: &PlatformAttachArgs,
        owner: Rc<RefCell<PlatformMenuItem>>,
        menu_id: u32,
    ) -> crate::Result<Retained<NSMenuItem>> {
        let mtm = MainThreadMarker::new().expect("can only create menu item on the main thread");
        let ns_menu_item = NsMenuItem::create(
            mtm,
            &args.text,
            Some(sel!(customAction:)),
            &args.accelerator,
        )?;

        unsafe {
            ns_menu_item.setTarget(Some(&ns_menu_item));
            ns_menu_item.setEnabled(args.enabled);
        }

        ns_menu_item.ivars().replace(Some(owner));

        self.ns_menu_items
            .entry(menu_id)
            .or_default()
            .push(Retained::into_super(ns_menu_item.retain()));

        Ok(Retained::into_super(ns_menu_item))
    }

    fn create_ns_predefined_item(
        &mut self,
        args: &PlatformAttachArgs,
        predefined_item_type: PredefinedMenuItemType,
        owner: Rc<RefCell<PlatformMenuItem>>,
        menu_id: u32,
    ) -> crate::Result<Retained<NSMenuItem>> {
        let mtm = MainThreadMarker::new().expect("can only create menu item on the main thread");
        let ns_menu_item = match &predefined_item_type {
            PredefinedMenuItemType::Separator => NSMenuItem::separatorItem(mtm),
            _ => {
                let selector = predefined_item_type.selector();
                let ns_menu_item =
                    NsMenuItem::create(mtm, &args.text, selector, &args.accelerator)?;

                if let PredefinedMenuItemType::About(_) = &predefined_item_type {
                    unsafe { ns_menu_item.setTarget(Some(&ns_menu_item)) };
                    ns_menu_item.ivars().set(Some(owner));
                }

                Retained::into_super(ns_menu_item)
            }
        };

        ns_menu_item.setEnabled(args.enabled);

        if let PredefinedMenuItemType::Services = &predefined_item_type {
            // we have to assign an empty menu as the app's services menu, and macOS will populate it
            let services_menu = NSMenu::new(mtm);
            NSApplication::sharedApplication(mtm).setServicesMenu(Some(&services_menu));
            ns_menu_item.setSubmenu(Some(&services_menu));
        }

        self.ns_menu_items
            .entry(menu_id)
            .or_default()
            .push(ns_menu_item.retain());

        Ok(ns_menu_item)
    }

    fn create_ns_check_item(
        &mut self,
        args: &PlatformAttachArgs,
        owner: Rc<RefCell<PlatformMenuItem>>,
        menu_id: u32,
    ) -> crate::Result<Retained<NSMenuItem>> {
        let mtm = MainThreadMarker::new().expect("can only create menu item on the main thread");
        let ns_menu_item = NsMenuItem::create(
            mtm,
            &args.text,
            Some(sel!(customAction:)),
            &args.accelerator,
        )?;

        unsafe {
            ns_menu_item.setTarget(Some(&ns_menu_item));
            ns_menu_item.setEnabled(args.enabled);
            if args.checked {
                ns_menu_item.setState(NSControlStateValueOn);
            }
        }

        ns_menu_item.ivars().replace(Some(owner));

        self.ns_menu_items
            .entry(menu_id)
            .or_default()
            .push(Retained::into_super(ns_menu_item.retain()));

        Ok(Retained::into_super(ns_menu_item))
    }

    fn create_ns_icon_item(
        &mut self,
        args: &PlatformAttachArgs,
        owner: Rc<RefCell<PlatformMenuItem>>,
        menu_id: u32,
    ) -> crate::Result<Retained<NSMenuItem>> {
        let mtm = MainThreadMarker::new().expect("can only create menu item on the main thread");
        let ns_menu_item = NsMenuItem::create(
            mtm,
            &args.text,
            Some(sel!(customAction:)),
            &args.accelerator,
        )?;

        unsafe {
            ns_menu_item.setTarget(Some(&ns_menu_item));
            ns_menu_item.setEnabled(args.enabled);
            menuitem_set_icon_type(&ns_menu_item, args.icon.as_ref());
        }

        ns_menu_item.ivars().replace(Some(owner));

        self.ns_menu_items
            .entry(menu_id)
            .or_default()
            .push(Retained::into_super(ns_menu_item.retain()));

        Ok(Retained::into_super(ns_menu_item))
    }
}

impl NsMenuItem {
    fn action(&self) {
        // SAFETY: The ivar is initialized before the menu item is exposed and is
        // never mutated afterward.
        let item = unsafe { &*self.ivars().as_ptr() };
        let item = item.as_ref().expect("PlatformMenuItem pointer was unset");
        let click = item.borrow().click.clone();

        match click {
            ClickAction::Emit(id) => MenuEvent::send(MenuEvent { id }),
            ClickAction::Toggle(id, state) => {
                if let Some(state) = state.upgrade() {
                    let checked = {
                        let mut state = state.borrow_mut();
                        state.checked = !state.checked;
                        state.checked
                    };
                    item.borrow_mut().set_checked(checked);
                }
                MenuEvent::send(MenuEvent { id });
            }
            ClickAction::Predefined(_) => {
                unreachable!("predefined menu item used the generic click action")
            }
        }
    }

    fn show_about_panel(&self) {
        // SAFETY: The ivar is initialized before the menu item is exposed and is
        // never mutated afterward.
        let item = unsafe { &*self.ivars().as_ptr() };
        let item = item.as_ref().expect("PlatformMenuItem pointer was unset");
        let click = item.borrow().click.clone();

        let ClickAction::Predefined(state) = click else {
            unreachable!("About menu item without predefined action");
        };
        let item_type = state
            .upgrade()
            .map(|state| state.borrow().predefined_item_type.clone());
        let Some(PredefinedMenuItemType::About(about_meta)) = item_type else {
            return;
        };

        let mtm = MainThreadMarker::from(self);
        let Some(about_meta) = about_meta else {
            NSApplication::sharedApplication(mtm).orderFrontStandardAboutPanel(Some(self));
            return;
        };

        let mut keys: Vec<&NSString> = Default::default();
        let mut objects: Vec<Retained<AnyObject>> = Default::default();

        if let Some(name) = &about_meta.name {
            keys.push(unsafe { NSAboutPanelOptionApplicationName });
            objects.push(Retained::into_super(Retained::into_super(
                NSString::from_str(name),
            )));
        }

        if let Some(version) = &about_meta.version {
            keys.push(unsafe { NSAboutPanelOptionApplicationVersion });
            objects.push(Retained::into_super(Retained::into_super(
                NSString::from_str(version),
            )));
        }

        if let Some(short_version) = &about_meta.short_version {
            keys.push(unsafe { NSAboutPanelOptionVersion });
            objects.push(Retained::into_super(Retained::into_super(
                NSString::from_str(short_version),
            )));
        }

        if let Some(copyright) = &about_meta.copyright {
            keys.push(ns_string!(NSAboutPanelOptionCopyright));
            objects.push(Retained::into_super(Retained::into_super(
                NSString::from_str(copyright),
            )));
        }

        if let Some(icon) = &about_meta.icon {
            keys.push(unsafe { NSAboutPanelOptionApplicationIcon });
            objects.push(Retained::into_super(Retained::into_super(
                icon.inner.to_nsimage(None),
            )));
        }

        if let Some(credits) = &about_meta.credits {
            keys.push(unsafe { NSAboutPanelOptionCredits });
            objects.push(Retained::into_super(Retained::into_super(
                NSAttributedString::from_nsstring(&NSString::from_str(credits)),
            )));
        }

        let dict = NSDictionary::from_retained_objects(&keys, &objects);

        unsafe {
            NSApplication::sharedApplication(mtm).orderFrontStandardAboutPanelWithOptions(&dict)
        };
    }
}

impl PredefinedMenuItemType {
    pub(crate) fn selector(&self) -> Option<Sel> {
        match self {
            PredefinedMenuItemType::Separator => None,
            PredefinedMenuItemType::Copy => Some(sel!(copy:)),
            PredefinedMenuItemType::Cut => Some(sel!(cut:)),
            PredefinedMenuItemType::Paste => Some(sel!(paste:)),
            PredefinedMenuItemType::PasteAndMatchStyle => Some(sel!(pasteAsPlainText:)),
            PredefinedMenuItemType::Delete => Some(sel!(delete:)),
            PredefinedMenuItemType::SelectAll => Some(sel!(selectAll:)),
            PredefinedMenuItemType::Undo => Some(sel!(undo:)),
            PredefinedMenuItemType::Redo => Some(sel!(redo:)),
            PredefinedMenuItemType::Minimize => Some(sel!(performMiniaturize:)),
            PredefinedMenuItemType::Maximize => Some(sel!(performZoom:)),
            PredefinedMenuItemType::ActualSize => Some(sel!(actualSize:)),
            PredefinedMenuItemType::ZoomIn => Some(sel!(zoomIn:)),
            PredefinedMenuItemType::ZoomOut => Some(sel!(zoomOut:)),
            PredefinedMenuItemType::Fullscreen => Some(sel!(toggleFullScreen:)),
            PredefinedMenuItemType::Hide => Some(sel!(hide:)),
            PredefinedMenuItemType::HideOthers => Some(sel!(hideOtherApplications:)),
            PredefinedMenuItemType::ShowAll => Some(sel!(unhideAllApplications:)),
            PredefinedMenuItemType::CloseWindow => Some(sel!(performClose:)),
            PredefinedMenuItemType::Quit => Some(sel!(terminate:)),
            PredefinedMenuItemType::About(_) => Some(sel!(customShowAboutPanel:)),
            PredefinedMenuItemType::Services => None,
            PredefinedMenuItemType::BringAllToFront => Some(sel!(arrangeInFront:)),
            PredefinedMenuItemType::StartSpeaking => Some(sel!(startSpeaking:)),
            PredefinedMenuItemType::StopSpeaking => Some(sel!(stopSpeaking:)),
            PredefinedMenuItemType::StartDictation => Some(sel!(startDictation:)),
            PredefinedMenuItemType::EmojiAndSymbols => Some(sel!(orderFrontCharacterPalette:)),
        }
    }
}

impl MenuItemKind {
    fn create_ns(&self, menu_id: u32) -> crate::Result<Retained<NSMenuItem>> {
        let args = self.platform_attach_args();
        let platform = self.platform();
        let mut item = platform.borrow_mut();

        match self {
            MenuItemKind::Submenu(_) => item.create_ns_submenu(&args, &self.children(), menu_id),
            MenuItemKind::MenuItem(_) => item.create_ns_item(&args, platform.clone(), menu_id),
            MenuItemKind::Predefined(i) => {
                let predefined_item_type = i.state.borrow().predefined_item_type.clone();
                item.create_ns_predefined_item(
                    &args,
                    predefined_item_type,
                    platform.clone(),
                    menu_id,
                )
            }
            MenuItemKind::Check(_) => item.create_ns_check_item(&args, platform.clone(), menu_id),
            MenuItemKind::Icon(_) => item.create_ns_icon_item(&args, platform.clone(), menu_id),
        }
    }
}

fn menuitem_set_icon_type(menuitem: &NSMenuItem, icon: Option<&IconType>) {
    match icon {
        Some(IconType::Custom(icon)) => menuitem_set_icon(menuitem, Some(icon)),
        Some(IconType::Native(icon)) => menuitem_set_native_icon(menuitem, Some(icon)),
        None => menuitem.setImage(None),
    }
}

fn menuitem_set_icon(menuitem: &NSMenuItem, icon: Option<&Icon>) {
    if let Some(icon) = icon {
        let nsimage = icon.inner.to_nsimage(Some(18.));
        menuitem.setImage(Some(&nsimage));
    } else {
        menuitem.setImage(None);
    }
}

fn menuitem_set_native_icon(menuitem: &NSMenuItem, icon: Option<&NativeIcon>) {
    let Some(icon) = icon else {
        menuitem.setImage(None);
        return;
    };

    let nsimage = match icon {
        NativeIcon::Raw(name) => {
            let named_img = NSString::from_str(name);
            NSImage::imageNamed(&named_img)
        }
        _ => unsafe { NSImage::imageNamed(icon.named_img()) },
    };

    if let Some(nsimage) = nsimage {
        let size = NSSize::new(18.0, 18.0);
        nsimage.setSize(size);
        menuitem.setImage(Some(&nsimage));
    } else {
        menuitem.setImage(None);
    }
}

impl NativeIcon {
    unsafe fn named_img(&self) -> &'static NSImageName {
        use objc2_app_kit as appkit;
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
            NativeIcon::Raw(_) => unreachable!("raw native icons are handled before named_img"),
        }
    }
}

unsafe fn show_context_menu(
    ns_menu: &NSMenu,
    view: *const c_void,
    position: Option<Position>,
) -> bool {
    // SAFETY: Caller verifies that the view is valid.
    let view: &NSView = unsafe { &*view.cast() };

    let window = view.window().expect("view must be installed in a window");
    let scale_factor = window.backingScaleFactor();
    let (location, in_view) = if let Some(pos) = position.map(|p| p.to_logical(scale_factor)) {
        let view_rect = view.frame();
        let location = NSPoint::new(pos.x, view_rect.size.height - pos.y);
        (location, Some(view))
    } else {
        let mouse_location = NSEvent::mouseLocation();
        let pos = LogicalPosition {
            x: mouse_location.x,
            y: mouse_location.y,
        };
        let location = NSPoint::new(pos.x, pos.y);
        (location, None)
    };

    ns_menu.popUpMenuPositioningItem_atLocation_inView(None, location, in_view)
}
