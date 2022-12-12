// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use std::fmt;

use keyboard_types::{Code, Modifiers};
use windows_sys::Win32::UI::{
    Input::KeyboardAndMouse::*,
    WindowsAndMessaging::{ACCEL, FALT, FCONTROL, FSHIFT, FVIRTKEY},
};

use crate::accelerator::Accelerator;

impl Accelerator {
    // Convert a hotkey to an accelerator.
    pub fn to_accel(&self, menu_id: u16) -> ACCEL {
        let mut virt_key = FVIRTKEY;
        let key_mods: Modifiers = self.mods;
        if key_mods.contains(Modifiers::CONTROL) {
            virt_key |= FCONTROL;
        }
        if key_mods.contains(Modifiers::ALT) {
            virt_key |= FALT;
        }
        if key_mods.contains(Modifiers::SHIFT) {
            virt_key |= FSHIFT;
        }

        let vk_code = key_to_vk(&self.key);
        let mod_code = vk_code >> 8;
        if mod_code & 0x1 != 0 {
            virt_key |= FSHIFT;
        }
        if mod_code & 0x02 != 0 {
            virt_key |= FCONTROL;
        }
        if mod_code & 0x04 != 0 {
            virt_key |= FALT;
        }
        let raw_key = vk_code & 0x00ff;

        ACCEL {
            fVirt: virt_key as u8,
            key: raw_key as u16,
            cmd: menu_id,
        }
    }
}

// used to build accelerators table from Key
fn key_to_vk(key: &Code) -> VIRTUAL_KEY {
    match key {
        Code::KeyA => unsafe { VkKeyScanW('a' as u16) as u16 },
        Code::KeyB => unsafe { VkKeyScanW('b' as u16) as u16 },
        Code::KeyC => unsafe { VkKeyScanW('c' as u16) as u16 },
        Code::KeyD => unsafe { VkKeyScanW('d' as u16) as u16 },
        Code::KeyE => unsafe { VkKeyScanW('e' as u16) as u16 },
        Code::KeyF => unsafe { VkKeyScanW('f' as u16) as u16 },
        Code::KeyG => unsafe { VkKeyScanW('g' as u16) as u16 },
        Code::KeyH => unsafe { VkKeyScanW('h' as u16) as u16 },
        Code::KeyI => unsafe { VkKeyScanW('i' as u16) as u16 },
        Code::KeyJ => unsafe { VkKeyScanW('j' as u16) as u16 },
        Code::KeyK => unsafe { VkKeyScanW('k' as u16) as u16 },
        Code::KeyL => unsafe { VkKeyScanW('l' as u16) as u16 },
        Code::KeyM => unsafe { VkKeyScanW('m' as u16) as u16 },
        Code::KeyN => unsafe { VkKeyScanW('n' as u16) as u16 },
        Code::KeyO => unsafe { VkKeyScanW('o' as u16) as u16 },
        Code::KeyP => unsafe { VkKeyScanW('p' as u16) as u16 },
        Code::KeyQ => unsafe { VkKeyScanW('q' as u16) as u16 },
        Code::KeyR => unsafe { VkKeyScanW('r' as u16) as u16 },
        Code::KeyS => unsafe { VkKeyScanW('s' as u16) as u16 },
        Code::KeyT => unsafe { VkKeyScanW('t' as u16) as u16 },
        Code::KeyU => unsafe { VkKeyScanW('u' as u16) as u16 },
        Code::KeyV => unsafe { VkKeyScanW('v' as u16) as u16 },
        Code::KeyW => unsafe { VkKeyScanW('w' as u16) as u16 },
        Code::KeyX => unsafe { VkKeyScanW('x' as u16) as u16 },
        Code::KeyY => unsafe { VkKeyScanW('y' as u16) as u16 },
        Code::KeyZ => unsafe { VkKeyScanW('z' as u16) as u16 },
        Code::Digit0 => unsafe { VkKeyScanW('0' as u16) as u16 },
        Code::Digit1 => unsafe { VkKeyScanW('1' as u16) as u16 },
        Code::Digit2 => unsafe { VkKeyScanW('2' as u16) as u16 },
        Code::Digit3 => unsafe { VkKeyScanW('3' as u16) as u16 },
        Code::Digit4 => unsafe { VkKeyScanW('4' as u16) as u16 },
        Code::Digit5 => unsafe { VkKeyScanW('5' as u16) as u16 },
        Code::Digit6 => unsafe { VkKeyScanW('6' as u16) as u16 },
        Code::Digit7 => unsafe { VkKeyScanW('7' as u16) as u16 },
        Code::Digit8 => unsafe { VkKeyScanW('8' as u16) as u16 },
        Code::Digit9 => unsafe { VkKeyScanW('9' as u16) as u16 },
        Code::Comma => VK_OEM_COMMA,
        Code::Minus => VK_OEM_MINUS,
        Code::Period => VK_OEM_PERIOD,
        Code::Equal => unsafe { VkKeyScanW('=' as u16) as u16 },
        Code::Semicolon => unsafe { VkKeyScanW(';' as u16) as u16 },
        Code::Slash => unsafe { VkKeyScanW('/' as u16) as u16 },
        Code::Backslash => unsafe { VkKeyScanW('\\' as u16) as u16 },
        Code::Quote => unsafe { VkKeyScanW('\'' as u16) as u16 },
        Code::Backquote => unsafe { VkKeyScanW('`' as u16) as u16 },
        Code::BracketLeft => unsafe { VkKeyScanW('[' as u16) as u16 },
        Code::BracketRight => unsafe { VkKeyScanW(']' as u16) as u16 },
        Code::Backspace => VK_BACK,
        Code::Tab => VK_TAB,
        Code::Space => VK_SPACE,
        Code::Enter => VK_RETURN,
        Code::Pause => VK_PAUSE,
        Code::CapsLock => VK_CAPITAL,
        Code::KanaMode => VK_KANA,
        Code::Escape => VK_ESCAPE,
        Code::NonConvert => VK_NONCONVERT,
        Code::PageUp => VK_PRIOR,
        Code::PageDown => VK_NEXT,
        Code::End => VK_END,
        Code::Home => VK_HOME,
        Code::ArrowLeft => VK_LEFT,
        Code::ArrowUp => VK_UP,
        Code::ArrowRight => VK_RIGHT,
        Code::ArrowDown => VK_DOWN,
        Code::PrintScreen => VK_SNAPSHOT,
        Code::Insert => VK_INSERT,
        Code::Delete => VK_DELETE,
        Code::Help => VK_HELP,
        Code::ContextMenu => VK_APPS,
        Code::F1 => VK_F1,
        Code::F2 => VK_F2,
        Code::F3 => VK_F3,
        Code::F4 => VK_F4,
        Code::F5 => VK_F5,
        Code::F6 => VK_F6,
        Code::F7 => VK_F7,
        Code::F8 => VK_F8,
        Code::F9 => VK_F9,
        Code::F10 => VK_F10,
        Code::F11 => VK_F11,
        Code::F12 => VK_F12,
        Code::F13 => VK_F13,
        Code::F14 => VK_F14,
        Code::F15 => VK_F15,
        Code::F16 => VK_F16,
        Code::F17 => VK_F17,
        Code::F18 => VK_F18,
        Code::F19 => VK_F19,
        Code::F20 => VK_F20,
        Code::F21 => VK_F21,
        Code::F22 => VK_F22,
        Code::F23 => VK_F23,
        Code::F24 => VK_F24,
        Code::NumLock => VK_NUMLOCK,
        Code::ScrollLock => VK_SCROLL,
        Code::BrowserBack => VK_BROWSER_BACK,
        Code::BrowserForward => VK_BROWSER_FORWARD,
        Code::BrowserRefresh => VK_BROWSER_REFRESH,
        Code::BrowserStop => VK_BROWSER_STOP,
        Code::BrowserSearch => VK_BROWSER_SEARCH,
        Code::BrowserFavorites => VK_BROWSER_FAVORITES,
        Code::BrowserHome => VK_BROWSER_HOME,
        Code::AudioVolumeMute => VK_VOLUME_MUTE,
        Code::AudioVolumeDown => VK_VOLUME_DOWN,
        Code::AudioVolumeUp => VK_VOLUME_UP,
        Code::MediaTrackNext => VK_MEDIA_NEXT_TRACK,
        Code::MediaTrackPrevious => VK_MEDIA_PREV_TRACK,
        Code::MediaStop => VK_MEDIA_STOP,
        Code::MediaPlayPause => VK_MEDIA_PLAY_PAUSE,
        Code::LaunchMail => VK_LAUNCH_MAIL,
        Code::Convert => VK_CONVERT,
        key => panic!("Unsupported key: {}", key),
    }
}

impl fmt::Display for Accelerator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let key_mods: Modifiers = self.mods;
        if key_mods.contains(Modifiers::CONTROL) {
            write!(f, "Ctrl+")?;
        }
        if key_mods.contains(Modifiers::SHIFT) {
            write!(f, "Shift+")?;
        }
        if key_mods.contains(Modifiers::ALT) {
            write!(f, "Alt+")?;
        }
        if key_mods.contains(Modifiers::SUPER) {
            write!(f, "Windows+")?;
        }
        match &self.key {
            Code::KeyA => write!(f, "A"),
            Code::KeyB => write!(f, "B"),
            Code::KeyC => write!(f, "C"),
            Code::KeyD => write!(f, "D"),
            Code::KeyE => write!(f, "E"),
            Code::KeyF => write!(f, "F"),
            Code::KeyG => write!(f, "G"),
            Code::KeyH => write!(f, "H"),
            Code::KeyI => write!(f, "I"),
            Code::KeyJ => write!(f, "J"),
            Code::KeyK => write!(f, "K"),
            Code::KeyL => write!(f, "L"),
            Code::KeyM => write!(f, "M"),
            Code::KeyN => write!(f, "N"),
            Code::KeyO => write!(f, "O"),
            Code::KeyP => write!(f, "P"),
            Code::KeyQ => write!(f, "Q"),
            Code::KeyR => write!(f, "R"),
            Code::KeyS => write!(f, "S"),
            Code::KeyT => write!(f, "T"),
            Code::KeyU => write!(f, "U"),
            Code::KeyV => write!(f, "V"),
            Code::KeyW => write!(f, "W"),
            Code::KeyX => write!(f, "X"),
            Code::KeyY => write!(f, "Y"),
            Code::KeyZ => write!(f, "Z"),
            Code::Digit0 => write!(f, "0"),
            Code::Digit1 => write!(f, "1"),
            Code::Digit2 => write!(f, "2"),
            Code::Digit3 => write!(f, "3"),
            Code::Digit4 => write!(f, "4"),
            Code::Digit5 => write!(f, "5"),
            Code::Digit6 => write!(f, "6"),
            Code::Digit7 => write!(f, "7"),
            Code::Digit8 => write!(f, "8"),
            Code::Digit9 => write!(f, "9"),
            Code::Comma => write!(f, ","),
            Code::Minus => write!(f, "-"),
            Code::Period => write!(f, "."),
            Code::Space => write!(f, "Space"),
            Code::Equal => write!(f, "="),
            Code::Semicolon => write!(f, ";"),
            Code::Slash => write!(f, "/"),
            Code::Backslash => write!(f, "\\"),
            Code::Quote => write!(f, "\'"),
            Code::Backquote => write!(f, "`"),
            Code::BracketLeft => write!(f, "["),
            Code::BracketRight => write!(f, "]"),
            Code::Tab => write!(f, "Tab"),
            Code::Escape => write!(f, "Esc"),
            Code::Delete => write!(f, "Del"),
            Code::Insert => write!(f, "Ins"),
            Code::PageUp => write!(f, "PgUp"),
            Code::PageDown => write!(f, "PgDn"),
            // These names match LibreOffice.
            Code::ArrowLeft => write!(f, "Left"),
            Code::ArrowRight => write!(f, "Right"),
            Code::ArrowUp => write!(f, "Up"),
            Code::ArrowDown => write!(f, "Down"),
            _ => write!(f, "{:?}", self.key),
        }
    }
}
