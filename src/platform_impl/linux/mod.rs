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
    native_items: Option<Rc<RefCell<Vec<gtk::MenuItem>>>>,
    native_menus: Option<Rc<RefCell<Vec<(gtk::MenuItem, gtk::Menu)>>>>,
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
    accel_group: gtk::AccelGroup,
}

#[derive(Clone)]
pub struct Menu(Rc<RefCell<InnerMenu>>);

impl Menu {
    pub fn new() -> Self {
        Self(Rc::new(RefCell::new(InnerMenu {
            entries: Vec::new(),
            native_menus: HashMap::new(),
            accel_group: gtk::AccelGroup::new(),
        })))
    }

    pub fn add_submenu(&mut self, label: impl AsRef<str>, enabled: bool) -> Submenu {
        let entry = Rc::new(RefCell::new(MenuEntry {
            label: label.as_ref().to_string(),
            enabled,
            entries: Some(Vec::new()),
            r#type: MenuEntryType::Submenu,
            native_menus: Some(Rc::new(RefCell::new(Vec::new()))),
            ..Default::default()
        }));
        self.0.borrow_mut().entries.push(entry.clone());
        Submenu(entry)
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
        window.add_accel_group(&inner.accel_group);

        // Show the menubar on the window
        vbox.pack_start(menu_bar.as_ref().unwrap(), false, false, 0);
        vbox.show_all();

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
            window.remove_accel_group(&inner.accel_group);
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
pub struct Submenu(Rc<RefCell<MenuEntry>>);

impl Submenu {
    pub fn label(&self) -> String {
        self.0.borrow().label.clone()
    }

    pub fn set_label(&mut self, label: impl AsRef<str>) {
        let label = label.as_ref().to_string();
        let mut entry = self.0.borrow_mut();
        for (item, _) in entry.native_menus.as_ref().unwrap().borrow().iter() {
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
        for (item, _) in entry.native_menus.as_ref().unwrap().borrow().iter() {
            item.set_sensitive(enabled);
        }
    }

    pub fn add_submenu(&mut self, label: impl AsRef<str>, enabled: bool) -> Submenu {
        let entry = Rc::new(RefCell::new(MenuEntry {
            label: label.as_ref().to_string(),
            enabled,
            entries: Some(Vec::new()),
            r#type: MenuEntryType::Submenu,
            native_menus: Some(Rc::new(RefCell::new(Vec::new()))),
            ..Default::default()
        }));
        self.0
            .borrow_mut()
            .entries
            .as_mut()
            .unwrap()
            .push(entry.clone());
        Submenu(entry)
    }

    pub fn add_text_item(
        &mut self,
        label: impl AsRef<str>,
        enabled: bool,
        accelerator: Option<&str>,
    ) -> TextMenuItem {
        let entry = Rc::new(RefCell::new(MenuEntry {
            label: label.as_ref().to_string(),
            enabled,
            r#type: MenuEntryType::Text,
            item_id: Some(COUNTER.next()),
            accelerator: accelerator.map(|s| s.to_string()),
            native_items: Some(Rc::new(RefCell::new(Vec::new()))),
            ..Default::default()
        }));
        self.0
            .borrow_mut()
            .entries
            .as_mut()
            .unwrap()
            .push(entry.clone());
        TextMenuItem(entry)
    }

    pub fn add_native_item(&mut self, item: NativeMenuItem) {
        let entry = Rc::new(RefCell::new(MenuEntry {
            r#type: MenuEntryType::Native,
            native_menu_item: Some(item),
            ..Default::default()
        }));
        self.0.borrow_mut().entries.as_mut().unwrap().push(entry);
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
        for item in entry.native_items.as_ref().unwrap().borrow().iter() {
            item.set_label(&to_gtk_menemenoic(&label));
        }
        entry.label = label;
    }

    pub fn enabled(&self) -> bool {
        self.0.borrow().enabled
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        let mut entry = self.0.borrow_mut();
        for item in entry.native_items.as_ref().unwrap().borrow().iter() {
            item.set_sensitive(enabled);
        }
        entry.enabled = enabled;
    }

    pub fn id(&self) -> u64 {
        self.0.borrow().item_id.unwrap()
    }
}

fn add_entries_to_menu<M>(
    gtk_menu: &M,
    entries: &Vec<Rc<RefCell<MenuEntry>>>,
    accel_group: &gtk::AccelGroup,
) where
    M: IsA<gtk::MenuShell>,
{
    for entry in entries {
        let mut entry = entry.borrow_mut();
        match entry.r#type {
            MenuEntryType::Submenu => {
                let gtk_item = gtk::MenuItem::with_mnemonic(&to_gtk_menemenoic(&entry.label));
                gtk_menu.append(&gtk_item);
                gtk_item.set_sensitive(entry.enabled);
                let gtk_menu = gtk::Menu::new();
                gtk_item.set_submenu(Some(&gtk_menu));
                add_entries_to_menu(&gtk_menu, entry.entries.as_ref().unwrap(), accel_group);
                entry
                    .native_menus
                    .as_mut()
                    .unwrap()
                    .borrow_mut()
                    .push((gtk_item, gtk_menu));
            }
            MenuEntryType::Text => {
                let gtk_item = gtk::MenuItem::with_mnemonic(&to_gtk_menemenoic(&entry.label));
                gtk_menu.append(&gtk_item);
                gtk_item.set_sensitive(entry.enabled);
                if let Some(accelerator) = &entry.accelerator {
                    let (key, modifiers) = gtk::accelerator_parse(&to_gtk_accelerator(accelerator));
                    gtk_item.add_accelerator(
                        "activate",
                        accel_group,
                        key,
                        modifiers,
                        gtk::AccelFlags::VISIBLE,
                    );
                }

                let id = entry.item_id.unwrap();
                gtk_item.connect_activate(move |_| {
                    let _ = crate::MENU_CHANNEL.0.send(crate::MenuEvent { id });
                });
                entry
                    .native_items
                    .as_mut()
                    .unwrap()
                    .borrow_mut()
                    .push(gtk_item);
            }
            MenuEntryType::Native => match entry.native_menu_item.as_ref().unwrap() {
                NativeMenuItem::Copy => {
                    let gtk_item = gtk::MenuItem::with_mnemonic("_Copy");
                    let (key, modifiers) = gtk::accelerator_parse("<Ctrl>X");
                    gtk_item
                        .child()
                        .unwrap()
                        .downcast::<gtk::AccelLabel>()
                        .unwrap()
                        .set_accel(key, modifiers);
                    gtk_item.connect_activate(move |_| {
                        // TODO: wayland
                        if let Ok(xdo) = libxdo::XDo::new(None) {
                            let _ = xdo.send_keysequence("ctrl+c", 0);
                        }
                    });
                    gtk_menu.append(&gtk_item);
                }
                NativeMenuItem::Cut => {
                    let gtk_item = gtk::MenuItem::with_mnemonic("Cu_t");
                    let (key, modifiers) = gtk::accelerator_parse("<Ctrl>X");
                    gtk_item
                        .child()
                        .unwrap()
                        .downcast::<gtk::AccelLabel>()
                        .unwrap()
                        .set_accel(key, modifiers);
                    gtk_item.connect_activate(move |_| {
                        // TODO: wayland
                        if let Ok(xdo) = libxdo::XDo::new(None) {
                            let _ = xdo.send_keysequence("ctrl+x", 0);
                        }
                    });
                    gtk_menu.append(&gtk_item);
                }
                NativeMenuItem::Paste => {
                    let gtk_item = gtk::MenuItem::with_mnemonic("_Paste");
                    let (key, modifiers) = gtk::accelerator_parse("<Ctrl>V");
                    gtk_item
                        .child()
                        .unwrap()
                        .downcast::<gtk::AccelLabel>()
                        .unwrap()
                        .set_accel(key, modifiers);
                    gtk_item.connect_activate(move |_| {
                        // TODO: wayland
                        if let Ok(xdo) = libxdo::XDo::new(None) {
                            let _ = xdo.send_keysequence("ctrl+v", 0);
                        }
                    });
                    gtk_menu.append(&gtk_item);
                }
                NativeMenuItem::SelectAll => {
                    let gtk_item = gtk::MenuItem::with_mnemonic("Select _All");
                    let (key, modifiers) = gtk::accelerator_parse("<Ctrl>A");
                    gtk_item
                        .child()
                        .unwrap()
                        .downcast::<gtk::AccelLabel>()
                        .unwrap()
                        .set_accel(key, modifiers);
                    gtk_item.connect_activate(move |_| {
                        // TODO: wayland
                        if let Ok(xdo) = libxdo::XDo::new(None) {
                            let _ = xdo.send_keysequence("ctrl+a", 0);
                        }
                    });
                    gtk_menu.append(&gtk_item);
                }
                NativeMenuItem::Separator => {
                    gtk_menu.append(&gtk::SeparatorMenuItem::new());
                }
                NativeMenuItem::Minimize => {
                    let gtk_item = gtk::MenuItem::with_mnemonic("_Minimize");
                    gtk_item.connect_activate(move |m| {
                        if let Some(window) = m.window() {
                            window.iconify()
                        }
                    });
                    gtk_menu.append(&gtk_item);
                }
                NativeMenuItem::CloseWindow => {
                    let gtk_item = gtk::MenuItem::with_mnemonic("C_lose Window");
                    gtk_item.connect_activate(move |m| {
                        if let Some(window) = m.window() {
                            window.destroy()
                        }
                    });
                    gtk_menu.append(&gtk_item);
                }
                NativeMenuItem::Quit => {
                    let gtk_item = gtk::MenuItem::with_mnemonic("_Quit");
                    gtk_item.connect_activate(move |_| {
                        std::process::exit(0);
                    });
                    gtk_menu.append(&gtk_item);
                }
                _ => {}
            },
        }
    }
}
