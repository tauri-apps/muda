// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

/// Converts from muda mnemonic to gtk mnemonic.
///
/// gtk uses underline (_) for mnemonic and two underlines (__) to escape it
/// into a single underline, while muda uses (&) and (&&).
pub fn to_gtk_mnemonic<S: AsRef<str>>(string: S) -> String {
    let string = string.as_ref();
    let mut converted = String::with_capacity(string.len());
    let mut characters = string.chars().peekable();

    while let Some(character) = characters.next() {
        match character {
            '_' => converted.push_str("__"),
            '&' if characters.peek() == Some(&'&') => {
                characters.next();
                converted.push('&');
            }
            '&' => converted.push('_'),
            _ => converted.push(character),
        }
    }

    converted
}

pub fn from_gtk_mnemonic<S: AsRef<str>>(string: S) -> String {
    let string = string.as_ref();
    let mut converted = String::with_capacity(string.len());
    let mut characters = string.chars().peekable();

    while let Some(character) = characters.next() {
        match character {
            '&' => converted.push_str("&&"),
            '_' if characters.peek() == Some(&'_') => {
                characters.next();
                converted.push('_');
            }
            '_' => converted.push('&'),
            _ => converted.push(character),
        }
    }

    converted
}

#[cfg(test)]
mod tests {
    use super::{from_gtk_mnemonic, to_gtk_mnemonic};

    #[test]
    fn converts_mnemonics() {
        assert_eq!(to_gtk_mnemonic("H&ello"), "H_ello");
        assert_eq!(to_gtk_mnemonic("H&&ello"), "H&ello");
        assert_eq!(to_gtk_mnemonic("H&&&ello"), "H&_ello");
        assert_eq!(to_gtk_mnemonic("H_ello"), "H__ello");

        assert_eq!(from_gtk_mnemonic("H_ello"), "H&ello");
        assert_eq!(from_gtk_mnemonic("H__ello"), "H_ello");
        assert_eq!(from_gtk_mnemonic("H&ello"), "H&&ello");
    }
}
