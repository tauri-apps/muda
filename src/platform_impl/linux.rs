use parking_lot::Mutex;
use std::sync::Arc;

use gtk::{prelude::*, Orientation};

use crate::util::Counter;

const COUNTER: Counter = Counter::new();

enum MenuEntryType {
    Submenu,
    Text,
}

struct MenuEntry {
    label: String,
    enabled: bool,
    entries: Option<Vec<Arc<Mutex<MenuEntry>>>>,
    etype: MenuEntryType,
    item_id: Option<u64>,
    menu_gtk_items: Option<Arc<Mutex<Vec<(gtk::MenuItem, gtk::Menu)>>>>,
    item_gtk_items: Option<Arc<Mutex<Vec<gtk::MenuItem>>>>,
}

struct InnerMenu {
    entries: Vec<Arc<Mutex<MenuEntry>>>,
    gtk_items: Vec<(gtk::MenuBar, gtk::Box)>,
}

pub struct Menu(Arc<Mutex<InnerMenu>>);

impl Menu {
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(InnerMenu {
            entries: Vec::new(),
            gtk_items: Vec::new(),
        })))
    }

    pub fn add_submenu(&mut self, label: impl AsRef<str>, enabled: bool) -> Submenu {
        let label = label.as_ref().to_string();
        let gtk_items = Arc::new(Mutex::new(Vec::new()));
        let entry = Arc::new(Mutex::new(MenuEntry {
            label: label.clone(),
            enabled,
            entries: Some(Vec::new()),
            etype: MenuEntryType::Submenu,
            item_id: None,
            menu_gtk_items: Some(gtk_items.clone()),
            item_gtk_items: None,
        }));
        self.0.lock().entries.push(entry.clone());
        Submenu {
            label,
            enabled,
            entry,
            gtk_items,
        }
    }

    pub fn init_for_gtk_window<W>(&self, w: &W)
    where
        W: IsA<gtk::Container>,
    {
        let menu_bar = gtk::MenuBar::new();
        add_entries_to_menu(&menu_bar, &self.0.lock().entries);

        let vbox = gtk::Box::new(Orientation::Vertical, 0);
        vbox.pack_start(&menu_bar, false, false, 0);
        w.add(&vbox);
        vbox.show_all();

        self.0.lock().gtk_items.push((menu_bar, vbox));
    }
}

fn add_entries_to_menu<M: IsA<gtk::MenuShell>>(gtk_menu: &M, entries: &Vec<Arc<Mutex<MenuEntry>>>) {
    for entry in entries {
        let mut entry = entry.lock();
        let gtk_item = gtk::MenuItem::with_label(&entry.label);
        gtk_menu.append(&gtk_item);
        gtk_item.set_sensitive(entry.enabled);
        if let MenuEntryType::Submenu = entry.etype {
            let gtk_menu = gtk::Menu::new();
            gtk_item.set_submenu(Some(&gtk_menu));
            add_entries_to_menu(&gtk_menu, entry.entries.as_ref().unwrap());
            entry
                .menu_gtk_items
                .as_mut()
                .unwrap()
                .lock()
                .push((gtk_item, gtk_menu));
        } else {
            let id = entry.item_id.unwrap_or_default();
            gtk_item.connect_activate(move |_| {
                let _ = crate::MENU_CHANNEL.0.send(crate::MenuEvent { id });
            });
            entry.item_gtk_items.as_mut().unwrap().lock().push(gtk_item);
        }
    }
}

#[derive(Clone)]
pub struct Submenu {
    label: String,
    enabled: bool,
    entry: Arc<Mutex<MenuEntry>>,
    gtk_items: Arc<Mutex<Vec<(gtk::MenuItem, gtk::Menu)>>>,
}

impl Submenu {
    pub fn set_label(&mut self, label: impl AsRef<str>) {
        let label = label.as_ref().to_string();
        for (item, _) in self.gtk_items.lock().iter() {
            item.set_label(&label);
        }

        self.label = label.clone();
        self.entry.lock().label = label;
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        for (item, _) in self.gtk_items.lock().iter() {
            item.set_sensitive(enabled);
        }

        self.enabled = enabled;
        self.entry.lock().enabled = enabled;
    }

    pub fn add_submenu(&mut self, label: impl AsRef<str>, enabled: bool) -> Submenu {
        let label = label.as_ref().to_string();
        let gtk_items = Arc::new(Mutex::new(Vec::new()));
        let entry = Arc::new(Mutex::new(MenuEntry {
            label: label.clone(),
            enabled,
            entries: Some(Vec::new()),
            etype: MenuEntryType::Submenu,
            item_id: None,
            menu_gtk_items: Some(gtk_items.clone()),
            item_gtk_items: None,
        }));
        self.entry
            .lock()
            .entries
            .as_mut()
            .unwrap()
            .push(entry.clone());
        Submenu {
            label,
            enabled,
            entry,
            gtk_items,
        }
    }

    pub fn add_text_item(&mut self, label: impl AsRef<str>, enabled: bool) -> TextMenuItem {
        let id = COUNTER.next();
        let label = label.as_ref().to_string();
        let gtk_items = Arc::new(Mutex::new(Vec::new()));
        let entry = Arc::new(Mutex::new(MenuEntry {
            label: label.clone(),
            enabled,
            entries: None,
            etype: MenuEntryType::Text,
            item_id: Some(id),
            menu_gtk_items: None,
            item_gtk_items: Some(gtk_items.clone()),
        }));
        self.entry
            .lock()
            .entries
            .as_mut()
            .unwrap()
            .push(entry.clone());
        TextMenuItem {
            label,
            enabled,
            entry,
            gtk_items,
            id,
        }
    }
}

#[derive(Clone)]
pub struct TextMenuItem {
    label: String,
    enabled: bool,
    entry: Arc<Mutex<MenuEntry>>,
    gtk_items: Arc<Mutex<Vec<gtk::MenuItem>>>,
    id: u64,
}

impl TextMenuItem {
    pub fn set_label(&mut self, label: impl AsRef<str>) {
        let label = label.as_ref().to_string();
        for item in self.gtk_items.lock().iter() {
            item.set_label(&label);
        }

        self.label = label.clone();
        self.entry.lock().label = label;
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        for item in self.gtk_items.lock().iter() {
            item.set_sensitive(enabled);
        }

        self.enabled = enabled;
        self.entry.lock().enabled = enabled;
    }

    pub fn id(&self) -> u64 {
        self.id
    }
}
