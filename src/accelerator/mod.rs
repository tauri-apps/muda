// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

//! Accelerators describe keyboard shortcuts for menu items.
//!
//! [`KeyAccelerator`s](crate::accelerator::KeyAccelerator) are used to define a keyboard
//! shortcut based on logical keys, which allows expressing shortcuts like `Ctrl++` or `Ctrl+€`
//! that physical key codes cannot represent.
//! For this reason, prefer to use [`KeyAccelerator`s](crate::accelerator::KeyAccelerator) over the older [`Accelerator`s](crate::accelerator::Accelerator).
//!
//! # Examples
//! They can be created directly
//! ```no_run
//! # use muda::accelerator::*;
//! let key_accelerator = KeyAccelerator::new(Modifiers::SHIFT, Key::Character("q".to_owned()));
//! let key_accelerator_without_mods = KeyAccelerator::new(Modifiers::empty(), Key::Character("q".to_owned()));
//!
//! let accelerator = Accelerator::new(Modifiers::SHIFT, Code::KeyQ);
//! let accelerator_without_mods = Accelerator::new(Modifiers::empty(), Code::KeyQ);
//! ```
//! or from `&str`, note that all modifiers
//! have to be listed before the non-modifier key, `shift+alt+KeyQ` is legal,
//! whereas `shift+q+alt` is not.
//! ```no_run
//! # use muda::accelerator::*;
//! let key_accelerator: KeyAccelerator = "shift+alt+q".parse().unwrap();
//!
//! // Or to parse Accelerator
//! let accelerator: Accelerator = "shift+alt+KeyQ".parse().unwrap();
//! let accelerator: Accelerator = "shift+alt+q".parse().unwrap();
//! # // This assert exists to ensure a test breaks once the
//! # // statement above about ordering is no longer valid.
//! # assert!("shift+KeyQ+alt".parse::<KeyAccelerator>().is_err());
//! # assert!("shift+KeyQ+alt".parse::<Accelerator>().is_err());
//! ```

pub use keyboard_types::{Code, Key, Modifiers, NamedKey};
use std::{borrow::Borrow, hash::Hash, str::FromStr};

#[cfg(all(
    any(
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd"
    ),
    feature = "gtk"
))]
pub(crate) mod gtk;
#[cfg(all(
    any(
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd"
    ),
    feature = "gtk4"
))]
pub(crate) mod gtk4;
#[cfg(target_os = "macos")]
pub(crate) mod macos;
#[cfg(target_os = "windows")]
pub(crate) mod windows;

#[cfg(target_os = "macos")]
pub const CMD_OR_CTRL: Modifiers = Modifiers::META;
#[cfg(not(target_os = "macos"))]
pub const CMD_OR_CTRL: Modifiers = Modifiers::CONTROL;

#[derive(thiserror::Error, Debug, PartialEq, Eq)]
pub enum AcceleratorParseError {
    #[error("Couldn't recognize \"{0}\" as a valid key for accelerator, if you feel like it should be, please report this to https://github.com/tauri-apps/muda")]
    UnsupportedKey(String),
    #[error("Found empty token while parsing accelerator: {0}")]
    EmptyToken(String),
    #[error("Invalid accelerator format: \"{0}\", an accelerator should have the modifiers first and only one main key, for example: \"Shift + Alt + K\"")]
    InvalidFormat(String),
}

/// A keyboard shortcut that consists of an optional combination
/// of modifier keys (provided by [`Modifiers`] and
/// one key ([`Code`]).
///
/// ## Warning
///
/// This struct cannot represent all shortcuts found on non-U.S. keyboard layouts and might
/// be deprecated in the future.
/// Please use [`KeyAccelerator`] instead.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Accelerator {
    pub(crate) mods: Modifiers,
    pub(crate) key: Code,
    id: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum MenuAccelerator {
    Physical(Accelerator),
    Logical(KeyAccelerator),
}

impl Accelerator {
    /// Creates a new accelerator to define keyboard shortcuts throughout your application.
    pub fn new(mods: Modifiers, key: Code) -> Self {
        let id = Self::generate_hash(mods, key);

        Self { mods, key, id }
    }

    fn generate_hash(mods: Modifiers, key: Code) -> u32 {
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
        if mods.contains(Modifiers::META) {
            accelerator_str.push_str("meta+")
        }
        #[allow(deprecated)]
        if mods.contains(Modifiers::SUPER) {
            accelerator_str.push_str("meta+")
        }
        accelerator_str.push_str(&format!("{:?}", key));

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

    /// Returns the code for this accelerator
    pub fn key(&self) -> Code {
        self.key
    }

    /// Returns `true` if this [`Code`] and [`Modifiers`] matches this `Accelerator`.
    pub fn matches(&self, modifiers: impl Borrow<Modifiers>, key: impl Borrow<Code>) -> bool {
        // Should be a const but const bit_or doesn't work here.
        #[allow(deprecated)]
        let base_mods = Modifiers::SHIFT
            | Modifiers::CONTROL
            | Modifiers::ALT
            | Modifiers::META
            | Modifiers::SUPER;
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
    let (modifiers_str, code_str) = split_key_and_modifiers(accelerator)?;
    let modifiers = parse_modifiers(accelerator, modifiers_str)?;
    let code = parse_code(accelerator, code_str)?;

    Ok(Accelerator::new(modifiers, code))
}

fn split_key_and_modifiers(accelerator: &str) -> Result<(&str, &str), AcceleratorParseError> {
    let accelerator = accelerator.trim();
    if accelerator.is_empty() {
        return Err(AcceleratorParseError::InvalidFormat(String::new()));
    }

    // Separate modifier part from key part using rfind('+').
    // This correctly handles '+' as the key: "Ctrl++" -> rfind gives the last '+',
    // leaving "Ctrl+" as the modifier part and "" as raw key -> key is '+'.
    let (modifiers_str, key_str) = match accelerator.rfind('+') {
        Some(pos) => {
            let raw_key = &accelerator[pos + 1..];
            if raw_key.trim().is_empty() {
                // The key is '+' itself; strip the trailing separator '+' from the modifier part
                let raw_mods = accelerator[..pos].trim_end_matches('+');
                (raw_mods, "+")
            } else {
                (&accelerator[..pos], raw_key.trim())
            }
        }
        None => ("", accelerator),
    };

    Ok((modifiers_str, key_str))
}

fn parse_modifiers(
    accelerator: &str,
    modifiers_str: &str,
) -> Result<Modifiers, AcceleratorParseError> {
    let mut modifiers = Modifiers::empty();
    if !modifiers_str.is_empty() {
        for token in modifiers_str.split('+') {
            modifiers |= parse_modifier(accelerator, token)?;
        }
    }

    Ok(modifiers)
}

fn parse_modifier(accelerator: &str, token: &str) -> Result<Modifiers, AcceleratorParseError> {
    let token = token.trim();
    if token.is_empty() {
        return Err(AcceleratorParseError::EmptyToken(accelerator.to_string()));
    }

    let modifier = match token.to_uppercase().as_str() {
        "OPTION" | "ALT" => Modifiers::ALT,
        "CONTROL" | "CTRL" => Modifiers::CONTROL,
        "COMMAND" | "CMD" | "SUPER" | "META" => Modifiers::META,
        "SHIFT" => Modifiers::SHIFT,
        #[cfg(target_os = "macos")]
        "COMMANDORCONTROL" | "COMMANDORCTRL" | "CMDORCTRL" | "CMDORCONTROL" => Modifiers::META,
        #[cfg(not(target_os = "macos"))]
        "COMMANDORCONTROL" | "COMMANDORCTRL" | "CMDORCTRL" | "CMDORCONTROL" => Modifiers::CONTROL,
        _ => {
            return Err(AcceleratorParseError::InvalidFormat(
                accelerator.to_string(),
            ));
        }
    };
    Ok(modifier)
}

fn parse_code(accelerator: &str, code: &str) -> Result<Code, AcceleratorParseError> {
    let code = code.trim();

    if code.is_empty() {
        return Err(AcceleratorParseError::EmptyToken(accelerator.to_string()));
    }

    parse_named_code(code)
        .or_else(|| code.parse::<Code>().ok())
        .ok_or_else(|| AcceleratorParseError::UnsupportedKey(code.to_string()))
}

fn parse_named_code(code: &str) -> Option<Code> {
    use Code::*;

    let code = match code.to_uppercase().as_str() {
        "BACKQUOTE" | "`" => Backquote,
        "BACKSLASH" | "\\" => Backslash,
        "BRACKETLEFT" | "[" => BracketLeft,
        "BRACKETRIGHT" | "]" => BracketRight,
        "COMMA" | "," => Comma,
        "DIGIT0" | "0" => Digit0,
        "DIGIT1" | "1" => Digit1,
        "DIGIT2" | "2" => Digit2,
        "DIGIT3" | "3" => Digit3,
        "DIGIT4" | "4" => Digit4,
        "DIGIT5" | "5" => Digit5,
        "DIGIT6" | "6" => Digit6,
        "DIGIT7" | "7" => Digit7,
        "DIGIT8" | "8" => Digit8,
        "DIGIT9" | "9" => Digit9,
        "EQUAL" | "=" => Equal,
        "INTLBACKSLASH" => IntlBackslash,
        "INTLRO" => IntlRo,
        "INTLYEN" => IntlYen,
        "KEYA" | "A" => KeyA,
        "KEYB" | "B" => KeyB,
        "KEYC" | "C" => KeyC,
        "KEYD" | "D" => KeyD,
        "KEYE" | "E" => KeyE,
        "KEYF" | "F" => KeyF,
        "KEYG" | "G" => KeyG,
        "KEYH" | "H" => KeyH,
        "KEYI" | "I" => KeyI,
        "KEYJ" | "J" => KeyJ,
        "KEYK" | "K" => KeyK,
        "KEYL" | "L" => KeyL,
        "KEYM" | "M" => KeyM,
        "KEYN" | "N" => KeyN,
        "KEYO" | "O" => KeyO,
        "KEYP" | "P" => KeyP,
        "KEYQ" | "Q" => KeyQ,
        "KEYR" | "R" => KeyR,
        "KEYS" | "S" => KeyS,
        "KEYT" | "T" => KeyT,
        "KEYU" | "U" => KeyU,
        "KEYV" | "V" => KeyV,
        "KEYW" | "W" => KeyW,
        "KEYX" | "X" => KeyX,
        "KEYY" | "Y" => KeyY,
        "KEYZ" | "Z" => KeyZ,
        "MINUS" | "-" => Minus,
        "PERIOD" | "." => Period,
        "QUOTE" | "'" => Quote,
        "SEMICOLON" | ";" => Semicolon,
        "SLASH" | "/" => Slash,
        "ALTLEFT" => AltLeft,
        "ALTRIGHT" => AltRight,
        "BACKSPACE" => Backspace,
        "CAPSLOCK" => CapsLock,
        "CONTROLLEFT" => ControlLeft,
        "CONTROLRIGHT" => ControlRight,
        "ENTER" => Enter,
        "METALEFT" => MetaLeft,
        "METARIGHT" => MetaRight,
        "SHIFTLEFT" => ShiftLeft,
        "SHIFTRIGHT" => ShiftRight,
        "SPACE" => Space,
        "TAB" => Tab,
        "CONVERT" => Convert,
        "KANAMODE" => KanaMode,
        "LANG1" => Lang1,
        "LANG2" => Lang2,
        "LANG3" => Lang3,
        "LANG4" => Lang4,
        "LANG5" => Lang5,
        "NONCONVERT" => NonConvert,
        "DELETE" => Delete,
        "END" => End,
        "HELP" => Help,
        "HOME" => Home,
        "INSERT" => Insert,
        "PAGEDOWN" => PageDown,
        "PAGEUP" => PageUp,
        "PRINTSCREEN" => PrintScreen,
        "SCROLLLOCK" => ScrollLock,
        "ARROWDOWN" | "DOWN" => ArrowDown,
        "ARROWLEFT" | "LEFT" => ArrowLeft,
        "ARROWRIGHT" | "RIGHT" => ArrowRight,
        "ARROWUP" | "UP" => ArrowUp,
        "CONTEXTMENU" | "MENU" | "APPS" => ContextMenu,
        "NUMLOCK" => NumLock,
        "NUMPAD0" | "NUM0" => Numpad0,
        "NUMPAD1" | "NUM1" => Numpad1,
        "NUMPAD2" | "NUM2" => Numpad2,
        "NUMPAD3" | "NUM3" => Numpad3,
        "NUMPAD4" | "NUM4" => Numpad4,
        "NUMPAD5" | "NUM5" => Numpad5,
        "NUMPAD6" | "NUM6" => Numpad6,
        "NUMPAD7" | "NUM7" => Numpad7,
        "NUMPAD8" | "NUM8" => Numpad8,
        "NUMPAD9" | "NUM9" => Numpad9,
        "NUMPADADD" | "NUMADD" | "NUMPADPLUS" | "NUMPLUS" => NumpadAdd,
        "NUMPADBACKSPACE" => NumpadBackspace,
        "NUMPADCLEAR" | "NUMCLEAR" => NumpadClear,
        "NUMPADCLEARENTRY" => NumpadClearEntry,
        "NUMPADCOMMA" => NumpadComma,
        "NUMPADDECIMAL" | "NUMDECIMAL" => NumpadDecimal,
        "NUMPADDIVIDE" | "NUMDIVIDE" => NumpadDivide,
        "NUMPADENTER" | "NUMENTER" => NumpadEnter,
        "NUMPADEQUAL" | "NUMEQUAL" => NumpadEqual,
        "NUMPADHASH" => NumpadHash,
        "NUMPADMEMORYADD" => NumpadMemoryAdd,
        "NUMPADMEMORYCLEAR" => NumpadMemoryClear,
        "NUMPADMEMORYRECALL" => NumpadMemoryRecall,
        "NUMPADMEMORYSTORE" => NumpadMemoryStore,
        "NUMPADMEMORYSUBTRACT" => NumpadMemorySubtract,
        "NUMPADMULTIPLY" | "NUMMULTIPLY" => NumpadMultiply,
        "NUMPADPARENLEFT" => NumpadParenLeft,
        "NUMPADPARENRIGHT" => NumpadParenRight,
        "NUMPADSTAR" => NumpadStar,
        "NUMPADSUBTRACT" | "NUMSUBTRACT" => NumpadSubtract,
        "ESCAPE" | "ESC" => Escape,
        "FN" => Fn,
        "FNLOCK" => FnLock,
        "PAUSE" | "PAUSEBREAK" => Pause,
        "BROWSERBACK" => BrowserBack,
        "BROWSERFAVORITES" => BrowserFavorites,
        "BROWSERFORWARD" => BrowserForward,
        "BROWSERHOME" => BrowserHome,
        "BROWSERREFRESH" => BrowserRefresh,
        "BROWSERSEARCH" => BrowserSearch,
        "BROWSERSTOP" => BrowserStop,
        "EJECT" => Eject,
        "LAUNCHAPP1" => LaunchApp1,
        "LAUNCHAPPLICATION1" | "MYCOMPUTER" => LaunchApp1,
        "LAUNCHAPP2" => LaunchApp2,
        "LAUNCHAPPLICATION2" | "CALCULATOR" => LaunchApp2,
        "LAUNCHMAIL" => LaunchMail,
        "LAUNCHMEDIAPLAYER" => MediaSelect,
        "MEDIAPLAYPAUSE" => MediaPlayPause,
        "MEDIASELECT" => MediaSelect,
        "MEDIASTOP" => MediaStop,
        "MEDIATRACKNEXT" => MediaTrackNext,
        "MEDIATRACKPREVIOUS" => MediaTrackPrevious,
        "POWER" => Power,
        "SLEEP" => Sleep,
        "AUDIOVOLUMEDOWN" | "VOLUMEDOWN" => AudioVolumeDown,
        "AUDIOVOLUMEUP" | "VOLUMEUP" => AudioVolumeUp,
        "AUDIOVOLUMEMUTE" | "VOLUMEMUTE" => AudioVolumeMute,
        "WAKEUP" => WakeUp,
        #[allow(deprecated)]
        "HYPER" => Hyper,
        #[allow(deprecated)]
        "SUPER" => Super,
        #[allow(deprecated)]
        "TURBO" => Turbo,
        "ABORT" => Abort,
        "RESUME" => Resume,
        "SUSPEND" => Suspend,
        "AGAIN" => Again,
        "COPY" => Copy,
        "CUT" => Cut,
        "FIND" => Find,
        "OPEN" => Open,
        "PASTE" => Paste,
        "PROPS" => Props,
        "SELECT" => Select,
        "UNDO" => Undo,
        "HIRAGANA" => Hiragana,
        "KATAKANA" => Katakana,
        "UNIDENTIFIED" => Unidentified,
        "F1" => F1,
        "F2" => F2,
        "F3" => F3,
        "F4" => F4,
        "F5" => F5,
        "F6" => F6,
        "F7" => F7,
        "F8" => F8,
        "F9" => F9,
        "F10" => F10,
        "F11" => F11,
        "F12" => F12,
        "F13" => F13,
        "F14" => F14,
        "F15" => F15,
        "F16" => F16,
        "F17" => F17,
        "F18" => F18,
        "F19" => F19,
        "F20" => F20,
        "F21" => F21,
        "F22" => F22,
        "F23" => F23,
        "F24" => F24,
        "F25" => F25,
        "F26" => F26,
        "F27" => F27,
        "F28" => F28,
        "F29" => F29,
        "F30" => F30,
        "F31" => F31,
        "F32" => F32,
        "F33" => F33,
        "F34" => F34,
        "F35" => F35,
        "BRIGHTNESSDOWN" => BrightnessDown,
        "BRIGHTNESSUP" => BrightnessUp,
        "DISPLAYTOGGLEINTEXT" => DisplayToggleIntExt,
        "KEYBOARDLAYOUTSELECT" => KeyboardLayoutSelect,
        "LAUNCHASSISTANT" => LaunchAssistant,
        "LAUNCHCONTROLPANEL" => LaunchControlPanel,
        "LAUNCHSCREENSAVER" => LaunchScreenSaver,
        "MAILFORWARD" => MailForward,
        "MAILREPLY" => MailReply,
        "MAILSEND" => MailSend,
        "MEDIAFASTFORWARD" => MediaFastForward,
        "MEDIAPAUSE" => MediaPause,
        "MEDIAPLAY" => MediaPlay,
        "MEDIARECORD" => MediaRecord,
        "MEDIAREWIND" => MediaRewind,
        "MICROPHONEMUTETOGGLE" => MicrophoneMuteToggle,
        "PRIVACYSCREENTOGGLE" => PrivacyScreenToggle,
        "SELECTTASK" => SelectTask,
        "SHOWALLWINDOWS" => ShowAllWindows,
        "ZOOMTOGGLE" => ZoomToggle,
        _ => return None,
    };
    Some(code)
}

/// A keyboard shortcut based on logical [`Key`] values.
///
/// Unlike [`Accelerator`] which uses physical [`Code`] keys,
/// `KeyAccelerator` uses logical [`Key`] values which can represent
/// any character including Unicode characters like `+`, `€`, `{`, etc.
///
/// # Examples
///
/// They can be created directly
/// ```no_run
/// # use muda::accelerator::{KeyAccelerator, Modifiers, Key};
/// let accel = KeyAccelerator::new(Modifiers::CONTROL, Key::Character("+".into()));
/// ```
/// or parsed from a string, which supports literal character keys
/// ```no_run
/// # use muda::accelerator::KeyAccelerator;
/// let accel: KeyAccelerator = "Ctrl++".parse().unwrap();
/// let accel2: KeyAccelerator = "Ctrl+€".parse().unwrap();
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct KeyAccelerator {
    pub(crate) mods: Modifiers,
    pub(crate) key: Key,
    id: u32,
}

impl KeyAccelerator {
    /// Creates a new key accelerator to define keyboard shortcuts throughout your application.
    /// Only [`Modifiers::ALT`], [`Modifiers::SHIFT`], [`Modifiers::CONTROL`], [`Modifiers::META`],
    /// and [`Modifiers::SUPER`].
    pub fn new(mods: Modifiers, key: Key) -> Self {
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
        if mods.contains(Modifiers::META) {
            accelerator_str.push_str("meta+")
        }
        #[allow(deprecated)]
        if mods.contains(Modifiers::SUPER) {
            accelerator_str.push_str("meta+")
        }
        accelerator_str.push_str(&format!("{:?}", key));

        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        accelerator_str.hash(&mut hasher);
        std::hash::Hasher::finish(&hasher) as u32
    }

    /// Returns the id associated with this accelerator
    /// which is a hash of the string representation of modifiers and key within this accelerator.
    pub fn id(&self) -> u32 {
        self.id
    }

    /// Returns the modifiers for this accelerator.
    pub fn modifiers(&self) -> Modifiers {
        self.mods
    }

    /// Returns the key for this accelerator.
    pub fn key(&self) -> &Key {
        &self.key
    }

    /// Returns `true` if this [`Key`] and [`Modifiers`] matches this `KeyAccelerator`.
    pub fn matches(&self, modifiers: impl Borrow<Modifiers>, key: impl Borrow<Key>) -> bool {
        #[allow(deprecated)]
        let base_mods = Modifiers::SHIFT
            | Modifiers::CONTROL
            | Modifiers::ALT
            | Modifiers::META
            | Modifiers::SUPER;
        let modifiers = modifiers.borrow();
        let key = key.borrow();
        self.mods == *modifiers & base_mods && self.key == *key
    }
}

impl FromStr for KeyAccelerator {
    type Err = AcceleratorParseError;
    fn from_str(accelerator_string: &str) -> Result<Self, Self::Err> {
        parse_key_accelerator(accelerator_string)
    }
}

impl TryFrom<&str> for KeyAccelerator {
    type Error = AcceleratorParseError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        parse_key_accelerator(value)
    }
}

impl TryFrom<String> for KeyAccelerator {
    type Error = AcceleratorParseError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        parse_key_accelerator(&value)
    }
}

fn parse_key_accelerator(accelerator: &str) -> Result<KeyAccelerator, AcceleratorParseError> {
    let (modifiers_str, key_str) = split_key_and_modifiers(accelerator)?;
    let mods = parse_modifiers(accelerator, modifiers_str)?;
    let key = parse_key(key_str)?;
    Ok(KeyAccelerator::new(mods, key))
}

fn parse_key(key: &str) -> Result<Key, AcceleratorParseError> {
    let trimmed = key.trim();
    if trimmed.is_empty() {
        return Err(AcceleratorParseError::UnsupportedKey(key.to_string()));
    }

    parse_named_key(trimmed)
        .or_else(|| trimmed.parse::<Key>().ok())
        .ok_or_else(|| AcceleratorParseError::UnsupportedKey(key.into()))
}

fn parse_named_key(key: &str) -> Option<Key> {
    use NamedKey::*;

    let key = match key.to_ascii_uppercase().as_str() {
        "SPACE" => Key::Character(" ".into()),
        "KEYA" | "A" => Key::Character("a".into()),
        "KEYB" | "B" => Key::Character("b".into()),
        "KEYC" | "C" => Key::Character("c".into()),
        "KEYD" | "D" => Key::Character("d".into()),
        "KEYE" | "E" => Key::Character("e".into()),
        "KEYF" | "F" => Key::Character("f".into()),
        "KEYG" | "G" => Key::Character("g".into()),
        "KEYH" | "H" => Key::Character("h".into()),
        "KEYI" | "I" => Key::Character("i".into()),
        "KEYJ" | "J" => Key::Character("j".into()),
        "KEYK" | "K" => Key::Character("k".into()),
        "KEYL" | "L" => Key::Character("l".into()),
        "KEYM" | "M" => Key::Character("m".into()),
        "KEYN" | "N" => Key::Character("n".into()),
        "KEYO" | "O" => Key::Character("o".into()),
        "KEYP" | "P" => Key::Character("p".into()),
        "KEYQ" | "Q" => Key::Character("q".into()),
        "KEYR" | "R" => Key::Character("r".into()),
        "KEYS" | "S" => Key::Character("s".into()),
        "KEYT" | "T" => Key::Character("t".into()),
        "KEYU" | "U" => Key::Character("u".into()),
        "KEYV" | "V" => Key::Character("v".into()),
        "KEYW" | "W" => Key::Character("w".into()),
        "KEYX" | "X" => Key::Character("x".into()),
        "KEYY" | "Y" => Key::Character("y".into()),
        "KEYZ" | "Z" => Key::Character("z".into()),
        "UNIDENTIFIED" => Key::Named(Unidentified),
        "ALT" => Key::Named(Alt),
        "ALTGRAPH" => Key::Named(AltGraph),
        "CAPSLOCK" => Key::Named(CapsLock),
        "CONTROL" => Key::Named(Control),
        "FN" => Key::Named(Fn),
        "FNLOCK" => Key::Named(FnLock),
        "META" => Key::Named(Meta),
        "NUMLOCK" => Key::Named(NumLock),
        "SCROLLLOCK" => Key::Named(ScrollLock),
        "SHIFT" => Key::Named(Shift),
        "SYMBOL" => Key::Named(Symbol),
        "SYMBOLLOCK" => Key::Named(SymbolLock),
        #[allow(deprecated)]
        "HYPER" => Key::Named(Hyper),
        #[allow(deprecated)]
        "SUPER" => Key::Named(Super),
        "ENTER" => Key::Named(Enter),
        "TAB" => Key::Named(Tab),
        "ARROWDOWN" | "DOWN" => Key::Named(ArrowDown),
        "ARROWLEFT" | "LEFT" => Key::Named(ArrowLeft),
        "ARROWRIGHT" | "RIGHT" => Key::Named(ArrowRight),
        "ARROWUP" | "UP" => Key::Named(ArrowUp),
        "END" => Key::Named(End),
        "HOME" => Key::Named(Home),
        "PAGEDOWN" => Key::Named(PageDown),
        "PAGEUP" => Key::Named(PageUp),
        "BACKSPACE" => Key::Named(Backspace),
        "CLEAR" => Key::Named(Clear),
        "COPY" => Key::Named(Copy),
        "CRSEL" => Key::Named(CrSel),
        "CUT" => Key::Named(Cut),
        "DELETE" => Key::Named(Delete),
        "ERASEEOF" => Key::Named(EraseEof),
        "EXSEL" => Key::Named(ExSel),
        "INSERT" => Key::Named(Insert),
        "PASTE" => Key::Named(Paste),
        "REDO" => Key::Named(Redo),
        "UNDO" => Key::Named(Undo),
        "ACCEPT" => Key::Named(Accept),
        "AGAIN" => Key::Named(Again),
        "ATTN" => Key::Named(Attn),
        "CANCEL" => Key::Named(Cancel),
        "CONTEXTMENU" => Key::Named(ContextMenu),
        "ESCAPE" | "ESC" => Key::Named(Escape),
        "EXECUTE" => Key::Named(Execute),
        "FIND" => Key::Named(Find),
        "HELP" => Key::Named(Help),
        "PAUSE" => Key::Named(Pause),
        "PLAY" => Key::Named(Play),
        "PROPS" => Key::Named(Props),
        "SELECT" => Key::Named(Select),
        "ZOOMIN" => Key::Named(ZoomIn),
        "ZOOMOUT" => Key::Named(ZoomOut),
        "BRIGHTNESSDOWN" => Key::Named(BrightnessDown),
        "BRIGHTNESSUP" => Key::Named(BrightnessUp),
        "EJECT" => Key::Named(Eject),
        "LOGOFF" => Key::Named(LogOff),
        "POWER" => Key::Named(Power),
        "POWEROFF" => Key::Named(PowerOff),
        "PRINTSCREEN" => Key::Named(PrintScreen),
        "HIBERNATE" => Key::Named(Hibernate),
        "STANDBY" => Key::Named(Standby),
        "WAKEUP" => Key::Named(WakeUp),
        "ALLCANDIDATES" => Key::Named(AllCandidates),
        "ALPHANUMERIC" => Key::Named(Alphanumeric),
        "CODEINPUT" => Key::Named(CodeInput),
        "COMPOSE" => Key::Named(Compose),
        "CONVERT" => Key::Named(Convert),
        "DEAD" => Key::Named(Dead),
        "FINALMODE" => Key::Named(FinalMode),
        "GROUPFIRST" => Key::Named(GroupFirst),
        "GROUPLAST" => Key::Named(GroupLast),
        "GROUPNEXT" => Key::Named(GroupNext),
        "GROUPPREVIOUS" => Key::Named(GroupPrevious),
        "MODECHANGE" => Key::Named(ModeChange),
        "NEXTCANDIDATE" => Key::Named(NextCandidate),
        "NONCONVERT" => Key::Named(NonConvert),
        "PREVIOUSCANDIDATE" => Key::Named(PreviousCandidate),
        "PROCESS" => Key::Named(Process),
        "SINGLECANDIDATE" => Key::Named(SingleCandidate),
        "HANGULMODE" => Key::Named(HangulMode),
        "HANJAMODE" => Key::Named(HanjaMode),
        "JUNJAMODE" => Key::Named(JunjaMode),
        "EISU" => Key::Named(Eisu),
        "HANKAKU" => Key::Named(Hankaku),
        "HIRAGANA" => Key::Named(Hiragana),
        "HIRAGANAKATAKANA" => Key::Named(HiraganaKatakana),
        "KANAMODE" => Key::Named(KanaMode),
        "KANJIMODE" => Key::Named(KanjiMode),
        "KATAKANA" => Key::Named(Katakana),
        "ROMAJI" => Key::Named(Romaji),
        "ZENKAKU" => Key::Named(Zenkaku),
        "ZENKAKUHANKAKU" => Key::Named(ZenkakuHankaku),
        "SOFT1" => Key::Named(Soft1),
        "SOFT2" => Key::Named(Soft2),
        "SOFT3" => Key::Named(Soft3),
        "SOFT4" => Key::Named(Soft4),
        "CHANNELDOWN" => Key::Named(ChannelDown),
        "CHANNELUP" => Key::Named(ChannelUp),
        "CLOSE" => Key::Named(Close),
        "MAILFORWARD" => Key::Named(MailForward),
        "MAILREPLY" => Key::Named(MailReply),
        "MAILSEND" => Key::Named(MailSend),
        "MEDIACLOSE" => Key::Named(MediaClose),
        "MEDIAFASTFORWARD" => Key::Named(MediaFastForward),
        "MEDIAPAUSE" => Key::Named(MediaPause),
        "MEDIAPLAY" => Key::Named(MediaPlay),
        "MEDIAPLAYPAUSE" => Key::Named(MediaPlayPause),
        "MEDIARECORD" => Key::Named(MediaRecord),
        "MEDIAREWIND" => Key::Named(MediaRewind),
        "MEDIASTOP" => Key::Named(MediaStop),
        "MEDIATRACKNEXT" => Key::Named(MediaTrackNext),
        "MEDIATRACKPREVIOUS" => Key::Named(MediaTrackPrevious),
        "NEW" => Key::Named(New),
        "OPEN" => Key::Named(Open),
        "PRINT" => Key::Named(Print),
        "SAVE" => Key::Named(Save),
        "SPELLCHECK" => Key::Named(SpellCheck),
        "KEY11" => Key::Named(Key11),
        "KEY12" => Key::Named(Key12),
        "AUDIOBALANCELEFT" => Key::Named(AudioBalanceLeft),
        "AUDIOBALANCERIGHT" => Key::Named(AudioBalanceRight),
        "AUDIOBASSBOOSTDOWN" => Key::Named(AudioBassBoostDown),
        "AUDIOBASSBOOSTTOGGLE" => Key::Named(AudioBassBoostToggle),
        "AUDIOBASSBOOSTUP" => Key::Named(AudioBassBoostUp),
        "AUDIOFADERFRONT" => Key::Named(AudioFaderFront),
        "AUDIOFADERREAR" => Key::Named(AudioFaderRear),
        "AUDIOSURROUNDMODENEXT" => Key::Named(AudioSurroundModeNext),
        "AUDIOTREBLEDOWN" => Key::Named(AudioTrebleDown),
        "AUDIOTREBLEUP" => Key::Named(AudioTrebleUp),
        "AUDIOVOLUMEDOWN" | "VOLUMEDOWN" => Key::Named(AudioVolumeDown),
        "AUDIOVOLUMEUP" | "VOLUMEUP" => Key::Named(AudioVolumeUp),
        "AUDIOVOLUMEMUTE" | "VOLUMEMUTE" => Key::Named(AudioVolumeMute),
        "MICROPHONETOGGLE" => Key::Named(MicrophoneToggle),
        "MICROPHONEVOLUMEDOWN" => Key::Named(MicrophoneVolumeDown),
        "MICROPHONEVOLUMEUP" => Key::Named(MicrophoneVolumeUp),
        "MICROPHONEVOLUMEMUTE" => Key::Named(MicrophoneVolumeMute),
        "SPEECHCORRECTIONLIST" => Key::Named(SpeechCorrectionList),
        "SPEECHINPUTTOGGLE" => Key::Named(SpeechInputToggle),
        "LAUNCHAPPLICATION1" => Key::Named(LaunchApplication1),
        "LAUNCHAPPLICATION2" => Key::Named(LaunchApplication2),
        "LAUNCHCALENDAR" => Key::Named(LaunchCalendar),
        "LAUNCHCONTACTS" => Key::Named(LaunchContacts),
        "LAUNCHMAIL" => Key::Named(LaunchMail),
        "LAUNCHMEDIAPLAYER" => Key::Named(LaunchMediaPlayer),
        "LAUNCHMUSICPLAYER" => Key::Named(LaunchMusicPlayer),
        "LAUNCHPHONE" => Key::Named(LaunchPhone),
        "LAUNCHSCREENSAVER" => Key::Named(LaunchScreenSaver),
        "LAUNCHSPREADSHEET" => Key::Named(LaunchSpreadsheet),
        "LAUNCHWEBBROWSER" => Key::Named(LaunchWebBrowser),
        "LAUNCHWEBCAM" => Key::Named(LaunchWebCam),
        "LAUNCHWORDPROCESSOR" => Key::Named(LaunchWordProcessor),
        "BROWSERBACK" => Key::Named(BrowserBack),
        "BROWSERFAVORITES" => Key::Named(BrowserFavorites),
        "BROWSERFORWARD" => Key::Named(BrowserForward),
        "BROWSERHOME" => Key::Named(BrowserHome),
        "BROWSERREFRESH" => Key::Named(BrowserRefresh),
        "BROWSERSEARCH" => Key::Named(BrowserSearch),
        "BROWSERSTOP" => Key::Named(BrowserStop),
        "APPSWITCH" => Key::Named(AppSwitch),
        "CALL" => Key::Named(Call),
        "CAMERA" => Key::Named(Camera),
        "CAMERAFOCUS" => Key::Named(CameraFocus),
        "ENDCALL" => Key::Named(EndCall),
        "GOBACK" => Key::Named(GoBack),
        "GOHOME" => Key::Named(GoHome),
        "HEADSETHOOK" => Key::Named(HeadsetHook),
        "LASTNUMBERREDIAL" => Key::Named(LastNumberRedial),
        "NOTIFICATION" => Key::Named(Notification),
        "MANNERMODE" => Key::Named(MannerMode),
        "VOICEDIAL" => Key::Named(VoiceDial),
        "TV" => Key::Named(TV),
        "TV3DMODE" => Key::Named(TV3DMode),
        "TVANTENNACABLE" => Key::Named(TVAntennaCable),
        "TVAUDIODESCRIPTION" => Key::Named(TVAudioDescription),
        "TVAUDIODESCRIPTIONMIXDOWN" => Key::Named(TVAudioDescriptionMixDown),
        "TVAUDIODESCRIPTIONMIXUP" => Key::Named(TVAudioDescriptionMixUp),
        "TVCONTENTSMENU" => Key::Named(TVContentsMenu),
        "TVDATASERVICE" => Key::Named(TVDataService),
        "TVINPUT" => Key::Named(TVInput),
        "TVINPUTCOMPONENT1" => Key::Named(TVInputComponent1),
        "TVINPUTCOMPONENT2" => Key::Named(TVInputComponent2),
        "TVINPUTCOMPOSITE1" => Key::Named(TVInputComposite1),
        "TVINPUTCOMPOSITE2" => Key::Named(TVInputComposite2),
        "TVINPUTHDMI1" => Key::Named(TVInputHDMI1),
        "TVINPUTHDMI2" => Key::Named(TVInputHDMI2),
        "TVINPUTHDMI3" => Key::Named(TVInputHDMI3),
        "TVINPUTHDMI4" => Key::Named(TVInputHDMI4),
        "TVINPUTVGA1" => Key::Named(TVInputVGA1),
        "TVMEDIACONTEXT" => Key::Named(TVMediaContext),
        "TVNETWORK" => Key::Named(TVNetwork),
        "TVNUMBERENTRY" => Key::Named(TVNumberEntry),
        "TVPOWER" => Key::Named(TVPower),
        "TVRADIOSERVICE" => Key::Named(TVRadioService),
        "TVSATELLITE" => Key::Named(TVSatellite),
        "TVSATELLITEBS" => Key::Named(TVSatelliteBS),
        "TVSATELLITECS" => Key::Named(TVSatelliteCS),
        "TVSATELLITETOGGLE" => Key::Named(TVSatelliteToggle),
        "TVTERRESTRIALANALOG" => Key::Named(TVTerrestrialAnalog),
        "TVTERRESTRIALDIGITAL" => Key::Named(TVTerrestrialDigital),
        "TVTIMER" => Key::Named(TVTimer),
        "AVRINPUT" => Key::Named(AVRInput),
        "AVRPOWER" => Key::Named(AVRPower),
        "COLORF0RED" => Key::Named(ColorF0Red),
        "COLORF1GREEN" => Key::Named(ColorF1Green),
        "COLORF2YELLOW" => Key::Named(ColorF2Yellow),
        "COLORF3BLUE" => Key::Named(ColorF3Blue),
        "COLORF4GREY" => Key::Named(ColorF4Grey),
        "COLORF5BROWN" => Key::Named(ColorF5Brown),
        "CLOSEDCAPTIONTOGGLE" => Key::Named(ClosedCaptionToggle),
        "DIMMER" => Key::Named(Dimmer),
        "DISPLAYSWAP" => Key::Named(DisplaySwap),
        "DVR" => Key::Named(DVR),
        "EXIT" => Key::Named(Exit),
        "FAVORITECLEAR0" => Key::Named(FavoriteClear0),
        "FAVORITECLEAR1" => Key::Named(FavoriteClear1),
        "FAVORITECLEAR2" => Key::Named(FavoriteClear2),
        "FAVORITECLEAR3" => Key::Named(FavoriteClear3),
        "FAVORITERECALL0" => Key::Named(FavoriteRecall0),
        "FAVORITERECALL1" => Key::Named(FavoriteRecall1),
        "FAVORITERECALL2" => Key::Named(FavoriteRecall2),
        "FAVORITERECALL3" => Key::Named(FavoriteRecall3),
        "FAVORITESTORE0" => Key::Named(FavoriteStore0),
        "FAVORITESTORE1" => Key::Named(FavoriteStore1),
        "FAVORITESTORE2" => Key::Named(FavoriteStore2),
        "FAVORITESTORE3" => Key::Named(FavoriteStore3),
        "GUIDE" => Key::Named(Guide),
        "GUIDENEXTDAY" => Key::Named(GuideNextDay),
        "GUIDEPREVIOUSDAY" => Key::Named(GuidePreviousDay),
        "INFO" => Key::Named(Info),
        "INSTANTREPLAY" => Key::Named(InstantReplay),
        "LINK" => Key::Named(Link),
        "LISTPROGRAM" => Key::Named(ListProgram),
        "LIVECONTENT" => Key::Named(LiveContent),
        "LOCK" => Key::Named(Lock),
        "MEDIAAPPS" => Key::Named(MediaApps),
        "MEDIAAUDIOTRACK" => Key::Named(MediaAudioTrack),
        "MEDIALAST" => Key::Named(MediaLast),
        "MEDIASKIPBACKWARD" => Key::Named(MediaSkipBackward),
        "MEDIASKIPFORWARD" => Key::Named(MediaSkipForward),
        "MEDIASTEPBACKWARD" => Key::Named(MediaStepBackward),
        "MEDIASTEPFORWARD" => Key::Named(MediaStepForward),
        "MEDIATOPMENU" => Key::Named(MediaTopMenu),
        "NAVIGATEIN" => Key::Named(NavigateIn),
        "NAVIGATENEXT" => Key::Named(NavigateNext),
        "NAVIGATEOUT" => Key::Named(NavigateOut),
        "NAVIGATEPREVIOUS" => Key::Named(NavigatePrevious),
        "NEXTFAVORITECHANNEL" => Key::Named(NextFavoriteChannel),
        "NEXTUSERPROFILE" => Key::Named(NextUserProfile),
        "ONDEMAND" => Key::Named(OnDemand),
        "PAIRING" => Key::Named(Pairing),
        "PINPDOWN" => Key::Named(PinPDown),
        "PINPMOVE" => Key::Named(PinPMove),
        "PINPTOGGLE" => Key::Named(PinPToggle),
        "PINPUP" => Key::Named(PinPUp),
        "PLAYSPEEDDOWN" => Key::Named(PlaySpeedDown),
        "PLAYSPEEDRESET" => Key::Named(PlaySpeedReset),
        "PLAYSPEEDUP" => Key::Named(PlaySpeedUp),
        "RANDOMTOGGLE" => Key::Named(RandomToggle),
        "RCLOWBATTERY" => Key::Named(RcLowBattery),
        "RECORDSPEEDNEXT" => Key::Named(RecordSpeedNext),
        "RFBYPASS" => Key::Named(RfBypass),
        "SCANCHANNELSTOGGLE" => Key::Named(ScanChannelsToggle),
        "SCREENMODENEXT" => Key::Named(ScreenModeNext),
        "SETTINGS" => Key::Named(Settings),
        "SPLITSCREENTOGGLE" => Key::Named(SplitScreenToggle),
        "STBINPUT" => Key::Named(STBInput),
        "STBPOWER" => Key::Named(STBPower),
        "SUBTITLE" => Key::Named(Subtitle),
        "TELETEXT" => Key::Named(Teletext),
        "VIDEOMODENEXT" => Key::Named(VideoModeNext),
        "WINK" => Key::Named(Wink),
        "ZOOMTOGGLE" => Key::Named(ZoomToggle),
        "F1" => Key::Named(F1),
        "F2" => Key::Named(F2),
        "F3" => Key::Named(F3),
        "F4" => Key::Named(F4),
        "F5" => Key::Named(F5),
        "F6" => Key::Named(F6),
        "F7" => Key::Named(F7),
        "F8" => Key::Named(F8),
        "F9" => Key::Named(F9),
        "F10" => Key::Named(F10),
        "F11" => Key::Named(F11),
        "F12" => Key::Named(F12),
        "F13" => Key::Named(F13),
        "F14" => Key::Named(F14),
        "F15" => Key::Named(F15),
        "F16" => Key::Named(F16),
        "F17" => Key::Named(F17),
        "F18" => Key::Named(F18),
        "F19" => Key::Named(F19),
        "F20" => Key::Named(F20),
        "F21" => Key::Named(F21),
        "F22" => Key::Named(F22),
        "F23" => Key::Named(F23),
        "F24" => Key::Named(F24),
        "F25" => Key::Named(F25),
        "F26" => Key::Named(F26),
        "F27" => Key::Named(F27),
        "F28" => Key::Named(F28),
        "F29" => Key::Named(F29),
        "F30" => Key::Named(F30),
        "F31" => Key::Named(F31),
        "F32" => Key::Named(F32),
        "F33" => Key::Named(F33),
        "F34" => Key::Named(F34),
        "F35" => Key::Named(F35),
        _ => return None,
    };
    Some(key)
}

#[test]
fn test_parse_accelerator() {
    macro_rules! assert_parse_accelerator {
        ($key:literal, $lrh:expr) => {
            let r = parse_accelerator($key).unwrap();
            let l = $lrh;
            assert_eq!(r.mods, l.mods);
            assert_eq!(r.key, l.key);
        };
    }

    assert_parse_accelerator!(
        "KeyX",
        Accelerator {
            mods: Modifiers::empty(),
            key: Code::KeyX,
            id: 0,
        }
    );

    assert_parse_accelerator!(
        "CTRL+KeyX",
        Accelerator {
            mods: Modifiers::CONTROL,
            key: Code::KeyX,
            id: 0,
        }
    );

    assert_parse_accelerator!(
        "SHIFT+KeyC",
        Accelerator {
            mods: Modifiers::SHIFT,
            key: Code::KeyC,
            id: 0,
        }
    );

    assert_parse_accelerator!(
        "SHIFT+KeyC",
        Accelerator {
            mods: Modifiers::SHIFT,
            key: Code::KeyC,
            id: 0,
        }
    );

    assert_parse_accelerator!(
        "meta+ctrl+SHIFT+alt+ArrowUp",
        Accelerator {
            mods: Modifiers::META | Modifiers::CONTROL | Modifiers::SHIFT | Modifiers::ALT,
            key: Code::ArrowUp,
            id: 0,
        }
    );
    assert_parse_accelerator!(
        "Digit5",
        Accelerator {
            mods: Modifiers::empty(),
            key: Code::Digit5,
            id: 0,
        }
    );
    assert_parse_accelerator!(
        "KeyG",
        Accelerator {
            mods: Modifiers::empty(),
            key: Code::KeyG,
            id: 0,
        }
    );

    assert_parse_accelerator!(
        "SHiFT+F12",
        Accelerator {
            mods: Modifiers::SHIFT,
            key: Code::F12,
            id: 0,
        }
    );

    assert_parse_accelerator!(
        "CmdOrCtrl+Space",
        Accelerator {
            #[cfg(target_os = "macos")]
            mods: Modifiers::META,
            #[cfg(not(target_os = "macos"))]
            mods: Modifiers::CONTROL,
            key: Code::Space,
            id: 0,
        }
    );
}

#[test]
fn test_parse_accelerator_error() {
    let cases = [
        (
            "Ctrl+Shift+C+A",
            AcceleratorParseError::InvalidFormat("Ctrl+Shift+C+A".into()),
        ),
        (
            "Ctrl+C+Shift",
            AcceleratorParseError::InvalidFormat("Ctrl+C+Shift".into()),
        ),
        ("Alt", AcceleratorParseError::UnsupportedKey("Alt".into())),
        ("Cmd", AcceleratorParseError::UnsupportedKey("Cmd".into())),
        ("Ctrl", AcceleratorParseError::UnsupportedKey("Ctrl".into())),
        ("+", AcceleratorParseError::UnsupportedKey("+".into())),
    ];
    for (text, err) in cases {
        let parsed = text.parse::<Accelerator>();
        assert_eq!(parsed, Err(err), "Expected parsing \"{text}\" to error!");
    }
}

#[test]
fn test_parse_key_accelerator_error() {
    let cases = [
        (
            "Ctrl+Shift+C+A",
            AcceleratorParseError::InvalidFormat("Ctrl+Shift+C+A".into()),
        ),
        (
            "Ctrl+C+Shift",
            AcceleratorParseError::InvalidFormat("Ctrl+C+Shift".into()),
        ),
        ("Cmd", AcceleratorParseError::UnsupportedKey("Cmd".into())),
        ("Ctrl", AcceleratorParseError::UnsupportedKey("Ctrl".into())),
    ];
    for (text, err) in cases {
        let parsed = text.parse::<KeyAccelerator>();
        assert_eq!(parsed, Err(err), "Expected parsing \"{text}\" to error!");
    }
}

#[test]
fn test_equality() {
    let h1 = parse_accelerator("Shift+KeyR").unwrap();
    let h2 = parse_accelerator("Shift+KeyR").unwrap();
    let h3 = Accelerator::new(Modifiers::SHIFT, Code::KeyR);
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
fn test_parse_key_accelerator() {
    // Basic logical key parsing
    let cases = [
        ("Ctrl+Q", Modifiers::CONTROL, Key::Character("q".into())),
        ("Ctrl+q", Modifiers::CONTROL, Key::Character("q".into())),
        ("Ctrl+KeyQ", Modifiers::CONTROL, Key::Character("q".into())),
        ("Shift+f12", Modifiers::SHIFT, Key::Named(NamedKey::F12)),
        // Literal '+' as key
        ("Ctrl++", Modifiers::CONTROL, Key::Character("+".into())),
        // Multiple modifiers + '+'
        (
            "Ctrl+Shift++",
            Modifiers::CONTROL | Modifiers::SHIFT,
            Key::Character("+".into()),
        ),
        // Just '+' alone
        ("+", Modifiers::empty(), Key::Character("+".into())),
        // Unicode character keys
        ("Ctrl+€", Modifiers::CONTROL, Key::Character("€".into())),
        // CmdOrCtrl works
        #[cfg(target_os = "macos")]
        (
            "CmdOrCtrl+Space",
            Modifiers::META,
            Key::Character(" ".into()),
        ),
        #[cfg(not(target_os = "macos"))]
        (
            "CmdOrCtrl+Space",
            Modifiers::CONTROL,
            Key::Character(" ".into()),
        ),
    ];

    for (string, modifiers, key) in cases {
        let accelerator: KeyAccelerator = string.parse().expect("Failed to parse KeyAccelerator");
        assert_eq!(
            accelerator.mods, modifiers,
            "Expected \"{string}\" to produce modifiers: {modifiers:?}"
        );
        assert_eq!(
            accelerator.key, key,
            "Expected \"{string}\" to produce key: {key}"
        );
    }
}

#[test]
fn test_key_accelerator_equality() {
    let h1: KeyAccelerator = "Shift+R".parse().unwrap();
    let h2: KeyAccelerator = "Shift+KeyR".parse().unwrap();
    let h3 = KeyAccelerator::new(Modifiers::SHIFT, Key::Character("r".into()));
    let h4: KeyAccelerator = "Alt+r".parse().unwrap();

    assert!(h1 == h2 && h2 == h3 && h3 != h4);
    assert!(h1.id() == h2.id() && h2.id() == h3.id() && h3.id() != h4.id());
}
