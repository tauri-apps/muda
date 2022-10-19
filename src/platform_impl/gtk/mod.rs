mod accelerator;

use crate::{
    accelerator::Accelerator,
    predefined::PredfinedMenuItemType,
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

/// Generic shared type describing a menu entry. It can be one of [`MenuItemType`]
#[derive(Debug, Default)]
pub(crate) struct MenuEntry {
    text: String,
    enabled: bool,
    checked: bool,
    id: u32,
    accelerator: Option<Accelerator>,
    type_: MenuItemType,
    entries: Option<Vec<Rc<RefCell<MenuEntry>>>>,
}

/// Be careful when cloning this type, use it only to match against the enum
/// and don't mutate the vectors but it is fine to clone it and
/// call the gtk methods on the elements
#[derive(Debug, Clone)]
enum MenuItemType {
    // because gtk doesn't allow using the same [`gtk::MenuItem`]
    // multiple times, and thus can't be used in multiple windows, each entry
    // keeps a vector of a [`gtk::MenuItem`] or a tuple of [`gtk::MenuItem`] and [`gtk::Menu`] if its a menu
    // and push to it every time [`Menu::init_for_gtk_window`] is called.
    Submenu(HashMap<u32, Vec<(gtk::MenuItem, gtk::Menu, Rc<gtk::AccelGroup>, u32)>>),
    Normal(HashMap<u32, Vec<gtk::MenuItem>>),
    Check {
        store: HashMap<u32, Vec<gtk::CheckMenuItem>>,
        is_syncing: Rc<AtomicBool>,
    },
    Predefined(HashMap<u32, Vec<gtk::MenuItem>>, PredfinedMenuItemType),
}

impl Default for MenuItemType {
    fn default() -> Self {
        Self::Normal(Default::default())
    }
}

struct InnerMenu {
    entries: Vec<Rc<RefCell<MenuEntry>>>,
    // because gtk doesn't allow using the same [`gtk::MenuBar`] and [`gtk::Box`]
    // multiple times, and thus can't be used in multiple windows. each menu
    // keeps a hashmap of window pointer as the key and a tuple of [`gtk::MenuBar`] and [`gtk::Box`] as the value
    // and push to it every time `Menu::init_for_gtk_window` is called.
    native_menus: HashMap<u32, (Option<gtk::MenuBar>, Rc<gtk::Box>)>,
    accel_group: Option<Rc<gtk::AccelGroup>>,
}

#[derive(Clone)]
pub struct Menu(Rc<RefCell<InnerMenu>>);

impl Menu {
    pub fn new() -> Self {
        Self(Rc::new(RefCell::new(InnerMenu {
            entries: Vec::new(),
            native_menus: HashMap::new(),
            accel_group: None,
        })))
    }

    pub fn append(&self, item: &dyn crate::MenuItem) {
        self.add_menu_item(item, AddOp::Append)
    }

    pub fn prepend(&self, item: &dyn crate::MenuItem) {
        self.add_menu_item(item, AddOp::Insert(0))
    }

    pub fn insert(&self, item: &dyn crate::MenuItem, position: usize) {
        self.add_menu_item(item, AddOp::Insert(position))
    }

    fn add_menu_item(&self, item: &dyn crate::MenuItem, op: AddOp) {
        let entry = match item.type_() {
            crate::MenuItemType::Submenu => {
                let submenu = item.as_any().downcast_ref::<crate::Submenu>().unwrap();
                let entry = &submenu.0 .0;
                for (menu_id, (menu_bar, _)) in &self.0.borrow().native_menus {
                    if let Some(menu_bar) = menu_bar {
                        add_gtk_submenu(
                            menu_bar,
                            self.0.borrow().accel_group.as_ref().unwrap(),
                            *menu_id,
                            entry,
                            op,
                            true,
                        );
                    }
                }
                entry
            }
            crate::MenuItemType::Normal => {
                let item = item.as_any().downcast_ref::<crate::MenuItem>().unwrap();
                let entry = &item.0 .0;
                for (menu_id, (menu_bar, _)) in &self.0.borrow().native_menus {
                    if let Some(menu_bar) = menu_bar {
                        add_gtk_text_menuitem(
                            menu_bar,
                            *menu_id,
                            entry,
                            &self.0.borrow().accel_group.as_ref().unwrap(),
                            op,
                            true,
                        );
                    }
                }
                entry
            }
            crate::MenuItemType::Predefined => {
                let item = item
                    .as_any()
                    .downcast_ref::<crate::PredefinedMenuItem>()
                    .unwrap();
                let entry = &item.0 .0;
                for (menu_id, (menu_bar, _)) in &self.0.borrow().native_menus {
                    if let Some(menu_bar) = menu_bar {
                        add_gtk_predefined_menuitm(
                            menu_bar,
                            *menu_id,
                            entry,
                            &self.0.borrow().accel_group.as_ref().unwrap(),
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
                            &self.0.borrow().accel_group.as_ref().unwrap(),
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
            AddOp::Insert(position) => inner.entries.insert(position, entry.clone()),
        }
    }

    pub fn remove(&self, item: &dyn crate::MenuItem) {
        match item.type_() {
            crate::MenuItemType::Submenu => {
                let submenu = item.as_any().downcast_ref::<crate::Submenu>().unwrap();
                let entry = &submenu.0 .0;
                for (menu_id, (menu_bar, _)) in &self.0.borrow().native_menus {
                    if let Some(menu_bar) = menu_bar {
                        for item in submenu.items() {
                            submenu.0.remove_gtk_by_parent_id(*menu_id, &*item);
                        }

                        if let MenuItemType::Submenu(store) = &mut entry.borrow_mut().type_ {
                            let items = store.remove(menu_id).unwrap();
                            for (item, _, _, _) in items {
                                menu_bar.remove(&item);
                            }
                        }
                    }
                }
            }
            crate::MenuItemType::Normal => {
                let item = item.as_any().downcast_ref::<crate::MenuItem>().unwrap();
                let entry = &item.0 .0;
                for (menu_id, (menu_bar, _)) in &self.0.borrow().native_menus {
                    if let Some(menu_bar) = menu_bar {
                        if let MenuItemType::Normal(store) = &mut entry.borrow_mut().type_ {
                            let items = store.remove(menu_id).unwrap();
                            for item in items {
                                menu_bar.remove(&item);
                            }
                        }
                    }
                }
            }
            crate::MenuItemType::Predefined => {
                let item = item
                    .as_any()
                    .downcast_ref::<crate::PredefinedMenuItem>()
                    .unwrap();
                let entry = &item.0 .0;
                for (menu_id, (menu_bar, _)) in &self.0.borrow().native_menus {
                    if let Some(menu_bar) = menu_bar {
                        if let MenuItemType::Predefined(store, _) = &mut entry.borrow_mut().type_ {
                            if let Some(items) = store.remove(menu_id) {
                                for item in items {
                                    menu_bar.remove(&item);
                                }
                            }
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
                        if let MenuItemType::Check { store, .. } = &mut entry.borrow_mut().type_ {
                            let items = store.remove(menu_id).unwrap();
                            for item in items {
                                menu_bar.remove(&item);
                            }
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

    fn remove_gtk_by_parent_id(&self, parent_id: u32, item: &dyn crate::MenuItem) {
        match item.type_() {
            crate::MenuItemType::Submenu => {
                let submenu = item.as_any().downcast_ref::<crate::Submenu>().unwrap();
                let entry = &submenu.0 .0;
                if let (Some(menu_bar), _) = self.0.borrow().native_menus.get(&parent_id).unwrap() {
                    for item in submenu.items() {
                        submenu.0.remove_gtk_by_parent_id(parent_id, &*item);
                    }

                    if let MenuItemType::Submenu(store) = &mut entry.borrow_mut().type_ {
                        let items = store.remove(&parent_id).unwrap();
                        for (item, _, _, _) in items {
                            menu_bar.remove(&item);
                        }
                    }
                }
            }
            crate::MenuItemType::Normal => {
                let item = item.as_any().downcast_ref::<crate::MenuItem>().unwrap();
                let entry = &item.0 .0;
                if let (Some(menu_bar), _) = self.0.borrow().native_menus.get(&parent_id).unwrap() {
                    if let MenuItemType::Normal(store) = &mut entry.borrow_mut().type_ {
                        let items = store.remove(&parent_id).unwrap();
                        for item in items {
                            menu_bar.remove(&item);
                        }
                    }
                }
            }
            crate::MenuItemType::Predefined => {
                let item = item
                    .as_any()
                    .downcast_ref::<crate::PredefinedMenuItem>()
                    .unwrap();
                let entry = &item.0 .0;
                if let (Some(menu_bar), _) = self.0.borrow().native_menus.get(&parent_id).unwrap() {
                    if let MenuItemType::Predefined(store, _) = &mut entry.borrow_mut().type_ {
                        if let Some(items) = store.remove(&parent_id) {
                            for item in items {
                                menu_bar.remove(&item);
                            }
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
                if let (Some(menu_bar), _) = self.0.borrow().native_menus.get(&parent_id).unwrap() {
                    if let MenuItemType::Check { store, .. } = &mut entry.borrow_mut().type_ {
                        let items = store.remove(&parent_id).unwrap();
                        for item in items {
                            menu_bar.remove(&item);
                        }
                    }
                }
            }
        }
    }

    pub fn items(&self) -> Vec<Box<dyn crate::MenuItem>> {
        self.0
            .borrow()
            .entries
            .iter()
            .map(|e| -> Box<dyn crate::MenuItem> {
                let entry = e.borrow();
                match entry.type_ {
                    MenuItemType::Submenu(_) => Box::new(crate::Submenu(Submenu(e.clone()))),
                    MenuItemType::Normal(_) | MenuItemType::Predefined(_, _) => {
                        Box::new(crate::MenuItem(MenuItem(e.clone())))
                    }
                    MenuItemType::Check { .. } => {
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

        if inner.accel_group.is_none() {
            inner.accel_group = Some(Rc::new(gtk::AccelGroup::new()));
        }

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
        add_entries_to_gtkmenu(
            menu_bar,
            id,
            &inner.entries,
            inner.accel_group.as_ref().unwrap(),
            true,
        );
        window.add_accel_group(inner.accel_group.as_ref().unwrap().as_ref());

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
        let id = window.as_ptr() as u32;
        let menu_bar = {
            let inner = self.0.borrow();
            inner.native_menus.get(&id).map(|m| m.clone())
        };
        if let Some((Some(menu_bar), vbox)) = menu_bar {
            for item in self.items() {
                self.remove_gtk_by_parent_id(id, &*item);
            }

            let mut inner = self.0.borrow_mut();
            // Remove the [`gtk::Menubar`] from the widget tree
            unsafe { menu_bar.destroy() };
            // Detach the accelerators from the window
            window.remove_accel_group(inner.accel_group.as_ref().unwrap().as_ref());
            // Remove the removed [`gtk::Menubar`] from our cache
            let vbox = Rc::clone(&vbox);
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
            add_entries_to_gtkmenu(
                &gtk_menu,
                0,
                &self.0.borrow().entries,
                &Rc::new(accel_group),
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
pub(crate) struct Submenu(Rc<RefCell<MenuEntry>>);

impl Submenu {
    pub fn new(text: &str, enabled: bool) -> Self {
        let entry = Rc::new(RefCell::new(MenuEntry {
            text: text.to_string(),
            enabled,
            entries: Some(Vec::new()),
            type_: MenuItemType::Submenu(HashMap::new()),
            ..Default::default()
        }));

        Self(entry)
    }

    pub fn id(&self) -> u32 {
        self.0.borrow().id
    }

    pub fn append(&self, item: &dyn crate::MenuItem) {
        self.add_menu_item(item, AddOp::Append)
    }

    pub fn prepend(&self, item: &dyn crate::MenuItem) {
        self.add_menu_item(item, AddOp::Insert(0))
    }

    pub fn insert(&self, item: &dyn crate::MenuItem, position: usize) {
        self.add_menu_item(item, AddOp::Insert(position))
    }

    fn add_menu_item(&self, item: &dyn crate::MenuItem, op: AddOp) {
        let type_ = self.0.borrow().type_.clone();
        if let MenuItemType::Submenu(store) = &type_ {
            let entry = match item.type_() {
                crate::MenuItemType::Submenu => {
                    let item = item.as_any().downcast_ref::<crate::Submenu>().unwrap();
                    let entry = &item.0 .0;
                    for items in store.values() {
                        for (_, menu, accel_group, menu_id) in items {
                            add_gtk_submenu(menu, accel_group, *menu_id, entry, op, true);
                        }
                    }
                    entry
                }
                crate::MenuItemType::Normal => {
                    let item = item.as_any().downcast_ref::<crate::MenuItem>().unwrap();
                    let entry = &item.0 .0;
                    for items in store.values() {
                        for (_, menu, accel_group, menu_id) in items {
                            add_gtk_text_menuitem(menu, *menu_id, entry, accel_group, op, true);
                        }
                    }
                    entry
                }
                crate::MenuItemType::Predefined => {
                    let item = item
                        .as_any()
                        .downcast_ref::<crate::PredefinedMenuItem>()
                        .unwrap();
                    let entry = &item.0 .0;
                    for items in store.values() {
                        for (_, menu, accel_group, menu_id) in items {
                            add_gtk_predefined_menuitm(
                                menu,
                                *menu_id,
                                entry,
                                accel_group,
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
                    for items in store.values() {
                        for (_, menu, accel_group, menu_id) in items {
                            add_gtk_text_menuitem(menu, *menu_id, entry, accel_group, op, true);
                        }
                    }
                    entry
                }
            };

            let mut inner = self.0.borrow_mut();
            let entries = inner.entries.as_mut().unwrap();

            match op {
                AddOp::Append => entries.push(entry.clone()),
                AddOp::Insert(position) => entries.insert(position, entry.clone()),
            }
        }
    }

    pub fn remove(&self, item: &dyn crate::MenuItem) {
        if let MenuItemType::Submenu(store) = self.0.borrow().type_.clone() {
            match item.type_() {
                crate::MenuItemType::Submenu => {
                    let submenu = item.as_any().downcast_ref::<crate::Submenu>().unwrap();
                    let entry = &submenu.0 .0;
                    for items in store.values() {
                        for (_, menu, _, menu_id) in items {
                            for item in submenu.items() {
                                submenu.0.remove_gtk_by_parent_id(*menu_id, &*item);
                            }

                            if let MenuItemType::Submenu(store) = &mut entry.borrow_mut().type_ {
                                let items = store.remove(menu_id).unwrap();
                                for (item, _, _, _) in items {
                                    menu.remove(&item);
                                }
                            }
                        }
                    }
                }
                crate::MenuItemType::Normal => {
                    let item = item.as_any().downcast_ref::<crate::MenuItem>().unwrap();
                    let entry = &item.0 .0;
                    for items in store.values() {
                        for (_, menu, _, menu_id) in items {
                            if let MenuItemType::Normal(store) = &mut entry.borrow_mut().type_ {
                                let items = store.remove(menu_id).unwrap();
                                for item in items {
                                    menu.remove(&item);
                                }
                            }
                        }
                    }
                }
                crate::MenuItemType::Predefined => {
                    let item = item
                        .as_any()
                        .downcast_ref::<crate::PredefinedMenuItem>()
                        .unwrap();
                    let entry = &item.0 .0;
                    for items in store.values() {
                        for (_, menu, _, menu_id) in items {
                            if let MenuItemType::Predefined(store, _) =
                                &mut entry.borrow_mut().type_
                            {
                                if let Some(items) = store.remove(menu_id) {
                                    for item in items {
                                        menu.remove(&item);
                                    }
                                }
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
                    for items in store.values() {
                        for (_, menu, _, menu_id) in items {
                            if let MenuItemType::Check { store, .. } = &mut entry.borrow_mut().type_
                            {
                                let items = store.remove(menu_id).unwrap();
                                for item in items {
                                    menu.remove(&item);
                                }
                            }
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

    fn remove_gtk_by_parent_id(&self, parent_id: u32, item: &dyn crate::MenuItem) {
        if let MenuItemType::Submenu(store) = self.0.borrow().type_.clone() {
            match item.type_() {
                crate::MenuItemType::Submenu => {
                    let submenu = item.as_any().downcast_ref::<crate::Submenu>().unwrap();
                    let entry = &submenu.0 .0;
                    let items = store.get(&parent_id).unwrap();
                    for (_, menu, _, menu_id) in items {
                        for item in submenu.items() {
                            submenu.0.remove_gtk_by_parent_id(*menu_id, &*item);
                        }

                        if let MenuItemType::Submenu(store) = &mut entry.borrow_mut().type_ {
                            let items = store.remove(menu_id).unwrap();
                            for (item, _, _, _) in items {
                                menu.remove(&item);
                            }
                        }
                    }
                }
                crate::MenuItemType::Normal => {
                    let item = item.as_any().downcast_ref::<crate::MenuItem>().unwrap();
                    let entry = &item.0 .0;
                    let items = store.get(&parent_id).unwrap();
                    for (_, menu, _, menu_id) in items {
                        if let MenuItemType::Normal(store) = &mut entry.borrow_mut().type_ {
                            let items = store.remove(menu_id).unwrap();
                            for item in items {
                                menu.remove(&item);
                            }
                        }
                    }
                }
                crate::MenuItemType::Predefined => {
                    let item = item
                        .as_any()
                        .downcast_ref::<crate::PredefinedMenuItem>()
                        .unwrap();
                    let entry = &item.0 .0;
                    let items = store.get(&parent_id).unwrap();
                    for (_, menu, _, menu_id) in items {
                        if let MenuItemType::Predefined(store, _) = &mut entry.borrow_mut().type_ {
                            if let Some(items) = store.remove(menu_id) {
                                for item in items {
                                    menu.remove(&item);
                                }
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
                    let items = store.get(&parent_id).unwrap();
                    for (_, menu, _, menu_id) in items {
                        if let MenuItemType::Check { store, .. } = &mut entry.borrow_mut().type_ {
                            let items = store.remove(menu_id).unwrap();
                            for item in items {
                                menu.remove(&item);
                            }
                        }
                    }
                }
            };
        }
    }

    pub fn items(&self) -> Vec<Box<dyn crate::MenuItem>> {
        self.0
            .borrow()
            .entries
            .as_ref()
            .unwrap()
            .iter()
            .map(|e| -> Box<dyn crate::MenuItem> {
                let entry = e.borrow();
                match entry.type_ {
                    MenuItemType::Submenu(_) => Box::new(crate::Submenu(Submenu(e.clone()))),
                    MenuItemType::Normal(_) | MenuItemType::Predefined(_, _) => {
                        Box::new(crate::MenuItem(MenuItem(e.clone())))
                    }
                    MenuItemType::Check { .. } => {
                        Box::new(crate::CheckMenuItem(CheckMenuItem(e.clone())))
                    }
                }
            })
            .collect()
    }

    pub fn text(&self) -> String {
        let entry = self.0.borrow();
        if let MenuItemType::Submenu(store) = &entry.type_ {
            store
                .get(&0)
                .map(|items| items.get(0))
                .map(|i| {
                    i.map(|i| {
                        i.0.label()
                            .map(|l| l.as_str().to_string())
                            .map(from_gtk_mnemonic)
                            .unwrap_or_default()
                    })
                    .unwrap_or_else(|| entry.text.clone())
                })
                .unwrap_or_else(|| entry.text.clone())
        } else {
            unreachable!()
        }
    }

    pub fn set_text(&self, text: &str) {
        let mut entry = self.0.borrow_mut();
        entry.text = text.to_string();

        if let MenuItemType::Submenu(store) = &entry.type_ {
            let text = to_gtk_mnemonic(text);
            for items in store.values() {
                for (i, _, _, _) in items {
                    i.set_label(&text);
                }
            }
        } else {
            unreachable!()
        }
    }

    pub fn is_enabled(&self) -> bool {
        let entry = self.0.borrow();
        if let MenuItemType::Submenu(store) = &entry.type_ {
            store
                .get(&0)
                .map(|items| items.get(0))
                .map(|i| {
                    i.map(|i| i.0.is_sensitive())
                        .unwrap_or_else(|| entry.enabled)
                })
                .unwrap_or_else(|| entry.enabled)
        } else {
            unreachable!()
        }
    }

    pub fn set_enabled(&self, enabled: bool) {
        let mut entry = self.0.borrow_mut();
        entry.enabled = enabled;

        if let MenuItemType::Submenu(store) = &entry.type_ {
            for items in store.values() {
                for (i, _, _, _) in items {
                    i.set_sensitive(enabled);
                }
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
                &Rc::new(accel_group),
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
            type_: MenuItemType::Normal(HashMap::new()),
            ..Default::default()
        }));

        Self(entry)
    }

    pub fn id(&self) -> u32 {
        self.0.borrow().id
    }

    pub fn text(&self) -> String {
        let entry = self.0.borrow();
        if let MenuItemType::Normal(store) = &entry.type_ {
            store
                .get(&0)
                .map(|items| items.get(0))
                .map(|i| {
                    i.map(|i| {
                        i.label()
                            .map(|l| l.as_str().to_string())
                            .map(from_gtk_mnemonic)
                            .unwrap_or_default()
                    })
                    .unwrap_or_else(|| entry.text.clone())
                })
                .unwrap_or_else(|| entry.text.clone())
        } else {
            unreachable!()
        }
    }

    pub fn set_text(&self, text: &str) {
        let mut entry = self.0.borrow_mut();
        entry.text = text.to_string();

        if let MenuItemType::Normal(store) = &entry.type_ {
            let text = to_gtk_mnemonic(text);
            for items in store.values() {
                for i in items {
                    i.set_label(&text);
                }
            }
        } else {
            unreachable!()
        }
    }

    pub fn is_enabled(&self) -> bool {
        let entry = self.0.borrow();
        if let MenuItemType::Normal(store) = &entry.type_ {
            store
                .get(&0)
                .map(|items| items.get(0))
                .map(|i| i.map(|i| i.is_sensitive()).unwrap_or_else(|| entry.enabled))
                .unwrap_or_else(|| entry.enabled)
        } else {
            unreachable!()
        }
    }

    pub fn set_enabled(&self, enabled: bool) {
        let mut entry = self.0.borrow_mut();
        entry.enabled = enabled;

        if let MenuItemType::Normal(store) = &entry.type_ {
            for items in store.values() {
                for i in items {
                    i.set_sensitive(enabled);
                }
            }
        } else {
            unreachable!()
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct PredefinedMenuItem(Rc<RefCell<MenuEntry>>);

impl PredefinedMenuItem {
    pub fn new(item: PredfinedMenuItemType, text: Option<String>) -> Self {
        let entry = Rc::new(RefCell::new(MenuEntry {
            text: text.unwrap_or_else(|| item.text().to_string()),
            enabled: true,
            accelerator: item.accelerator(),
            id: COUNTER.next(),
            type_: MenuItemType::Predefined(HashMap::new(), item),
            ..Default::default()
        }));

        Self(entry)
    }

    pub fn id(&self) -> u32 {
        self.0.borrow().id
    }

    pub fn text(&self) -> String {
        let entry = self.0.borrow();
        if let MenuItemType::Predefined(store, _) = &entry.type_ {
            store
                .get(&0)
                .map(|items| items.get(0))
                .map(|i| {
                    i.map(|i| {
                        i.label()
                            .map(|l| l.as_str().to_string())
                            .map(from_gtk_mnemonic)
                            .unwrap_or_default()
                    })
                    .unwrap_or_else(|| entry.text.clone())
                })
                .unwrap_or_else(|| entry.text.clone())
        } else {
            unreachable!()
        }
    }

    pub fn set_text(&self, text: &str) {
        let mut entry = self.0.borrow_mut();
        entry.text = text.to_string();

        if let MenuItemType::Normal(store) = &entry.type_ {
            let text = to_gtk_mnemonic(text);
            for items in store.values() {
                for i in items {
                    i.set_label(&text);
                }
            }
        } else {
            unreachable!()
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
            type_: MenuItemType::Check {
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
        if let MenuItemType::Check { store, .. } = &entry.type_ {
            store
                .get(&0)
                .map(|items| items.get(0))
                .map(|i| {
                    i.map(|i| {
                        i.label()
                            .map(|l| l.as_str().to_string())
                            .map(from_gtk_mnemonic)
                            .unwrap_or_default()
                    })
                    .unwrap_or_else(|| entry.text.clone())
                })
                .unwrap_or_else(|| entry.text.clone())
        } else {
            unreachable!()
        }
    }

    pub fn set_text(&self, text: &str) {
        let mut entry = self.0.borrow_mut();
        entry.text = text.to_string();

        if let MenuItemType::Check { store, .. } = &entry.type_ {
            let text = to_gtk_mnemonic(text);
            for items in store.values() {
                for i in items {
                    i.set_label(&text);
                }
            }
        } else {
            unreachable!()
        }
    }

    pub fn is_enabled(&self) -> bool {
        let entry = self.0.borrow();
        if let MenuItemType::Check { store, .. } = &entry.type_ {
            store
                .get(&0)
                .map(|items| items.get(0))
                .map(|i| i.map(|i| i.is_sensitive()).unwrap_or(entry.enabled))
                .unwrap_or(entry.enabled)
        } else {
            unreachable!()
        }
    }

    pub fn set_enabled(&self, enabled: bool) {
        let mut entry = self.0.borrow_mut();
        entry.enabled = enabled;

        if let MenuItemType::Check { store, .. } = &entry.type_ {
            for items in store.values() {
                for i in items {
                    i.set_sensitive(enabled);
                }
            }
        } else {
            unreachable!()
        }
    }

    pub fn is_checked(&self) -> bool {
        let entry = self.0.borrow();
        if let MenuItemType::Check { store, .. } = &entry.type_ {
            store
                .get(&0)
                .map(|items| items.get(0))
                .map(|i| i.map(|i| i.is_active()).unwrap_or(entry.checked))
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

        if let MenuItemType::Check { store, is_syncing } = &type_ {
            is_syncing.store(true, Ordering::Release);
            for items in store.values() {
                for i in items {
                    i.set_active(checked);
                }
            }
            is_syncing.store(false, Ordering::Release);
        } else {
            unreachable!()
        }
    }
}

fn add_gtk_submenu(
    menu: &impl IsA<gtk::MenuShell>,
    accel_group: &Rc<gtk::AccelGroup>,
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
        AddOp::Insert(position) => menu.insert(&item, position as i32),
    }

    item.show();
    let id = COUNTER.next();
    add_entries_to_gtkmenu(
        &submenu,
        id,
        entry.entries.as_ref().unwrap(),
        accel_group,
        add_to_store,
    );
    if add_to_store {
        if let MenuItemType::Submenu(store) = &mut entry.type_ {
            let item = (item, submenu, accel_group.clone(), id);
            if let Some(items) = store.get_mut(&menu_id) {
                items.push(item);
            } else {
                store.insert(menu_id, vec![item]);
            }
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
    let mut entry = entry.borrow_mut();
    if let MenuItemType::Normal(_) = &entry.type_ {
        let item = gtk::MenuItem::builder()
            .label(&to_gtk_mnemonic(&entry.text))
            .use_underline(true)
            .sensitive(entry.enabled)
            .build();
        let id = entry.id;

        match op {
            AddOp::Append => menu.append(&item),
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
            if let MenuItemType::Normal(store) = &mut entry.type_ {
                if let Some(items) = store.get_mut(&menu_id) {
                    items.push(item);
                } else {
                    store.insert(menu_id, vec![item]);
                }
            }
        }
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

    if let MenuItemType::Predefined(store, predefined_item) = &mut entry.type_ {
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
            PredfinedMenuItemType::Separator => {
                Some(gtk::SeparatorMenuItem::new().upcast::<gtk::MenuItem>())
            }
            PredfinedMenuItemType::Copy
            | PredfinedMenuItemType::Cut
            | PredfinedMenuItemType::Paste
            | PredfinedMenuItemType::SelectAll => {
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
            PredfinedMenuItemType::About(metadata) => {
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
                AddOp::Insert(position) => menu.insert(&item, position as i32),
            }
            item.show();

            if add_to_store {
                if let Some(items) = store.get_mut(&menu_id) {
                    items.push(item);
                } else {
                    store.insert(menu_id, vec![item]);
                }
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
        let should_dispatch = matches!(&entry_c.borrow().type_, MenuItemType::Check { is_syncing, .. } if is_syncing
                       .compare_exchange(false, true, Ordering::Release, Ordering::Relaxed)
                             .is_ok());

        if should_dispatch {
            let checked = i.is_active();
            let type_ = {
                let mut entry = entry_c.borrow_mut();
                entry.checked = checked;
                entry.type_.clone()
            };

            if let MenuItemType::Check { store, .. } = &type_ {
                 for items in store.values() {
                for i in items {
                    i.set_active(checked);
                    }
                }
                if let MenuItemType::Check { is_syncing, .. } = &mut entry_c.borrow_mut().type_ {
                    is_syncing.store(false, Ordering::Release);
                }
            }

            let _ = crate::MENU_CHANNEL.0.send(crate::MenuEvent { id });
        }
    });

    match op {
        AddOp::Append => menu.append(&item),
        AddOp::Insert(position) => menu.insert(&item, position as i32),
    }

    item.show();

    if add_to_store {
        if let MenuItemType::Check { store, .. } = &mut entry.type_ {
            if let Some(items) = store.get_mut(&menu_id) {
                items.push(item);
            } else {
                store.insert(menu_id, vec![item]);
            }
        }
    }
}

fn add_entries_to_gtkmenu<M: IsA<gtk::MenuShell>>(
    menu: &M,
    menu_id: u32,
    entries: &Vec<Rc<RefCell<MenuEntry>>>,
    accel_group: &Rc<gtk::AccelGroup>,
    add_to_store: bool,
) {
    for entry in entries {
        let type_ = entry.borrow().type_.clone();
        match type_ {
            MenuItemType::Submenu(_) => add_gtk_submenu(
                menu,
                accel_group,
                menu_id,
                entry,
                AddOp::Append,
                add_to_store,
            ),
            MenuItemType::Normal(_) => add_gtk_text_menuitem(
                menu,
                menu_id,
                entry,
                accel_group,
                AddOp::Append,
                add_to_store,
            ),
            MenuItemType::Predefined(_, _) => add_gtk_predefined_menuitm(
                menu,
                menu_id,
                entry,
                accel_group,
                AddOp::Append,
                add_to_store,
            ),
            MenuItemType::Check { .. } => add_gtk_check_menuitem(
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

impl PredfinedMenuItemType {
    fn xdo_keys(&self) -> &str {
        match self {
            PredfinedMenuItemType::Copy => "ctrl+c",
            PredfinedMenuItemType::Cut => "ctrl+X",
            PredfinedMenuItemType::Paste => "ctrl+v",
            PredfinedMenuItemType::SelectAll => "ctrl+a",
            _ => "",
        }
    }
}
