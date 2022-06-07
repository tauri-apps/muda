use windows_sys::Win32::UI::WindowsAndMessaging::{FALT, FCONTROL, FSHIFT, FVIRTKEY};

/// Returns a tuple of (Key, Modifier, a string representation to be used in menu items)
pub fn parse_accelerator(accelerator: impl AsRef<str>) -> (u16, u32, String) {
    let accelerator = accelerator.as_ref();
    let mut s = accelerator.split("+");
    let count = s.clone().count();
    let (mod1, mod2, key) = {
        if count == 2 {
            (s.next().unwrap(), None, s.next().unwrap())
        } else if count == 3 {
            (
                s.next().unwrap(),
                Some(s.next().unwrap()),
                s.next().unwrap(),
            )
        } else {
            panic!("Unsupported accelerator format: {}", accelerator)
        }
    };

    let mut accel_str = String::new();
    let mut mods_vk = FVIRTKEY;

    let (mod1_vk, mod1_str) = parse_mod(mod1);
    accel_str.push_str(mod1_str);
    accel_str.push_str("+");
    mods_vk |= mod1_vk;
    if let Some(mod2) = mod2 {
        let (mod2_vk, mod2_str) = parse_mod(mod2);
        accel_str.push_str(mod2_str);
        accel_str.push_str("+");
        mods_vk |= mod2_vk;
    }
    let (key_vk, key_str) = parse_key(key);
    accel_str.push_str(key_str);

    (key_vk, mods_vk, accel_str)
}

fn parse_mod(modifier: &str) -> (u32, &str) {
    match modifier.to_uppercase().as_str() {
        "SHIFT" => (FSHIFT, "Shift"),
        "CONTROL" | "CTRL" | "COMMAND" | "COMMANDORCONTROL" | "COMMANDORCTRL" => (FCONTROL, "Ctrl"),
        "ALT" => (FALT, "Alt"),
        _ => panic!("Unsupported modifier: {}", modifier),
    }
}

fn parse_key(key: &str) -> (u16, &str) {
    match key.to_uppercase().as_str() {
        "SPACE" => (0x20, "Space"),
        "BACKSPACE" => (0x08, "Backspace"),
        "TAB" => (0x09, "Tab"),
        "ENTER" | "RETURN" => (0x0D, "Enter"),
        "CAPSLOCK" => (0x14, "Caps Lock"),
        "ESC" | "ESCAPE" => (0x1B, "Esc"),
        "PAGEUP" => (0x21, "Page Up"),
        "PAGEDOWN" => (0x22, "Page Down"),
        "END" => (0x23, "End"),
        "HOME" => (0x24, "Home"),
        "LEFTARROW" => (0x25, "Left Arrow"),
        "UPARROW" => (0x26, "Up Arrow"),
        "RIGHTARROW" => (0x27, "Right Arrow"),
        "DOWNARROW" => (0x28, "Down Arrow"),
        "DELETE" => (0x2E, "Del"),
        "0" => (0x30, "0"),
        "1" => (0x31, "1"),
        "2" => (0x32, "2"),
        "3" => (0x33, "3"),
        "4" => (0x34, "4"),
        "5" => (0x35, "5"),
        "6" => (0x36, "6"),
        "7" => (0x37, "7"),
        "8" => (0x38, "8"),
        "9" => (0x39, "9"),
        "A" => (0x41, "A"),
        "B" => (0x42, "B"),
        "C" => (0x43, "C"),
        "D" => (0x44, "D"),
        "E" => (0x45, "E"),
        "F" => (0x46, "F"),
        "G" => (0x47, "G"),
        "H" => (0x48, "H"),
        "I" => (0x49, "I"),
        "J" => (0x4A, "J"),
        "K" => (0x4B, "K"),
        "L" => (0x4C, "L"),
        "M" => (0x4D, "M"),
        "N" => (0x4E, "N"),
        "O" => (0x4F, "O"),
        "P" => (0x50, "P"),
        "Q" => (0x51, "Q"),
        "R" => (0x52, "R"),
        "S" => (0x53, "S"),
        "T" => (0x54, "T"),
        "U" => (0x55, "U"),
        "V" => (0x56, "V"),
        "W" => (0x57, "W"),
        "X" => (0x58, "X"),
        "Y" => (0x59, "Y"),
        "Z" => (0x5A, "Z"),
        "NUM0" | "NUMPAD0" => (0x60, "Num 0"),
        "NUM1" | "NUMPAD1" => (0x61, "Num 1"),
        "NUM2" | "NUMPAD2" => (0x62, "Num 2"),
        "NUM3" | "NUMPAD3" => (0x63, "Num 3"),
        "NUM4" | "NUMPAD4" => (0x64, "Num 4"),
        "NUM5" | "NUMPAD5" => (0x65, "Num 5"),
        "NUM6" | "NUMPAD6" => (0x66, "Num 6"),
        "NUM7" | "NUMPAD7" => (0x67, "Num 7"),
        "NUM8" | "NUMPAD8" => (0x68, "Num 8"),
        "NUM9" | "NUMPAD9" => (0x69, "Num 9"),
        _ => panic!("Unsupported modifier: {}", key),
    }
}
