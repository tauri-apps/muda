// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

//! Emulates the predefined edit menu items, which GTK has no action for, on the window the menu
//! belongs to.

use std::ffi::c_void;

use gtk::{
    gdk::{self, prelude::*},
    glib,
    glib::translate::{IntoGlib, ToGlibPtr, ToGlibPtrMut},
    prelude::*,
};

use crate::{items::PredefinedMenuItemType, platform_impl::gtk_common::webkit};

/// Runs an edit command on the window the menu belongs to.
pub(crate) fn send(item: &gtk::MenuItem, item_type: &PredefinedMenuItemType) {
    let Some(window) = menu_window(item) else {
        return;
    };

    // A web view has no key binding for undo, because GTK 3 has none to translate, so its own
    // editing commands are used for every item rather than a key sequence.
    if let Some(web_view) = focused_web_view(&window) {
        // SAFETY: the widget was matched against the `WebKitWebView` type, and outlives the call.
        if unsafe { webkit::execute_editing_command(web_view.as_ptr() as *mut c_void, item_type) } {
            return;
        }
    }

    let Some(target) = window.window().filter(|window| window.is_viewable()) else {
        return;
    };

    let Some((key, modifiers)) = key_sequence(item_type) else {
        return;
    };

    send_key_sequence(&target, key, modifiers)
}

/// The key sequence GTK translates into the editing command of an edit menu item, if it is one.
fn key_sequence(item_type: &PredefinedMenuItemType) -> Option<(u32, gdk::ModifierType)> {
    let control = gdk::ModifierType::CONTROL_MASK;

    let (key, modifiers) = match item_type {
        PredefinedMenuItemType::Copy => (gdk::keys::constants::c, control),
        PredefinedMenuItemType::Cut => (gdk::keys::constants::x, control),
        PredefinedMenuItemType::Paste => (gdk::keys::constants::v, control),
        PredefinedMenuItemType::SelectAll => (gdk::keys::constants::a, control),
        // GTK 3 has no undo of its own, so there is no key sequence for it to translate. A web
        // view is handled before this, through its own editing commands.
        _ => return None,
    };

    Some((*key, modifiers))
}

/// Returns the focused `WebKitWebView` of `window`, if the focus is in one.
///
/// The type is looked up by name because WebKitGTK is not a dependency: it is only registered
/// once the application has created a web view of its own.
fn focused_web_view(window: &gtk::Window) -> Option<gtk::Widget> {
    let focus = GtkWindowExt::focused_widget(window)?;
    let web_view_type = glib::Type::from_name("WebKitWebView")?;

    std::iter::successors(Some(focus), |widget| widget.parent())
        .find(|widget| widget.type_().is_a(web_view_type))
}

/// Returns the window the menu the item belongs to was opened from.
///
/// An open menu has a window of its own that holds the keyboard focus, so the window is taken
/// from what the menu is attached to rather than from whichever window is active. A menu handed
/// to a status icon is attached to nothing, and falls back to the application's active window.
fn menu_window(item: &gtk::MenuItem) -> Option<gtk::Window> {
    attached_window(item).or_else(active_window)
}

/// Climbs out of the menus an item is nested in, to the window they were opened from.
fn attached_window(item: &gtk::MenuItem) -> Option<gtk::Window> {
    let mut widget: gtk::Widget = item.clone().upcast();

    loop {
        if let Some(window) = widget.downcast_ref::<gtk::Window>() {
            return (GtkWindowExt::type_(window) == gtk::WindowType::Toplevel)
                .then(|| window.clone());
        }

        let parent = widget.parent()?;

        // A menu is shown in a window of its own, so climbing through its parents leads to that
        // window rather than to the application's. Its attach widget leads back to the menu bar.
        widget = match parent.downcast::<gtk::Menu>() {
            Ok(menu) => menu.attach_widget()?,
            Err(parent) => parent,
        };
    }
}

/// Returns the application's active window, for a menu that is attached to nothing.
fn active_window() -> Option<gtk::Window> {
    gtk::Window::list_toplevels()
        .into_iter()
        .filter_map(|widget| widget.downcast::<gtk::Window>().ok())
        .find(|window| {
            GtkWindowExt::type_(window) == gtk::WindowType::Toplevel && window.is_active()
        })
}

/// Presses and releases `keyval` with `modifiers` held down, by handing GTK the key events its
/// own key bindings and focused widget would otherwise receive from the display.
fn send_key_sequence(window: &gdk::Window, keyval: u32, modifiers: gdk::ModifierType) {
    let Some(display) = gdk::Display::default() else {
        return;
    };
    // The keycode is what a physical press would carry, and widgets that read it rather than the
    // key value need it to match the key value.
    let keymap = gdk::Keymap::for_display(&display);
    let entry = keymap.and_then(|keymap| keymap.entries_for_keyval(keyval).into_iter().next());
    let keyboard = display.default_seat().and_then(|seat| seat.keyboard());

    for event_type in [gdk::EventType::KeyPress, gdk::EventType::KeyRelease] {
        let mut event = gdk::Event::new(event_type);

        // SAFETY: the event was just created as a key event, so its payload is a `GdkEventKey`,
        // and it takes ownership of the window reference.
        unsafe {
            let event: *mut gdk::ffi::GdkEvent = event.to_glib_none_mut().0;
            let key_event = event as *mut gdk::ffi::GdkEventKey;
            (*key_event).window = window.to_glib_full();
            (*key_event).send_event = true.into_glib() as i8;
            (*key_event).time = gdk::ffi::GDK_CURRENT_TIME as u32;
            (*key_event).state = modifiers.into_glib();
            (*key_event).keyval = keyval;
            (*key_event).hardware_keycode = entry.as_ref().map_or(0, |e| e.keycode() as u16);
            (*key_event).group = entry.as_ref().map_or(0, |e| e.group() as u8);
        }

        event.set_device(keyboard.as_ref());
        gtk::main_do_event(&mut event);
    }
}
