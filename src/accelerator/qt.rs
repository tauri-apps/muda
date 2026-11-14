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
        // Qt key names are upper case, but keep letters whose upper case is several characters
        // ("ß" -> "SS") as they are: one key.
        ch if ch.is_alphabetic() => {
            let mut upper = ch.to_uppercase();
            match (upper.next(), upper.next()) {
                (Some(u), None) => u.to_string(),
                _ => ch.to_string(),
            }
        }
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
        Print => "Printer", // the Print command key; "Print" is Qt's Print Screen
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

    fn physical(mods: Modifiers, code: Code) -> Option<String> {
        Accelerator::new(mods, code).to_qt()
    }

    fn logical(mods: Modifiers, key: Key) -> Option<String> {
        KeyAccelerator::new(mods, key).to_qt()
    }

    fn character(c: &str) -> Option<String> {
        logical(Modifiers::empty(), Key::Character(c.into()))
    }

    #[test]
    fn to_qt() {
        let a = Accelerator::new(Modifiers::CONTROL | Modifiers::SHIFT, Code::KeyN);
        assert_eq!(a.to_qt().as_deref(), Some("Ctrl+Shift+N"));
        let a = Accelerator::new(Modifiers::ALT, Code::F4);
        assert_eq!(a.to_qt().as_deref(), Some("Alt+F4"));
        let a = Accelerator::new(Modifiers::CONTROL, Code::Equal);
        assert_eq!(a.to_qt().as_deref(), Some("Ctrl+="));
    }

    #[test]
    fn menu_accelerator_dispatches_to_both_kinds() {
        let p = MenuAccelerator::Physical(Accelerator::new(Modifiers::CONTROL, Code::KeyQ));
        assert_eq!(p.to_qt().as_deref(), Some("Ctrl+Q"));
        let l = MenuAccelerator::Logical(KeyAccelerator::new(
            Modifiers::ALT,
            Key::Named(NamedKey::Escape),
        ));
        assert_eq!(l.to_qt().as_deref(), Some("Alt+Esc"));
        let none = MenuAccelerator::Physical(Accelerator::new(Modifiers::CONTROL, Code::Fn));
        assert_eq!(none.to_qt(), None);
        let none = MenuAccelerator::Logical(KeyAccelerator::new(
            Modifiers::CONTROL,
            Key::Character("ab".into()),
        ));
        assert_eq!(none.to_qt(), None);
    }

    #[test]
    #[allow(deprecated)] // `Modifiers::SUPER`
    fn modifiers_in_fixed_order() {
        let m = |mods| physical(mods, Code::KeyA).unwrap();
        assert_eq!(m(Modifiers::empty()), "A");
        assert_eq!(m(Modifiers::CONTROL), "Ctrl+A");
        assert_eq!(m(Modifiers::ALT), "Alt+A");
        assert_eq!(m(Modifiers::SHIFT), "Shift+A");
        assert_eq!(m(Modifiers::META), "Meta+A");
        assert_eq!(m(Modifiers::SUPER), "Meta+A");
        // Meta and Super are the same key: named once.
        assert_eq!(m(Modifiers::META | Modifiers::SUPER), "Meta+A");
        assert_eq!(
            m(Modifiers::SUPER | Modifiers::SHIFT | Modifiers::ALT | Modifiers::CONTROL),
            "Ctrl+Alt+Shift+Meta+A"
        );
        assert_eq!(m(Modifiers::SHIFT | Modifiers::CONTROL), "Ctrl+Shift+A");
        // Modifiers Qt has no name for are dropped.
        assert_eq!(m(Modifiers::CAPS_LOCK | Modifiers::FN), "A");
        assert_eq!(
            logical(
                Modifiers::CONTROL | Modifiers::ALT | Modifiers::SHIFT | Modifiers::META,
                Key::Named(NamedKey::F1)
            )
            .as_deref(),
            Some("Ctrl+Alt+Shift+Meta+F1")
        );
    }

    #[test]
    fn every_code() {
        use Code::*;
        let table: &[(Code, &str)] = &[
            (KeyA, "A"),
            (KeyB, "B"),
            (KeyC, "C"),
            (KeyD, "D"),
            (KeyE, "E"),
            (KeyF, "F"),
            (KeyG, "G"),
            (KeyH, "H"),
            (KeyI, "I"),
            (KeyJ, "J"),
            (KeyK, "K"),
            (KeyL, "L"),
            (KeyM, "M"),
            (KeyN, "N"),
            (KeyO, "O"),
            (KeyP, "P"),
            (KeyQ, "Q"),
            (KeyR, "R"),
            (KeyS, "S"),
            (KeyT, "T"),
            (KeyU, "U"),
            (KeyV, "V"),
            (KeyW, "W"),
            (KeyX, "X"),
            (KeyY, "Y"),
            (KeyZ, "Z"),
            (Digit0, "0"),
            (Digit1, "1"),
            (Digit2, "2"),
            (Digit3, "3"),
            (Digit4, "4"),
            (Digit5, "5"),
            (Digit6, "6"),
            (Digit7, "7"),
            (Digit8, "8"),
            (Digit9, "9"),
            (Numpad0, "0"),
            (Numpad1, "1"),
            (Numpad2, "2"),
            (Numpad3, "3"),
            (Numpad4, "4"),
            (Numpad5, "5"),
            (Numpad6, "6"),
            (Numpad7, "7"),
            (Numpad8, "8"),
            (Numpad9, "9"),
            (Backquote, "`"),
            (Backslash, "\\"),
            (IntlBackslash, "\\"),
            (BracketLeft, "["),
            (BracketRight, "]"),
            (Comma, ","),
            (NumpadComma, ","),
            (Equal, "="),
            (NumpadEqual, "="),
            (Minus, "-"),
            (NumpadSubtract, "-"),
            (Period, "."),
            (NumpadDecimal, "."),
            (Quote, "'"),
            (Semicolon, ";"),
            (Slash, "/"),
            (NumpadDivide, "/"),
            (NumpadAdd, "+"),
            (NumpadMultiply, "*"),
            (NumpadStar, "*"),
            (Space, "Space"),
            (Tab, "Tab"),
            (Enter, "Return"),
            (NumpadEnter, "Enter"),
            (Backspace, "Backspace"),
            (NumpadBackspace, "Backspace"),
            (Delete, "Del"),
            (Insert, "Ins"),
            (Home, "Home"),
            (End, "End"),
            (PageUp, "PgUp"),
            (PageDown, "PgDown"),
            (ArrowUp, "Up"),
            (ArrowDown, "Down"),
            (ArrowLeft, "Left"),
            (ArrowRight, "Right"),
            (Escape, "Esc"),
            (PrintScreen, "Print"),
            (ScrollLock, "ScrollLock"),
            (Pause, "Pause"),
            (CapsLock, "CapsLock"),
            (NumLock, "NumLock"),
            (ContextMenu, "Menu"),
            (Help, "Help"),
            (F1, "F1"),
            (F2, "F2"),
            (F3, "F3"),
            (F4, "F4"),
            (F5, "F5"),
            (F6, "F6"),
            (F7, "F7"),
            (F8, "F8"),
            (F9, "F9"),
            (F10, "F10"),
            (F11, "F11"),
            (F12, "F12"),
            (F13, "F13"),
            (F14, "F14"),
            (F15, "F15"),
            (F16, "F16"),
            (F17, "F17"),
            (F18, "F18"),
            (F19, "F19"),
            (F20, "F20"),
            (F21, "F21"),
            (F22, "F22"),
            (F23, "F23"),
            (F24, "F24"),
            (AudioVolumeDown, "Volume Down"),
            (AudioVolumeUp, "Volume Up"),
            (AudioVolumeMute, "Volume Mute"),
            (MediaPlayPause, "Media Play"),
            (MediaStop, "Media Stop"),
            (MediaTrackNext, "Media Next"),
            (MediaTrackPrevious, "Media Previous"),
        ];
        for (code, name) in table {
            assert_eq!(
                physical(Modifiers::empty(), *code).as_deref(),
                Some(*name),
                "{code:?}"
            );
            assert_eq!(
                physical(Modifiers::CONTROL, *code),
                Some(format!("Ctrl+{name}")),
                "{code:?}"
            );
        }
    }

    #[test]
    fn unsupported_codes() {
        use Code::*;
        for code in [
            Fn,
            FnLock,
            ShiftLeft,
            ShiftRight,
            ControlLeft,
            AltRight,
            MetaLeft,
            NumpadParenLeft,
            NumpadHash,
            IntlRo,
            IntlYen,
            Lang1,
            BrowserBack,
            LaunchMail,
            Power,
            Unidentified,
        ] {
            assert_eq!(physical(Modifiers::CONTROL, code), None, "{code:?}");
        }
    }

    #[test]
    fn every_named_key() {
        use NamedKey::*;
        let table: &[(NamedKey, &str)] = &[
            (Enter, "Return"),
            (Tab, "Tab"),
            (Backspace, "Backspace"),
            (Delete, "Del"),
            (Insert, "Ins"),
            (Home, "Home"),
            (End, "End"),
            (PageUp, "PgUp"),
            (PageDown, "PgDown"),
            (ArrowUp, "Up"),
            (ArrowDown, "Down"),
            (ArrowLeft, "Left"),
            (ArrowRight, "Right"),
            (Escape, "Esc"),
            (PrintScreen, "Print"),
            (ScrollLock, "ScrollLock"),
            (Pause, "Pause"),
            (CapsLock, "CapsLock"),
            (NumLock, "NumLock"),
            (ContextMenu, "Menu"),
            (Help, "Help"),
            (Copy, "Copy"),
            (Cut, "Cut"),
            (Paste, "Paste"),
            (Undo, "Undo"),
            (Redo, "Redo"),
            (Find, "Find"),
            (Open, "Open"),
            (Save, "Save"),
            (New, "New"),
            (Close, "Close"),
            (Print, "Printer"),
            (ZoomIn, "Zoom In"),
            (ZoomOut, "Zoom Out"),
            (AudioVolumeDown, "Volume Down"),
            (AudioVolumeUp, "Volume Up"),
            (AudioVolumeMute, "Volume Mute"),
            (MediaPlayPause, "Media Play"),
            (MediaStop, "Media Stop"),
            (MediaTrackNext, "Media Next"),
            (MediaTrackPrevious, "Media Previous"),
            (F1, "F1"),
            (F2, "F2"),
            (F3, "F3"),
            (F4, "F4"),
            (F5, "F5"),
            (F6, "F6"),
            (F7, "F7"),
            (F8, "F8"),
            (F9, "F9"),
            (F10, "F10"),
            (F11, "F11"),
            (F12, "F12"),
            (F13, "F13"),
            (F14, "F14"),
            (F15, "F15"),
            (F16, "F16"),
            (F17, "F17"),
            (F18, "F18"),
            (F19, "F19"),
            (F20, "F20"),
            (F21, "F21"),
            (F22, "F22"),
            (F23, "F23"),
            (F24, "F24"),
        ];
        for (key, name) in table {
            assert_eq!(
                logical(Modifiers::empty(), Key::Named(*key)).as_deref(),
                Some(*name),
                "{key:?}"
            );
            assert_eq!(
                logical(Modifiers::SHIFT, Key::Named(*key)),
                Some(format!("Shift+{name}")),
                "{key:?}"
            );
        }
    }

    #[test]
    fn unsupported_named_keys() {
        use NamedKey::*;
        for key in [
            Alt,
            Control,
            Shift,
            Meta,
            Fn,
            Unidentified,
            Clear,
            BrowserBack,
            LaunchMail,
            Power,
            F25,
            F35,
        ] {
            assert_eq!(
                logical(Modifiers::CONTROL, Key::Named(key)),
                None,
                "{key:?}"
            );
        }
    }

    #[test]
    fn characters() {
        assert_eq!(character("a").as_deref(), Some("A"));
        assert_eq!(character("Z").as_deref(), Some("Z"));
        assert_eq!(character("é").as_deref(), Some("É"));
        assert_eq!(character("ж").as_deref(), Some("Ж"));
        assert_eq!(character(" ").as_deref(), Some("Space"));
        for symbol in [
            "1", "+", "-", "=", ",", ".", "/", "\\", "[", "]", "`", "'", ";", "*",
        ] {
            assert_eq!(character(symbol).as_deref(), Some(symbol), "{symbol:?}");
        }
        assert_eq!(
            logical(
                Modifiers::CONTROL | Modifiers::SHIFT,
                Key::Character("s".into())
            )
            .as_deref(),
            Some("Ctrl+Shift+S")
        );
        assert_eq!(
            logical(Modifiers::CONTROL, Key::Character("+".into())).as_deref(),
            Some("Ctrl++")
        );
        // Not a single character: no key.
        assert_eq!(character(""), None);
        assert_eq!(character("ab"), None);
        assert_eq!(character("F1"), None);
        assert_eq!(character("e\u{301}"), None);
    }

    #[test]
    fn character_with_multi_char_uppercase() {
        let s = character("ß").unwrap();
        assert_eq!(s.chars().count(), 1, "{s:?}");
    }

    #[test]
    fn named_print_is_the_printer_key() {
        assert_eq!(
            logical(Modifiers::empty(), Key::Named(NamedKey::Print)).as_deref(),
            Some("Printer")
        );
    }
}
