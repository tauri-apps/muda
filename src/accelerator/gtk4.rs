// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use gtk4::gdk;
use keyboard_types::{Code, Key, Modifiers};

use crate::accelerator::{Accelerator, KeyAccelerator, MenuAccelerator};

impl MenuAccelerator {
    /// Builds a GTK accelerator string (e.g. `<Shift><Control>a`) that can be
    /// passed to `gtk::Application::set_accels_for_action`.
    pub(crate) fn to_gtk(&self) -> Option<String> {
        match self {
            MenuAccelerator::Physical(accelerator) => accelerator.to_gtk(),
            MenuAccelerator::Logical(accelerator) => accelerator.to_gtk(),
        }
    }
}

impl Accelerator {
    /// Builds a GTK accelerator string (e.g. `<Shift><Control>a`) that can be
    /// passed to `gtk::Application::set_accels_for_action`.
    pub(crate) fn to_gtk(&self) -> Option<String> {
        let mut gtk = modifiers_to_gtk(self.mods);
        gtk.push_str(&code_to_gtk(&self.key)?);
        Some(gtk)
    }
}

impl KeyAccelerator {
    /// Builds a GTK accelerator string (e.g. `<Shift><Control>a`) that can be
    /// passed to `gtk::Application::set_accels_for_action`.
    pub fn to_gtk(&self) -> Option<String> {
        let mut gtk = modifiers_to_gtk(self.mods);
        gtk.push_str(&key_to_gtk(&self.key)?);
        Some(gtk)
    }
}

fn modifiers_to_gtk(mods: Modifiers) -> String {
    let mut gtk = String::new();

    if mods.shift() {
        gtk.push_str("<Shift>");
    }
    if mods.ctrl() {
        gtk.push_str("<Control>");
    }
    if mods.alt() {
        gtk.push_str("<Alt>");
    }
    if mods.meta() {
        gtk.push_str("<Meta>");
    }
    if mods.contains(Modifiers::SUPER) {
        gtk.push_str("<Super>");
    }
    if mods.contains(Modifiers::HYPER) {
        gtk.push_str("<Hyper>");
    }

    gtk
}

fn code_to_gtk(code: &Code) -> Option<String> {
    gdk_key_name_to_gtk(code_to_gdk_key_name(code)?)
}

fn key_to_gtk(key: &Key) -> Option<String> {
    match key {
        Key::Character(character) => character_to_gtk(character),
        key => gdk_key_name_to_gtk(key_to_gdk_key_name(key)?),
    }
}

fn gdk_key_name_to_gtk(name: &'static str) -> Option<String> {
    gdk::Key::from_name(name).map(|_| name.to_string())
}

fn character_to_gtk(character: &str) -> Option<String> {
    let mut chars = character.chars();
    let character = chars.next()?;
    if chars.next().is_some() {
        return None;
    }

    let name = match character {
        ' ' => "space",
        '!' => "exclam",
        '"' => "quotedbl",
        '#' => "numbersign",
        '$' => "dollar",
        '%' => "percent",
        '&' => "ampersand",
        '\'' => "apostrophe",
        '(' => "parenleft",
        ')' => "parenright",
        '*' => "asterisk",
        '+' => "plus",
        ',' => "comma",
        '-' => "minus",
        '.' => "period",
        '/' => "slash",
        ':' => "colon",
        ';' => "semicolon",
        '<' => "less",
        '=' => "equal",
        '>' => "greater",
        '?' => "question",
        '@' => "at",
        '[' => "bracketleft",
        '\\' => "backslash",
        ']' => "bracketright",
        '^' => "asciicircum",
        '_' => "underscore",
        '`' => "grave",
        '{' => "braceleft",
        '|' => "bar",
        '}' => "braceright",
        '~' => "asciitilde",
        character if character.is_ascii() => return Some(character.to_string()),
        character => return unicode_character_to_gtk(character),
    };

    Some(name.to_string())
}

fn unicode_character_to_gtk(character: char) -> Option<String> {
    let name = format!("U{:04X}", character as u32);
    gdk::Key::from_name(name.as_str()).map(|_| name)
}

fn code_to_gdk_key_name(code: &Code) -> Option<&'static str> {
    use Code::*;

    let name = match code {
        Backquote => "grave",
        Backslash => "backslash",
        BracketLeft => "bracketleft",
        BracketRight => "bracketright",
        Comma => "comma",
        Digit0 => "0",
        Digit1 => "1",
        Digit2 => "2",
        Digit3 => "3",
        Digit4 => "4",
        Digit5 => "5",
        Digit6 => "6",
        Digit7 => "7",
        Digit8 => "8",
        Digit9 => "9",
        Equal => "equal",
        IntlBackslash => "less",
        IntlRo => "Romaji",
        IntlYen => "yen",
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
        Minus => "minus",
        Period => "period",
        Quote => "apostrophe",
        Semicolon => "semicolon",
        Slash => "slash",
        AltLeft => "Alt_L",
        AltRight => "Alt_R",
        Backspace => "BackSpace",
        CapsLock => "Caps_Lock",
        ContextMenu => "Menu",
        ControlLeft => "Control_L",
        ControlRight => "Control_R",
        Enter => "Return",
        MetaLeft => "Meta_L",
        MetaRight => "Meta_R",
        ShiftLeft => "Shift_L",
        ShiftRight => "Shift_R",
        Space => "space",
        Tab => "Tab",
        Convert => "Henkan_Mode",
        KanaMode => "Kana_Lock",
        Lang1 => "Hangul",
        Lang2 => "Hangul_Hanja",
        Lang3 => "Katakana",
        Lang4 => "Hiragana",
        Lang5 => "Zenkaku_Hankaku",
        NonConvert => "Muhenkan",
        Delete => "Delete",
        End => "End",
        Help => "Help",
        Home => "Home",
        Insert => "Insert",
        PageDown => "Page_Down",
        PageUp => "Page_Up",
        ArrowDown => "Down",
        ArrowLeft => "Left",
        ArrowRight => "Right",
        ArrowUp => "Up",
        NumLock => "Num_Lock",
        Numpad0 => "KP_0",
        Numpad1 => "KP_1",
        Numpad2 => "KP_2",
        Numpad3 => "KP_3",
        Numpad4 => "KP_4",
        Numpad5 => "KP_5",
        Numpad6 => "KP_6",
        Numpad7 => "KP_7",
        Numpad8 => "KP_8",
        Numpad9 => "KP_9",
        NumpadAdd => "KP_Add",
        NumpadBackspace => "BackSpace",
        NumpadClear => "Clear",
        NumpadComma => "KP_Separator",
        NumpadDecimal => "KP_Decimal",
        NumpadDivide => "KP_Divide",
        NumpadEnter => "KP_Enter",
        NumpadEqual => "KP_Equal",
        NumpadHash => "numbersign",
        NumpadMultiply => "KP_Multiply",
        NumpadParenLeft => "parenleft",
        NumpadParenRight => "parenright",
        NumpadStar => "asterisk",
        NumpadSubtract => "KP_Subtract",
        Escape => "Escape",
        PrintScreen => "Print",
        ScrollLock => "Scroll_Lock",
        Pause => "Pause",
        BrowserBack => "Back",
        BrowserFavorites => "Favorites",
        BrowserForward => "Forward",
        BrowserHome => "HomePage",
        BrowserRefresh => "Refresh",
        BrowserSearch => "Search",
        BrowserStop => "Stop",
        Eject => "Eject",
        LaunchApp1 => "Launch1",
        LaunchApp2 => "Launch2",
        LaunchMail => "Mail",
        LaunchScreenSaver => "ScreenSaver",
        MediaFastForward => "AudioForward",
        MediaPause => "AudioPause",
        MediaPlay => "AudioPlay",
        MediaPlayPause => "AudioPlay",
        MediaRecord => "AudioRecord",
        MediaRewind => "AudioRewind",
        MediaSelect => "AudioMedia",
        MediaStop => "AudioStop",
        MediaTrackNext => "AudioNext",
        MediaTrackPrevious => "AudioPrev",
        Power => "PowerOff",
        Sleep => "Sleep",
        AudioVolumeDown => "AudioLowerVolume",
        AudioVolumeMute => "AudioMute",
        AudioVolumeUp => "AudioRaiseVolume",
        WakeUp => "WakeUp",
        Hyper => "Hyper_L",
        Super => "Super_L",
        Copy => "Copy",
        Cut => "Cut",
        Find => "Find",
        Open => "Open",
        Paste => "Paste",
        Select => "Select",
        Undo => "Undo",
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
        F25 => "F25",
        F26 => "F26",
        F27 => "F27",
        F28 => "F28",
        F29 => "F29",
        F30 => "F30",
        F31 => "F31",
        F32 => "F32",
        F33 => "F33",
        F34 => "F34",
        F35 => "F35",
        BrightnessDown => "MonBrightnessDown",
        BrightnessUp => "MonBrightnessUp",
        DisplayToggleIntExt => "Display",
        MailForward => "MailForward",
        MicrophoneMuteToggle => "AudioMicMute",
        _ => return None,
    };

    Some(name)
}

fn key_to_gdk_key_name(key: &Key) -> Option<&'static str> {
    use Key::*;

    let name = match key {
        Character(_) | Unidentified => return None,
        Alt => "Alt_L",
        AltGraph => "ISO_Level3_Shift",
        CapsLock => "Caps_Lock",
        Control => "Control_L",
        Meta => "Meta_L",
        NumLock => "Num_Lock",
        ScrollLock => "Scroll_Lock",
        Shift => "Shift_L",
        Hyper => "Hyper_L",
        Super => "Super_L",
        Enter => "Return",
        Tab => "Tab",
        ArrowDown => "Down",
        ArrowLeft => "Left",
        ArrowRight => "Right",
        ArrowUp => "Up",
        End => "End",
        Home => "Home",
        PageDown => "Page_Down",
        PageUp => "Page_Up",
        Backspace => "BackSpace",
        Clear => "Clear",
        Copy => "Copy",
        CrSel => "3270_CursorSelect",
        Cut => "Cut",
        Delete => "Delete",
        EraseEof => "3270_EraseEOF",
        ExSel => "3270_ExSelect",
        Insert => "Insert",
        Paste => "Paste",
        Redo => "Redo",
        Undo => "Undo",
        Attn => "3270_Attn",
        Cancel => "Cancel",
        ContextMenu => "Menu",
        Escape => "Escape",
        Execute => "Execute",
        Find => "Find",
        Help => "Help",
        Pause => "Pause",
        Play => "AudioPlay",
        Select => "Select",
        ZoomIn => "ZoomIn",
        ZoomOut => "ZoomOut",
        BrightnessDown => "MonBrightnessDown",
        BrightnessUp => "MonBrightnessUp",
        Eject => "Eject",
        LogOff => "LogOff",
        Power => "PowerOff",
        PowerOff => "PowerOff",
        PrintScreen => "Print",
        Hibernate => "Hibernate",
        Standby => "Standby",
        WakeUp => "WakeUp",
        Alphanumeric => "Hangul",
        Compose => "Multi_key",
        Convert => "Henkan_Mode",
        FinalMode => "Hangul_Jeonja",
        GroupFirst => "ISO_First_Group",
        GroupLast => "ISO_Last_Group",
        GroupNext => "ISO_Next_Group",
        GroupPrevious => "ISO_Prev_Group",
        ModeChange => "Mode_switch",
        NextCandidate => "Hangul_MultipleCandidate",
        NonConvert => "Muhenkan",
        PreviousCandidate => "Hangul_PreviousCandidate",
        SingleCandidate => "Hangul_SingleCandidate",
        HangulMode => "Hangul",
        HanjaMode => "Hangul_Hanja",
        JunjaMode => "Hangul_Jeonja",
        Eisu => "Eisu_toggle",
        Hiragana => "Hiragana",
        HiraganaKatakana => "Hiragana_Katakana",
        KanaMode => "Kana_Lock",
        KanjiMode => "Kanji",
        Katakana => "Katakana",
        Romaji => "Romaji",
        Zenkaku => "Zenkaku",
        ZenkakuHankaku => "Zenkaku_Hankaku",
        ChannelDown => "ChannelDown",
        ChannelUp => "ChannelUp",
        Close => "Close",
        MailForward => "MailForward",
        MailReply => "Reply",
        MailSend => "Send",
        MediaFastForward => "AudioForward",
        MediaPause => "AudioPause",
        MediaPlay => "AudioPlay",
        MediaPlayPause => "AudioPlay",
        MediaRecord => "AudioRecord",
        MediaRewind => "AudioRewind",
        MediaStop => "AudioStop",
        MediaTrackNext => "AudioNext",
        MediaTrackPrevious => "AudioPrev",
        New => "New",
        Open => "Open",
        Save => "Save",
        SpellCheck => "Spell",
        AudioVolumeDown => "AudioLowerVolume",
        AudioVolumeUp => "AudioRaiseVolume",
        AudioVolumeMute => "AudioMute",
        MicrophoneToggle => "AudioMicMute",
        MicrophoneVolumeMute => "AudioMicMute",
        LaunchApplication1 => "Launch1",
        LaunchApplication2 => "Launch2",
        LaunchCalendar => "Calendar",
        LaunchMail => "Mail",
        LaunchMediaPlayer => "AudioMedia",
        LaunchMusicPlayer => "Music",
        LaunchPhone => "Phone",
        LaunchScreenSaver => "ScreenSaver",
        LaunchWebBrowser => "WWW",
        LaunchWebCam => "WebCam",
        LaunchWordProcessor => "Word",
        BrowserBack => "Back",
        BrowserFavorites => "Favorites",
        BrowserForward => "Forward",
        BrowserHome => "HomePage",
        BrowserRefresh => "Refresh",
        BrowserSearch => "Search",
        BrowserStop => "Stop",
        DisplaySwap => "Display",
        MediaApps => "AudioMedia",
        MediaSkipForward => "AudioForward",
        MediaTopMenu => "TopMenu",
        RandomToggle => "AudioRandomPlay",
        Settings => "Tools",
        Subtitle => "Subtitle",
        ColorF0Red => "Red",
        ColorF1Green => "Green",
        ColorF2Yellow => "Yellow",
        ColorF3Blue => "Blue",
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
        F25 => "F25",
        F26 => "F26",
        F27 => "F27",
        F28 => "F28",
        F29 => "F29",
        F30 => "F30",
        F31 => "F31",
        F32 => "F32",
        F33 => "F33",
        F34 => "F34",
        F35 => "F35",
        _ => return None,
    };

    Some(name)
}
