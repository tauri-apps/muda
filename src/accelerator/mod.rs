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

pub use keyboard_types::{Code, Key, Modifiers};
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
        if mods.contains(Modifiers::SUPER) {
            accelerator_str.push_str("super+")
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
        "COMMAND" | "CMD" | "META" => Modifiers::META,
        "SUPER" => Modifiers::SUPER,
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
        "HYPER" => Hyper,
        "SUPER" => Super,
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
        if mods.contains(Modifiers::SUPER) {
            accelerator_str.push_str("super+")
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

// Parses a logical [`Key`] only; physical [`Code`] names belong to [`Accelerator`].
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
    use Key::*;

    let key = match key.to_ascii_uppercase().as_str() {
        "SPACE" => Character(" ".into()),
        "KEYA" | "A" => Character("a".into()),
        "KEYB" | "B" => Character("b".into()),
        "KEYC" | "C" => Character("c".into()),
        "KEYD" | "D" => Character("d".into()),
        "KEYE" | "E" => Character("e".into()),
        "KEYF" | "F" => Character("f".into()),
        "KEYG" | "G" => Character("g".into()),
        "KEYH" | "H" => Character("h".into()),
        "KEYI" | "I" => Character("i".into()),
        "KEYJ" | "J" => Character("j".into()),
        "KEYK" | "K" => Character("k".into()),
        "KEYL" | "L" => Character("l".into()),
        "KEYM" | "M" => Character("m".into()),
        "KEYN" | "N" => Character("n".into()),
        "KEYO" | "O" => Character("o".into()),
        "KEYP" | "P" => Character("p".into()),
        "KEYQ" | "Q" => Character("q".into()),
        "KEYR" | "R" => Character("r".into()),
        "KEYS" | "S" => Character("s".into()),
        "KEYT" | "T" => Character("t".into()),
        "KEYU" | "U" => Character("u".into()),
        "KEYV" | "V" => Character("v".into()),
        "KEYW" | "W" => Character("w".into()),
        "KEYX" | "X" => Character("x".into()),
        "KEYY" | "Y" => Character("y".into()),
        "KEYZ" | "Z" => Character("z".into()),
        "UNIDENTIFIED" => Unidentified,
        "ALT" => Alt,
        "ALTGRAPH" => AltGraph,
        "CAPSLOCK" => CapsLock,
        "CONTROL" => Control,
        "FN" => Fn,
        "FNLOCK" => FnLock,
        "META" => Meta,
        "NUMLOCK" => NumLock,
        "SCROLLLOCK" => ScrollLock,
        "SHIFT" => Shift,
        "SYMBOL" => Symbol,
        "SYMBOLLOCK" => SymbolLock,
        "HYPER" => Hyper,
        "SUPER" => Super,
        "ENTER" => Enter,
        "TAB" => Tab,
        "ARROWDOWN" | "DOWN" => ArrowDown,
        "ARROWLEFT" | "LEFT" => ArrowLeft,
        "ARROWRIGHT" | "RIGHT" => ArrowRight,
        "ARROWUP" | "UP" => ArrowUp,
        "END" => End,
        "HOME" => Home,
        "PAGEDOWN" => PageDown,
        "PAGEUP" => PageUp,
        "BACKSPACE" => Backspace,
        "CLEAR" => Clear,
        "COPY" => Copy,
        "CRSEL" => CrSel,
        "CUT" => Cut,
        "DELETE" => Delete,
        "ERASEEOF" => EraseEof,
        "EXSEL" => ExSel,
        "INSERT" => Insert,
        "PASTE" => Paste,
        "REDO" => Redo,
        "UNDO" => Undo,
        "ACCEPT" => Accept,
        "AGAIN" => Again,
        "ATTN" => Attn,
        "CANCEL" => Cancel,
        "CONTEXTMENU" => ContextMenu,
        "ESCAPE" | "ESC" => Escape,
        "EXECUTE" => Execute,
        "FIND" => Find,
        "HELP" => Help,
        "PAUSE" => Pause,
        "PLAY" => Play,
        "PROPS" => Props,
        "SELECT" => Select,
        "ZOOMIN" => ZoomIn,
        "ZOOMOUT" => ZoomOut,
        "BRIGHTNESSDOWN" => BrightnessDown,
        "BRIGHTNESSUP" => BrightnessUp,
        "EJECT" => Eject,
        "LOGOFF" => LogOff,
        "POWER" => Power,
        "POWEROFF" => PowerOff,
        "PRINTSCREEN" => PrintScreen,
        "HIBERNATE" => Hibernate,
        "STANDBY" => Standby,
        "WAKEUP" => WakeUp,
        "ALLCANDIDATES" => AllCandidates,
        "ALPHANUMERIC" => Alphanumeric,
        "CODEINPUT" => CodeInput,
        "COMPOSE" => Compose,
        "CONVERT" => Convert,
        "DEAD" => Dead,
        "FINALMODE" => FinalMode,
        "GROUPFIRST" => GroupFirst,
        "GROUPLAST" => GroupLast,
        "GROUPNEXT" => GroupNext,
        "GROUPPREVIOUS" => GroupPrevious,
        "MODECHANGE" => ModeChange,
        "NEXTCANDIDATE" => NextCandidate,
        "NONCONVERT" => NonConvert,
        "PREVIOUSCANDIDATE" => PreviousCandidate,
        "PROCESS" => Process,
        "SINGLECANDIDATE" => SingleCandidate,
        "HANGULMODE" => HangulMode,
        "HANJAMODE" => HanjaMode,
        "JUNJAMODE" => JunjaMode,
        "EISU" => Eisu,
        "HANKAKU" => Hankaku,
        "HIRAGANA" => Hiragana,
        "HIRAGANAKATAKANA" => HiraganaKatakana,
        "KANAMODE" => KanaMode,
        "KANJIMODE" => KanjiMode,
        "KATAKANA" => Katakana,
        "ROMAJI" => Romaji,
        "ZENKAKU" => Zenkaku,
        "ZENKAKUHANKAKU" => ZenkakuHankaku,
        "SOFT1" => Soft1,
        "SOFT2" => Soft2,
        "SOFT3" => Soft3,
        "SOFT4" => Soft4,
        "CHANNELDOWN" => ChannelDown,
        "CHANNELUP" => ChannelUp,
        "CLOSE" => Close,
        "MAILFORWARD" => MailForward,
        "MAILREPLY" => MailReply,
        "MAILSEND" => MailSend,
        "MEDIACLOSE" => MediaClose,
        "MEDIAFASTFORWARD" => MediaFastForward,
        "MEDIAPAUSE" => MediaPause,
        "MEDIAPLAY" => MediaPlay,
        "MEDIAPLAYPAUSE" => MediaPlayPause,
        "MEDIARECORD" => MediaRecord,
        "MEDIAREWIND" => MediaRewind,
        "MEDIASTOP" => MediaStop,
        "MEDIATRACKNEXT" => MediaTrackNext,
        "MEDIATRACKPREVIOUS" => MediaTrackPrevious,
        "NEW" => New,
        "OPEN" => Open,
        "PRINT" => Print,
        "SAVE" => Save,
        "SPELLCHECK" => SpellCheck,
        "KEY11" => Key11,
        "KEY12" => Key12,
        "AUDIOBALANCELEFT" => AudioBalanceLeft,
        "AUDIOBALANCERIGHT" => AudioBalanceRight,
        "AUDIOBASSBOOSTDOWN" => AudioBassBoostDown,
        "AUDIOBASSBOOSTTOGGLE" => AudioBassBoostToggle,
        "AUDIOBASSBOOSTUP" => AudioBassBoostUp,
        "AUDIOFADERFRONT" => AudioFaderFront,
        "AUDIOFADERREAR" => AudioFaderRear,
        "AUDIOSURROUNDMODENEXT" => AudioSurroundModeNext,
        "AUDIOTREBLEDOWN" => AudioTrebleDown,
        "AUDIOTREBLEUP" => AudioTrebleUp,
        "AUDIOVOLUMEDOWN" | "VOLUMEDOWN" => AudioVolumeDown,
        "AUDIOVOLUMEUP" | "VOLUMEUP" => AudioVolumeUp,
        "AUDIOVOLUMEMUTE" | "VOLUMEMUTE" => AudioVolumeMute,
        "MICROPHONETOGGLE" => MicrophoneToggle,
        "MICROPHONEVOLUMEDOWN" => MicrophoneVolumeDown,
        "MICROPHONEVOLUMEUP" => MicrophoneVolumeUp,
        "MICROPHONEVOLUMEMUTE" => MicrophoneVolumeMute,
        "SPEECHCORRECTIONLIST" => SpeechCorrectionList,
        "SPEECHINPUTTOGGLE" => SpeechInputToggle,
        "LAUNCHAPPLICATION1" => LaunchApplication1,
        "LAUNCHAPPLICATION2" => LaunchApplication2,
        "LAUNCHCALENDAR" => LaunchCalendar,
        "LAUNCHCONTACTS" => LaunchContacts,
        "LAUNCHMAIL" => LaunchMail,
        "LAUNCHMEDIAPLAYER" => LaunchMediaPlayer,
        "LAUNCHMUSICPLAYER" => LaunchMusicPlayer,
        "LAUNCHPHONE" => LaunchPhone,
        "LAUNCHSCREENSAVER" => LaunchScreenSaver,
        "LAUNCHSPREADSHEET" => LaunchSpreadsheet,
        "LAUNCHWEBBROWSER" => LaunchWebBrowser,
        "LAUNCHWEBCAM" => LaunchWebCam,
        "LAUNCHWORDPROCESSOR" => LaunchWordProcessor,
        "BROWSERBACK" => BrowserBack,
        "BROWSERFAVORITES" => BrowserFavorites,
        "BROWSERFORWARD" => BrowserForward,
        "BROWSERHOME" => BrowserHome,
        "BROWSERREFRESH" => BrowserRefresh,
        "BROWSERSEARCH" => BrowserSearch,
        "BROWSERSTOP" => BrowserStop,
        "APPSWITCH" => AppSwitch,
        "CALL" => Call,
        "CAMERA" => Camera,
        "CAMERAFOCUS" => CameraFocus,
        "ENDCALL" => EndCall,
        "GOBACK" => GoBack,
        "GOHOME" => GoHome,
        "HEADSETHOOK" => HeadsetHook,
        "LASTNUMBERREDIAL" => LastNumberRedial,
        "NOTIFICATION" => Notification,
        "MANNERMODE" => MannerMode,
        "VOICEDIAL" => VoiceDial,
        "TV" => TV,
        "TV3DMODE" => TV3DMode,
        "TVANTENNACABLE" => TVAntennaCable,
        "TVAUDIODESCRIPTION" => TVAudioDescription,
        "TVAUDIODESCRIPTIONMIXDOWN" => TVAudioDescriptionMixDown,
        "TVAUDIODESCRIPTIONMIXUP" => TVAudioDescriptionMixUp,
        "TVCONTENTSMENU" => TVContentsMenu,
        "TVDATASERVICE" => TVDataService,
        "TVINPUT" => TVInput,
        "TVINPUTCOMPONENT1" => TVInputComponent1,
        "TVINPUTCOMPONENT2" => TVInputComponent2,
        "TVINPUTCOMPOSITE1" => TVInputComposite1,
        "TVINPUTCOMPOSITE2" => TVInputComposite2,
        "TVINPUTHDMI1" => TVInputHDMI1,
        "TVINPUTHDMI2" => TVInputHDMI2,
        "TVINPUTHDMI3" => TVInputHDMI3,
        "TVINPUTHDMI4" => TVInputHDMI4,
        "TVINPUTVGA1" => TVInputVGA1,
        "TVMEDIACONTEXT" => TVMediaContext,
        "TVNETWORK" => TVNetwork,
        "TVNUMBERENTRY" => TVNumberEntry,
        "TVPOWER" => TVPower,
        "TVRADIOSERVICE" => TVRadioService,
        "TVSATELLITE" => TVSatellite,
        "TVSATELLITEBS" => TVSatelliteBS,
        "TVSATELLITECS" => TVSatelliteCS,
        "TVSATELLITETOGGLE" => TVSatelliteToggle,
        "TVTERRESTRIALANALOG" => TVTerrestrialAnalog,
        "TVTERRESTRIALDIGITAL" => TVTerrestrialDigital,
        "TVTIMER" => TVTimer,
        "AVRINPUT" => AVRInput,
        "AVRPOWER" => AVRPower,
        "COLORF0RED" => ColorF0Red,
        "COLORF1GREEN" => ColorF1Green,
        "COLORF2YELLOW" => ColorF2Yellow,
        "COLORF3BLUE" => ColorF3Blue,
        "COLORF4GREY" => ColorF4Grey,
        "COLORF5BROWN" => ColorF5Brown,
        "CLOSEDCAPTIONTOGGLE" => ClosedCaptionToggle,
        "DIMMER" => Dimmer,
        "DISPLAYSWAP" => DisplaySwap,
        "DVR" => DVR,
        "EXIT" => Exit,
        "FAVORITECLEAR0" => FavoriteClear0,
        "FAVORITECLEAR1" => FavoriteClear1,
        "FAVORITECLEAR2" => FavoriteClear2,
        "FAVORITECLEAR3" => FavoriteClear3,
        "FAVORITERECALL0" => FavoriteRecall0,
        "FAVORITERECALL1" => FavoriteRecall1,
        "FAVORITERECALL2" => FavoriteRecall2,
        "FAVORITERECALL3" => FavoriteRecall3,
        "FAVORITESTORE0" => FavoriteStore0,
        "FAVORITESTORE1" => FavoriteStore1,
        "FAVORITESTORE2" => FavoriteStore2,
        "FAVORITESTORE3" => FavoriteStore3,
        "GUIDE" => Guide,
        "GUIDENEXTDAY" => GuideNextDay,
        "GUIDEPREVIOUSDAY" => GuidePreviousDay,
        "INFO" => Info,
        "INSTANTREPLAY" => InstantReplay,
        "LINK" => Link,
        "LISTPROGRAM" => ListProgram,
        "LIVECONTENT" => LiveContent,
        "LOCK" => Lock,
        "MEDIAAPPS" => MediaApps,
        "MEDIAAUDIOTRACK" => MediaAudioTrack,
        "MEDIALAST" => MediaLast,
        "MEDIASKIPBACKWARD" => MediaSkipBackward,
        "MEDIASKIPFORWARD" => MediaSkipForward,
        "MEDIASTEPBACKWARD" => MediaStepBackward,
        "MEDIASTEPFORWARD" => MediaStepForward,
        "MEDIATOPMENU" => MediaTopMenu,
        "NAVIGATEIN" => NavigateIn,
        "NAVIGATENEXT" => NavigateNext,
        "NAVIGATEOUT" => NavigateOut,
        "NAVIGATEPREVIOUS" => NavigatePrevious,
        "NEXTFAVORITECHANNEL" => NextFavoriteChannel,
        "NEXTUSERPROFILE" => NextUserProfile,
        "ONDEMAND" => OnDemand,
        "PAIRING" => Pairing,
        "PINPDOWN" => PinPDown,
        "PINPMOVE" => PinPMove,
        "PINPTOGGLE" => PinPToggle,
        "PINPUP" => PinPUp,
        "PLAYSPEEDDOWN" => PlaySpeedDown,
        "PLAYSPEEDRESET" => PlaySpeedReset,
        "PLAYSPEEDUP" => PlaySpeedUp,
        "RANDOMTOGGLE" => RandomToggle,
        "RCLOWBATTERY" => RcLowBattery,
        "RECORDSPEEDNEXT" => RecordSpeedNext,
        "RFBYPASS" => RfBypass,
        "SCANCHANNELSTOGGLE" => ScanChannelsToggle,
        "SCREENMODENEXT" => ScreenModeNext,
        "SETTINGS" => Settings,
        "SPLITSCREENTOGGLE" => SplitScreenToggle,
        "STBINPUT" => STBInput,
        "STBPOWER" => STBPower,
        "SUBTITLE" => Subtitle,
        "TELETEXT" => Teletext,
        "VIDEOMODENEXT" => VideoModeNext,
        "WINK" => Wink,
        "ZOOMTOGGLE" => ZoomToggle,
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
        "super+ctrl+SHIFT+alt+ArrowUp",
        Accelerator {
            mods: Modifiers::SUPER | Modifiers::CONTROL | Modifiers::SHIFT | Modifiers::ALT,
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
        ("Shift+f12", Modifiers::SHIFT, Key::F12),
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
