mod accelerator;

use crate::{counter::Counter, NativeMenuItem};
use accelerator::{to_gtk_accelerator, to_gtk_menemenoic};
use gtk::{prelude::*, Orientation};
use std::{cell::RefCell, collections::HashMap, rc::Rc};

static COUNTER: Counter = Counter::new();

/// Generic shared type describing a menu entry. It can be one of [`MenuEntryType`]
#[derive(Debug, Default)]
struct MenuEntry {
    label: String,
    enabled: bool,
    r#type: MenuEntryType,
    item_id: Option<u64>,
    accelerator: Option<String>,
    native_menu_item: Option<NativeMenuItem>,
    // NOTE(amrbashir): because gtk doesn't allow using the same [`gtk::MenuItem`]
    // multiple times, and thus can't be used in multiple windows, each entry
    // keeps a vector of a [`gtk::MenuItem`] or a tuple of [`gtk::MenuItem`] and [`gtk::Menu`] if its a menu
    // and push to it every time [`Menu::init_for_gtk_window`] is called.
    native_items: Option<Vec<gtk::MenuItem>>,
    native_menus: Option<Vec<(gtk::MenuItem, gtk::Menu)>>,
    entries: Option<Vec<Rc<RefCell<MenuEntry>>>>,
}

#[derive(PartialEq, Eq, Debug)]
enum MenuEntryType {
    Submenu,
    Text,
    Native,
}

impl Default for MenuEntryType {
    fn default() -> Self {
        MenuEntryType::Text
    }
}

struct InnerMenu {
    entries: Vec<Rc<RefCell<MenuEntry>>>,
    // NOTE(amrbashir): because gtk doesn't allow using the same [`gtk::MenuBar`] and [`gtk::Box`]
    // multiple times, and thus can't be used in multiple windows. each menu
    // keeps a hashmap of window pointer as the key and a tuple of [`gtk::MenuBar`] and [`gtk::Box`] as the value
    // and push to it every time `Menu::init_for_gtk_window` is called.
    native_menus: HashMap<isize, (Option<gtk::MenuBar>, Rc<gtk::Box>)>,
    accel_group: Rc<gtk::AccelGroup>,
}

#[derive(Clone)]
pub struct Menu(Rc<RefCell<InnerMenu>>);

impl Menu {
    pub fn new() -> Self {
        Self(Rc::new(RefCell::new(InnerMenu {
            entries: Vec::new(),
            native_menus: HashMap::new(),
            accel_group: Rc::new(gtk::AccelGroup::new()),
        })))
    }

    pub fn add_submenu(&mut self, label: impl AsRef<str>, enabled: bool) -> Submenu {
        let label = label.as_ref().to_string();

        let entry = Rc::new(RefCell::new(MenuEntry {
            label: label.clone(),
            enabled,
            entries: Some(Vec::new()),
            r#type: MenuEntryType::Submenu,
            native_menus: Some(Vec::new()),
            ..Default::default()
        }));

        let mut inner = self.0.borrow_mut();
        for (_, (menu_bar, _)) in inner.native_menus.iter() {
            if let Some(menu_bar) = menu_bar {
                let (item, submenu) = create_gtk_submenu(&label, enabled);
                menu_bar.append(&item);
                entry
                    .borrow_mut()
                    .native_menus
                    .as_mut()
                    .unwrap()
                    .push((item, submenu));
            }
        }

        inner.entries.push(entry.clone());
        Submenu(entry, Rc::clone(&inner.accel_group))
    }

    pub fn init_for_gtk_window<W>(&self, window: &W) -> Rc<gtk::Box>
    where
        W: IsA<gtk::ApplicationWindow>,
        W: IsA<gtk::Container>,
        W: IsA<gtk::Window>,
    {
        let mut inner = self.0.borrow_mut();

        // This is the first time this method has been called on this window
        // so we need to create the menubar and its parent box
        if inner.native_menus.get(&(window.as_ptr() as _)).is_none() {
            let menu_bar = gtk::MenuBar::new();
            let vbox = gtk::Box::new(Orientation::Vertical, 0);
            window.add(&vbox);
            vbox.show();
            inner
                .native_menus
                .insert(window.as_ptr() as _, (Some(menu_bar), Rc::new(vbox)));
        }

        if let Some((menu_bar, vbox)) = inner.native_menus.get(&(window.as_ptr() as _)) {
            // This is NOT the first time this method has been called on a window.
            // So it already contains a [`gtk::Box`] but it doesn't have a [`gtk::MenuBar`]
            // because it was probably removed using [`Menu::remove_for_gtk_window`]
            // so we only need to create the menubar
            if menu_bar.is_none() {
                let vbox = Rc::clone(vbox);
                inner
                    .native_menus
                    .insert(window.as_ptr() as _, (Some(gtk::MenuBar::new()), vbox));
            }
        }

        // Construct the entries of the menubar
        let (menu_bar, vbox) = inner.native_menus.get(&(window.as_ptr() as _)).unwrap();
        add_entries_to_menu(
            menu_bar.as_ref().unwrap(),
            &inner.entries,
            &inner.accel_group,
        );
        window.add_accel_group(&*inner.accel_group);

        // Show the menubar on the window
        vbox.pack_start(menu_bar.as_ref().unwrap(), false, false, 0);
        menu_bar.as_ref().unwrap().show();

        Rc::clone(vbox)
    }

    pub fn remove_for_gtk_window<W>(&self, window: &W)
    where
        W: IsA<gtk::ApplicationWindow>,
        W: IsA<gtk::Window>,
    {
        let mut inner = self.0.borrow_mut();

        if let Some((Some(menu_bar), vbox)) = inner.native_menus.get(&(window.as_ptr() as _)) {
            // Remove the [`gtk::Menubar`] from the widget tree
            unsafe { menu_bar.destroy() };
            // Detach the accelerators from the window
            window.remove_accel_group(&*inner.accel_group);
            // Remove the removed [`gtk::Menubar`] from our cache
            let vbox = Rc::clone(vbox);
            inner
                .native_menus
                .insert(window.as_ptr() as _, (None, vbox));
        }
    }

    pub fn hide_for_gtk_window<W>(&self, window: &W)
    where
        W: IsA<gtk::ApplicationWindow>,
    {
        if let Some((Some(menu_bar), _)) = self
            .0
            .borrow()
            .native_menus
            .get(&(window.as_ptr() as isize))
        {
            menu_bar.hide();
        }
    }

    pub fn show_for_gtk_window<W>(&self, window: &W)
    where
        W: IsA<gtk::ApplicationWindow>,
    {
        if let Some((Some(menu_bar), _)) = self
            .0
            .borrow()
            .native_menus
            .get(&(window.as_ptr() as isize))
        {
            menu_bar.show_all();
        }
    }
}

#[derive(Clone)]
pub struct Submenu(Rc<RefCell<MenuEntry>>, Rc<gtk::AccelGroup>);

impl Submenu {
    pub fn label(&self) -> String {
        self.0.borrow().label.clone()
    }

    pub fn set_label(&mut self, label: impl AsRef<str>) {
        let label = label.as_ref().to_string();
        let mut entry = self.0.borrow_mut();
        for (item, _) in entry.native_menus.as_ref().unwrap() {
            item.set_label(&to_gtk_menemenoic(&label));
        }
        entry.label = label;
    }

    pub fn enabled(&self) -> bool {
        self.0.borrow().enabled
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        let mut entry = self.0.borrow_mut();
        entry.enabled = true;
        for (item, _) in entry.native_menus.as_ref().unwrap() {
            item.set_sensitive(enabled);
        }
    }

    pub fn add_submenu(&mut self, label: impl AsRef<str>, enabled: bool) -> Submenu {
        let label = label.as_ref().to_string();

        let entry = Rc::new(RefCell::new(MenuEntry {
            label: label.clone(),
            enabled,
            entries: Some(Vec::new()),
            r#type: MenuEntryType::Submenu,
            native_menus: Some(Vec::new()),
            ..Default::default()
        }));

        let mut inner = self.0.borrow_mut();
        for (_, menu) in inner.native_menus.as_ref().unwrap() {
            let (item, submenu) = create_gtk_submenu(&label, enabled);
            menu.append(&item);
            entry
                .borrow_mut()
                .native_menus
                .as_mut()
                .unwrap()
                .push((item, submenu));
        }

        inner.entries.as_mut().unwrap().push(entry.clone());
        Submenu(entry, Rc::clone(&self.1))
    }

    pub fn add_text_item(
        &mut self,
        label: impl AsRef<str>,
        enabled: bool,
        accelerator: Option<&str>,
    ) -> TextMenuItem {
        let label = label.as_ref().to_string();
        let id = COUNTER.next();

        let entry = Rc::new(RefCell::new(MenuEntry {
            label: label.clone(),
            enabled,
            r#type: MenuEntryType::Text,
            item_id: Some(id),
            accelerator: accelerator.map(|s| s.to_string()),
            native_items: Some(Vec::new()),
            ..Default::default()
        }));

        let mut inner = self.0.borrow_mut();

        for (_, menu) in inner.native_menus.as_ref().unwrap() {
            let item = create_gtk_text_menu_item(
                &label,
                enabled,
                &accelerator.map(|s| s.to_string()),
                id,
                &*self.1,
            );
            menu.append(&item);
            entry.borrow_mut().native_items.as_mut().unwrap().push(item);
        }

        inner.entries.as_mut().unwrap().push(entry.clone());
        TextMenuItem(entry)
    }

    pub fn add_native_item(&mut self, item: NativeMenuItem) {
        let mut inner = self.0.borrow_mut();

        for (_, menu) in inner.native_menus.as_ref().unwrap() {
            item.add_to_gtk_menu(menu);
        }

        let entry = Rc::new(RefCell::new(MenuEntry {
            r#type: MenuEntryType::Native,
            native_menu_item: Some(item),
            ..Default::default()
        }));
        inner.entries.as_mut().unwrap().push(entry);
    }
}

#[derive(Clone)]
pub struct TextMenuItem(Rc<RefCell<MenuEntry>>);

impl TextMenuItem {
    pub fn label(&self) -> String {
        self.0.borrow().label.clone()
    }

    pub fn set_label(&mut self, label: impl AsRef<str>) {
        let label = label.as_ref().to_string();
        let mut entry = self.0.borrow_mut();
        for item in entry.native_items.as_ref().unwrap() {
            item.set_label(&to_gtk_menemenoic(&label));
        }
        entry.label = label;
    }

    pub fn enabled(&self) -> bool {
        self.0.borrow().enabled
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        let mut entry = self.0.borrow_mut();
        for item in entry.native_items.as_ref().unwrap() {
            item.set_sensitive(enabled);
        }
        entry.enabled = enabled;
    }

    pub fn id(&self) -> u64 {
        self.0.borrow().item_id.unwrap()
    }
}

fn add_entries_to_menu<M: IsA<gtk::MenuShell>>(
    gtk_menu: &M,
    entries: &Vec<Rc<RefCell<MenuEntry>>>,
    accel_group: &gtk::AccelGroup,
) {
    for entry in entries {
        let mut entry = entry.borrow_mut();
        match entry.r#type {
            MenuEntryType::Submenu => {
                let (item, submenu) = create_gtk_submenu(&entry.label, entry.enabled);
                gtk_menu.append(&item);
                add_entries_to_menu(&submenu, entry.entries.as_ref().unwrap(), accel_group);
                entry.native_menus.as_mut().unwrap().push((item, submenu));
            }
            MenuEntryType::Text => {
                let item = create_gtk_text_menu_item(
                    &entry.label,
                    entry.enabled,
                    &entry.accelerator,
                    entry.item_id.unwrap(),
                    accel_group,
                );
                gtk_menu.append(&item);
                entry.native_items.as_mut().unwrap().push(item);
            }
            MenuEntryType::Native => entry
                .native_menu_item
                .as_ref()
                .unwrap()
                .add_to_gtk_menu(gtk_menu),
        }
    }
}

fn create_gtk_submenu(label: &str, enabled: bool) -> (gtk::MenuItem, gtk::Menu) {
    let item = gtk::MenuItem::with_mnemonic(&to_gtk_menemenoic(label));
    item.set_sensitive(enabled);
    let menu = gtk::Menu::new();
    item.set_submenu(Some(&menu));
    item.show();
    (item, menu)
}

fn create_gtk_text_menu_item(
    label: &str,
    enabled: bool,
    accelerator: &Option<String>,
    id: u64,
    accel_group: &gtk::AccelGroup,
) -> gtk::MenuItem {
    let item = gtk::MenuItem::with_mnemonic(&to_gtk_menemenoic(label));
    item.set_sensitive(enabled);
    if let Some(accelerator) = accelerator {
        let (key, modifiers) = gtk::accelerator_parse(&to_gtk_accelerator(accelerator));
        item.add_accelerator(
            "activate",
            accel_group,
            key,
            modifiers,
            gtk::AccelFlags::VISIBLE,
        );
    }
    item.connect_activate(move |_| {
        let _ = crate::MENU_CHANNEL.0.send(crate::MenuEvent { id });
    });
    item.show();
    item
}

impl NativeMenuItem {
    fn add_to_gtk_menu<M: IsA<gtk::MenuShell>>(&self, gtk_menu: &M) {
        match self {
            NativeMenuItem::Copy => {
                let item = gtk::MenuItem::with_mnemonic("_Copy");
                let (key, modifiers) = gtk::accelerator_parse("<Ctrl>X");
                item.child()
                    .unwrap()
                    .downcast::<gtk::AccelLabel>()
                    .unwrap()
                    .set_accel(key, modifiers);
                item.connect_activate(move |_| {
                    // TODO: wayland
                    if let Ok(xdo) = libxdo::XDo::new(None) {
                        let _ = xdo.send_keysequence("ctrl+c", 0);
                    }
                });
                item.show();
                gtk_menu.append(&item);
            }
            NativeMenuItem::Cut => {
                let item = gtk::MenuItem::with_mnemonic("Cu_t");
                let (key, modifiers) = gtk::accelerator_parse("<Ctrl>X");
                item.child()
                    .unwrap()
                    .downcast::<gtk::AccelLabel>()
                    .unwrap()
                    .set_accel(key, modifiers);
                item.connect_activate(move |_| {
                    // TODO: wayland
                    if let Ok(xdo) = libxdo::XDo::new(None) {
                        let _ = xdo.send_keysequence("ctrl+x", 0);
                    }
                });
                item.show();
                gtk_menu.append(&item);
            }
            NativeMenuItem::Paste => {
                let item = gtk::MenuItem::with_mnemonic("_Paste");
                let (key, modifiers) = gtk::accelerator_parse("<Ctrl>V");
                item.child()
                    .unwrap()
                    .downcast::<gtk::AccelLabel>()
                    .unwrap()
                    .set_accel(key, modifiers);
                item.connect_activate(move |_| {
                    // TODO: wayland
                    if let Ok(xdo) = libxdo::XDo::new(None) {
                        let _ = xdo.send_keysequence("ctrl+v", 0);
                    }
                });
                item.show();
                gtk_menu.append(&item);
            }
            NativeMenuItem::SelectAll => {
                let item = gtk::MenuItem::with_mnemonic("Select _All");
                let (key, modifiers) = gtk::accelerator_parse("<Ctrl>A");
                item.child()
                    .unwrap()
                    .downcast::<gtk::AccelLabel>()
                    .unwrap()
                    .set_accel(key, modifiers);
                item.connect_activate(move |_| {
                    // TODO: wayland
                    if let Ok(xdo) = libxdo::XDo::new(None) {
                        let _ = xdo.send_keysequence("ctrl+a", 0);
                    }
                });
                item.show();
                gtk_menu.append(&item);
            }
            NativeMenuItem::Separator => {
                gtk_menu.append(&gtk::SeparatorMenuItem::new());
            }
            NativeMenuItem::Minimize => {
                let item = gtk::MenuItem::with_mnemonic("_Minimize");
                item.connect_activate(move |m| {
                    if let Some(window) = m.window() {
                        window.iconify()
                    }
                });
                item.show();
                gtk_menu.append(&item);
            }
            NativeMenuItem::CloseWindow => {
                let item = gtk::MenuItem::with_mnemonic("C_lose Window");
                item.connect_activate(move |m| {
                    if let Some(window) = m.window() {
                        window.destroy()
                    }
                });
                item.show();
                gtk_menu.append(&item);
            }
            NativeMenuItem::Quit => {
                let item = gtk::MenuItem::with_mnemonic("_Quit");
                item.connect_activate(move |_| {
                    std::process::exit(0);
                });
                item.show();
                gtk_menu.append(&item);
            }
            _ => {}
        }
    }
}
