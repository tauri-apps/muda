use crossbeam_channel::{unbounded, Receiver, Sender};
use once_cell::sync::Lazy;

mod platform_impl;
mod util;

static MENU_CHANNEL: Lazy<(Sender<MenuEvent>, Receiver<MenuEvent>)> = Lazy::new(|| unbounded());

/// Event channel for receiving menu events.
pub fn menu_event_receiver<'a>() -> &'a Receiver<MenuEvent> {
    &MENU_CHANNEL.1
}

/// Describes a menu event emitted when a menu item is activated
pub struct MenuEvent {
    pub id: u64,
}

pub struct Menu(platform_impl::Menu);

impl Menu {
    pub fn new() -> Self {
        Self(platform_impl::Menu::new())
    }

    pub fn add_submenu(&mut self, label: impl AsRef<str>, enabled: bool) -> Submenu {
        Submenu(self.0.add_submenu(label, enabled))
    }

    #[cfg(target_os = "linux")]
    pub fn init_for_gtk_window<W>(&self, w: &W)
    where
        W: gtk::prelude::IsA<gtk::Container>,
    {
        self.0.init_for_gtk_window(w)
    }

    #[cfg(target_os = "windows")]
    pub fn init_for_hwnd(&self, hwnd: isize) {
        self.0.init_for_hwnd(hwnd)
    }
}

#[derive(Clone)]
pub struct Submenu(platform_impl::Submenu);

impl Submenu {
    pub fn label(&self) -> String {
        self.0.label()
    }

    pub fn set_label(&mut self, label: impl AsRef<str>) {
        self.0.set_label(label)
    }

    pub fn enabled(&self) -> bool {
        self.0.enabled()
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.0.set_enabled(enabled)
    }
    pub fn add_submenu(&mut self, label: impl AsRef<str>, enabled: bool) -> Submenu {
        Submenu(self.0.add_submenu(label, enabled))
    }

    pub fn add_text_item(&mut self, label: impl AsRef<str>, enabled: bool) -> TextMenuItem {
        TextMenuItem(self.0.add_text_item(label, enabled))
    }
}

#[derive(Clone)]
pub struct TextMenuItem(platform_impl::TextMenuItem);

impl TextMenuItem {
    pub fn label(&self) -> String {
        self.0.label()
    }

    pub fn set_label(&mut self, label: impl AsRef<str>) {
        self.0.set_label(label)
    }

    pub fn enabled(&self) -> bool {
        self.0.enabled()
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.0.set_enabled(enabled)
    }

    pub fn id(&self) -> u64 {
        self.0.id()
    }
}
