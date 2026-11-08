// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

//! Accelerators as portable `QKeySequence` strings (e.g. `Ctrl+Shift+N`).

use keyboard_types::{Code, Key, Modifiers, NamedKey};

use crate::accelerator::{Accelerator, KeyAccelerator, MenuAccelerator};

impl MenuAccelerator {
    /// A `QKeySequence::PortableText` string, or `None` if Qt has no name for the key.
    pub(crate) fn to_qt(&self) -> Option<String> {
        match self {
            MenuAccelerator::Physical(a) => a.to_qt(),
            MenuAccelerator::Logical(a) => a.to_qt(),
        }
    }
}

impl Accelerator {
    pub(crate) fn to_qt(self) -> Option<String> {
        Some(modifiers_to_qt(self.mods) + &code_to_qt(&self.key)?)
    }
}

impl KeyAccelerator {
    pub(crate) fn to_qt(&self) -> Option<String> {
        let key = match &self.key {
            Key::Character(c) => character_to_qt(c)?,
            Key::Named(k) => named_key_to_qt(k)?.to_string(),
        };
        Some(modifiers_to_qt(self.mods) + &key)
    }
}

fn modifiers_to_qt(mods: Modifiers) -> String {
    let mut s = String::new();
    if mods.ctrl() {
        s.push_str("Ctrl+");
    }
    if mods.alt() {
        s.push_str("Alt+");
    }
    if mods.shift() {
        s.push_str("Shift+");
    }
    // In Qt, "Meta" is the Super/Windows key on Linux.
    #[allow(deprecated)]
    if mods.meta() || mods.contains(Modifiers::SUPER) {
        s.push_str("Meta+");
    }
    s
}

fn character_to_qt(c: &str) -> Option<String> {
    let mut chars = c.chars();
    let ch = chars.next()?;
    if chars.next().is_some() {
        return None;
    }
    Some(match ch {
        ' ' => "Space".into(),
        ch if ch.is_alphabetic() => ch.to_uppercase().collect(),
        ch => ch.to_string(),
    })
}

fn code_to_qt(code: &Code) -> Option<String> {
    use Code::*;
    let s = match code {
        KeyA => "A",
        KeyB => "B",
        KeyC => "C",
        KeyD => "D",
        KeyE => "E",
        KeyF => "F",
        KeyG => "G",
        KeyH => "H",
        KeyI => "I",
        KeyJ => "J",
        KeyK => "K",
        KeyL => "L",
        KeyM => "M",
        KeyN => "N",
        KeyO => "O",
        KeyP => "P",
        KeyQ => "Q",
        KeyR => "R",
        KeyS => "S",
        KeyT => "T",
        KeyU => "U",
        KeyV => "V",
        KeyW => "W",
        KeyX => "X",
        KeyY => "Y",
        KeyZ => "Z",
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
        Backquote => "`",
        Backslash | IntlBackslash => "\\",
        BracketLeft => "[",
        BracketRight => "]",
        Comma | NumpadComma => ",",
        Equal | NumpadEqual => "=",
        Minus | NumpadSubtract => "-",
        Period | NumpadDecimal => ".",
        Quote => "'",
        Semicolon => ";",
        Slash | NumpadDivide => "/",
        NumpadAdd => "+",
        NumpadMultiply | NumpadStar => "*",
        Space => "Space",
        Tab => "Tab",
        Enter => "Return",
        NumpadEnter => "Enter",
        Backspace | NumpadBackspace => "Backspace",
        Delete => "Del",
        Insert => "Ins",
        Home => "Home",
        End => "End",
        PageUp => "PgUp",
        PageDown => "PgDown",
        ArrowUp => "Up",
        ArrowDown => "Down",
        ArrowLeft => "Left",
        ArrowRight => "Right",
        Escape => "Esc",
        PrintScreen => "Print",
        ScrollLock => "ScrollLock",
        Pause => "Pause",
        CapsLock => "CapsLock",
        NumLock => "NumLock",
        ContextMenu => "Menu",
        Help => "Help",
        F1 => "F1",
        F2 => "F2",
        F3 => "F3",
        F4 => "F4",
        F5 => "F5",
        F6 => "F6",
        F7 => "F7",
        F8 => "F8",
        F9 => "F9",
        F10 => "F10",
        F11 => "F11",
        F12 => "F12",
        F13 => "F13",
        F14 => "F14",
        F15 => "F15",
        F16 => "F16",
        F17 => "F17",
        F18 => "F18",
        F19 => "F19",
        F20 => "F20",
        F21 => "F21",
        F22 => "F22",
        F23 => "F23",
        F24 => "F24",
        AudioVolumeDown => "Volume Down",
        AudioVolumeUp => "Volume Up",
        AudioVolumeMute => "Volume Mute",
        MediaPlayPause => "Media Play",
        MediaStop => "Media Stop",
        MediaTrackNext => "Media Next",
        MediaTrackPrevious => "Media Previous",
        _ => return None,
    };
    Some(s.to_string())
}

fn named_key_to_qt(key: &NamedKey) -> Option<&'static str> {
    use NamedKey::*;
    Some(match key {
        Enter => "Return",
        Tab => "Tab",
        Backspace => "Backspace",
        Delete => "Del",
        Insert => "Ins",
        Home => "Home",
        End => "End",
        PageUp => "PgUp",
        PageDown => "PgDown",
        ArrowUp => "Up",
        ArrowDown => "Down",
        ArrowLeft => "Left",
        ArrowRight => "Right",
        Escape => "Esc",
        PrintScreen => "Print",
        ScrollLock => "ScrollLock",
        Pause => "Pause",
        CapsLock => "CapsLock",
        NumLock => "NumLock",
        ContextMenu => "Menu",
        Help => "Help",
        Copy => "Copy",
        Cut => "Cut",
        Paste => "Paste",
        Undo => "Undo",
        Redo => "Redo",
        Find => "Find",
        Open => "Open",
        Save => "Save",
        New => "New",
        Close => "Close",
        Print => "Print",
        ZoomIn => "Zoom In",
        ZoomOut => "Zoom Out",
        AudioVolumeDown => "Volume Down",
        AudioVolumeUp => "Volume Up",
        AudioVolumeMute => "Volume Mute",
        MediaPlayPause => "Media Play",
        MediaStop => "Media Stop",
        MediaTrackNext => "Media Next",
        MediaTrackPrevious => "Media Previous",
        F1 => "F1",
        F2 => "F2",
        F3 => "F3",
        F4 => "F4",
        F5 => "F5",
        F6 => "F6",
        F7 => "F7",
        F8 => "F8",
        F9 => "F9",
        F10 => "F10",
        F11 => "F11",
        F12 => "F12",
        F13 => "F13",
        F14 => "F14",
        F15 => "F15",
        F16 => "F16",
        F17 => "F17",
        F18 => "F18",
        F19 => "F19",
        F20 => "F20",
        F21 => "F21",
        F22 => "F22",
        F23 => "F23",
        F24 => "F24",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_qt() {
        let a = Accelerator::new(Modifiers::CONTROL | Modifiers::SHIFT, Code::KeyN);
        assert_eq!(a.to_qt().as_deref(), Some("Ctrl+Shift+N"));
        let a = Accelerator::new(Modifiers::ALT, Code::F4);
        assert_eq!(a.to_qt().as_deref(), Some("Alt+F4"));
        let a = Accelerator::new(Modifiers::CONTROL, Code::Equal);
        assert_eq!(a.to_qt().as_deref(), Some("Ctrl+="));
    }
}
