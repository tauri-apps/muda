use cocoa::appkit::NSEventModifierFlags;

/// Mnemonic is deprecated since macOS 10
pub fn remove_mnemonic(string: impl AsRef<str>) -> String {
    string.as_ref().replace("&", "")
}

/// Returns a tuple of (Key, Modifier)
pub fn parse_accelerator(accelerator: impl AsRef<str>) -> (String, NSEventModifierFlags) {
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

    let mut mods = NSEventModifierFlags::empty();
    let mod1_flag = parse_mod(mod1);
    mods |= mod1_flag;
    if let Some(mod2) = mod2 {
        let mod2_flag = parse_mod(mod2);
        mods |= mod2_flag;
    }

    let key_equivalent = parse_key(key);

    (key_equivalent, mods)
}

fn parse_mod(modifier: &str) -> NSEventModifierFlags {
    match modifier.to_uppercase().as_str() {
        "SHIFT" => NSEventModifierFlags::NSShiftKeyMask,
        "CONTROL" | "CTRL" => NSEventModifierFlags::NSControlKeyMask,
        "OPTION" | "ALT" => NSEventModifierFlags::NSAlternateKeyMask,
        "COMMAND" | "CMD" | "SUPER" | "COMMANDORCONTROL" | "COMMANDORCTRL" | "CMDORCTRL"
        | "CMDORCONTROL" => NSEventModifierFlags::NSCommandKeyMask,
        _ => panic!("Unsupported modifier: {}", modifier),
    }
}

fn parse_key(key: &str) -> String {
    match key.to_uppercase().as_str() {
        "SPACE" => "\u{0020}".into(),
        "BACKSPACE" => "\u{0008}".into(),
        "TAB" => "⇥".into(),
        "ENTER" | "RETURN" => "\u{0003}".into(),
        "ESC" | "ESCAPE" => "\u{001b}".into(),
        "PAGEUP" => "\u{F72C}".into(),
        "PAGEDOWN" => "\u{F72D}".into(),
        "END" => "\u{F72B}".into(),
        "HOME" => "\u{F729}".into(),
        "LEFTARROW" => "\u{F702}".into(),
        "UPARROW" => "\u{F700}".into(),
        "RIGHTARROW" => "\u{F703}".into(),
        "DOWNARROW" => "\u{F701}".into(),
        "DELETE" => "\u{007f}".into(),
        "0" => "0".into(),
        "1" => "1".into(),
        "2" => "2".into(),
        "3" => "3".into(),
        "4" => "4".into(),
        "5" => "5".into(),
        "6" => "6".into(),
        "7" => "7".into(),
        "8" => "8".into(),
        "9" => "9".into(),
        "A" => "a".into(),
        "B" => "b".into(),
        "C" => "c".into(),
        "D" => "d".into(),
        "E" => "e".into(),
        "F" => "f".into(),
        "G" => "g".into(),
        "H" => "h".into(),
        "I" => "i".into(),
        "J" => "j".into(),
        "K" => "k".into(),
        "L" => "l".into(),
        "M" => "m".into(),
        "N" => "n".into(),
        "O" => "o".into(),
        "P" => "p".into(),
        "Q" => "q".into(),
        "R" => "r".into(),
        "S" => "s".into(),
        "T" => "t".into(),
        "U" => "u".into(),
        "V" => "v".into(),
        "W" => "w".into(),
        "X" => "x".into(),
        "Y" => "y".into(),
        "Z" => "z".into(),
        "," => ",".into(),
        "." => ".".into(),
        "/" => "/".into(),
        _ => panic!("Unsupported modifier: {}", key),
    }
}
