// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use keyboard_types::{Code, Key, Modifiers};
use objc2_app_kit::NSEventModifierFlags;

use crate::accelerator::{Accelerator, AcceleratorParseError, KeyAccelerator, MenuAccelerator};

impl MenuAccelerator {
    /// Return the string value of this hotkey, without modifiers.
    pub(crate) fn key_equivalent(&self) -> Result<String, AcceleratorParseError> {
        match self {
            MenuAccelerator::Physical(accelerator) => accelerator.key_equivalent(),
            MenuAccelerator::Logical(accelerator) => accelerator.key_equivalent(),
        }
    }

    /// Return the modifiers of this hotkey, as an NSEventModifierFlags bitflag.
    pub(crate) fn modifier_mask(&self) -> NSEventModifierFlags {
        match self {
            MenuAccelerator::Physical(accelerator) => accelerator.modifier_mask(),
            MenuAccelerator::Logical(accelerator) => accelerator.modifier_mask(),
        }
    }
}

impl Accelerator {
    pub(crate) fn key_equivalent(&self) -> Result<String, AcceleratorParseError> {
        code_key_equivalent(&self.key)
    }

    pub(crate) fn modifier_mask(&self) -> NSEventModifierFlags {
        let mut flags = modifiers_to_mask(self.mods);
        if code_is_numpad(self.key) {
            flags.insert(NSEventModifierFlags::NumericPad);
        }
        flags
    }
}

impl KeyAccelerator {
    pub(crate) fn key_equivalent(&self) -> Result<String, AcceleratorParseError> {
        key_key_equivalent(&self.key)
    }

    pub(crate) fn modifier_mask(&self) -> NSEventModifierFlags {
        modifiers_to_mask(self.mods)
    }
}

fn modifiers_to_mask(modifiers: Modifiers) -> NSEventModifierFlags {
    let mut flags = NSEventModifierFlags::empty();
    if modifiers.contains(Modifiers::SHIFT) {
        flags.insert(NSEventModifierFlags::Shift);
    }
    if modifiers.contains(Modifiers::ALT) {
        flags.insert(NSEventModifierFlags::Option);
    }
    if modifiers.contains(Modifiers::CONTROL) {
        flags.insert(NSEventModifierFlags::Control);
    }
    if modifiers.contains(Modifiers::META) || modifiers.contains(Modifiers::SUPER) {
        flags.insert(NSEventModifierFlags::Command);
    }
    flags
}

fn code_key_equivalent(code: &Code) -> Result<String, AcceleratorParseError> {
    use Code::*;

    let key = match code {
        Backquote => "`",
        Backslash => "\\",
        BracketLeft => "[",
        BracketRight => "]",
        Comma => ",",
        Digit0 | Numpad0 => "0",
        Digit1 | Numpad1 => "1",
        Digit2 | Numpad2 => "2",
        Digit3 | Numpad3 => "3",
        Digit4 | Numpad4 => "4",
        Digit5 | Numpad5 => "5",
        Digit6 | Numpad6 => "6",
        Digit7 | Numpad7 => "7",
        Digit8 | Numpad8 => "8",
        Digit9 | Numpad9 => "9",
        Equal | NumpadEqual => "=",
        IntlYen => "\u{00a5}",
        KeyA => "a",
        KeyB => "b",
        KeyC => "c",
        KeyD => "d",
        KeyE => "e",
        KeyF => "f",
        KeyG => "g",
        KeyH => "h",
        KeyI => "i",
        KeyJ => "j",
        KeyK => "k",
        KeyL => "l",
        KeyM => "m",
        KeyN => "n",
        KeyO => "o",
        KeyP => "p",
        KeyQ => "q",
        KeyR => "r",
        KeyS => "s",
        KeyT => "t",
        KeyU => "u",
        KeyV => "v",
        KeyW => "w",
        KeyX => "x",
        KeyY => "y",
        KeyZ => "z",
        Minus | NumpadSubtract => "-",
        Period | NumpadDecimal => ".",
        Quote => "'",
        Semicolon => ";",
        Slash | NumpadDivide => "/",
        Space => " ",
        Tab => "\u{0009}",
        Backspace | NumpadBackspace => "\u{0008}",
        Enter => "\u{000d}",
        NumpadEnter => "\u{0003}",
        Delete => "\u{007f}",
        Insert => "\u{F727}",
        Home => "\u{F729}",
        End => "\u{F72B}",
        PageDown => "\u{F72D}",
        PageUp => "\u{F72C}",
        PrintScreen => "\u{F72E}",
        ScrollLock => "\u{F72F}",
        ArrowDown => "\u{F701}",
        ArrowLeft => "\u{F702}",
        ArrowRight => "\u{F703}",
        ArrowUp => "\u{F700}",
        Escape => "\u{001b}",
        Pause => "\u{F730}",
        ContextMenu => "\u{F735}",
        NumpadAdd => "+",
        NumpadComma => ",",
        NumpadMultiply | NumpadStar => "*",
        NumpadHash => "#",
        NumpadParenLeft => "(",
        NumpadParenRight => ")",
        Help => "\u{F746}",
        Find => "\u{F745}",
        Select => "\u{F741}",
        Undo => "\u{F743}",
        F1 => "\u{F704}",
        F2 => "\u{F705}",
        F3 => "\u{F706}",
        F4 => "\u{F707}",
        F5 => "\u{F708}",
        F6 => "\u{F709}",
        F7 => "\u{F70A}",
        F8 => "\u{F70B}",
        F9 => "\u{F70C}",
        F10 => "\u{F70D}",
        F11 => "\u{F70E}",
        F12 => "\u{F70F}",
        F13 => "\u{F710}",
        F14 => "\u{F711}",
        F15 => "\u{F712}",
        F16 => "\u{F713}",
        F17 => "\u{F714}",
        F18 => "\u{F715}",
        F19 => "\u{F716}",
        F20 => "\u{F717}",
        F21 => "\u{F718}",
        F22 => "\u{F719}",
        F23 => "\u{F71A}",
        F24 => "\u{F71B}",
        F25 => "\u{F71C}",
        F26 => "\u{F71D}",
        F27 => "\u{F71E}",
        F28 => "\u{F71F}",
        F29 => "\u{F720}",
        F30 => "\u{F721}",
        F31 => "\u{F722}",
        F32 => "\u{F723}",
        F33 => "\u{F724}",
        F34 => "\u{F725}",
        F35 => "\u{F726}",
        _ => return Err(AcceleratorParseError::UnsupportedKey(format!("{code:?}"))),
    };

    Ok(key.to_string())
}

fn key_key_equivalent(key: &Key) -> Result<String, AcceleratorParseError> {
    use Key::*;

    let key = match key {
        Character(character) => return character_key_equivalent(character),
        Enter => "\u{000d}",
        Tab => "\u{0009}",
        ArrowDown => "\u{F701}",
        ArrowLeft => "\u{F702}",
        ArrowRight => "\u{F703}",
        ArrowUp => "\u{F700}",
        End => "\u{F72B}",
        Home => "\u{F729}",
        PageDown => "\u{F72D}",
        PageUp => "\u{F72C}",
        Backspace => "\u{0008}",
        Delete => "\u{007f}",
        Insert => "\u{F727}",
        Redo => "\u{F744}",
        Undo => "\u{F743}",
        ContextMenu => "\u{F735}",
        Escape => "\u{001b}",
        Find => "\u{F745}",
        Help => "\u{F746}",
        Pause => "\u{F730}",
        Select => "\u{F741}",
        PrintScreen => "\u{F72E}",
        ScrollLock => "\u{F72F}",
        F1 => "\u{F704}",
        F2 => "\u{F705}",
        F3 => "\u{F706}",
        F4 => "\u{F707}",
        F5 => "\u{F708}",
        F6 => "\u{F709}",
        F7 => "\u{F70A}",
        F8 => "\u{F70B}",
        F9 => "\u{F70C}",
        F10 => "\u{F70D}",
        F11 => "\u{F70E}",
        F12 => "\u{F70F}",
        F13 => "\u{F710}",
        F14 => "\u{F711}",
        F15 => "\u{F712}",
        F16 => "\u{F713}",
        F17 => "\u{F714}",
        F18 => "\u{F715}",
        F19 => "\u{F716}",
        F20 => "\u{F717}",
        F21 => "\u{F718}",
        F22 => "\u{F719}",
        F23 => "\u{F71A}",
        F24 => "\u{F71B}",
        F25 => "\u{F71C}",
        F26 => "\u{F71D}",
        F27 => "\u{F71E}",
        F28 => "\u{F71F}",
        F29 => "\u{F720}",
        F30 => "\u{F721}",
        F31 => "\u{F722}",
        F32 => "\u{F723}",
        F33 => "\u{F724}",
        F34 => "\u{F725}",
        F35 => "\u{F726}",
        _ => return Err(AcceleratorParseError::UnsupportedKey(format!("{key:?}"))),
    };

    Ok(key.to_string())
}

fn character_key_equivalent(character: &str) -> Result<String, AcceleratorParseError> {
    if character.chars().count() == 1 {
        Ok(character.to_string())
    } else {
        Err(AcceleratorParseError::UnsupportedKey(character.to_string()))
    }
}

fn code_is_numpad(code: Code) -> bool {
    use Code::*;
    matches!(
        code,
        Numpad0
            | Numpad1
            | Numpad2
            | Numpad3
            | Numpad4
            | Numpad5
            | Numpad6
            | Numpad7
            | Numpad8
            | Numpad9
            | NumpadAdd
            | NumpadBackspace
            | NumpadClear
            | NumpadClearEntry
            | NumpadComma
            | NumpadDecimal
            | NumpadDivide
            | NumpadEnter
            | NumpadEqual
            | NumpadHash
            | NumpadMemoryAdd
            | NumpadMemoryClear
            | NumpadMemoryRecall
            | NumpadMemoryStore
            | NumpadMemorySubtract
            | NumpadMultiply
            | NumpadParenLeft
            | NumpadParenRight
            | NumpadStar
            | NumpadSubtract
    )
}
