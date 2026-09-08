// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use std::fmt;

use keyboard_types::{Code, Key, Modifiers, NamedKey};
use windows_sys::Win32::UI::{
    Input::KeyboardAndMouse::*,
    WindowsAndMessaging::{ACCEL, FALT, FCONTROL, FSHIFT, FVIRTKEY},
};

use crate::accelerator::{Accelerator, AcceleratorParseError, KeyAccelerator, MenuAccelerator};

// VkKeyScanW packs the virtual key in the low byte and a shift-state bitset
// in the high byte: 1 = Shift, 2 = Ctrl, 4 = Alt.
const VK_KEY_SCAN_KEY_MASK: VIRTUAL_KEY = 0x00ff;
const VK_KEY_SCAN_SHIFT: VIRTUAL_KEY = 0x01;
const VK_KEY_SCAN_CONTROL: VIRTUAL_KEY = 0x02;
const VK_KEY_SCAN_ALT: VIRTUAL_KEY = 0x04;

impl MenuAccelerator {
    pub fn to_accel(&self, menu_id: u16) -> crate::Result<ACCEL> {
        match self {
            MenuAccelerator::Physical(accelerator) => accelerator.to_accel(menu_id),
            MenuAccelerator::Logical(accelerator) => accelerator.to_accel(menu_id),
        }
    }
}

impl Accelerator {
    pub fn to_accel(self, menu_id: u16) -> crate::Result<ACCEL> {
        let vk = code_to_vk(&self.key)?;
        accelerator_to_accel(self.mods, vk, menu_id)
    }
}

impl KeyAccelerator {
    pub fn to_accel(&self, menu_id: u16) -> crate::Result<ACCEL> {
        let vk = key_to_vk(&self.key)?;
        accelerator_to_accel(self.mods, vk, menu_id)
    }
}

fn accelerator_to_accel(
    modifiers: Modifiers,
    vk_code: VIRTUAL_KEY,
    menu_id: u16,
) -> crate::Result<ACCEL> {
    let supported_modifiers = Modifiers::SHIFT | Modifiers::CONTROL | Modifiers::ALT;
    let unsupported_modifiers = modifiers & !supported_modifiers;
    if !unsupported_modifiers.is_empty() {
        return Err(AcceleratorParseError::UnsupportedKey(format!(
            "Windows accelerator modifier: {unsupported_modifiers:?}"
        ))
        .into());
    }

    let mut virt_key = FVIRTKEY;
    if modifiers.contains(Modifiers::SHIFT) {
        virt_key |= FSHIFT;
    }
    if modifiers.contains(Modifiers::CONTROL) {
        virt_key |= FCONTROL;
    }
    if modifiers.contains(Modifiers::ALT) {
        virt_key |= FALT;
    }

    // VkKeyScanW packs the virtual key in the low byte and a shift-state bitset
    // in the high byte: 1 = Shift, 2 = Ctrl, 4 = Alt.
    // We need to check the shift state and apply appropriate flags, so that
    // accelerators like Ctrl++ (which requires Shift) are correctly recognized.
    let shift_state = vk_code >> 8;
    if shift_state & VK_KEY_SCAN_SHIFT != 0 {
        virt_key |= FSHIFT;
    }
    if shift_state & VK_KEY_SCAN_CONTROL != 0 {
        virt_key |= FCONTROL;
    }
    if shift_state & VK_KEY_SCAN_ALT != 0 {
        virt_key |= FALT;
    }

    Ok(ACCEL {
        fVirt: virt_key,
        key: vk_code & VK_KEY_SCAN_KEY_MASK,
        cmd: menu_id,
    })
}

fn code_to_vk(code: &Code) -> Result<VIRTUAL_KEY, AcceleratorParseError> {
    use Code::*;

    let vk_code = match code {
        Backquote => VK_OEM_3,
        Backslash => VK_OEM_5,
        BracketLeft => VK_OEM_4,
        BracketRight => VK_OEM_6,
        Comma => VK_OEM_COMMA,
        Digit0 => VK_0,
        Digit1 => VK_1,
        Digit2 => VK_2,
        Digit3 => VK_3,
        Digit4 => VK_4,
        Digit5 => VK_5,
        Digit6 => VK_6,
        Digit7 => VK_7,
        Digit8 => VK_8,
        Digit9 => VK_9,
        Equal => VK_OEM_PLUS,
        IntlBackslash => VK_OEM_102,
        IntlYen => VK_OEM_5,
        KeyA => VK_A,
        KeyB => VK_B,
        KeyC => VK_C,
        KeyD => VK_D,
        KeyE => VK_E,
        KeyF => VK_F,
        KeyG => VK_G,
        KeyH => VK_H,
        KeyI => VK_I,
        KeyJ => VK_J,
        KeyK => VK_K,
        KeyL => VK_L,
        KeyM => VK_M,
        KeyN => VK_N,
        KeyO => VK_O,
        KeyP => VK_P,
        KeyQ => VK_Q,
        KeyR => VK_R,
        KeyS => VK_S,
        KeyT => VK_T,
        KeyU => VK_U,
        KeyV => VK_V,
        KeyW => VK_W,
        KeyX => VK_X,
        KeyY => VK_Y,
        KeyZ => VK_Z,
        Minus => VK_OEM_MINUS,
        Period => VK_OEM_PERIOD,
        Quote => VK_OEM_7,
        Semicolon => VK_OEM_1,
        Slash => VK_OEM_2,
        AltLeft => VK_LMENU,
        AltRight => VK_RMENU,
        Backspace => VK_BACK,
        CapsLock => VK_CAPITAL,
        ContextMenu => VK_APPS,
        ControlLeft => VK_LCONTROL,
        ControlRight => VK_RCONTROL,
        Enter => VK_RETURN,
        MetaLeft => VK_LWIN,
        MetaRight => VK_RWIN,
        ShiftLeft => VK_LSHIFT,
        ShiftRight => VK_RSHIFT,
        Space => VK_SPACE,
        Tab => VK_TAB,
        Convert => VK_CONVERT,
        KanaMode | Lang1 => VK_KANA,
        Lang2 => VK_KANJI,
        NonConvert => VK_NONCONVERT,
        Delete => VK_DELETE,
        End => VK_END,
        Help => VK_HELP,
        Home => VK_HOME,
        Insert => VK_INSERT,
        PageDown => VK_NEXT,
        PageUp => VK_PRIOR,
        ArrowDown => VK_DOWN,
        ArrowLeft => VK_LEFT,
        ArrowRight => VK_RIGHT,
        ArrowUp => VK_UP,
        NumLock => VK_NUMLOCK,
        Numpad0 => VK_NUMPAD0,
        Numpad1 => VK_NUMPAD1,
        Numpad2 => VK_NUMPAD2,
        Numpad3 => VK_NUMPAD3,
        Numpad4 => VK_NUMPAD4,
        Numpad5 => VK_NUMPAD5,
        Numpad6 => VK_NUMPAD6,
        Numpad7 => VK_NUMPAD7,
        Numpad8 => VK_NUMPAD8,
        Numpad9 => VK_NUMPAD9,
        NumpadAdd => VK_ADD,
        NumpadBackspace => VK_BACK,
        NumpadClear => VK_CLEAR,
        NumpadComma => VK_SEPARATOR,
        NumpadDecimal => VK_DECIMAL,
        NumpadDivide => VK_DIVIDE,
        NumpadEnter => VK_RETURN,
        NumpadEqual => VK_OEM_NEC_EQUAL,
        NumpadMultiply => VK_MULTIPLY,
        NumpadSubtract => VK_SUBTRACT,
        Escape => VK_ESCAPE,
        PrintScreen => VK_SNAPSHOT,
        ScrollLock => VK_SCROLL,
        Pause => VK_PAUSE,
        BrowserBack => VK_BROWSER_BACK,
        BrowserFavorites => VK_BROWSER_FAVORITES,
        BrowserForward => VK_BROWSER_FORWARD,
        BrowserHome => VK_BROWSER_HOME,
        BrowserRefresh => VK_BROWSER_REFRESH,
        BrowserSearch => VK_BROWSER_SEARCH,
        BrowserStop => VK_BROWSER_STOP,
        LaunchApp1 => VK_LAUNCH_APP1,
        LaunchApp2 => VK_LAUNCH_APP2,
        LaunchMail => VK_LAUNCH_MAIL,
        MediaPlayPause => VK_MEDIA_PLAY_PAUSE,
        MediaSelect => VK_LAUNCH_MEDIA_SELECT,
        MediaStop => VK_MEDIA_STOP,
        MediaTrackNext => VK_MEDIA_NEXT_TRACK,
        MediaTrackPrevious => VK_MEDIA_PREV_TRACK,
        Sleep => VK_SLEEP,
        AudioVolumeDown => VK_VOLUME_DOWN,
        AudioVolumeMute => VK_VOLUME_MUTE,
        AudioVolumeUp => VK_VOLUME_UP,
        Hiragana => VK_DBE_HIRAGANA,
        Katakana => VK_DBE_KATAKANA,
        F1 => VK_F1,
        F2 => VK_F2,
        F3 => VK_F3,
        F4 => VK_F4,
        F5 => VK_F5,
        F6 => VK_F6,
        F7 => VK_F7,
        F8 => VK_F8,
        F9 => VK_F9,
        F10 => VK_F10,
        F11 => VK_F11,
        F12 => VK_F12,
        F13 => VK_F13,
        F14 => VK_F14,
        F15 => VK_F15,
        F16 => VK_F16,
        F17 => VK_F17,
        F18 => VK_F18,
        F19 => VK_F19,
        F20 => VK_F20,
        F21 => VK_F21,
        F22 => VK_F22,
        F23 => VK_F23,
        F24 => VK_F24,
        _ => return Err(AcceleratorParseError::UnsupportedKey(format!("{code:?}"))),
    };

    Ok(vk_code)
}

fn key_to_vk(key: &Key) -> Result<VIRTUAL_KEY, AcceleratorParseError> {
    match key {
        Key::Character(character) => character_to_vk(character),
        Key::Named(key) => named_key_to_vk(key),
    }
}

fn named_key_to_vk(key: &NamedKey) -> Result<VIRTUAL_KEY, AcceleratorParseError> {
    use NamedKey::*;

    let vk_code = match key {
        Alt => VK_MENU,
        AltGraph => VK_RMENU,
        CapsLock => VK_CAPITAL,
        Control => VK_CONTROL,
        #[allow(deprecated)]
        Meta | Super => VK_LWIN,
        NumLock => VK_NUMLOCK,
        ScrollLock => VK_SCROLL,
        Shift => VK_SHIFT,
        Enter => VK_RETURN,
        Tab => VK_TAB,
        ArrowDown => VK_DOWN,
        ArrowLeft => VK_LEFT,
        ArrowRight => VK_RIGHT,
        ArrowUp => VK_UP,
        End => VK_END,
        Home => VK_HOME,
        PageDown => VK_NEXT,
        PageUp => VK_PRIOR,
        Backspace => VK_BACK,
        Clear => VK_CLEAR,
        CrSel => VK_CRSEL,
        Delete => VK_DELETE,
        EraseEof => VK_EREOF,
        ExSel => VK_EXSEL,
        Insert => VK_INSERT,
        Accept => VK_ACCEPT,
        Attn => VK_ATTN,
        Cancel => VK_CANCEL,
        ContextMenu => VK_APPS,
        Escape => VK_ESCAPE,
        Execute => VK_EXECUTE,
        Help => VK_HELP,
        Pause => VK_PAUSE,
        Play => VK_PLAY,
        Select => VK_SELECT,
        ZoomIn | ZoomOut | ZoomToggle => VK_ZOOM,
        PrintScreen => VK_SNAPSHOT,
        Standby => VK_SLEEP,
        Alphanumeric => VK_DBE_ALPHANUMERIC,
        CodeInput => VK_DBE_CODEINPUT,
        Convert => VK_CONVERT,
        FinalMode => VK_FINAL,
        ModeChange => VK_MODECHANGE,
        NonConvert => VK_NONCONVERT,
        Process => VK_PROCESSKEY,
        HangulMode | KanaMode => VK_KANA,
        HanjaMode | KanjiMode => VK_KANJI,
        JunjaMode => VK_JUNJA,
        Hankaku => VK_DBE_SBCSCHAR,
        Hiragana => VK_DBE_HIRAGANA,
        Katakana => VK_DBE_KATAKANA,
        Romaji => VK_DBE_ROMAN,
        Zenkaku => VK_DBE_DBCSCHAR,
        MediaPlayPause => VK_MEDIA_PLAY_PAUSE,
        MediaStop => VK_MEDIA_STOP,
        MediaTrackNext => VK_MEDIA_NEXT_TRACK,
        MediaTrackPrevious => VK_MEDIA_PREV_TRACK,
        AudioVolumeDown => VK_VOLUME_DOWN,
        AudioVolumeMute => VK_VOLUME_MUTE,
        AudioVolumeUp => VK_VOLUME_UP,
        LaunchApplication1 => VK_LAUNCH_APP1,
        LaunchApplication2 => VK_LAUNCH_APP2,
        LaunchMail => VK_LAUNCH_MAIL,
        LaunchMediaPlayer => VK_LAUNCH_MEDIA_SELECT,
        BrowserBack => VK_BROWSER_BACK,
        BrowserFavorites => VK_BROWSER_FAVORITES,
        BrowserForward => VK_BROWSER_FORWARD,
        BrowserHome => VK_BROWSER_HOME,
        BrowserRefresh => VK_BROWSER_REFRESH,
        BrowserSearch => VK_BROWSER_SEARCH,
        BrowserStop => VK_BROWSER_STOP,
        MediaApps => VK_APPS,
        F1 => VK_F1,
        F2 => VK_F2,
        F3 => VK_F3,
        F4 => VK_F4,
        F5 => VK_F5,
        F6 => VK_F6,
        F7 => VK_F7,
        F8 => VK_F8,
        F9 => VK_F9,
        F10 => VK_F10,
        F11 => VK_F11,
        F12 => VK_F12,
        F13 => VK_F13,
        F14 => VK_F14,
        F15 => VK_F15,
        F16 => VK_F16,
        F17 => VK_F17,
        F18 => VK_F18,
        F19 => VK_F19,
        F20 => VK_F20,
        F21 => VK_F21,
        F22 => VK_F22,
        F23 => VK_F23,
        F24 => VK_F24,
        _ => return Err(AcceleratorParseError::UnsupportedKey(format!("{key:?}"))),
    };

    Ok(vk_code)
}

fn character_to_vk(character: &str) -> Result<VIRTUAL_KEY, AcceleratorParseError> {
    let mut chars = character.chars();
    let Some(character) = chars.next() else {
        return Err(AcceleratorParseError::UnsupportedKey(character.to_string()));
    };
    if chars.next().is_some() || character as u32 > u16::MAX as u32 {
        return Err(AcceleratorParseError::UnsupportedKey(character.to_string()));
    }

    let result = unsafe { VkKeyScanW(character as u16) };
    if result == -1 {
        Err(AcceleratorParseError::UnsupportedKey(character.to_string()))
    } else {
        Ok(result as VIRTUAL_KEY)
    }
}

impl fmt::Display for MenuAccelerator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MenuAccelerator::Physical(accelerator) => fmt::Display::fmt(accelerator, f),
            MenuAccelerator::Logical(accelerator) => fmt::Display::fmt(accelerator, f),
        }
    }
}

impl fmt::Display for Accelerator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_modifiers(f, self.mods)?;
        f.write_str(
            code_display(&self.key)
                .unwrap_or_else(|| self.key.to_string())
                .as_str(),
        )
    }
}

impl fmt::Display for KeyAccelerator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_modifiers(f, self.mods)?;
        f.write_str(
            key_display(&self.key)
                .unwrap_or_else(|| self.key.to_string())
                .as_str(),
        )
    }
}

fn write_modifiers(f: &mut fmt::Formatter<'_>, modifiers: Modifiers) -> fmt::Result {
    if modifiers.contains(Modifiers::CONTROL) {
        f.write_str("Ctrl+")?;
    }
    if modifiers.contains(Modifiers::SHIFT) {
        f.write_str("Shift+")?;
    }
    if modifiers.contains(Modifiers::ALT) {
        f.write_str("Alt+")?;
    }
    if modifiers.contains(Modifiers::META) {
        f.write_str("Meta+")?;
    }
    #[allow(deprecated)]
    if modifiers.contains(Modifiers::SUPER) {
        f.write_str("Super+")?;
    }
    #[allow(deprecated)]
    if modifiers.contains(Modifiers::HYPER) {
        f.write_str("Hyper+")?;
    }
    Ok(())
}

fn code_display(code: &Code) -> Option<String> {
    use Code::*;

    let label = match code {
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
        IntlYen => "\\",
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
        Minus | NumpadSubtract => "-",
        Period | NumpadDecimal => ".",
        Quote => "'",
        Semicolon => ";",
        Slash | NumpadDivide => "/",
        AltLeft | AltRight => "Alt",
        Backspace | NumpadBackspace => "Backspace",
        CapsLock => "CapsLock",
        ContextMenu => "Menu",
        ControlLeft | ControlRight => "Ctrl",
        Enter | NumpadEnter => "Enter",
        MetaLeft | MetaRight => "Win",
        ShiftLeft | ShiftRight => "Shift",
        Space => "Space",
        Tab => "Tab",
        Convert => "Convert",
        KanaMode | Lang1 => "Kana",
        Lang2 => "Kanji",
        NonConvert => "NonConvert",
        Delete => "Del",
        End => "End",
        Help => "Help",
        Home => "Home",
        Insert => "Ins",
        PageDown => "PgDn",
        PageUp => "PgUp",
        ArrowDown => "Down",
        ArrowLeft => "Left",
        ArrowRight => "Right",
        ArrowUp => "Up",
        NumLock => "NumLock",
        NumpadAdd => "+",
        NumpadClear => "Clear",
        NumpadComma => ",",
        NumpadMultiply => "*",
        Escape => "Esc",
        PrintScreen => "PrintScreen",
        ScrollLock => "ScrollLock",
        Pause => "Pause",
        BrowserBack => "BrowserBack",
        BrowserFavorites => "BrowserFavorites",
        BrowserForward => "BrowserForward",
        BrowserHome => "BrowserHome",
        BrowserRefresh => "BrowserRefresh",
        BrowserSearch => "BrowserSearch",
        BrowserStop => "BrowserStop",
        LaunchApp1 => "LaunchApp1",
        LaunchApp2 => "LaunchApp2",
        LaunchMail => "LaunchMail",
        MediaPlayPause => "MediaPlayPause",
        MediaSelect => "MediaSelect",
        MediaStop => "MediaStop",
        MediaTrackNext => "MediaTrackNext",
        MediaTrackPrevious => "MediaTrackPrevious",
        Sleep => "Sleep",
        AudioVolumeDown => "VolumeDown",
        AudioVolumeMute => "VolumeMute",
        AudioVolumeUp => "VolumeUp",
        Hiragana => "Hiragana",
        Katakana => "Katakana",
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
    };

    Some(label.to_string())
}

fn key_display(key: &Key) -> Option<String> {
    match key {
        Key::Character(character) => character_display(character),
        Key::Named(key) => named_key_display(key),
    }
}

fn named_key_display(key: &NamedKey) -> Option<String> {
    use NamedKey::*;

    match key {
        Alt => Some("Alt".into()),
        AltGraph => Some("AltGr".into()),
        CapsLock => Some("CapsLock".into()),
        Control => Some("Ctrl".into()),
        #[allow(deprecated)]
        Meta | Super => Some("Win".into()),
        NumLock => Some("NumLock".into()),
        ScrollLock => Some("ScrollLock".into()),
        Shift => Some("Shift".into()),
        Enter => Some("Enter".into()),
        Tab => Some("Tab".into()),
        ArrowDown => Some("Down".into()),
        ArrowLeft => Some("Left".into()),
        ArrowRight => Some("Right".into()),
        ArrowUp => Some("Up".into()),
        End => Some("End".into()),
        Home => Some("Home".into()),
        PageDown => Some("PgDn".into()),
        PageUp => Some("PgUp".into()),
        Backspace => Some("Backspace".into()),
        Clear => Some("Clear".into()),
        CrSel => Some("CrSel".into()),
        Delete => Some("Del".into()),
        EraseEof => Some("EraseEof".into()),
        ExSel => Some("ExSel".into()),
        Insert => Some("Ins".into()),
        Accept => Some("Accept".into()),
        Attn => Some("Attn".into()),
        Cancel => Some("Cancel".into()),
        ContextMenu => Some("Menu".into()),
        Escape => Some("Esc".into()),
        Execute => Some("Execute".into()),
        Help => Some("Help".into()),
        Pause => Some("Pause".into()),
        Play => Some("Play".into()),
        Select => Some("Select".into()),
        ZoomIn | ZoomOut | ZoomToggle => Some("Zoom".into()),
        PrintScreen => Some("PrintScreen".into()),
        Standby => Some("Sleep".into()),
        Alphanumeric => Some("Alphanumeric".into()),
        CodeInput => Some("CodeInput".into()),
        Convert => Some("Convert".into()),
        FinalMode => Some("FinalMode".into()),
        ModeChange => Some("ModeChange".into()),
        NonConvert => Some("NonConvert".into()),
        Process => Some("Process".into()),
        HangulMode | KanaMode => Some("Kana".into()),
        HanjaMode | KanjiMode => Some("Kanji".into()),
        JunjaMode => Some("JunjaMode".into()),
        Hankaku => Some("Hankaku".into()),
        Hiragana => Some("Hiragana".into()),
        Katakana => Some("Katakana".into()),
        Romaji => Some("Romaji".into()),
        Zenkaku => Some("Zenkaku".into()),
        MediaPlayPause => Some("MediaPlayPause".into()),
        MediaStop => Some("MediaStop".into()),
        MediaTrackNext => Some("MediaTrackNext".into()),
        MediaTrackPrevious => Some("MediaTrackPrevious".into()),
        AudioVolumeDown => Some("VolumeDown".into()),
        AudioVolumeMute => Some("VolumeMute".into()),
        AudioVolumeUp => Some("VolumeUp".into()),
        LaunchApplication1 => Some("LaunchApplication1".into()),
        LaunchApplication2 => Some("LaunchApplication2".into()),
        LaunchMail => Some("LaunchMail".into()),
        LaunchMediaPlayer => Some("LaunchMediaPlayer".into()),
        BrowserBack => Some("BrowserBack".into()),
        BrowserFavorites => Some("BrowserFavorites".into()),
        BrowserForward => Some("BrowserForward".into()),
        BrowserHome => Some("BrowserHome".into()),
        BrowserRefresh => Some("BrowserRefresh".into()),
        BrowserSearch => Some("BrowserSearch".into()),
        BrowserStop => Some("BrowserStop".into()),
        MediaApps => Some("MediaApps".into()),
        F1 => Some("F1".into()),
        F2 => Some("F2".into()),
        F3 => Some("F3".into()),
        F4 => Some("F4".into()),
        F5 => Some("F5".into()),
        F6 => Some("F6".into()),
        F7 => Some("F7".into()),
        F8 => Some("F8".into()),
        F9 => Some("F9".into()),
        F10 => Some("F10".into()),
        F11 => Some("F11".into()),
        F12 => Some("F12".into()),
        F13 => Some("F13".into()),
        F14 => Some("F14".into()),
        F15 => Some("F15".into()),
        F16 => Some("F16".into()),
        F17 => Some("F17".into()),
        F18 => Some("F18".into()),
        F19 => Some("F19".into()),
        F20 => Some("F20".into()),
        F21 => Some("F21".into()),
        F22 => Some("F22".into()),
        F23 => Some("F23".into()),
        F24 => Some("F24".into()),
        _ => None,
    }
}

fn character_display(character: &str) -> Option<String> {
    match character {
        " " => Some("Space".into()),
        _ if character.chars().count() == 1 => Some(character.to_uppercase()),
        _ => None,
    }
}
