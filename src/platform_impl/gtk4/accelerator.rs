// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use keyboard_types::{Key, Modifiers};

use crate::accelerator::KeyAccelerator;

impl KeyAccelerator {
    /// Builds a GTK accelerator string (e.g. `<Shift><Control>a`) that can be
    /// passed to `gtk::Application::set_accels_for_action`.
    pub fn to_gtk(&self) -> Option<String> {
        let mut gtk = modifiers_to_gtk(self.mods);
        gtk.push_str(&key_to_gtk(&self.key)?);
        Some(gtk)
    }
}

fn modifiers_to_gtk(mods: Modifiers) -> String {
    let mut gtk = String::new();

    if mods.shift() {
        gtk.push_str("<Shift>");
    }
    if mods.ctrl() {
        gtk.push_str("<Control>");
    }
    if mods.alt() {
        gtk.push_str("<Alt>");
    }
    if mods.contains(Modifiers::SUPER) || mods.meta() {
        gtk.push_str("<Super>");
    }

    gtk
}

/// Maps a logical [`Key`] to the matching GDK keysym name understood by
/// `gtk_accelerator_parse`.
fn key_to_gtk(key: &Key) -> Option<String> {
    let name = match key {
        Key::Character(c) => return character_to_gtk(c),
        Key::Escape => "Escape",
        Key::Backspace => "BackSpace",
        Key::Tab => "Tab",
        Key::Enter => "Return",
        Key::CapsLock => "Caps_Lock",
        Key::F1 => "F1",
        Key::F2 => "F2",
        Key::F3 => "F3",
        Key::F4 => "F4",
        Key::F5 => "F5",
        Key::F6 => "F6",
        Key::F7 => "F7",
        Key::F8 => "F8",
        Key::F9 => "F9",
        Key::F10 => "F10",
        Key::F11 => "F11",
        Key::F12 => "F12",
        Key::F13 => "F13",
        Key::F14 => "F14",
        Key::F15 => "F15",
        Key::F16 => "F16",
        Key::F17 => "F17",
        Key::F18 => "F18",
        Key::F19 => "F19",
        Key::F20 => "F20",
        Key::F21 => "F21",
        Key::F22 => "F22",
        Key::F23 => "F23",
        Key::F24 => "F24",
        Key::PrintScreen => "Print",
        Key::ScrollLock => "Scroll_Lock",
        Key::Pause => "Pause",
        Key::Insert => "Insert",
        Key::Delete => "Delete",
        Key::Home => "Home",
        Key::End => "End",
        Key::PageUp => "Page_Up",
        Key::PageDown => "Page_Down",
        Key::NumLock => "Num_Lock",
        Key::ArrowUp => "Up",
        Key::ArrowDown => "Down",
        Key::ArrowLeft => "Left",
        Key::ArrowRight => "Right",
        Key::ContextMenu => "Menu",
        Key::WakeUp => "WakeUp",
        _ => return None,
    };
    Some(name.to_string())
}

fn character_to_gtk(character: &str) -> Option<String> {
    let mut chars = character.chars();
    let character = chars.next()?;
    if chars.next().is_some() {
        return None;
    }

    let name = match character {
        ' ' => "space",
        '!' => "exclam",
        '"' => "quotedbl",
        '#' => "numbersign",
        '$' => "dollar",
        '%' => "percent",
        '&' => "ampersand",
        '\'' => "apostrophe",
        '(' => "parenleft",
        ')' => "parenright",
        '*' => "asterisk",
        '+' => "plus",
        ',' => "comma",
        '-' => "minus",
        '.' => "period",
        '/' => "slash",
        ':' => "colon",
        ';' => "semicolon",
        '<' => "less",
        '=' => "equal",
        '>' => "greater",
        '?' => "question",
        '@' => "at",
        '[' => "bracketleft",
        '\\' => "backslash",
        ']' => "bracketright",
        '^' => "asciicircum",
        '_' => "underscore",
        '`' => "grave",
        '{' => "braceleft",
        '|' => "bar",
        '}' => "braceright",
        '~' => "asciitilde",
        _ => return Some(character.to_string()),
    };

    Some(name.to_string())
}
