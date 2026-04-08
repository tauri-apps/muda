// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

//! Accelerators describe keyboard shortcuts for menu items.
//!
//! [`Accelerator`s](crate::accelerator::Accelerator) are used to define a keyboard shortcut consisting
//! of an optional combination of modifier keys (provided by [`Modifiers`]) and
//! one key ([`Code`] or [`Key`]).
//!
//! # Examples
//! They can be created directly
//! ```no_run
//! # use muda::accelerator::{Accelerator, Modifiers, Code, Key};
//! // Using a physical key code
//! let accelerator = Accelerator::new(Some(Modifiers::SHIFT), Code::KeyQ);
//! let accelerator_without_mods = Accelerator::new(None, Code::KeyQ);
//!
//! // Using a logical key for Unicode character shortcuts
//! let accelerator = Accelerator::with_key(Some(Modifiers::CONTROL), Key::Character("+".into()));
//! ```
//! or from `&str`, note that all modifiers
//! have to be listed before the non-modifier key, `shift+alt+KeyQ` is legal,
//! whereas `shift+q+alt` is not.
//! ```no_run
//! # use muda::accelerator::{Accelerator};
//! let accelerator: Accelerator = "shift+alt+KeyQ".parse().unwrap();
//! # // This assert exists to ensure a test breaks once the
//! # // statement above about ordering is no longer valid.
//! # assert!("shift+KeyQ+alt".parse::<Accelerator>().is_err());
//! ```
//!

pub use keyboard_types::{Code, Key, Modifiers};
use std::{borrow::Borrow, hash::Hash, str::FromStr};

#[cfg(target_os = "macos")]
pub const CMD_OR_CTRL: Modifiers = Modifiers::SUPER;
#[cfg(not(target_os = "macos"))]
pub const CMD_OR_CTRL: Modifiers = Modifiers::CONTROL;

#[derive(thiserror::Error, Debug)]
pub enum AcceleratorParseError {
    #[error("Couldn't recognize \"{0}\" as a valid key for accelerator, if you feel like it should be, please report this to https://github.com/tauri-apps/muda")]
    UnsupportedKey(String),
    #[error("Found empty token while parsing accelerator: {0}")]
    EmptyToken(String),
    #[error("Invalid accelerator format: \"{0}\", an accelerator should have the modifiers first and only one main key, for example: \"Shift + Alt + K\"")]
    InvalidFormat(String),
}

/// A keyboard shortcut that consists of an optional combination
/// of modifier keys (provided by [`Modifiers`]) and
/// one key ([`Code`] or [`Key`]).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Accelerator {
    pub(crate) mods: Modifiers,
    pub(crate) key: Key,
    id: u32,
}

impl Accelerator {
    /// Creates a new accelerator from a physical key [`Code`] to define keyboard shortcuts
    /// throughout your application.
    /// Only [`Modifiers::ALT`], [`Modifiers::SHIFT`], [`Modifiers::CONTROL`], and [`Modifiers::SUPER`]
    pub fn new(mods: Option<Modifiers>, key: Code) -> Self {
        Self::with_key(mods, code_to_key(key))
    }

    /// Creates a new accelerator from a logical [`Key`] to define keyboard shortcuts
    /// throughout your application. This allows expressing shortcuts like
    /// `Ctrl++` or `Ctrl+€` that cannot be represented by physical key codes.
    ///
    /// Only [`Modifiers::ALT`], [`Modifiers::SHIFT`], [`Modifiers::CONTROL`], and [`Modifiers::SUPER`]
    pub fn with_key(mods: Option<Modifiers>, key: Key) -> Self {
        let mut mods = mods.unwrap_or_else(Modifiers::empty);
        if mods.contains(Modifiers::META) {
            mods.remove(Modifiers::META);
            mods.insert(Modifiers::SUPER);
        }

        let id = Self::generate_hash(mods, &key);

        Self { mods, key, id }
    }

    fn generate_hash(mods: Modifiers, key: &Key) -> u32 {
        let mut accelerator_str = String::new();
        if mods.contains(Modifiers::SHIFT) {
            accelerator_str.push_str("shift+")
        }
        if mods.contains(Modifiers::CONTROL) {
            accelerator_str.push_str("control+")
        }
        if mods.contains(Modifiers::ALT) {
            accelerator_str.push_str("alt+")
        }
        if mods.contains(Modifiers::SUPER) {
            accelerator_str.push_str("super+")
        }
        accelerator_str.push_str(&key.to_string());

        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        accelerator_str.hash(&mut hasher);
        std::hash::Hasher::finish(&hasher) as u32
    }

    /// Returns the id associated with this accelerator
    /// which is a hash of the string representation of modifiers and key within this accelerator.
    pub fn id(&self) -> u32 {
        self.id
    }

    /// Returns the modifier for this accelerator
    pub fn modifiers(&self) -> Modifiers {
        self.mods
    }

    /// Returns the key for this accelerator.
    pub fn key(&self) -> &Key {
        &self.key
    }

    /// Returns `true` if this [`Code`] and [`Modifiers`] matches this `Accelerator`.
    ///
    /// For accelerators created with [`Accelerator::with_key`], this method
    /// compares by converting the provided `Code` to a `Key` first.
    pub fn matches(&self, modifiers: impl Borrow<Modifiers>, key: impl Borrow<Code>) -> bool {
        // Should be a const but const bit_or doesn't work here.
        let base_mods = Modifiers::SHIFT | Modifiers::CONTROL | Modifiers::ALT | Modifiers::SUPER;
        let modifiers = modifiers.borrow();
        let key = key.borrow();
        self.mods == *modifiers & base_mods && self.key == code_to_key(*key)
    }

    /// Returns `true` if this [`Key`] and [`Modifiers`] matches this `Accelerator`.
    pub fn matches_key(&self, modifiers: impl Borrow<Modifiers>, key: impl Borrow<Key>) -> bool {
        let base_mods = Modifiers::SHIFT | Modifiers::CONTROL | Modifiers::ALT | Modifiers::SUPER;
        let modifiers = modifiers.borrow();
        let key = key.borrow();
        self.mods == *modifiers & base_mods && self.key == *key
    }
}

impl FromStr for Accelerator {
    type Err = AcceleratorParseError;
    fn from_str(accelerator_string: &str) -> Result<Self, Self::Err> {
        parse_accelerator(accelerator_string)
    }
}

impl TryFrom<&str> for Accelerator {
    type Error = AcceleratorParseError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        parse_accelerator(value)
    }
}

impl TryFrom<String> for Accelerator {
    type Error = AcceleratorParseError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        parse_accelerator(&value)
    }
}

fn parse_accelerator(accelerator: &str) -> Result<Accelerator, AcceleratorParseError> {
    let tokens = accelerator.split('+').collect::<Vec<&str>>();

    let mut mods = Modifiers::empty();
    let mut key = None;

    match tokens.len() {
        // single key accelerator
        1 => {
            key = Some(parse_key(tokens[0])?);
        }

        // modifiers and key combo accelerator
        _ => {
            for raw in tokens {
                let token = raw.trim();

                if token.is_empty() {
                    // An empty token after the last '+' means '+' itself is the key.
                    // For example "Ctrl++" splits into ["Ctrl", "", ""],
                    // but we also get empty from "Ctrl+" (trailing plus).
                    // Only treat as the '+' key if we already have modifiers and no key yet.
                    if key.is_none() && !mods.is_empty() {
                        key = Some(Key::Character("+".into()));
                        continue;
                    }
                    return Err(AcceleratorParseError::EmptyToken(accelerator.to_string()));
                }

                if key.is_some() {
                    // At this point we have parsed the modifiers and a main key, so by reaching
                    // this code, the function either received more than one main key or
                    //  the accelerator is not in the right order
                    // examples:
                    // 1. "Ctrl+Shift+C+A" => only one main key should be allowd.
                    // 2. "Ctrl+C+Shift" => wrong order
                    return Err(AcceleratorParseError::InvalidFormat(
                        accelerator.to_string(),
                    ));
                }

                match token.to_uppercase().as_str() {
                    "OPTION" | "ALT" => {
                        mods |= Modifiers::ALT;
                    }
                    "CONTROL" | "CTRL" => {
                        mods |= Modifiers::CONTROL;
                    }
                    "COMMAND" | "CMD" | "SUPER" => {
                        mods |= Modifiers::META;
                    }
                    "SHIFT" => {
                        mods |= Modifiers::SHIFT;
                    }
                    #[cfg(target_os = "macos")]
                    "COMMANDORCONTROL" | "COMMANDORCTRL" | "CMDORCTRL" | "CMDORCONTROL" => {
                        mods |= Modifiers::SUPER;
                    }
                    #[cfg(not(target_os = "macos"))]
                    "COMMANDORCONTROL" | "COMMANDORCTRL" | "CMDORCTRL" | "CMDORCONTROL" => {
                        mods |= Modifiers::CONTROL;
                    }
                    _ => {
                        key = Some(parse_key(token)?);
                    }
                }
            }
        }
    }

    let key = key.ok_or_else(|| AcceleratorParseError::InvalidFormat(accelerator.to_string()))?;
    Ok(Accelerator::with_key(Some(mods), key))
}

fn parse_key(key: &str) -> Result<Key, AcceleratorParseError> {
    use Code::*;
    match key.to_uppercase().as_str() {
        "BACKQUOTE" | "`" => Ok(code_to_key(Backquote)),
        "BACKSLASH" | "\\" => Ok(code_to_key(Backslash)),
        "BRACKETLEFT" | "[" => Ok(code_to_key(BracketLeft)),
        "BRACKETRIGHT" | "]" => Ok(code_to_key(BracketRight)),
        "COMMA" | "," => Ok(code_to_key(Comma)),
        "DIGIT0" | "0" => Ok(code_to_key(Digit0)),
        "DIGIT1" | "1" => Ok(code_to_key(Digit1)),
        "DIGIT2" | "2" => Ok(code_to_key(Digit2)),
        "DIGIT3" | "3" => Ok(code_to_key(Digit3)),
        "DIGIT4" | "4" => Ok(code_to_key(Digit4)),
        "DIGIT5" | "5" => Ok(code_to_key(Digit5)),
        "DIGIT6" | "6" => Ok(code_to_key(Digit6)),
        "DIGIT7" | "7" => Ok(code_to_key(Digit7)),
        "DIGIT8" | "8" => Ok(code_to_key(Digit8)),
        "DIGIT9" | "9" => Ok(code_to_key(Digit9)),
        "EQUAL" | "=" => Ok(code_to_key(Equal)),
        "KEYA" | "A" => Ok(code_to_key(KeyA)),
        "KEYB" | "B" => Ok(code_to_key(KeyB)),
        "KEYC" | "C" => Ok(code_to_key(KeyC)),
        "KEYD" | "D" => Ok(code_to_key(KeyD)),
        "KEYE" | "E" => Ok(code_to_key(KeyE)),
        "KEYF" | "F" => Ok(code_to_key(KeyF)),
        "KEYG" | "G" => Ok(code_to_key(KeyG)),
        "KEYH" | "H" => Ok(code_to_key(KeyH)),
        "KEYI" | "I" => Ok(code_to_key(KeyI)),
        "KEYJ" | "J" => Ok(code_to_key(KeyJ)),
        "KEYK" | "K" => Ok(code_to_key(KeyK)),
        "KEYL" | "L" => Ok(code_to_key(KeyL)),
        "KEYM" | "M" => Ok(code_to_key(KeyM)),
        "KEYN" | "N" => Ok(code_to_key(KeyN)),
        "KEYO" | "O" => Ok(code_to_key(KeyO)),
        "KEYP" | "P" => Ok(code_to_key(KeyP)),
        "KEYQ" | "Q" => Ok(code_to_key(KeyQ)),
        "KEYR" | "R" => Ok(code_to_key(KeyR)),
        "KEYS" | "S" => Ok(code_to_key(KeyS)),
        "KEYT" | "T" => Ok(code_to_key(KeyT)),
        "KEYU" | "U" => Ok(code_to_key(KeyU)),
        "KEYV" | "V" => Ok(code_to_key(KeyV)),
        "KEYW" | "W" => Ok(code_to_key(KeyW)),
        "KEYX" | "X" => Ok(code_to_key(KeyX)),
        "KEYY" | "Y" => Ok(code_to_key(KeyY)),
        "KEYZ" | "Z" => Ok(code_to_key(KeyZ)),
        "MINUS" | "-" => Ok(code_to_key(Minus)),
        "PERIOD" | "." => Ok(code_to_key(Period)),
        "QUOTE" | "'" => Ok(code_to_key(Quote)),
        "SEMICOLON" | ";" => Ok(code_to_key(Semicolon)),
        "SLASH" | "/" => Ok(code_to_key(Slash)),
        "BACKSPACE" => Ok(Key::Backspace),
        "CAPSLOCK" => Ok(Key::CapsLock),
        "ENTER" => Ok(Key::Enter),
        "SPACE" => Ok(Key::Character(" ".into())),
        "TAB" => Ok(Key::Tab),
        "DELETE" => Ok(Key::Delete),
        "END" => Ok(Key::End),
        "HOME" => Ok(Key::Home),
        "INSERT" => Ok(Key::Insert),
        "PAGEDOWN" => Ok(Key::PageDown),
        "PAGEUP" => Ok(Key::PageUp),
        "PRINTSCREEN" => Ok(Key::PrintScreen),
        "SCROLLLOCK" => Ok(Key::ScrollLock),
        "ARROWDOWN" | "DOWN" => Ok(Key::ArrowDown),
        "ARROWLEFT" | "LEFT" => Ok(Key::ArrowLeft),
        "ARROWRIGHT" | "RIGHT" => Ok(Key::ArrowRight),
        "ARROWUP" | "UP" => Ok(Key::ArrowUp),
        "NUMLOCK" => Ok(Key::NumLock),
        "NUMPAD0" | "NUM0" => Ok(code_to_key(Numpad0)),
        "NUMPAD1" | "NUM1" => Ok(code_to_key(Numpad1)),
        "NUMPAD2" | "NUM2" => Ok(code_to_key(Numpad2)),
        "NUMPAD3" | "NUM3" => Ok(code_to_key(Numpad3)),
        "NUMPAD4" | "NUM4" => Ok(code_to_key(Numpad4)),
        "NUMPAD5" | "NUM5" => Ok(code_to_key(Numpad5)),
        "NUMPAD6" | "NUM6" => Ok(code_to_key(Numpad6)),
        "NUMPAD7" | "NUM7" => Ok(code_to_key(Numpad7)),
        "NUMPAD8" | "NUM8" => Ok(code_to_key(Numpad8)),
        "NUMPAD9" | "NUM9" => Ok(code_to_key(Numpad9)),
        "NUMPADADD" | "NUMADD" | "NUMPADPLUS" | "NUMPLUS" => Ok(code_to_key(NumpadAdd)),
        "NUMPADDECIMAL" | "NUMDECIMAL" => Ok(code_to_key(NumpadDecimal)),
        "NUMPADDIVIDE" | "NUMDIVIDE" => Ok(code_to_key(NumpadDivide)),
        "NUMPADENTER" | "NUMENTER" => Ok(Key::Enter),
        "NUMPADEQUAL" | "NUMEQUAL" => Ok(code_to_key(NumpadEqual)),
        "NUMPADMULTIPLY" | "NUMMULTIPLY" => Ok(code_to_key(NumpadMultiply)),
        "NUMPADSUBTRACT" | "NUMSUBTRACT" => Ok(code_to_key(NumpadSubtract)),
        "ESCAPE" | "ESC" => Ok(Key::Escape),
        "F1" => Ok(Key::F1),
        "F2" => Ok(Key::F2),
        "F3" => Ok(Key::F3),
        "F4" => Ok(Key::F4),
        "F5" => Ok(Key::F5),
        "F6" => Ok(Key::F6),
        "F7" => Ok(Key::F7),
        "F8" => Ok(Key::F8),
        "F9" => Ok(Key::F9),
        "F10" => Ok(Key::F10),
        "F11" => Ok(Key::F11),
        "F12" => Ok(Key::F12),
        "AUDIOVOLUMEDOWN" | "VOLUMEDOWN" => Ok(Key::AudioVolumeDown),
        "AUDIOVOLUMEUP" | "VOLUMEUP" => Ok(Key::AudioVolumeUp),
        "AUDIOVOLUMEMUTE" | "VOLUMEMUTE" => Ok(Key::AudioVolumeMute),
        "F13" => Ok(Key::F13),
        "F14" => Ok(Key::F14),
        "F15" => Ok(Key::F15),
        "F16" => Ok(Key::F16),
        "F17" => Ok(Key::F17),
        "F18" => Ok(Key::F18),
        "F19" => Ok(Key::F19),
        "F20" => Ok(Key::F20),
        "F21" => Ok(Key::F21),
        "F22" => Ok(Key::F22),
        "F23" => Ok(Key::F23),
        "F24" => Ok(Key::F24),

        _ => Err(AcceleratorParseError::UnsupportedKey(key.to_string())),
    }
}

/// Converts a physical [`Code`] to a logical [`Key`].
///
/// Character-producing codes (letters, digits, punctuation) are mapped to
/// their corresponding `Key::Character` values (lowercase for letters).
/// Non-character codes (function keys, navigation, etc.) are mapped to their
/// corresponding named `Key` variants.
pub fn code_to_key(code: Code) -> Key {
    match code {
        // Letters → lowercase characters
        Code::KeyA => Key::Character("a".into()),
        Code::KeyB => Key::Character("b".into()),
        Code::KeyC => Key::Character("c".into()),
        Code::KeyD => Key::Character("d".into()),
        Code::KeyE => Key::Character("e".into()),
        Code::KeyF => Key::Character("f".into()),
        Code::KeyG => Key::Character("g".into()),
        Code::KeyH => Key::Character("h".into()),
        Code::KeyI => Key::Character("i".into()),
        Code::KeyJ => Key::Character("j".into()),
        Code::KeyK => Key::Character("k".into()),
        Code::KeyL => Key::Character("l".into()),
        Code::KeyM => Key::Character("m".into()),
        Code::KeyN => Key::Character("n".into()),
        Code::KeyO => Key::Character("o".into()),
        Code::KeyP => Key::Character("p".into()),
        Code::KeyQ => Key::Character("q".into()),
        Code::KeyR => Key::Character("r".into()),
        Code::KeyS => Key::Character("s".into()),
        Code::KeyT => Key::Character("t".into()),
        Code::KeyU => Key::Character("u".into()),
        Code::KeyV => Key::Character("v".into()),
        Code::KeyW => Key::Character("w".into()),
        Code::KeyX => Key::Character("x".into()),
        Code::KeyY => Key::Character("y".into()),
        Code::KeyZ => Key::Character("z".into()),
        // Digits
        Code::Digit0 => Key::Character("0".into()),
        Code::Digit1 => Key::Character("1".into()),
        Code::Digit2 => Key::Character("2".into()),
        Code::Digit3 => Key::Character("3".into()),
        Code::Digit4 => Key::Character("4".into()),
        Code::Digit5 => Key::Character("5".into()),
        Code::Digit6 => Key::Character("6".into()),
        Code::Digit7 => Key::Character("7".into()),
        Code::Digit8 => Key::Character("8".into()),
        Code::Digit9 => Key::Character("9".into()),
        // Punctuation and symbols
        Code::Backquote => Key::Character("`".into()),
        Code::Backslash => Key::Character("\\".into()),
        Code::BracketLeft => Key::Character("[".into()),
        Code::BracketRight => Key::Character("]".into()),
        Code::Comma => Key::Character(",".into()),
        Code::Equal => Key::Character("=".into()),
        Code::Minus => Key::Character("-".into()),
        Code::Period => Key::Character(".".into()),
        Code::Quote => Key::Character("'".into()),
        Code::Semicolon => Key::Character(";".into()),
        Code::Slash => Key::Character("/".into()),
        Code::Space => Key::Character(" ".into()),
        // Numpad
        Code::Numpad0 => Key::Character("0".into()),
        Code::Numpad1 => Key::Character("1".into()),
        Code::Numpad2 => Key::Character("2".into()),
        Code::Numpad3 => Key::Character("3".into()),
        Code::Numpad4 => Key::Character("4".into()),
        Code::Numpad5 => Key::Character("5".into()),
        Code::Numpad6 => Key::Character("6".into()),
        Code::Numpad7 => Key::Character("7".into()),
        Code::Numpad8 => Key::Character("8".into()),
        Code::Numpad9 => Key::Character("9".into()),
        Code::NumpadAdd => Key::Character("+".into()),
        Code::NumpadDecimal => Key::Character(".".into()),
        Code::NumpadDivide => Key::Character("/".into()),
        Code::NumpadEqual => Key::Character("=".into()),
        Code::NumpadMultiply => Key::Character("*".into()),
        Code::NumpadSubtract => Key::Character("-".into()),
        Code::NumpadEnter => Key::Enter,
        // Named keys
        Code::Backspace => Key::Backspace,
        Code::CapsLock => Key::CapsLock,
        Code::Enter => Key::Enter,
        Code::Tab => Key::Tab,
        Code::Escape => Key::Escape,
        Code::Delete => Key::Delete,
        Code::End => Key::End,
        Code::Home => Key::Home,
        Code::Insert => Key::Insert,
        Code::PageDown => Key::PageDown,
        Code::PageUp => Key::PageUp,
        Code::PrintScreen => Key::PrintScreen,
        Code::ScrollLock => Key::ScrollLock,
        Code::NumLock => Key::NumLock,
        Code::Pause => Key::Pause,
        Code::ContextMenu => Key::ContextMenu,
        Code::Help => Key::Help,
        // Arrows
        Code::ArrowDown => Key::ArrowDown,
        Code::ArrowLeft => Key::ArrowLeft,
        Code::ArrowRight => Key::ArrowRight,
        Code::ArrowUp => Key::ArrowUp,
        // Function keys
        Code::F1 => Key::F1,
        Code::F2 => Key::F2,
        Code::F3 => Key::F3,
        Code::F4 => Key::F4,
        Code::F5 => Key::F5,
        Code::F6 => Key::F6,
        Code::F7 => Key::F7,
        Code::F8 => Key::F8,
        Code::F9 => Key::F9,
        Code::F10 => Key::F10,
        Code::F11 => Key::F11,
        Code::F12 => Key::F12,
        Code::F13 => Key::F13,
        Code::F14 => Key::F14,
        Code::F15 => Key::F15,
        Code::F16 => Key::F16,
        Code::F17 => Key::F17,
        Code::F18 => Key::F18,
        Code::F19 => Key::F19,
        Code::F20 => Key::F20,
        Code::F21 => Key::F21,
        Code::F22 => Key::F22,
        Code::F23 => Key::F23,
        Code::F24 => Key::F24,
        // Media and browser keys
        Code::AudioVolumeDown => Key::AudioVolumeDown,
        Code::AudioVolumeUp => Key::AudioVolumeUp,
        Code::AudioVolumeMute => Key::AudioVolumeMute,
        Code::MediaTrackNext => Key::MediaTrackNext,
        Code::MediaTrackPrevious => Key::MediaTrackPrevious,
        Code::MediaStop => Key::MediaStop,
        Code::MediaPlayPause => Key::MediaPlayPause,
        Code::LaunchMail => Key::LaunchMail,
        Code::BrowserBack => Key::BrowserBack,
        Code::BrowserForward => Key::BrowserForward,
        Code::BrowserRefresh => Key::BrowserRefresh,
        Code::BrowserStop => Key::BrowserStop,
        Code::BrowserSearch => Key::BrowserSearch,
        Code::BrowserFavorites => Key::BrowserFavorites,
        Code::BrowserHome => Key::BrowserHome,
        // IME keys
        Code::Convert => Key::Convert,
        Code::NonConvert => Key::NonConvert,
        Code::KanaMode => Key::KanaMode,
        // Modifier keys (mapped but typically not used as accelerator keys)
        Code::ControlLeft | Code::ControlRight => Key::Control,
        Code::AltLeft | Code::AltRight => Key::Alt,
        Code::ShiftLeft | Code::ShiftRight => Key::Shift,
        Code::MetaLeft | Code::MetaRight => Key::Meta,
        // Fallback
        _ => Key::Unidentified,
    }
}

#[test]
fn test_parse_accelerator() {
    macro_rules! assert_parse_accelerator {
        ($key:literal, $mods:expr, $expected_key:expr) => {
            let r = parse_accelerator($key).unwrap();
            assert_eq!(r.mods, $mods);
            assert_eq!(r.key, $expected_key);
        };
    }

    assert_parse_accelerator!("KeyX", Modifiers::empty(), Key::Character("x".into()));
    assert_parse_accelerator!("CTRL+KeyX", Modifiers::CONTROL, Key::Character("x".into()));
    assert_parse_accelerator!("SHIFT+KeyC", Modifiers::SHIFT, Key::Character("c".into()));

    assert_parse_accelerator!(
        "super+ctrl+SHIFT+alt+ArrowUp",
        Modifiers::SUPER | Modifiers::CONTROL | Modifiers::SHIFT | Modifiers::ALT,
        Key::ArrowUp
    );

    assert_parse_accelerator!("Digit5", Modifiers::empty(), Key::Character("5".into()));
    assert_parse_accelerator!("KeyG", Modifiers::empty(), Key::Character("g".into()));
    assert_parse_accelerator!("SHiFT+F12", Modifiers::SHIFT, Key::F12);

    assert_parse_accelerator!(
        "CmdOrCtrl+Space",
        {
            #[cfg(target_os = "macos")]
            {
                Modifiers::SUPER
            }
            #[cfg(not(target_os = "macos"))]
            {
                Modifiers::CONTROL
            }
        },
        Key::Character(" ".into())
    );

    // Test parsing of '+' character as key
    assert_parse_accelerator!("Ctrl++", Modifiers::CONTROL, Key::Character("+".into()));
}

#[test]
fn test_equality() {
    let h1 = parse_accelerator("Shift+KeyR").unwrap();
    let h2 = parse_accelerator("Shift+KeyR").unwrap();
    let h3 = Accelerator::new(Some(Modifiers::SHIFT), Code::KeyR);
    let h4 = parse_accelerator("Alt+KeyR").unwrap();
    let h5 = parse_accelerator("Alt+KeyR").unwrap();
    let h6 = parse_accelerator("KeyR").unwrap();

    assert!(h1 == h2 && h2 == h3 && h3 != h4 && h4 == h5 && h5 != h6);
    assert!(
        h1.id() == h2.id()
            && h2.id() == h3.id()
            && h3.id() != h4.id()
            && h4.id() == h5.id()
            && h5.id() != h6.id()
    );
}

#[test]
fn test_with_key() {
    let accel = Accelerator::with_key(Some(Modifiers::CONTROL), Key::Character("+".into()));
    assert_eq!(accel.mods, Modifiers::CONTROL);
    assert_eq!(accel.key, Key::Character("+".into()));
    assert!(accel.matches_key(Modifiers::CONTROL, Key::Character("+".into())));
}

#[test]
fn test_code_to_key_roundtrip() {
    // Verify that new(Code) and with_key(code_to_key(Code)) produce the same accelerator
    let a1 = Accelerator::new(Some(Modifiers::CONTROL), Code::KeyA);
    let a2 = Accelerator::with_key(Some(Modifiers::CONTROL), Key::Character("a".into()));
    assert_eq!(a1, a2);
    assert_eq!(a1.id(), a2.id());
}
