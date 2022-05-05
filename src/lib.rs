mod platform_impl;
mod util;

pub struct Menu(platform_impl::Menu);

impl Menu {
    pub fn new() -> Self {
        Self(platform_impl::Menu::new())
    }

    pub fn add_submenu(&mut self, label: impl AsRef<str>, enabled: bool) -> Submenu {
        Submenu(self.0.add_submenu(label, enabled))
    }

    pub fn add_text_item(&mut self, label: impl AsRef<str>, enabled: bool) -> TextMenuItem {
        TextMenuItem(self.0.add_text_item(label, enabled))
    }

    #[cfg(target_os = "linux")]
    pub fn init_for_gtk_window<W>(&self, w: &W)
    where
        W: gtk::prelude::IsA<gtk::Container>,
    {
        self.0.init_for_gtk_window(w)
    }
}

#[derive(Clone)]
pub struct Submenu(platform_impl::Submenu);

impl Submenu {
    pub fn set_label(&mut self, label: impl AsRef<str>) {
        self.0.set_label(label)
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
    pub fn set_label(&mut self, label: impl AsRef<str>) {
        self.0.set_label(label)
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.0.set_enabled(enabled)
    }

    pub fn id(&self) -> u64 {
        self.0.id()
    }
}
