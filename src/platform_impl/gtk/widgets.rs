use gio::prelude::*;
use gtk4::{gio, glib};

mod menubar;

fn get_icon_for_item(menu: &gio::Menu, index: i32) -> Option<gio::Icon> {
    menu.item_attribute_value(index, "icon", None)
        .and_then(|v| gio::Icon::deserialize(&v))
}

fn get_label_for_item(menu: &gio::Menu, index: i32) -> Option<String> {
    menu.item_attribute_value(index, "label", Some(glib::VariantTy::STRING))
        .and_then(|v| v.str().map(String::from))
}

pub use menubar::*;
