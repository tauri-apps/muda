// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use std::str;

use objc2_app_kit::NSRunningApplication;

pub(crate) fn app_name() -> Option<String> {
    NSRunningApplication::currentApplication()
        .localizedName()
        .map(|name| name.to_string())
}

/// Strips single `&` characters from the string.
///
/// `&` can be escaped as `&&` to prevent stripping, in which case a single `&` will be output.
pub fn strip_mnemonic<S: AsRef<str>>(string: S) -> String {
    let string = string.as_ref();
    let mut stripped = String::with_capacity(string.len());
    let mut characters = string.chars().peekable();

    while let Some(character) = characters.next() {
        match character {
            '&' if characters.peek() == Some(&'&') => {
                characters.next();
                stripped.push('&');
            }
            '&' => {}
            _ => stripped.push(character),
        }
    }

    stripped
}

#[cfg(test)]
mod tests {
    use super::strip_mnemonic;

    #[test]
    fn strips_mnemonics() {
        assert_eq!(strip_mnemonic("H&ello"), "Hello");
        assert_eq!(strip_mnemonic("H&&ello"), "H&ello");
        assert_eq!(strip_mnemonic("H&&&ello"), "H&ello");
    }
}
