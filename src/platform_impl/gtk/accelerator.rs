// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use gtk::gdk;
use keyboard_types::{Key, Modifiers};

use crate::accelerator::{Accelerator, AcceleratorParseError};

pub fn to_gtk_mnemonic<S: AsRef<str>>(string: S) -> String {
    string
        .as_ref()
        .replace("_", "__")
        .replace("&&", "[~~]")
        .replace('&', "_")
        .replace("[~~]", "&")
}

pub fn from_gtk_mnemonic<S: AsRef<str>>(string: S) -> String {
    string
        .as_ref()
        .replace("&", "&&")
        .replace("__", "[~~]")
        .replace('_', "&")
        .replace("[~~]", "_")
}

pub fn parse_accelerator(
    accelerator: &Accelerator,
) -> Result<(gdk::ModifierType, u32), AcceleratorParseError> {
    let key = match &accelerator.key {
        Key::Character(s) => {
            if let Some(c) = s.chars().next() {
                let cp = c as u32;
                if cp > 0x100 {
                    // Non-ASCII: GDK uses 0x01000000 | unicode_codepoint
                    0x01000000 | cp
                } else {
                    // For ASCII letters, GDK expects uppercase keyvals
                    c.to_ascii_uppercase() as u32
                }
            } else {
                return Err(AcceleratorParseError::UnsupportedKey(
                    "empty character".to_string(),
                ));
            }
        }
        key => {
            if let Some(gdk_key) = named_key_to_gdk(key) {
                *gdk_key
            } else {
                return Err(AcceleratorParseError::UnsupportedKey(key.to_string()));
            }
        }
    };

    Ok((modifiers_to_gdk_modifier_type(accelerator.mods), key))
}

fn modifiers_to_gdk_modifier_type(modifiers: Modifiers) -> gdk::ModifierType {
    let mut result = gdk::ModifierType::empty();

    result.set(
        gdk::ModifierType::MOD1_MASK,
        modifiers.contains(Modifiers::ALT),
    );
    result.set(
        gdk::ModifierType::CONTROL_MASK,
        modifiers.contains(Modifiers::CONTROL),
    );
    result.set(
        gdk::ModifierType::SHIFT_MASK,
        modifiers.contains(Modifiers::SHIFT),
    );
    result.set(
        gdk::ModifierType::META_MASK,
        modifiers.contains(Modifiers::SUPER),
    );

    result
}

fn named_key_to_gdk(src: &Key) -> Option<gdk::keys::Key> {
    use gdk::keys::constants::*;
    Some(match src {
        Key::Escape => Escape,
        Key::Backspace => BackSpace,
        Key::Tab => Tab,
        Key::Enter => Return,
        Key::Control => Control_L,
        Key::Alt => Alt_L,
        Key::Shift => Shift_L,
        Key::Meta | Key::Super => Super_L,
        Key::CapsLock => Caps_Lock,
        Key::F1 => F1,
        Key::F2 => F2,
        Key::F3 => F3,
        Key::F4 => F4,
        Key::F5 => F5,
        Key::F6 => F6,
        Key::F7 => F7,
        Key::F8 => F8,
        Key::F9 => F9,
        Key::F10 => F10,
        Key::F11 => F11,
        Key::F12 => F12,
        Key::F13 => F13,
        Key::F14 => F14,
        Key::F15 => F15,
        Key::F16 => F16,
        Key::F17 => F17,
        Key::F18 => F18,
        Key::F19 => F19,
        Key::F20 => F20,
        Key::F21 => F21,
        Key::F22 => F22,
        Key::F23 => F23,
        Key::F24 => F24,
        Key::PrintScreen => Print,
        Key::ScrollLock => Scroll_Lock,
        Key::Pause => Pause,
        Key::Insert => Insert,
        Key::Delete => Delete,
        Key::Home => Home,
        Key::End => End,
        Key::PageUp => Page_Up,
        Key::PageDown => Page_Down,
        Key::NumLock => Num_Lock,
        Key::ArrowUp => Up,
        Key::ArrowDown => Down,
        Key::ArrowLeft => Left,
        Key::ArrowRight => Right,
        Key::ContextMenu => Menu,
        Key::WakeUp => WakeUp,
        _ => return None,
    })
}
