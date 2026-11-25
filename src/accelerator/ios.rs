// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use keyboard_types::{Code, Key, Modifiers, NamedKey};
use objc2::rc::Retained;
use objc2_foundation::{NSCopying, NSString};
use objc2_ui_kit::UIKeyModifierFlags;
use std::ffi::{c_char, c_void, CStr};

use crate::accelerator::{Accelerator, AcceleratorParseError, KeyAccelerator, MenuAccelerator};

extern "C" {
    fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
}

const RTLD_DEFAULT: *mut c_void = -2isize as *mut c_void;

/// Look up a `UIKeyInput*` constant at runtime.
///
/// Linking the constants directly would make dyld refuse to launch the app on any iOS version
/// older than the newest one we use (`UIKeyInputDelete` is iOS 15, `UIKeyInputHome`, `End` and
/// `F1`-`F12` are iOS 13.4), even if no accelerator uses those keys.
fn key_input_constant(name: &CStr) -> Result<Retained<NSString>, AcceleratorParseError> {
    // The symbol is the address of the `NSString *const` variable.
    let ptr = unsafe { dlsym(RTLD_DEFAULT, name.as_ptr()) } as *const &'static NSString;
    if ptr.is_null() {
        let name = name.to_string_lossy();
        let name = name.strip_prefix("UIKeyInput").unwrap_or(&name);
        return Err(AcceleratorParseError::UnsupportedKey(name.to_string()));
    }
    Ok(unsafe { *ptr }.copy())
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
        Delete => return key_input_constant(c"UIKeyInputDelete"),
        Home => return key_input_constant(c"UIKeyInputHome"),
        End => return key_input_constant(c"UIKeyInputEnd"),
        PageDown => return key_input_constant(c"UIKeyInputPageDown"),
        PageUp => return key_input_constant(c"UIKeyInputPageUp"),
        ArrowDown => return key_input_constant(c"UIKeyInputDownArrow"),
        ArrowLeft => return key_input_constant(c"UIKeyInputLeftArrow"),
        ArrowRight => return key_input_constant(c"UIKeyInputRightArrow"),
        ArrowUp => return key_input_constant(c"UIKeyInputUpArrow"),
        Escape => return key_input_constant(c"UIKeyInputEscape"),
        F1 => return key_input_constant(c"UIKeyInputF1"),
        F2 => return key_input_constant(c"UIKeyInputF2"),
        F3 => return key_input_constant(c"UIKeyInputF3"),
        F4 => return key_input_constant(c"UIKeyInputF4"),
        F5 => return key_input_constant(c"UIKeyInputF5"),
        F6 => return key_input_constant(c"UIKeyInputF6"),
        F7 => return key_input_constant(c"UIKeyInputF7"),
        F8 => return key_input_constant(c"UIKeyInputF8"),
        F9 => return key_input_constant(c"UIKeyInputF9"),
        F10 => return key_input_constant(c"UIKeyInputF10"),
        F11 => return key_input_constant(c"UIKeyInputF11"),
        F12 => return key_input_constant(c"UIKeyInputF12"),
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
        Delete => return key_input_constant(c"UIKeyInputDelete"),
        Home => return key_input_constant(c"UIKeyInputHome"),
        End => return key_input_constant(c"UIKeyInputEnd"),
        PageDown => return key_input_constant(c"UIKeyInputPageDown"),
        PageUp => return key_input_constant(c"UIKeyInputPageUp"),
        ArrowDown => return key_input_constant(c"UIKeyInputDownArrow"),
        ArrowLeft => return key_input_constant(c"UIKeyInputLeftArrow"),
        ArrowRight => return key_input_constant(c"UIKeyInputRightArrow"),
        ArrowUp => return key_input_constant(c"UIKeyInputUpArrow"),
        Escape => return key_input_constant(c"UIKeyInputEscape"),
        F1 => return key_input_constant(c"UIKeyInputF1"),
        F2 => return key_input_constant(c"UIKeyInputF2"),
        F3 => return key_input_constant(c"UIKeyInputF3"),
        F4 => return key_input_constant(c"UIKeyInputF4"),
        F5 => return key_input_constant(c"UIKeyInputF5"),
        F6 => return key_input_constant(c"UIKeyInputF6"),
        F7 => return key_input_constant(c"UIKeyInputF7"),
        F8 => return key_input_constant(c"UIKeyInputF8"),
        F9 => return key_input_constant(c"UIKeyInputF9"),
        F10 => return key_input_constant(c"UIKeyInputF10"),
        F11 => return key_input_constant(c"UIKeyInputF11"),
        F12 => return key_input_constant(c"UIKeyInputF12"),
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
