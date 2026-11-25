// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use keyboard_types::{Code, Key, Modifiers, NamedKey};
use objc2::rc::Retained;
use objc2_foundation::{NSCopying, NSString};
use objc2_ui_kit::{
    UIKeyInputDelete, UIKeyInputDownArrow, UIKeyInputEnd, UIKeyInputEscape, UIKeyInputF10,
    UIKeyInputF11, UIKeyInputF12, UIKeyInputF2, UIKeyInputF3, UIKeyInputF4, UIKeyInputF5,
    UIKeyInputF6, UIKeyInputF7, UIKeyInputF8, UIKeyInputF9, UIKeyInputHome, UIKeyInputLeftArrow,
    UIKeyInputPageDown, UIKeyInputPageUp, UIKeyInputRightArrow, UIKeyInputUpArrow,
    UIKeyModifierFlags,
};

use crate::accelerator::{Accelerator, AcceleratorParseError, KeyAccelerator, MenuAccelerator};

extern "C" {
    // Missing from `objc2-ui-kit`, though UIKit exports it next to `UIKeyInputF2`.
    static UIKeyInputF1: &'static NSString;
}

impl MenuAccelerator {
    /// Return the `UIKeyCommand` input of this hotkey, without modifiers.
    pub(crate) fn key_input(&self) -> Result<Retained<NSString>, AcceleratorParseError> {
        match self {
            MenuAccelerator::Physical(accelerator) => accelerator.key_input(),
            MenuAccelerator::Logical(accelerator) => accelerator.key_input(),
        }
    }

    /// Return the modifiers of this hotkey, as a `UIKeyModifierFlags` bitflag.
    pub(crate) fn modifier_flags(&self) -> UIKeyModifierFlags {
        match self {
            MenuAccelerator::Physical(accelerator) => accelerator.modifier_flags(),
            MenuAccelerator::Logical(accelerator) => modifiers_to_flags(accelerator.mods),
        }
    }
}

impl Accelerator {
    fn key_input(&self) -> Result<Retained<NSString>, AcceleratorParseError> {
        code_key_input(&self.key)
    }

    fn modifier_flags(&self) -> UIKeyModifierFlags {
        let mut flags = modifiers_to_flags(self.mods);
        if code_is_numpad(self.key) {
            flags |= UIKeyModifierFlags::NumericPad;
        }
        flags
    }
}

impl KeyAccelerator {
    fn key_input(&self) -> Result<Retained<NSString>, AcceleratorParseError> {
        match &self.key {
            Key::Character(character) if character.chars().count() == 1 => {
                Ok(NSString::from_str(character))
            }
            Key::Character(character) => {
                Err(AcceleratorParseError::UnsupportedKey(character.to_string()))
            }
            Key::Named(key) => named_key_input(key),
        }
    }
}

fn modifiers_to_flags(modifiers: Modifiers) -> UIKeyModifierFlags {
    let mut flags = UIKeyModifierFlags::empty();
    if modifiers.contains(Modifiers::SHIFT) {
        flags |= UIKeyModifierFlags::Shift;
    }
    if modifiers.contains(Modifiers::ALT) {
        flags |= UIKeyModifierFlags::Alternate;
    }
    if modifiers.contains(Modifiers::CONTROL) {
        flags |= UIKeyModifierFlags::Control;
    }
    #[allow(deprecated)]
    if modifiers.contains(Modifiers::META) || modifiers.contains(Modifiers::SUPER) {
        flags |= UIKeyModifierFlags::Command;
    }
    flags
}

fn code_key_input(code: &Code) -> Result<Retained<NSString>, AcceleratorParseError> {
    use Code::*;

    let key = match code {
        Backquote => "`",
        Backslash => "\\",
        BracketLeft => "[",
        BracketRight => "]",
        Comma | NumpadComma => ",",
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
        Tab => "\t",
        Backspace | NumpadBackspace => "\u{0008}",
        Enter | NumpadEnter => "\r",
        NumpadAdd => "+",
        NumpadMultiply | NumpadStar => "*",
        NumpadHash => "#",
        NumpadParenLeft => "(",
        NumpadParenRight => ")",
        Delete => return Ok(unsafe { UIKeyInputDelete }.copy()),
        Home => return Ok(unsafe { UIKeyInputHome }.copy()),
        End => return Ok(unsafe { UIKeyInputEnd }.copy()),
        PageDown => return Ok(unsafe { UIKeyInputPageDown }.copy()),
        PageUp => return Ok(unsafe { UIKeyInputPageUp }.copy()),
        ArrowDown => return Ok(unsafe { UIKeyInputDownArrow }.copy()),
        ArrowLeft => return Ok(unsafe { UIKeyInputLeftArrow }.copy()),
        ArrowRight => return Ok(unsafe { UIKeyInputRightArrow }.copy()),
        ArrowUp => return Ok(unsafe { UIKeyInputUpArrow }.copy()),
        Escape => return Ok(unsafe { UIKeyInputEscape }.copy()),
        F1 => return Ok(unsafe { UIKeyInputF1 }.copy()),
        F2 => return Ok(unsafe { UIKeyInputF2 }.copy()),
        F3 => return Ok(unsafe { UIKeyInputF3 }.copy()),
        F4 => return Ok(unsafe { UIKeyInputF4 }.copy()),
        F5 => return Ok(unsafe { UIKeyInputF5 }.copy()),
        F6 => return Ok(unsafe { UIKeyInputF6 }.copy()),
        F7 => return Ok(unsafe { UIKeyInputF7 }.copy()),
        F8 => return Ok(unsafe { UIKeyInputF8 }.copy()),
        F9 => return Ok(unsafe { UIKeyInputF9 }.copy()),
        F10 => return Ok(unsafe { UIKeyInputF10 }.copy()),
        F11 => return Ok(unsafe { UIKeyInputF11 }.copy()),
        F12 => return Ok(unsafe { UIKeyInputF12 }.copy()),
        _ => return Err(AcceleratorParseError::UnsupportedKey(format!("{code:?}"))),
    };

    Ok(NSString::from_str(key))
}

fn named_key_input(key: &NamedKey) -> Result<Retained<NSString>, AcceleratorParseError> {
    use NamedKey::*;

    let key = match key {
        Enter => "\r",
        Tab => "\t",
        Backspace => "\u{0008}",
        Delete => return Ok(unsafe { UIKeyInputDelete }.copy()),
        Home => return Ok(unsafe { UIKeyInputHome }.copy()),
        End => return Ok(unsafe { UIKeyInputEnd }.copy()),
        PageDown => return Ok(unsafe { UIKeyInputPageDown }.copy()),
        PageUp => return Ok(unsafe { UIKeyInputPageUp }.copy()),
        ArrowDown => return Ok(unsafe { UIKeyInputDownArrow }.copy()),
        ArrowLeft => return Ok(unsafe { UIKeyInputLeftArrow }.copy()),
        ArrowRight => return Ok(unsafe { UIKeyInputRightArrow }.copy()),
        ArrowUp => return Ok(unsafe { UIKeyInputUpArrow }.copy()),
        Escape => return Ok(unsafe { UIKeyInputEscape }.copy()),
        F1 => return Ok(unsafe { UIKeyInputF1 }.copy()),
        F2 => return Ok(unsafe { UIKeyInputF2 }.copy()),
        F3 => return Ok(unsafe { UIKeyInputF3 }.copy()),
        F4 => return Ok(unsafe { UIKeyInputF4 }.copy()),
        F5 => return Ok(unsafe { UIKeyInputF5 }.copy()),
        F6 => return Ok(unsafe { UIKeyInputF6 }.copy()),
        F7 => return Ok(unsafe { UIKeyInputF7 }.copy()),
        F8 => return Ok(unsafe { UIKeyInputF8 }.copy()),
        F9 => return Ok(unsafe { UIKeyInputF9 }.copy()),
        F10 => return Ok(unsafe { UIKeyInputF10 }.copy()),
        F11 => return Ok(unsafe { UIKeyInputF11 }.copy()),
        F12 => return Ok(unsafe { UIKeyInputF12 }.copy()),
        _ => return Err(AcceleratorParseError::UnsupportedKey(format!("{key:?}"))),
    };

    Ok(NSString::from_str(key))
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
            | NumpadComma
            | NumpadDecimal
            | NumpadDivide
            | NumpadEnter
            | NumpadEqual
            | NumpadHash
            | NumpadMultiply
            | NumpadParenLeft
            | NumpadParenRight
            | NumpadStar
            | NumpadSubtract
    )
}
