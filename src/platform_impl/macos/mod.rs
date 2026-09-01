// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

mod icon;
mod util;

pub(crate) use icon::PlatformIcon;

use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    ffi::c_void,
    rc::Rc,
};

use objc2::{
    define_class, msg_send,
    rc::Retained,
    runtime::{AnyObject, NSObjectProtocol, ProtocolObject, Sel},
    sel, AnyThread, DeclaredClass, MainThreadOnly, Message,
};
use objc2_app_kit::{
    NSAboutPanelOptionApplicationIcon, NSAboutPanelOptionApplicationName,
    NSAboutPanelOptionApplicationVersion, NSAboutPanelOptionCredits, NSAboutPanelOptionVersion,
    NSApplication, NSControlStateValueOff, NSControlStateValueOn, NSEvent, NSEventModifierFlags,
    NSFont, NSFontAttributeName, NSForegroundColorAttributeName, NSImage, NSImageName, NSMenu,
    NSMenuDelegate, NSMenuItem, NSRunningApplication, NSView, NSWindow,
};
use objc2_foundation::{
    ns_string, MainThreadMarker, NSAttributedString, NSDictionary, NSInteger, NSObject, NSPoint,
    NSRect, NSSize, NSString,
};

use self::util::strip_mnemonic;
use crate::{
    accelerator::MenuAccelerator,
    dpi::{LogicalPosition, Position},
    icon::Icon,
    items::*,
    util::{AddOp, Counter},
    IsMenuItem, MenuEvent, MenuId, MenuItemKind, MenuItemType, NativeIcon,
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

#[derive(Debug)]
pub struct Menu {
    id: MenuId,
    ns_menu: NsMenuRef,
    children: Vec<Rc<RefCell<MenuChild>>>,
}

impl Drop for Menu {
    fn drop(&mut self) {
        for child in &self.children {
            let mut child_ = child.borrow_mut();
            child_.ns_menu_items.remove(&self.ns_menu.0);
            if child_.item_type == MenuItemType::Submenu {
                child_.ns_menus.as_mut().unwrap().remove(&self.ns_menu.0);
            }
        }
    }
}

impl Menu {
    pub fn new(id: Option<MenuId>) -> Self {
        let mtm =
            MainThreadMarker::new().expect("`muda::Menu` can only be created on the main thread");
        let ns_menu = NSMenu::new(mtm);
        ns_menu.setAutoenablesItems(false);
        Self {
            id: id.unwrap_or_else(|| MenuId(COUNTER.next().to_string())),
            ns_menu: NsMenuRef::new(mtm, COUNTER.next(), ns_menu),
            children: Vec::new(),
        }
    }

    pub fn id(&self) -> &MenuId {
        &self.id
    }

    pub fn add_menu_item(&mut self, item: &dyn crate::IsMenuItem, op: AddOp) -> crate::Result<()> {
        let ns_menu_item = item.make_ns_item_for_menu(self.ns_menu.0)?;
        let child = item.child();

        match op {
            AddOp::Append => {
                self.ns_menu.1.addItem(&ns_menu_item);
                self.children.push(child);
            }
            AddOp::Insert(position) => {
                self.ns_menu
                    .1
                    .insertItem_atIndex(&ns_menu_item, position as NSInteger);
                self.children.insert(position, child);
            }
        }

        Ok(())
    }

    pub fn remove(&mut self, item: &dyn crate::IsMenuItem) -> crate::Result<()> {
        let child = item.child();
        let positions = self
            .children
            .iter()
            .enumerate()
            .filter_map(|(index, current)| Rc::ptr_eq(current, &child).then_some(index))
            .collect::<Vec<_>>();

        if positions.is_empty() {
            return Err(crate::Error::NotAChildOfThisMenu);
        }

        for position in positions.into_iter().rev() {
            self.remove_at(position);
        }

        Ok(())
    }

    pub fn remove_at(&mut self, position: usize) -> Option<MenuItemKind> {
        if position >= self.children.len() {
            return None;
        }

        let child = self.children.remove(position);
        let item = child.borrow_mut().kind(child.clone());

        child
            .borrow_mut()
            .remove_instance_for_parent_at_position(&self.ns_menu, position);

        Some(item)
    }

    pub fn items(&self) -> Vec<MenuItemKind> {
        self.children
            .iter()
            .map(|c| c.borrow().kind(c.clone()))
            .collect()
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
#[derive(Debug, Default)]
pub struct MenuChild {
    // shared fields between submenus and menu items
    item_type: MenuItemType,
    id: MenuId,
    text: String,
    enabled: bool,

    ns_menu_items: HashMap<u32, Vec<Retained<NSMenuItem>>>,

    /// Set by `set_styled_text`. muda creates one `NSMenuItem` per menu an item is
    /// attached to, lazily, so the parts are kept here and re-applied at creation time.
    styled_text: Option<Vec<(String, TextStyle)>>,

    // menu item fields
    accelerator: Option<MenuAccelerator>,

    // predefined menu item fields
    predefined_item_type: Option<PredefinedMenuItemType>,

    // check menu item fields
    checked: Cell<bool>,

    // icon menu item fields
    icon: Option<Icon>,
    native_icon: Option<NativeIcon>,

    // submenu fields
    pub children: Option<Vec<Rc<RefCell<MenuChild>>>>,
    ns_menus: Option<HashMap<u32, Vec<NsMenuRef>>>,
    ns_menu: Option<NsMenuRef>,
}

impl Drop for MenuChild {
    fn drop(&mut self) {
        fn drop_children(id: u32, children: &Vec<Rc<RefCell<MenuChild>>>) {
            for child in children {
                let mut child_ = child.borrow_mut();
                child_.ns_menu_items.remove(&id);

                if child_.item_type == MenuItemType::Submenu {
                    if let Some(menus) = child_.ns_menus.as_mut().unwrap().remove(&id) {
                        for menu in menus {
                            drop_children(menu.0, child_.children.as_ref().unwrap());
                        }
                    }
                }
            }
        }

        if self.item_type == MenuItemType::Submenu {
            for menus in self.ns_menus.as_ref().unwrap().values() {
                for menu in menus {
                    drop_children(menu.0, self.children.as_ref().unwrap())
                }
            }

            if let Some(menu) = &self.ns_menu {
                drop_children(menu.0, self.children.as_ref().unwrap());
            }
        }
    }
}

/// Constructors
impl MenuChild {
    pub fn new(
        text: &str,
        enabled: bool,
        accelerator: Option<MenuAccelerator>,
        id: Option<MenuId>,
    ) -> Self {
        Self {
            item_type: MenuItemType::MenuItem,
            text: strip_mnemonic(text),
            enabled,
            id: id.unwrap_or_else(|| MenuId(COUNTER.next().to_string())),
            accelerator,
            checked: Cell::new(false),
            children: None,
            icon: None,
            native_icon: None,
            ns_menu: None,
            ns_menu_items: HashMap::new(),
            styled_text: None,
            ns_menus: None,
            predefined_item_type: None,
        }
    }

    pub fn new_submenu(text: &str, enabled: bool, id: Option<MenuId>) -> Self {
        let mtm = if cfg!(test) {
            unsafe { MainThreadMarker::new_unchecked() }
        } else {
            MainThreadMarker::new()
                .expect("`muda::MenuChild` can only be created on the main thread")
        };
        Self {
            item_type: MenuItemType::Submenu,
            text: strip_mnemonic(text),
            id: id.unwrap_or_else(|| MenuId(COUNTER.next().to_string())),
            enabled,
            children: Some(Vec::new()),
            ns_menu: Some({
                let menu = NSMenu::new(mtm);
                menu.setAutoenablesItems(false);
                NsMenuRef::new(mtm, COUNTER.next(), menu)
            }),
            accelerator: None,
            checked: Cell::new(false),
            icon: None,
            native_icon: None,
            ns_menu_items: HashMap::new(),
            styled_text: None,
            ns_menus: Some(HashMap::new()),
            predefined_item_type: None,
        }
    }

    pub(crate) fn new_predefined(item_type: PredefinedMenuItemType, text: Option<String>) -> Self {
        let enabled = item_type.is_supported_on_macos();
        let text = strip_mnemonic(text.unwrap_or_else(|| {
            // Gets the app's name from `NSRunningApplication::localizedName`.
            let app_name = || {
                let app = NSRunningApplication::currentApplication();
                app.localizedName().unwrap_or_default()
            };

            match item_type {
                PredefinedMenuItemType::About(_) => {
                    format!("About {}", app_name()).trim().to_string()
                }
                PredefinedMenuItemType::Hide => format!("Hide {}", app_name()).trim().to_string(),
                PredefinedMenuItemType::Quit => format!("Quit {}", app_name()).trim().to_string(),
                _ => item_type.text().to_string(),
            }
        }));

        Self {
            item_type: MenuItemType::Predefined,
            text,
            enabled,
            id: MenuId(COUNTER.next().to_string()),
            accelerator: item_type.accelerator(),
            predefined_item_type: Some(item_type),
            checked: Cell::new(false),
            children: None,
            icon: None,
            native_icon: None,
            ns_menu: None,
            ns_menu_items: HashMap::new(),
            styled_text: None,
            ns_menus: None,
        }
    }

    pub fn new_check(
        text: &str,
        enabled: bool,
        checked: bool,
        accelerator: Option<MenuAccelerator>,
        id: Option<MenuId>,
    ) -> Self {
        Self {
            item_type: MenuItemType::Check,
            text: text.to_string(),
            enabled,
            id: id.unwrap_or_else(|| MenuId(COUNTER.next().to_string())),
            accelerator,
            checked: Cell::new(checked),
            children: None,
            icon: None,
            native_icon: None,
            ns_menu: None,
            ns_menu_items: HashMap::new(),
            styled_text: None,
            ns_menus: None,
            predefined_item_type: None,
        }
    }

    pub fn new_icon(
        text: &str,
        enabled: bool,
        icon: Option<Icon>,
        accelerator: Option<MenuAccelerator>,
        id: Option<MenuId>,
    ) -> Self {
        Self {
            item_type: MenuItemType::Icon,
            text: text.to_string(),
            enabled,
            id: id.unwrap_or_else(|| MenuId(COUNTER.next().to_string())),
            icon,
            accelerator,
            checked: Cell::new(false),
            children: None,
            native_icon: None,
            ns_menu: None,
            ns_menu_items: HashMap::new(),
            styled_text: None,
            ns_menus: None,
            predefined_item_type: None,
        }
    }

    pub fn new_native_icon(
        text: &str,
        enabled: bool,
        native_icon: Option<NativeIcon>,
        accelerator: Option<MenuAccelerator>,
        id: Option<MenuId>,
    ) -> Self {
        Self {
            item_type: MenuItemType::Icon,
            text: text.to_string(),
            enabled,
            id: id.unwrap_or_else(|| MenuId(COUNTER.next().to_string())),
            native_icon,
            accelerator,
            checked: Cell::new(false),
            children: None,
            icon: None,
            ns_menu: None,
            ns_menu_items: HashMap::new(),
            styled_text: None,
            ns_menus: None,
            predefined_item_type: None,
        }
    }
}

/// Shared methods
impl MenuChild {
    pub(crate) fn item_type(&self) -> MenuItemType {
        self.item_type
    }

    pub fn id(&self) -> &MenuId {
        &self.id
    }

    pub fn text(&self) -> String {
        self.text.clone()
    }

    pub fn set_text(&mut self, text: &str) {
        self.text = strip_mnemonic(text);

        self.styled_text = None;

        let title = NSString::from_str(&self.text);
        for ns_items in self.ns_menu_items.values() {
            for ns_item in ns_items {
                ns_item.setAttributedTitle(None);
                ns_item.setTitle(&title);
                if let Some(submenu) = ns_item.submenu() {
                    submenu.setTitle(&title);
                }
            }
        }
    }

    pub fn set_styled_text<S: AsRef<str>>(
        &mut self,
        parts: impl IntoIterator<Item = (S, TextStyle)>,
    ) {
        let parts: Vec<(String, TextStyle)> = parts
            .into_iter()
            .map(|(text, style)| (strip_mnemonic(text.as_ref()), style))
            .collect();

        self.text = parts
            .iter()
            .map(|(text, _)| text.as_str())
            .collect::<String>();

        self.styled_text = Some(parts);

        let title = NSString::from_str(&self.text);
        let attributed = self.styled_text.as_deref().map(build_attributed_title);
        for ns_items in self.ns_menu_items.values() {
            for ns_item in ns_items {
                ns_item.setAttributedTitle(attributed.as_deref());
                ns_item.setTitle(&title);
                if let Some(submenu) = ns_item.submenu() {
                    submenu.setTitle(&title);
                }
            }
        }
    }

    fn apply_styled_text_if_any(&self, ns_menu_item: &NSMenuItem) {
        if let Some(parts) = self.styled_text.as_deref() {
            ns_menu_item.setAttributedTitle(Some(&build_attributed_title(parts)));
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        for ns_items in self.ns_menu_items.values() {
            for ns_item in ns_items {
                ns_item.setEnabled(enabled);
            }
        }
    }

    pub fn set_accelerator(&mut self, accelerator: Option<MenuAccelerator>) -> crate::Result<()> {
        let key_equivalent = accelerator
            .as_ref()
            .map(MenuAccelerator::key_equivalent)
            .transpose()?;

        if let Some(key_equivalent) = key_equivalent {
            let key_equivalent = NSString::from_str(key_equivalent.as_str());

            let modifier_mask = accelerator
                .as_ref()
                .map(MenuAccelerator::modifier_mask)
                .unwrap_or_else(NSEventModifierFlags::empty);

            for ns_items in self.ns_menu_items.values() {
                for ns_item in ns_items {
                    ns_item.setKeyEquivalent(&key_equivalent);
                    ns_item.setKeyEquivalentModifierMask(modifier_mask);
                }
            }
        }

        self.accelerator = accelerator;

        Ok(())
    }
}

/// CheckMenuItem methods
impl MenuChild {
    pub fn is_checked(&self) -> bool {
        self.checked.get()
    }

    pub fn set_checked(&self, checked: bool) {
        self.checked.set(checked);
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
impl MenuChild {
    pub fn set_icon(&mut self, icon: Option<Icon>) {
        self.icon.clone_from(&icon);
        self.native_icon = None;
        for ns_items in self.ns_menu_items.values() {
            for ns_item in ns_items {
                menuitem_set_icon(ns_item, icon.as_ref());
            }
        }
    }

    pub fn set_native_icon(&mut self, icon: Option<NativeIcon>) {
        self.native_icon = icon;
        self.icon = None;
        for ns_items in self.ns_menu_items.values() {
            for ns_item in ns_items {
                menuitem_set_native_icon(ns_item, self.native_icon.as_ref());
            }
        }
    }
}

/// Submenu methods
impl MenuChild {
    pub fn add_menu_item(&mut self, item: &dyn crate::IsMenuItem, op: AddOp) -> crate::Result<()> {
        let child = item.child();

        match op {
            AddOp::Append => {
                for menus in self.ns_menus.as_ref().unwrap().values() {
                    for ns_menu in menus {
                        let ns_menu_item = item.make_ns_item_for_menu(ns_menu.0)?;
                        ns_menu.1.addItem(&ns_menu_item);
                    }
                }

                let ns_menu_item = item.make_ns_item_for_menu(self.ns_menu.as_ref().unwrap().0)?;
                self.ns_menu.as_ref().unwrap().1.addItem(&ns_menu_item);

                self.children.as_mut().unwrap().push(child);
            }
            AddOp::Insert(position) => {
                for menus in self.ns_menus.as_ref().unwrap().values() {
                    for ns_menu in menus {
                        let ns_menu_item = item.make_ns_item_for_menu(ns_menu.0)?;
                        ns_menu
                            .1
                            .insertItem_atIndex(&ns_menu_item, position as NSInteger);
                    }
                }

                let ns_menu_item = item.make_ns_item_for_menu(self.ns_menu.as_ref().unwrap().0)?;
                self.ns_menu
                    .as_ref()
                    .unwrap()
                    .1
                    .insertItem_atIndex(&ns_menu_item, position as NSInteger);

                self.children.as_mut().unwrap().insert(position, child);
            }
        }

        Ok(())
    }

    pub fn remove(&mut self, item: &dyn crate::IsMenuItem) -> crate::Result<()> {
        let child = item.child();
        let children = self.children.as_ref().unwrap();
        let positions = children
            .iter()
            .enumerate()
            .filter_map(|(index, current)| Rc::ptr_eq(current, &child).then_some(index))
            .collect::<Vec<_>>();

        if positions.is_empty() {
            return Err(crate::Error::NotAChildOfThisMenu);
        }

        for position in positions.into_iter().rev() {
            self.remove_at(position);
        }

        Ok(())
    }

    pub fn remove_at(&mut self, position: usize) -> Option<MenuItemKind> {
        let children = self.children.as_mut().unwrap();
        if position >= children.len() {
            return None;
        }

        let child = children.remove(position);
        let item = child.borrow().kind(child.clone());

        //  Join the ns_menus and ns_menu into a single iterator of parent menus to remove the child from
        let ns_menus = self.ns_menus.as_ref().unwrap();
        let ns_menus = ns_menus.values().flatten().cloned();
        let parent_menus = ns_menus.chain(self.ns_menu.iter().cloned());

        for parent_menu in parent_menus {
            let mut child = child.borrow_mut();
            child.remove_instance_for_parent_at_position(&parent_menu, position);
        }

        Some(item)
    }

    fn remove_instance_for_parent_at_position(&mut self, parent_menu: &NsMenuRef, position: usize) {
        let Some(ns_item) = parent_menu.1.itemAtIndex(position as NSInteger) else {
            return;
        };

        if self.item_type == MenuItemType::Submenu {
            self.remove_ns_menu_for_parent_item(parent_menu.0, &ns_item);
        }

        self.remove_ns_menu_item_for_parent(parent_menu.0, &ns_item);
        parent_menu.1.removeItemAtIndex(position as NSInteger);
    }

    fn remove_ns_menu_for_parent_item(&mut self, parent_id: u32, ns_item: &NSMenuItem) {
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

        self.remove_ns_instances_for_parent(removed.0);
    }

    fn remove_ns_instances_for_parent(&mut self, parent_id: u32) {
        self.ns_menu_items.remove(&parent_id);

        if self.item_type != MenuItemType::Submenu {
            return;
        }

        if let Some(menus) = self.ns_menus.as_mut().unwrap().remove(&parent_id) {
            for menu in menus {
                for child in self.children.as_mut().unwrap() {
                    child.borrow_mut().remove_ns_instances_for_parent(menu.0);
                }
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

    pub fn items(&self) -> Vec<MenuItemKind> {
        self.children
            .as_ref()
            .unwrap()
            .iter()
            .map(|c| c.borrow().kind(c.clone()))
            .collect()
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
impl MenuChild {
    pub fn create_ns_item_for_submenu(
        &mut self,
        menu_id: u32,
    ) -> crate::Result<Retained<NSMenuItem>> {
        let mtm = MainThreadMarker::new().expect("can only create menu item on the main thread");
        let ns_menu_item;
        let ns_submenu;

        let title = NSString::from_str(&self.text);
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

            ns_menu_item.setEnabled(self.enabled);

            if self.native_icon.is_some() {
                menuitem_set_native_icon(&ns_menu_item, self.native_icon.as_ref());
            }

            if let Some(icon) = self.icon.as_ref() {
                menuitem_set_icon(&ns_menu_item, Some(icon));
            }
        }

        let id = COUNTER.next();

        for item in self.children.as_ref().unwrap() {
            let ns_item = item.borrow_mut().make_ns_item_for_menu(item.clone(), id)?;
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

    pub fn create_ns_item_for_menu_item(
        &mut self,
        owner: Rc<RefCell<MenuChild>>,
        menu_id: u32,
    ) -> crate::Result<Retained<NSMenuItem>> {
        let mtm = MainThreadMarker::new().expect("can only create menu item on the main thread");
        let ns_menu_item = MenuItem::create(
            mtm,
            &self.text,
            Some(sel!(fireMenuItemAction:)),
            &self.accelerator,
        )?;

        unsafe {
            ns_menu_item.setTarget(Some(&ns_menu_item));
            ns_menu_item.setEnabled(self.enabled);
        }

        ns_menu_item.ivars().replace(Some(owner));

        self.apply_styled_text_if_any(&ns_menu_item);

        self.ns_menu_items
            .entry(menu_id)
            .or_default()
            .push(Retained::into_super(ns_menu_item.retain()));

        Ok(Retained::into_super(ns_menu_item))
    }

    pub fn create_ns_item_for_predefined_menu_item(
        &mut self,
        owner: Rc<RefCell<MenuChild>>,
        menu_id: u32,
    ) -> crate::Result<Retained<NSMenuItem>> {
        let mtm = MainThreadMarker::new().expect("can only create menu item on the main thread");
        let item_type = self.predefined_item_type.as_ref().unwrap();
        let ns_menu_item = match item_type {
            PredefinedMenuItemType::Separator => NSMenuItem::separatorItem(mtm),
            _ => {
                let ns_menu_item =
                    MenuItem::create(mtm, &self.text, item_type.selector(), &self.accelerator)?;

                if let PredefinedMenuItemType::About(_) = item_type {
                    unsafe { ns_menu_item.setTarget(Some(&ns_menu_item)) };
                    ns_menu_item.ivars().set(Some(owner));
                }

                Retained::into_super(ns_menu_item)
            }
        };

        ns_menu_item.setEnabled(self.enabled);

        if let PredefinedMenuItemType::Services = item_type {
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

    pub fn create_ns_item_for_check_menu_item(
        &mut self,
        owner: Rc<RefCell<MenuChild>>,
        menu_id: u32,
    ) -> crate::Result<Retained<NSMenuItem>> {
        let mtm = MainThreadMarker::new().expect("can only create menu item on the main thread");
        let ns_menu_item = MenuItem::create(
            mtm,
            &self.text,
            Some(sel!(fireMenuItemAction:)),
            &self.accelerator,
        )?;

        unsafe {
            ns_menu_item.setTarget(Some(&ns_menu_item));
            ns_menu_item.setEnabled(self.enabled);
            if self.checked.get() {
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

    pub fn create_ns_item_for_icon_menu_item(
        &mut self,
        owner: Rc<RefCell<MenuChild>>,
        menu_id: u32,
    ) -> crate::Result<Retained<NSMenuItem>> {
        let mtm = MainThreadMarker::new().expect("can only create menu item on the main thread");
        let ns_menu_item = MenuItem::create(
            mtm,
            &self.text,
            Some(sel!(fireMenuItemAction:)),
            &self.accelerator,
        )?;

        unsafe {
            ns_menu_item.setTarget(Some(&ns_menu_item));
            ns_menu_item.setEnabled(self.enabled);

            if self.icon.is_some() {
                menuitem_set_icon(&ns_menu_item, self.icon.as_ref());
            } else if self.native_icon.is_some() {
                menuitem_set_native_icon(&ns_menu_item, self.native_icon.as_ref());
            }
        }

        ns_menu_item.ivars().replace(Some(owner));

        self.apply_styled_text_if_any(&ns_menu_item);

        self.ns_menu_items
            .entry(menu_id)
            .or_default()
            .push(Retained::into_super(ns_menu_item.retain()));

        Ok(Retained::into_super(ns_menu_item))
    }

    fn make_ns_item_for_menu(
        &mut self,
        owner: Rc<RefCell<MenuChild>>,
        menu_id: u32,
    ) -> crate::Result<Retained<NSMenuItem>> {
        match self.item_type {
            MenuItemType::Submenu => self.create_ns_item_for_submenu(menu_id),
            MenuItemType::MenuItem => self.create_ns_item_for_menu_item(owner, menu_id),
            MenuItemType::Predefined => {
                self.create_ns_item_for_predefined_menu_item(owner, menu_id)
            }
            MenuItemType::Check => self.create_ns_item_for_check_menu_item(owner, menu_id),
            MenuItemType::Icon => self.create_ns_item_for_icon_menu_item(owner, menu_id),
        }
    }
}

impl PredefinedMenuItemType {
    fn is_supported_on_macos(&self) -> bool {
        matches!(
            self,
            PredefinedMenuItemType::Separator
                | PredefinedMenuItemType::Copy
                | PredefinedMenuItemType::Cut
                | PredefinedMenuItemType::Paste
                | PredefinedMenuItemType::PasteAndMatchStyle
                | PredefinedMenuItemType::Delete
                | PredefinedMenuItemType::SelectAll
                | PredefinedMenuItemType::Undo
                | PredefinedMenuItemType::Redo
                | PredefinedMenuItemType::Minimize
                | PredefinedMenuItemType::Maximize
                | PredefinedMenuItemType::ActualSize
                | PredefinedMenuItemType::ZoomIn
                | PredefinedMenuItemType::ZoomOut
                | PredefinedMenuItemType::Fullscreen
                | PredefinedMenuItemType::Hide
                | PredefinedMenuItemType::HideOthers
                | PredefinedMenuItemType::ShowAll
                | PredefinedMenuItemType::CloseWindow
                | PredefinedMenuItemType::Quit
                | PredefinedMenuItemType::About(_)
                | PredefinedMenuItemType::Services
                | PredefinedMenuItemType::BringAllToFront
                | PredefinedMenuItemType::StartSpeaking
                | PredefinedMenuItemType::StopSpeaking
                | PredefinedMenuItemType::StartDictation
                | PredefinedMenuItemType::EmojiAndSymbols
        )
    }

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
            // manual implementation in `fire_menu_item_click`
            PredefinedMenuItemType::About(_) => Some(sel!(fireMenuItemAction:)),
            PredefinedMenuItemType::Services => None,
            PredefinedMenuItemType::BringAllToFront => Some(sel!(arrangeInFront:)),
            PredefinedMenuItemType::StartSpeaking => Some(sel!(startSpeaking:)),
            PredefinedMenuItemType::StopSpeaking => Some(sel!(stopSpeaking:)),
            PredefinedMenuItemType::StartDictation => Some(sel!(startDictation:)),
            PredefinedMenuItemType::EmojiAndSymbols => Some(sel!(orderFrontCharacterPalette:)),
        }
    }
}

impl dyn IsMenuItem + '_ {
    fn make_ns_item_for_menu(&self, menu_id: u32) -> crate::Result<Retained<NSMenuItem>> {
        match self.kind() {
            MenuItemKind::Submenu(i) => i.inner.borrow_mut().create_ns_item_for_submenu(menu_id),
            MenuItemKind::MenuItem(i) => i
                .inner
                .borrow_mut()
                .create_ns_item_for_menu_item(i.inner.clone(), menu_id),
            MenuItemKind::Predefined(i) => i
                .inner
                .borrow_mut()
                .create_ns_item_for_predefined_menu_item(i.inner.clone(), menu_id),
            MenuItemKind::Check(i) => i
                .inner
                .borrow_mut()
                .create_ns_item_for_check_menu_item(i.inner.clone(), menu_id),
            MenuItemKind::Icon(i) => i
                .inner
                .borrow_mut()
                .create_ns_item_for_icon_menu_item(i.inner.clone(), menu_id),
        }
    }
}

define_class!(
    #[unsafe(super(NSMenuItem))]
    #[name = "MudaMenuItem"]
    #[thread_kind = MainThreadOnly]
    #[ivars = Cell<Option<Rc<RefCell<MenuChild>>>>]
    struct MenuItem;

    impl MenuItem {
        #[unsafe(method(fireMenuItemAction:))]
        fn fire_menu_item_action(&self, _sender: Option<&AnyObject>) {
            self.fire_menu_item_click();
        }
    }
);

impl MenuItem {
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

    fn fire_menu_item_click(&self) {
        let mtm = MainThreadMarker::from(self);
        // SAFETY: The ivar is initialized before the menu item is exposed and is
        // never mutated afterward.
        let item = unsafe { &*self.ivars().as_ptr() };
        let item = item.as_ref().expect("MenuChild pointer was unset");
        let item = item.borrow();

        if let Some(PredefinedMenuItemType::About(about_meta)) = &item.predefined_item_type {
            match about_meta {
                Some(about_meta) => {
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
                        NSApplication::sharedApplication(mtm)
                            .orderFrontStandardAboutPanelWithOptions(&dict)
                    };
                }

                None => {
                    NSApplication::sharedApplication(mtm).orderFrontStandardAboutPanel(Some(self));
                }
            }
        } else {
            if item.item_type == MenuItemType::Check {
                item.set_checked(!item.is_checked());
            }

            let id = (*item).id().clone();
            MenuEvent::send(crate::MenuEvent { id });
        }
    }

    fn create(
        mtm: MainThreadMarker,
        title: &str,
        selector: Option<Sel>,
        accelerator: &Option<MenuAccelerator>,
    ) -> crate::Result<Retained<MenuItem>> {
        let title = NSString::from_str(title);

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

        let item = MenuItem::new(mtm, &title, selector, &key_equivalent);
        item.setKeyEquivalentModifierMask(modifier_mask);

        Ok(item)
    }
}

/// Builds an attributed title from the parts: every part at the standard menu font, and each
/// non-[`TextStyle::Default`] run additionally carrying its style's color.
fn build_attributed_title(parts: &[(String, TextStyle)]) -> Retained<NSAttributedString> {
    let combined: String = parts.iter().map(|(text, _)| text.as_str()).collect();
    let ns_combined = NSString::from_str(&combined);
    let attributed =
        NSMutableAttributedString::initWithString(NSMutableAttributedString::alloc(), &ns_combined);

    let font = NSFont::menuFontOfSize(0.0);
    unsafe {
        attributed.addAttribute_value_range(
            NSFontAttributeName,
            &font,
            NSRange::new(0, ns_combined.length()),
        );
    }

    // `NSRange` counts UTF-16 code units, which is exactly what `NSString` stores.
    let mut offset = 0usize;
    for (text, style) in parts {
        let len = text.encode_utf16().count();
        if len > 0 {
            if let Some(color) = style.ns_color() {
                unsafe {
                    attributed.addAttribute_value_range(
                        NSForegroundColorAttributeName,
                        &color,
                        NSRange::new(offset, len),
                    );
                }
            }
        }
        offset += len;
    }

    attributed.into_super()
}

impl TextStyle {
    /// The color this style draws in, or `None` to leave the platform default in place.
    fn ns_color(self) -> Option<Retained<NSColor>> {
        match self {
            TextStyle::Default => None,
            TextStyle::Secondary => Some(NSColor::secondaryLabelColor()),
        }
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

/// How far off the screen edge a nudged-up menu is kept, in points.
const SCREEN_EDGE_MARGIN: f64 = 4.0;

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

    // `location` is in the space `in_view` implies: the view's own when popping up inside
    // a view, the screen's when not. Round-trip through screen space to do the fitting.
    // Convert rather than offset, so a flipped or transformed view stays correct.
    let location = match in_view {
        Some(view) => {
            let anchor = window.convertPointToScreen(view.convertPoint_toView(location, None));
            let anchor = fit_menu_on_screen(ns_menu, &window, anchor);
            view.convertPoint_fromView(window.convertPointFromScreen(anchor), None)
        }
        None => fit_menu_on_screen(ns_menu, &window, location),
    };

    ns_menu.popUpMenuPositioningItem_atLocation_inView(None, location, in_view)
}

/// Nudges a popup anchor so the menu it opens fits inside the screen's visible frame.
///
/// `anchor` and the returned point are both in screen coordinates.
fn fit_menu_on_screen(ns_menu: &NSMenu, window: &NSWindow, anchor: NSPoint) -> NSPoint {
    // No screen means the window is off-screen or hidden; nothing sensible to fit against.
    let Some(screen) = window.screen() else {
        return anchor;
    };
    fit_anchor(anchor, ns_menu.size(), screen.visibleFrame())
}

/// Where to actually anchor the popup so a `menu_size` menu lands inside `visible`.
///
/// `popUpMenuPositioningItem:atLocation:inView:` hangs the menu below and to the right of
/// the anchor and doesn't reposition it when that runs off-screen: it shows scroll arrows
/// instead, leaving most of the screen empty. So do the fitting up front, the way Electron
/// does in `electron_api_menu_mac.mm`.
///
/// All three arguments are in screen coordinates (bottom-left origin), and `visible` is the
/// screen's visible frame, so the menu bar and Dock are already excluded.
fn fit_anchor(mut anchor: NSPoint, menu_size: NSSize, visible: NSRect) -> NSPoint {
    // The menu hangs below the anchor, so its bottom edge sits `height` below it. Push the
    // whole thing up by however much it overflows, and keep it off the screen edge.
    let overflow_below = visible.origin.y - (anchor.y - menu_size.height);
    if overflow_below > 0.0 {
        anchor.y += overflow_below + SCREEN_EDGE_MARGIN;
    }
    // A menu taller than the screen can't fit either way; keep the anchor on-screen so it
    // opens where the user clicked instead of somewhere above the display.
    anchor.y = anchor.y.min(visible.origin.y + visible.size.height);

    // The menu extends to the right of the anchor. When it doesn't fit, flip it to the
    // left of the anchor, which is what AppKit's own menus do.
    if anchor.x + menu_size.width > visible.origin.x + visible.size.width {
        anchor.x -= menu_size.width;
    }

    anchor
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg_attr(miri, ignore = "calls into the ObjC runtime, which Miri can't emulate")]
    fn build_attributed_title_colors_only_the_non_normal_parts() {
        let parts = [
            ("Preview".to_owned(), TextStyle::Default),
            (" (default)".to_owned(), TextStyle::Secondary),
        ];
        let attributed = build_attributed_title(&parts);
        assert_eq!(attributed.string().to_string(), "Preview (default)");

        let color_at = |index: usize| unsafe {
            attributed
                .attribute_atIndex_effectiveRange(NSForegroundColorAttributeName, index, null_mut())
                .map(|value| Retained::cast_unchecked::<NSColor>(value))
        };

        // The primary run keeps the platform default, so it carries no color attribute.
        assert!(color_at(0).is_none());
        assert_eq!(color_at(8), Some(NSColor::secondaryLabelColor()));

        // Every run gets the menu font, so the two halves line up.
        let font_at = |index: usize| unsafe {
            attributed
                .attribute_atIndex_effectiveRange(NSFontAttributeName, index, null_mut())
                .map(|value| Retained::cast_unchecked::<NSFont>(value))
        };
        assert_eq!(font_at(0), font_at(8));
        assert_eq!(font_at(0), Some(NSFont::menuFontOfSize(0.0)));
    }

    /// A 1440x900 screen whose visible frame starts 50pt up (Dock) and stops 25pt short of
    /// the top (menu bar), like a real single-display setup.
    fn visible_frame() -> NSRect {
        NSRect::new(NSPoint::new(0.0, 50.0), NSSize::new(1440.0, 825.0))
    }

    #[test]
    fn a_menu_that_already_fits_is_left_alone() {
        let anchor = NSPoint::new(200.0, 700.0);
        assert_eq!(
            fit_anchor(anchor, NSSize::new(180.0, 300.0), visible_frame()),
            anchor
        );
    }

    #[test]
    fn a_menu_overflowing_the_bottom_is_pushed_up_to_clear_the_dock() {
        // Anchored 100pt above the Dock with a 300pt menu: 200pt hangs below the visible
        // frame, so the anchor rises by that much plus the edge margin.
        let fitted = fit_anchor(
            NSPoint::new(200.0, 150.0),
            NSSize::new(180.0, 300.0),
            visible_frame(),
        );
        assert_eq!(fitted.y, 150.0 + 200.0 + SCREEN_EDGE_MARGIN);
        assert_eq!(
            fitted.x, 200.0,
            "a vertical fit must not move the menu sideways"
        );

        // The whole menu now sits inside the visible frame.
        assert!(fitted.y - 300.0 >= visible_frame().origin.y);
    }

    #[test]
    fn a_menu_taller_than_the_screen_keeps_its_anchor_on_screen() {
        // Nothing can make an oversized menu fit, but the anchor must still land on-screen
        // rather than somewhere far above it.
        let visible = visible_frame();
        let fitted = fit_anchor(
            NSPoint::new(200.0, 400.0),
            NSSize::new(180.0, 2000.0),
            visible,
        );
        assert!(fitted.y <= visible.origin.y + visible.size.height);
    }

    #[test]
    fn a_menu_overflowing_the_right_edge_flips_to_the_left_of_the_anchor() {
        let fitted = fit_anchor(
            NSPoint::new(1400.0, 700.0),
            NSSize::new(180.0, 300.0),
            visible_frame(),
        );
        assert_eq!(fitted.x, 1400.0 - 180.0);
        assert_eq!(
            fitted.y, 700.0,
            "a horizontal flip must not move the menu vertically"
        );
    }

    #[test]
    fn a_corner_overflow_is_fixed_on_both_axes_at_once() {
        let fitted = fit_anchor(
            NSPoint::new(1400.0, 150.0),
            NSSize::new(180.0, 300.0),
            visible_frame(),
        );
        assert_eq!(fitted.x, 1400.0 - 180.0);
        assert_eq!(fitted.y, 150.0 + 200.0 + SCREEN_EDGE_MARGIN);
    }

    #[test]
    fn fitting_is_relative_to_the_screen_the_window_is_on() {
        // A second display to the right of the primary one, with its own origin.
        let visible = NSRect::new(NSPoint::new(1440.0, 0.0), NSSize::new(1920.0, 1080.0));
        let fitted = fit_anchor(
            NSPoint::new(3300.0, 100.0),
            NSSize::new(180.0, 300.0),
            visible,
        );
        assert_eq!(
            fitted.x,
            3300.0 - 180.0,
            "flips against that screen's right edge"
        );
        assert_eq!(fitted.y, 100.0 + 200.0 + SCREEN_EDGE_MARGIN);
    }
}
