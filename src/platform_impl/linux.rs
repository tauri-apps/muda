use crate::util::Counter;
use gtk::{prelude::*, Orientation};
use parking_lot::Mutex;
use std::sync::Arc;

const COUNTER: Counter = Counter::new();

enum MenuEntryType {
    Submenu,
    Text,
}

/// Generic shared type describing a menu entry. It can be one of [`MenuEntryType`]
struct MenuEntry {
    label: String,
    enabled: bool,
    r#type: MenuEntryType,
    item_id: Option<u64>,
    // NOTE(amrbashir): because gtk doesn't allow using the same `gtk::MenuItem`
    // multiple times, and thus can't be used in multiple windows, each entry
    // keeps a vector of a `gtk::MenuItem` or a tuple of `gtk::MenuItem` and `gtk::Menu`
    // and push to it every time `Menu::init_for_gtk_window` is called.
    item_gtk_items: Option<Arc<Mutex<Vec<gtk::MenuItem>>>>,
    menu_gtk_items: Option<Arc<Mutex<Vec<(gtk::MenuItem, gtk::Menu)>>>>,
    entries: Option<Vec<Arc<Mutex<MenuEntry>>>>,
}

struct InnerMenu {
    entries: Vec<Arc<Mutex<MenuEntry>>>,
    // NOTE(amrbashir): because gtk doesn't allow using the same `gtk::MenuBar` and `gtk::Box`
    // multiple times, and thus can't be used in multiple windows, entry
    // keeps a vector of a tuple of `gtk::MenuBar` and `gtk::Box`
    // and push to it every time `Menu::init_for_gtk_window` is called.
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
        let entry = Arc::new(Mutex::new(MenuEntry {
            label: label.clone(),
            enabled,
            entries: Some(Vec::new()),
            r#type: MenuEntryType::Submenu,
            item_id: None,
            menu_gtk_items: Some(Arc::new(Mutex::new(Vec::new()))),
            item_gtk_items: None,
        }));
        self.0.lock().entries.push(entry.clone());
        Submenu(entry)
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
        if let MenuEntryType::Submenu = entry.r#type {
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
pub struct Submenu(Arc<Mutex<MenuEntry>>);

impl Submenu {
    pub fn set_label(&mut self, label: impl AsRef<str>) {
        let label = label.as_ref().to_string();
        let mut entry = self.0.lock();
        for (item, _) in entry.menu_gtk_items.as_ref().unwrap().lock().iter() {
            item.set_label(&label);
        }
        entry.label = label;
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        let mut entry = self.0.lock();
        entry.enabled = true;
        for (item, _) in entry.menu_gtk_items.as_ref().unwrap().lock().iter() {
            item.set_sensitive(enabled);
        }
    }

    pub fn add_submenu(&mut self, label: impl AsRef<str>, enabled: bool) -> Submenu {
        let entry = Arc::new(Mutex::new(MenuEntry {
            label: label.as_ref().to_string(),
            enabled,
            entries: Some(Vec::new()),
            r#type: MenuEntryType::Submenu,
            item_id: None,
            menu_gtk_items: Some(Arc::new(Mutex::new(Vec::new()))),
            item_gtk_items: None,
        }));
        self.0.lock().entries.as_mut().unwrap().push(entry.clone());
        Submenu(entry)
    }

    pub fn add_text_item(&mut self, label: impl AsRef<str>, enabled: bool) -> TextMenuItem {
        let entry = Arc::new(Mutex::new(MenuEntry {
            label: label.as_ref().to_string(),
            enabled,
            entries: None,
            r#type: MenuEntryType::Text,
            item_id: Some(COUNTER.next()),
            menu_gtk_items: None,
            item_gtk_items: Some(Arc::new(Mutex::new(Vec::new()))),
        }));
        self.0.lock().entries.as_mut().unwrap().push(entry.clone());
        TextMenuItem(entry)
    }
}

#[derive(Clone)]
pub struct TextMenuItem(Arc<Mutex<MenuEntry>>);

impl TextMenuItem {
    pub fn set_label(&mut self, label: impl AsRef<str>) {
        let label = label.as_ref().to_string();
        let mut entry = self.0.lock();
        for item in entry.item_gtk_items.as_ref().unwrap().lock().iter() {
            item.set_label(&label);
        }
        entry.label = label;
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        let mut entry = self.0.lock();
        for item in entry.item_gtk_items.as_ref().unwrap().lock().iter() {
            item.set_sensitive(enabled);
        }
        entry.enabled = enabled;
    }

    pub fn id(&self) -> u64 {
        self.0.lock().item_id.unwrap()
    }
}
