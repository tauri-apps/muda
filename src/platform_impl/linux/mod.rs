mod accelerator;

use crate::counter::Counter;
use gtk::{prelude::*, Orientation};
use std::{cell::RefCell, rc::Rc};

use self::accelerator::{to_gtk_accelerator, to_gtk_menemenoic};

static COUNTER: Counter = Counter::new();

#[derive(PartialEq, Eq)]
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
    accelerator: Option<String>,
    // NOTE(amrbashir): because gtk doesn't allow using the same `gtk::MenuItem`
    // multiple times, and thus can't be used in multiple windows, each entry
    // keeps a vector of a `gtk::MenuItem` or a tuple of `gtk::MenuItem` and `gtk::Menu`
    // and push to it every time `Menu::init_for_gtk_window` is called.
    item_gtk_items: Option<Rc<RefCell<Vec<gtk::MenuItem>>>>,
    menu_gtk_items: Option<Rc<RefCell<Vec<(gtk::MenuItem, gtk::Menu)>>>>,
    entries: Option<Vec<Rc<RefCell<MenuEntry>>>>,
}

struct InnerMenu {
    entries: Vec<Rc<RefCell<MenuEntry>>>,
    // NOTE(amrbashir): because gtk doesn't allow using the same `gtk::MenuBar` and `gtk::Box`
    // multiple times, and thus can't be used in multiple windows, entry
    // keeps a vector of a tuple of `gtk::MenuBar` and `gtk::Box`
    // and push to it every time `Menu::init_for_gtk_window` is called.
    gtk_items: Vec<(gtk::MenuBar, Rc<gtk::Box>)>,
    accel_group: gtk::AccelGroup,
}

#[derive(Clone)]
pub struct Menu(Rc<RefCell<InnerMenu>>);

impl Menu {
    pub fn new() -> Self {
        Self(Rc::new(RefCell::new(InnerMenu {
            entries: Vec::new(),
            gtk_items: Vec::new(),
            accel_group: gtk::AccelGroup::new(),
        })))
    }

    pub fn add_submenu(&mut self, label: impl AsRef<str>, enabled: bool) -> Submenu {
        let label = label.as_ref().to_string();
        let entry = Rc::new(RefCell::new(MenuEntry {
            label: label.clone(),
            enabled,
            entries: Some(Vec::new()),
            r#type: MenuEntryType::Submenu,
            item_id: None,
            accelerator: None,
            menu_gtk_items: Some(Rc::new(RefCell::new(Vec::new()))),
            item_gtk_items: None,
        }));
        self.0.borrow_mut().entries.push(entry.clone());
        Submenu(entry)
    }

    pub fn init_for_gtk_window<W>(&self, w: &W) -> Rc<gtk::Box>
    where
        W: IsA<gtk::Container>,
        W: IsA<gtk::Window>,
    {
        let mut inner = self.0.borrow_mut();
        let menu_bar = gtk::MenuBar::new();
        add_entries_to_menu(&menu_bar, &inner.entries, &inner.accel_group);
        w.add_accel_group(&inner.accel_group);

        let vbox = gtk::Box::new(Orientation::Vertical, 0);
        vbox.pack_start(&menu_bar, false, false, 0);
        w.add(&vbox);
        vbox.show_all();

        let vbox = Rc::new(vbox);
        let vbox_c = Rc::clone(&vbox);

        inner.gtk_items.push((menu_bar, vbox));

        vbox_c
    }
}

fn add_entries_to_menu<M: IsA<gtk::MenuShell>>(
    gtk_menu: &M,
    entries: &Vec<Rc<RefCell<MenuEntry>>>,
    accel_group: &gtk::AccelGroup,
) {
    for entry in entries {
        let mut entry = entry.borrow_mut();
        let gtk_item = gtk::MenuItem::with_mnemonic(&to_gtk_menemenoic(&entry.label));
        gtk_menu.append(&gtk_item);
        gtk_item.set_sensitive(entry.enabled);
        if entry.r#type == MenuEntryType::Submenu {
            let gtk_menu = gtk::Menu::new();
            gtk_item.set_submenu(Some(&gtk_menu));
            add_entries_to_menu(&gtk_menu, entry.entries.as_ref().unwrap(), accel_group);
            entry
                .menu_gtk_items
                .as_mut()
                .unwrap()
                .borrow_mut()
                .push((gtk_item, gtk_menu));
        } else {
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

            let id = entry.item_id.unwrap_or_default();
            gtk_item.connect_activate(move |_| {
                let _ = crate::MENU_CHANNEL.0.send(crate::MenuEvent { id });
            });
            entry
                .item_gtk_items
                .as_mut()
                .unwrap()
                .borrow_mut()
                .push(gtk_item);
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
        for (item, _) in entry.menu_gtk_items.as_ref().unwrap().borrow().iter() {
            item.set_label(&label);
        }
        entry.label = label;
    }

    pub fn enabled(&self) -> bool {
        self.0.borrow().enabled
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        let mut entry = self.0.borrow_mut();
        entry.enabled = true;
        for (item, _) in entry.menu_gtk_items.as_ref().unwrap().borrow().iter() {
            item.set_sensitive(enabled);
        }
    }

    pub fn add_submenu(&mut self, label: impl AsRef<str>, enabled: bool) -> Submenu {
        let entry = Rc::new(RefCell::new(MenuEntry {
            label: label.as_ref().to_string(),
            enabled,
            entries: Some(Vec::new()),
            r#type: MenuEntryType::Submenu,
            item_id: None,
            accelerator: None,
            menu_gtk_items: Some(Rc::new(RefCell::new(Vec::new()))),
            item_gtk_items: None,
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
            label: to_gtk_menemenoic(label),
            enabled,
            entries: None,
            r#type: MenuEntryType::Text,
            item_id: Some(COUNTER.next()),
            accelerator: accelerator.map(|s| s.to_string()),
            menu_gtk_items: None,
            item_gtk_items: Some(Rc::new(RefCell::new(Vec::new()))),
        }));
        self.0
            .borrow_mut()
            .entries
            .as_mut()
            .unwrap()
            .push(entry.clone());
        TextMenuItem(entry)
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
        for item in entry.item_gtk_items.as_ref().unwrap().borrow().iter() {
            item.set_label(&label);
        }
        entry.label = label;
    }

    pub fn enabled(&self) -> bool {
        self.0.borrow().enabled
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        let mut entry = self.0.borrow_mut();
        for item in entry.item_gtk_items.as_ref().unwrap().borrow().iter() {
            item.set_sensitive(enabled);
        }
        entry.enabled = enabled;
    }

    pub fn id(&self) -> u64 {
        self.0.borrow().item_id.unwrap()
    }
}
