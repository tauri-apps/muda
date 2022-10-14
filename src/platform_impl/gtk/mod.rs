mod accelerator;

use crate::{
    accelerator::Accelerator,
    predefined::PredfinedMenuItem,
    util::{AddOp, Counter},
};
use accelerator::{from_gtk_mnemonic, parse_accelerator, register_accelerator, to_gtk_mnemonic};
use gtk::{prelude::*, Orientation};
use std::{
    cell::RefCell,
    collections::HashMap,
    rc::Rc,
    sync::atomic::{AtomicBool, Ordering},
};

static COUNTER: Counter = Counter::new();

/// Generic shared type describing a menu entry. It can be one of [`MenuEntryType`]
#[derive(Debug, Default)]
pub(crate) struct MenuEntry {
    text: String,
    enabled: bool,
    checked: bool,
    id: u32,
    accelerator: Option<Accelerator>,
    accel_group: Option<gtk::AccelGroup>,
    type_: MenuEntryType,
    entries: Option<Vec<Rc<RefCell<MenuEntry>>>>,
}

/// Be careful when cloning this type, use it only to match against the enum
/// and don't mutate the vectors but it is fine to clone it and
/// call the gtk methods on the elements
#[derive(Debug, Clone)]
enum MenuEntryType {
    // because gtk doesn't allow using the same [`gtk::MenuItem`]
    // multiple times, and thus can't be used in multiple windows, each entry
    // keeps a vector of a [`gtk::MenuItem`] or a tuple of [`gtk::MenuItem`] and [`gtk::Menu`] if its a menu
    // and push to it every time [`Menu::init_for_gtk_window`] is called.
    Submenu(HashMap<u32, (gtk::MenuItem, gtk::Menu, u32)>),
    Text(HashMap<u32, gtk::MenuItem>),
    Check {
        store: HashMap<u32, gtk::CheckMenuItem>,
        is_syncing: Rc<AtomicBool>,
    },
    Predefined(HashMap<u32, gtk::MenuItem>, PredfinedMenuItem),
}

impl Default for MenuEntryType {
    fn default() -> Self {
        Self::Text(Default::default())
    }
}

struct InnerMenu {
    entries: Vec<Rc<RefCell<MenuEntry>>>,
    id: u32,
    // NOTE(amrbashir): because gtk doesn't allow using the same [`gtk::MenuBar`] and [`gtk::Box`]
    // multiple times, and thus can't be used in multiple windows. each menu
    // keeps a hashmap of window pointer as the key and a tuple of [`gtk::MenuBar`] and [`gtk::Box`] as the value
    // and push to it every time `Menu::init_for_gtk_window` is called.
    native_menus: HashMap<u32, (Option<gtk::MenuBar>, Rc<gtk::Box>)>,
    accel_group: Rc<gtk::AccelGroup>,
}

#[derive(Clone)]
pub struct Menu(Rc<RefCell<InnerMenu>>);

impl Menu {
    pub fn new() -> Self {
        Self(Rc::new(RefCell::new(InnerMenu {
            id: COUNTER.next(),
            entries: Vec::new(),
            native_menus: HashMap::new(),
            accel_group: Rc::new(gtk::AccelGroup::new()),
        })))
    }

    pub fn append(&self, item: &dyn crate::MenuEntry) {
        self.add_menu_item(item, AddOp::Append)
    }

    pub fn prepend(&self, item: &dyn crate::MenuEntry) {
        self.add_menu_item(item, AddOp::Prepend)
    }

    pub fn insert(&self, item: &dyn crate::MenuEntry, position: usize) {
        self.add_menu_item(item, AddOp::Insert(position))
    }

    fn add_menu_item(&self, item: &dyn crate::MenuEntry, op: AddOp) {
        let entry = match item.type_() {
            crate::MenuItemType::Submenu => {
                let submenu = item.as_any().downcast_ref::<crate::Submenu>().unwrap();
                let entry = &submenu.0 .0;
                for (menu_id, (menu_bar, _)) in &self.0.borrow().native_menus {
                    if let Some(menu_bar) = menu_bar {
                        add_gtk_submenu(menu_bar, *menu_id, entry, op, true);
                    }
                }
                entry
            }
            crate::MenuItemType::Text => {
                let item = item.as_any().downcast_ref::<crate::MenuItem>().unwrap();
                let entry = &item.0 .0;
                for (menu_id, (menu_bar, _)) in &self.0.borrow().native_menus {
                    if let Some(menu_bar) = menu_bar {
                        add_gtk_text_menuitem(
                            menu_bar,
                            *menu_id,
                            entry,
                            &self.0.borrow().accel_group,
                            op,
                            true,
                        );
                    }
                }
                entry
            }
            crate::MenuItemType::Check => {
                let item = item
                    .as_any()
                    .downcast_ref::<crate::CheckMenuItem>()
                    .unwrap();
                let entry = &item.0 .0;
                for (menu_id, (menu_bar, _)) in &self.0.borrow().native_menus {
                    if let Some(menu_bar) = menu_bar {
                        add_gtk_check_menuitem(
                            menu_bar,
                            *menu_id,
                            entry,
                            &self.0.borrow().accel_group,
                            op,
                            true,
                        )
                    }
                }
                entry
            }
        };

        let mut inner = self.0.borrow_mut();
        match op {
            AddOp::Append => inner.entries.push(entry.clone()),
            AddOp::Prepend => inner.entries.insert(0, entry.clone()),
            AddOp::Insert(position) => inner.entries.insert(position, entry.clone()),
        }
    }

    pub fn remove(&self, item: &dyn crate::MenuEntry) {
        match item.type_() {
            crate::MenuItemType::Submenu => {
                let submenu = item.as_any().downcast_ref::<crate::Submenu>().unwrap();
                for item in submenu.0.items() {
                    submenu.0.remove(&*item);
                }

                let entry = &submenu.0 .0;
                for (menu_id, (menu_bar, _)) in &self.0.borrow().native_menus {
                    if let Some(menu_bar) = menu_bar {
                        if let MenuEntryType::Submenu(store) = &entry.borrow().type_ {
                            menu_bar.remove(&store.get(menu_id).unwrap().0);
                        }
                    }
                }
            }
            crate::MenuItemType::Text => {
                let item = item.as_any().downcast_ref::<crate::MenuItem>().unwrap();
                let entry = &item.0 .0;
                for (menu_id, (menu_bar, _)) in &self.0.borrow().native_menus {
                    if let Some(menu_bar) = menu_bar {
                        match &entry.borrow().type_ {
                            MenuEntryType::Text(store) => {
                                menu_bar.remove(store.get(menu_id).unwrap());
                            }
                            MenuEntryType::Predefined(store, _) => {
                                if let Some(item) = store.get(menu_id) {
                                    menu_bar.remove(item);
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            crate::MenuItemType::Check => {
                let item = item
                    .as_any()
                    .downcast_ref::<crate::CheckMenuItem>()
                    .unwrap();
                let entry = &item.0 .0;
                for (menu_id, (menu_bar, _)) in &self.0.borrow().native_menus {
                    if let Some(menu_bar) = menu_bar {
                        if let MenuEntryType::Check { store, .. } = &entry.borrow().type_ {
                            menu_bar.remove(store.get(menu_id).unwrap());
                        }
                    }
                }
            }
        };

        let index = self
            .0
            .borrow()
            .entries
            .iter()
            .position(|e| e.borrow().id == item.id())
            .unwrap();
        self.0.borrow_mut().entries.remove(index);
    }

    pub fn items(&self) -> Vec<Box<dyn crate::MenuEntry>> {
        self.0
            .borrow()
            .entries
            .iter()
            .map(|e| -> Box<dyn crate::MenuEntry> {
                let entry = e.borrow();
                match entry.type_ {
                    MenuEntryType::Submenu(_) => Box::new(crate::Submenu(Submenu(e.clone()))),
                    MenuEntryType::Text(_) | MenuEntryType::Predefined(_, _) => {
                        Box::new(crate::MenuItem(MenuItem(e.clone())))
                    }
                    MenuEntryType::Check { .. } => {
                        Box::new(crate::CheckMenuItem(CheckMenuItem(e.clone())))
                    }
                }
            })
            .collect()
    }

    pub fn init_for_gtk_window<W>(&self, window: &W) -> Rc<gtk::Box>
    where
        W: IsA<gtk::ApplicationWindow>,
        W: IsA<gtk::Container>,
        W: IsA<gtk::Window>,
    {
        let mut inner = self.0.borrow_mut();
        let id = window.as_ptr() as u32;

        // This is the first time this method has been called on this window
        // so we need to create the menubar and its parent box
        if inner.native_menus.get(&(window.as_ptr() as _)).is_none() {
            let menu_bar = gtk::MenuBar::new();
            let vbox = gtk::Box::new(Orientation::Vertical, 0);
            window.add(&vbox);
            vbox.show();
            inner
                .native_menus
                .insert(id, (Some(menu_bar), Rc::new(vbox)));
        }

        if let Some((menu_bar, vbox)) = inner.native_menus.get(&(id)) {
            // This is NOT the first time this method has been called on a window.
            // So it already contains a [`gtk::Box`] but it doesn't have a [`gtk::MenuBar`]
            // because it was probably removed using [`Menu::remove_for_gtk_window`]
            // so we only need to create the menubar
            if menu_bar.is_none() {
                let vbox = Rc::clone(vbox);
                inner
                    .native_menus
                    .insert(id, (Some(gtk::MenuBar::new()), vbox));
            }
        }

        // Construct the entries of the menubar
        let (menu_bar, vbox) = inner.native_menus.get(&id).unwrap();
        let menu_bar = menu_bar.as_ref().unwrap();
        add_entries_to_gtkmenu(menu_bar, id, &inner.entries, &inner.accel_group, true);
        window.add_accel_group(&*inner.accel_group);

        // Show the menubar on the window
        vbox.pack_start(menu_bar, false, false, 0);
        menu_bar.show();

        Rc::clone(vbox)
    }

    pub fn remove_for_gtk_window<W>(&self, window: &W)
    where
        W: IsA<gtk::ApplicationWindow>,
        W: IsA<gtk::Window>,
    {
        let mut inner = self.0.borrow_mut();
        let id = window.as_ptr() as u32;

        if let Some((Some(menu_bar), vbox)) = inner.native_menus.get(&(id)) {
            // Remove the [`gtk::Menubar`] from the widget tree
            unsafe { menu_bar.destroy() };
            // Detach the accelerators from the window
            window.remove_accel_group(&*inner.accel_group);
            // Remove the removed [`gtk::Menubar`] from our cache
            let vbox = Rc::clone(vbox);
            inner.native_menus.insert(id, (None, vbox));
        }
    }

    pub fn hide_for_gtk_window<W>(&self, window: &W)
    where
        W: IsA<gtk::ApplicationWindow>,
    {
        if let Some((Some(menu_bar), _)) =
            self.0.borrow().native_menus.get(&(window.as_ptr() as u32))
        {
            menu_bar.hide();
        }
    }

    pub fn show_for_gtk_window<W>(&self, window: &W)
    where
        W: IsA<gtk::ApplicationWindow>,
    {
        if let Some((Some(menu_bar), _)) =
            self.0.borrow().native_menus.get(&(window.as_ptr() as u32))
        {
            menu_bar.show_all();
        }
    }

    pub fn show_context_menu_for_gtk_window<W>(&self, window: &W, x: f64, y: f64)
    where
        W: IsA<gtk::ApplicationWindow>,
        W: IsA<gtk::Widget>,
    {
        if let Some(window) = window.window() {
            let gtk_menu = gtk::Menu::new();
            let accel_group = gtk::AccelGroup::new();
            add_entries_to_gtkmenu(&gtk_menu, 0, &self.0.borrow().entries, &accel_group, false);
            gtk_menu.popup_at_rect(
                &window,
                &gdk::Rectangle::new(x as _, y as _, 0, 0),
                gdk::Gravity::NorthWest,
                gdk::Gravity::NorthWest,
                None,
            );
        }
    }
}

#[derive(Clone)]
pub(crate) struct Submenu(Rc<RefCell<MenuEntry>>);

impl Submenu {
    pub fn new(text: &str, enabled: bool) -> Self {
        let entry = Rc::new(RefCell::new(MenuEntry {
            text: text.to_string(),
            enabled,
            entries: Some(Vec::new()),
            type_: MenuEntryType::Submenu(HashMap::new()),
            accel_group: Some(gtk::AccelGroup::new()),
            ..Default::default()
        }));

        Self(entry)
    }

    pub fn id(&self) -> u32 {
        self.0.borrow().id
    }

    pub fn append(&self, item: &dyn crate::MenuEntry) {
        self.add_menu_item(item, AddOp::Append)
    }

    pub fn prepend(&self, item: &dyn crate::MenuEntry) {
        self.add_menu_item(item, AddOp::Prepend)
    }

    pub fn insert(&self, item: &dyn crate::MenuEntry, position: usize) {
        self.add_menu_item(item, AddOp::Insert(position))
    }

    fn add_menu_item(&self, item: &dyn crate::MenuEntry, op: AddOp) {
        let type_ = self.0.borrow().type_.clone();
        if let MenuEntryType::Submenu(store) = &type_ {
            let entry = match item.type_() {
                crate::MenuItemType::Submenu => {
                    let item = item.as_any().downcast_ref::<crate::Submenu>().unwrap();
                    let entry = &item.0 .0;
                    for (_, menu, menu_id) in store.values() {
                        add_gtk_submenu(menu, *menu_id, entry, op, true);
                    }
                    entry
                }
                crate::MenuItemType::Text => {
                    let item = item.as_any().downcast_ref::<crate::MenuItem>().unwrap();
                    let entry = &item.0 .0;
                    for (_, menu, menu_id) in store.values() {
                        add_gtk_text_menuitem(
                            menu,
                            *menu_id,
                            entry,
                            self.0.borrow().accel_group.as_ref().unwrap(),
                            op,
                            true,
                        );
                    }
                    entry
                }
                crate::MenuItemType::Check => {
                    let item = item
                        .as_any()
                        .downcast_ref::<crate::CheckMenuItem>()
                        .unwrap();
                    let entry = &item.0 .0;
                    for (_, menu, menu_id) in store.values() {
                        add_gtk_text_menuitem(
                            menu,
                            *menu_id,
                            entry,
                            self.0.borrow().accel_group.as_ref().unwrap(),
                            op,
                            true,
                        );
                    }
                    entry
                }
            };

            let mut inner = self.0.borrow_mut();
            let entries = inner.entries.as_mut().unwrap();

            match op {
                AddOp::Append => entries.push(entry.clone()),
                AddOp::Prepend => entries.insert(0, entry.clone()),
                AddOp::Insert(position) => entries.insert(position, entry.clone()),
            }
        }
    }

    pub fn remove(&self, item: &dyn crate::MenuEntry) {
        if let MenuEntryType::Submenu(store) = self.0.borrow().type_.clone() {
            match item.type_() {
                crate::MenuItemType::Submenu => {
                    let submenu = item.as_any().downcast_ref::<crate::Submenu>().unwrap();
                    for item in submenu.0.items() {
                        submenu.0.remove(&*item);
                    }

                    let entry = &submenu.0 .0;
                    for (_, menu, menu_id) in store.values() {
                        if let MenuEntryType::Submenu(store) = &entry.borrow().type_ {
                            menu.remove(&store.get(menu_id).unwrap().0);
                        }
                    }
                }
                crate::MenuItemType::Text => {
                    let item = item.as_any().downcast_ref::<crate::MenuItem>().unwrap();
                    let entry = &item.0 .0;
                    for (_, menu, menu_id) in store.values() {
                        match &entry.borrow().type_ {
                            MenuEntryType::Text(store) => {
                                menu.remove(store.get(menu_id).unwrap());
                            }
                            MenuEntryType::Predefined(store, _) => {
                                if let Some(item) = store.get(menu_id) {
                                    menu.remove(item);
                                }
                            }
                            _ => {}
                        }
                    }
                }
                crate::MenuItemType::Check => {
                    let item = item
                        .as_any()
                        .downcast_ref::<crate::CheckMenuItem>()
                        .unwrap();
                    let entry = &item.0 .0;
                    for (_, menu, menu_id) in store.values() {
                        if let MenuEntryType::Check { store, .. } = &entry.borrow().type_ {
                            menu.remove(store.get(menu_id).unwrap());
                        }
                    }
                }
            };
        }

        let index = self
            .0
            .borrow()
            .entries
            .as_ref()
            .unwrap()
            .iter()
            .position(|e| e.borrow().id == item.id())
            .unwrap();
        self.0.borrow_mut().entries.as_mut().unwrap().remove(index);
    }

    pub fn items(&self) -> Vec<Box<dyn crate::MenuEntry>> {
        self.0
            .borrow()
            .entries
            .as_ref()
            .unwrap()
            .iter()
            .map(|e| -> Box<dyn crate::MenuEntry> {
                let entry = e.borrow();
                match entry.type_ {
                    MenuEntryType::Submenu(_) => Box::new(crate::Submenu(Submenu(e.clone()))),
                    MenuEntryType::Text(_) | MenuEntryType::Predefined(_, _) => {
                        Box::new(crate::MenuItem(MenuItem(e.clone())))
                    }
                    MenuEntryType::Check { .. } => {
                        Box::new(crate::CheckMenuItem(CheckMenuItem(e.clone())))
                    }
                }
            })
            .collect()
    }

    pub fn text(&self) -> String {
        let entry = self.0.borrow();
        if let MenuEntryType::Submenu(store) = &entry.type_ {
            store
                .get(&0)
                .map(|i| {
                    i.0.label()
                        .map(|l| l.as_str().to_string())
                        .map(from_gtk_mnemonic)
                        .unwrap_or_default()
                })
                .unwrap_or_else(|| entry.text.clone())
        } else {
            unreachable!()
        }
    }

    pub fn set_text(&self, text: &str) {
        let mut entry = self.0.borrow_mut();
        entry.text = text.to_string();

        if let MenuEntryType::Submenu(store) = &entry.type_ {
            let text = to_gtk_mnemonic(text);
            for (i, _, _) in store.values() {
                i.set_label(&text);
            }
        } else {
            unreachable!()
        }
    }

    pub fn is_enabled(&self) -> bool {
        let entry = self.0.borrow();
        if let MenuEntryType::Submenu(store) = &entry.type_ {
            store
                .get(&0)
                .map(|i| i.0.is_sensitive())
                .unwrap_or_else(|| entry.enabled)
        } else {
            unreachable!()
        }
    }

    pub fn set_enabled(&self, enabled: bool) {
        let mut entry = self.0.borrow_mut();
        entry.enabled = enabled;

        if let MenuEntryType::Submenu(store) = &entry.type_ {
            for (i, _, _) in store.values() {
                i.set_sensitive(enabled);
            }
        } else {
            unreachable!()
        }
    }

    pub fn show_context_menu_for_gtk_window<W>(&self, window: &W, x: f64, y: f64)
    where
        W: IsA<gtk::ApplicationWindow>,
        W: IsA<gtk::Widget>,
    {
        if let Some(window) = window.window() {
            let gtk_menu = gtk::Menu::new();
            let accel_group = gtk::AccelGroup::new();
            add_entries_to_gtkmenu(
                &gtk_menu,
                0,
                &self.0.borrow().entries.as_ref().unwrap(),
                &accel_group,
                false,
            );
            gtk_menu.popup_at_rect(
                &window,
                &gdk::Rectangle::new(x as _, y as _, 0, 0),
                gdk::Gravity::NorthWest,
                gdk::Gravity::NorthWest,
                None,
            );
        }
    }
}

#[derive(Clone)]
pub(crate) struct MenuItem(Rc<RefCell<MenuEntry>>);

impl MenuItem {
    pub fn new(text: &str, enabled: bool, accelerator: Option<Accelerator>) -> Self {
        let entry = Rc::new(RefCell::new(MenuEntry {
            text: text.to_string(),
            enabled,
            accelerator,
            id: COUNTER.next(),
            type_: MenuEntryType::Text(HashMap::new()),
            ..Default::default()
        }));

        Self(entry)
    }

    pub fn predefined(item: PredfinedMenuItem, text: Option<String>) -> Self {
        let entry = Rc::new(RefCell::new(MenuEntry {
            text: text.unwrap_or_else(|| item.text().to_string()),
            enabled: true,
            accelerator: item.accelerator(),
            id: COUNTER.next(),
            type_: MenuEntryType::Predefined(HashMap::new(), item),
            ..Default::default()
        }));

        Self(entry)
    }

    pub fn id(&self) -> u32 {
        self.0.borrow().id
    }

    pub fn text(&self) -> String {
        let entry = self.0.borrow();
        match &entry.type_ {
            MenuEntryType::Text(store) | MenuEntryType::Predefined(store, _) => store
                .get(&0)
                .map(|i| {
                    i.label()
                        .map(|l| l.as_str().to_string())
                        .map(from_gtk_mnemonic)
                        .unwrap_or_default()
                })
                .unwrap_or_else(|| entry.text.clone()),
            _ => unreachable!(),
        }
    }

    pub fn set_text(&self, text: &str) {
        let mut entry = self.0.borrow_mut();
        entry.text = text.to_string();

        match &entry.type_ {
            MenuEntryType::Text(store) | MenuEntryType::Predefined(store, _) => {
                let text = to_gtk_mnemonic(text);
                for i in store.values() {
                    i.set_label(&text);
                }
            }
            _ => unreachable!(),
        }
    }

    pub fn is_enabled(&self) -> bool {
        let entry = self.0.borrow();
        match &entry.type_ {
            MenuEntryType::Text(store) | MenuEntryType::Predefined(store, _) => store
                .get(&0)
                .map(|i| i.is_sensitive())
                .unwrap_or_else(|| entry.enabled),
            _ => unreachable!(),
        }
    }

    pub fn set_enabled(&self, enabled: bool) {
        let mut entry = self.0.borrow_mut();
        entry.enabled = enabled;

        match &entry.type_ {
            MenuEntryType::Text(store) | MenuEntryType::Predefined(store, _) => {
                for i in store.values() {
                    i.set_sensitive(enabled);
                }
            }
            _ => unreachable!(),
        }
    }
}

#[derive(Clone)]
pub(crate) struct CheckMenuItem(Rc<RefCell<MenuEntry>>);

impl CheckMenuItem {
    pub fn new(text: &str, enabled: bool, checked: bool, accelerator: Option<Accelerator>) -> Self {
        let entry = Rc::new(RefCell::new(MenuEntry {
            text: text.to_string(),
            enabled,
            checked,
            accelerator,
            id: COUNTER.next(),
            type_: MenuEntryType::Check {
                store: HashMap::new(),
                is_syncing: Rc::new(AtomicBool::new(false)),
            },
            ..Default::default()
        }));

        Self(entry)
    }

    pub fn id(&self) -> u32 {
        self.0.borrow().id
    }

    pub fn text(&self) -> String {
        let entry = self.0.borrow();
        if let MenuEntryType::Check { store, .. } = &entry.type_ {
            store
                .get(&0)
                .map(|i| {
                    i.label()
                        .map(|l| l.as_str().to_string())
                        .map(from_gtk_mnemonic)
                        .unwrap_or_default()
                })
                .unwrap_or_else(|| entry.text.clone())
        } else {
            unreachable!()
        }
    }

    pub fn set_text(&self, text: &str) {
        let mut entry = self.0.borrow_mut();
        entry.text = text.to_string();

        if let MenuEntryType::Check { store, .. } = &entry.type_ {
            let text = to_gtk_mnemonic(text);
            for i in store.values() {
                i.set_label(&text);
            }
        } else {
            unreachable!()
        }
    }

    pub fn is_enabled(&self) -> bool {
        let entry = self.0.borrow();
        if let MenuEntryType::Check { store, .. } = &entry.type_ {
            store
                .get(&0)
                .map(|i| i.is_sensitive())
                .unwrap_or(entry.enabled)
        } else {
            unreachable!()
        }
    }

    pub fn set_enabled(&self, enabled: bool) {
        let mut entry = self.0.borrow_mut();
        entry.enabled = enabled;

        if let MenuEntryType::Check { store, .. } = &entry.type_ {
            for i in store.values() {
                i.set_sensitive(enabled);
            }
        } else {
            unreachable!()
        }
    }

    pub fn is_checked(&self) -> bool {
        let entry = self.0.borrow();
        if let MenuEntryType::Check { store, .. } = &entry.type_ {
            store
                .get(&0)
                .map(|i| i.is_active())
                .unwrap_or(entry.checked)
        } else {
            unreachable!()
        }
    }

    pub fn set_checked(&self, checked: bool) {
        let type_ = {
            let mut entry = self.0.borrow_mut();
            entry.checked = checked;
            entry.type_.clone()
        };

        if let MenuEntryType::Check { store, is_syncing } = &type_ {
            is_syncing.store(true, Ordering::Release);
            for i in store.values() {
                i.set_active(checked);
            }
            is_syncing.store(false, Ordering::Release);
        } else {
            unreachable!()
        }
    }
}

fn add_gtk_submenu(
    menu: &impl IsA<gtk::MenuShell>,
    menu_id: u32,
    entry: &Rc<RefCell<MenuEntry>>,
    op: AddOp,
    add_to_store: bool,
) {
    let mut entry = entry.borrow_mut();
    let submenu = gtk::Menu::new();
    let item = gtk::MenuItem::builder()
        .label(&to_gtk_mnemonic(&entry.text))
        .use_underline(true)
        .submenu(&submenu)
        .sensitive(entry.enabled)
        .build();

    match op {
        AddOp::Append => menu.append(&item),
        AddOp::Prepend => menu.prepend(&item),
        AddOp::Insert(position) => menu.insert(&item, position as i32),
    }

    item.show();
    let id = COUNTER.next();
    add_entries_to_gtkmenu(
        &submenu,
        id,
        entry.entries.as_ref().unwrap(),
        entry.accel_group.as_ref().unwrap(),
        add_to_store,
    );
    if add_to_store {
        if let MenuEntryType::Submenu(store) = &mut entry.type_ {
            store.insert(menu_id, (item, submenu, id));
        }
    }
}

fn add_gtk_text_menuitem(
    menu: &impl IsA<gtk::MenuShell>,
    menu_id: u32,
    entry: &Rc<RefCell<MenuEntry>>,
    accel_group: &gtk::AccelGroup,
    op: AddOp,
    add_to_store: bool,
) {
    let type_ = entry.borrow().type_.clone();
    if let MenuEntryType::Text(_) = &type_ {
        let mut entry = entry.borrow_mut();
        let item = gtk::MenuItem::builder()
            .label(&to_gtk_mnemonic(&entry.text))
            .use_underline(true)
            .sensitive(entry.enabled)
            .build();
        let id = entry.id;

        match op {
            AddOp::Append => menu.append(&item),
            AddOp::Prepend => menu.prepend(&item),
            AddOp::Insert(position) => menu.insert(&item, position as i32),
        }

        item.show();
        if let Some(accelerator) = &entry.accelerator {
            register_accelerator(&item, accel_group, accelerator);
        }
        item.connect_activate(move |_| {
            let _ = crate::MENU_CHANNEL.0.send(crate::MenuEvent { id });
        });

        if add_to_store {
            if let MenuEntryType::Text(store) = &mut entry.type_ {
                store.insert(menu_id, item);
            }
        }
    } else if let MenuEntryType::Predefined(_, _) = &type_ {
        add_gtk_predefined_menuitm(menu, menu_id, entry, accel_group, op, add_to_store);
    }
}

fn add_gtk_predefined_menuitm(
    menu: &impl IsA<gtk::MenuShell>,
    menu_id: u32,
    entry: &Rc<RefCell<MenuEntry>>,
    accel_group: &gtk::AccelGroup,
    op: AddOp,
    add_to_store: bool,
) {
    let mut entry = entry.borrow_mut();
    let text = entry.text.clone();
    let accelerator = entry.accelerator.clone();

    if let MenuEntryType::Predefined(store, predefined_item) = &mut entry.type_ {
        let predefined_item = predefined_item.clone();
        let make_item = || {
            gtk::MenuItem::builder()
                .label(&to_gtk_mnemonic(text))
                .use_underline(true)
                .sensitive(true)
                .build()
        };
        let register_accel = |item: &gtk::MenuItem| {
            if let Some(accelerator) = accelerator {
                register_accelerator(item, accel_group, &accelerator);
            }
        };

        let item = match predefined_item {
            PredfinedMenuItem::Separator => {
                Some(gtk::SeparatorMenuItem::new().upcast::<gtk::MenuItem>())
            }
            PredfinedMenuItem::Copy
            | PredfinedMenuItem::Cut
            | PredfinedMenuItem::Paste
            | PredfinedMenuItem::SelectAll => {
                let item = make_item();
                let (mods, key) =
                    parse_accelerator(&predefined_item.accelerator().unwrap()).unwrap();
                item.child()
                    .unwrap()
                    .downcast::<gtk::AccelLabel>()
                    .unwrap()
                    .set_accel(key, mods);
                item.connect_activate(move |_| {
                    // TODO: wayland
                    if let Ok(xdo) = libxdo::XDo::new(None) {
                        let _ = xdo.send_keysequence(predefined_item.xdo_keys(), 0);
                    }
                });
                Some(item)
            }
            PredfinedMenuItem::About(metadata) => {
                let item = make_item();
                register_accel(&item);
                item.connect_activate(move |_| {
                    if let Some(metadata) = &metadata {
                        let mut builder = gtk::builders::AboutDialogBuilder::new()
                            .modal(true)
                            .resizable(false);

                        if let Some(name) = &metadata.name {
                            builder = builder.program_name(name);
                        }
                        if let Some(version) = &metadata.version {
                            builder = builder.version(version);
                        }
                        if let Some(authors) = &metadata.authors {
                            builder = builder.authors(authors.clone());
                        }
                        if let Some(comments) = &metadata.comments {
                            builder = builder.comments(comments);
                        }
                        if let Some(copyright) = &metadata.copyright {
                            builder = builder.copyright(copyright);
                        }
                        if let Some(license) = &metadata.license {
                            builder = builder.license(license);
                        }
                        if let Some(website) = &metadata.website {
                            builder = builder.website(website);
                        }
                        if let Some(website_label) = &metadata.website_label {
                            builder = builder.website_label(website_label);
                        }

                        let about = builder.build();
                        about.run();
                        unsafe {
                            about.destroy();
                        }
                    }
                });
                Some(item)
            }
            _ => None,
        };

        if let Some(item) = item {
            match op {
                AddOp::Append => menu.append(&item),
                AddOp::Prepend => menu.prepend(&item),
                AddOp::Insert(position) => menu.insert(&item, position as i32),
            }
            item.show();

            if add_to_store {
                store.insert(menu_id, item);
            }
        }
    }
}

fn add_gtk_check_menuitem(
    menu: &impl IsA<gtk::MenuShell>,
    menu_id: u32,
    entry: &Rc<RefCell<MenuEntry>>,
    accel_group: &gtk::AccelGroup,
    op: AddOp,
    add_to_store: bool,
) {
    let entry_c = entry.clone();
    let mut entry = entry.borrow_mut();

    let item = gtk::CheckMenuItem::builder()
        .label(&to_gtk_mnemonic(&entry.text))
        .use_underline(true)
        .sensitive(entry.enabled)
        .active(entry.checked)
        .build();
    if let Some(accelerator) = &entry.accelerator {
        register_accelerator(&item, accel_group, accelerator);
    }
    let id = entry.id;

    item.connect_toggled(move |i| {
        let should_dispatch = matches!(&entry_c.borrow().type_, MenuEntryType::Check { is_syncing, .. } if is_syncing
                       .compare_exchange(false, true, Ordering::Release, Ordering::Relaxed)
                             .is_ok());

        if should_dispatch {
            let checked = i.is_active();
            let type_ = {
                let mut entry = entry_c.borrow_mut();
                entry.checked = checked;
                entry.type_.clone()
            };

            if let MenuEntryType::Check { store, .. } = &type_ {
                for i in store.values() {
                    i.set_active(checked);
                }
                if let MenuEntryType::Check { is_syncing, .. } = &mut entry_c.borrow_mut().type_ {
                    is_syncing.store(false, Ordering::Release);
                }
            }

            let _ = crate::MENU_CHANNEL.0.send(crate::MenuEvent { id });
        }
    });

    match op {
        AddOp::Append => menu.append(&item),
        AddOp::Prepend => menu.prepend(&item),
        AddOp::Insert(position) => menu.insert(&item, position as i32),
    }

    item.show();

    if add_to_store {
        if let MenuEntryType::Check { store, .. } = &mut entry.type_ {
            store.insert(menu_id, item);
        }
    }
}

fn add_entries_to_gtkmenu<M: IsA<gtk::MenuShell>>(
    menu: &M,
    menu_id: u32,
    entries: &Vec<Rc<RefCell<MenuEntry>>>,
    accel_group: &gtk::AccelGroup,
    add_to_store: bool,
) {
    for entry in entries {
        let type_ = entry.borrow().type_.clone();
        match type_ {
            MenuEntryType::Submenu(_) => {
                add_gtk_submenu(menu, menu_id, entry, AddOp::Append, add_to_store)
            }
            MenuEntryType::Text(_) | MenuEntryType::Predefined(_, _) => add_gtk_text_menuitem(
                menu,
                menu_id,
                entry,
                accel_group,
                AddOp::Append,
                add_to_store,
            ),
            MenuEntryType::Check { .. } => add_gtk_check_menuitem(
                menu,
                menu_id,
                entry,
                accel_group,
                AddOp::Append,
                add_to_store,
            ),
        }
    }
}

impl PredfinedMenuItem {
    fn xdo_keys(&self) -> &str {
        match self {
            PredfinedMenuItem::Copy => "ctrl+c",
            PredfinedMenuItem::Cut => "ctrl+X",
            PredfinedMenuItem::Paste => "ctrl+v",
            PredfinedMenuItem::SelectAll => "ctrl+a",
            _ => "",
        }
    }
}
