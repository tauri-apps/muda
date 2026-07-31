// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

/// Converts from muda mnemonic to gtk mnemonic.
///
/// gtk uses underline (_) for mnemonic and two underlines (__) to escape it
/// into a single underline, while muda uses (&) and (&&).
pub fn to_gtk_mnemonic<S: AsRef<str>>(string: S) -> String {
    string
        .as_ref()
        .replace("_", "__")
        .replace("&&", "[~~]")
        .replace('&', "_")
        .replace("[~~]", "&")
}

pub fn from_gtk_mnemonic<S: AsRef<str>>(string: S) -> String {
    string
        .as_ref()
        .replace("&", "&&")
        .replace("__", "[~~]")
        .replace('_', "&")
        .replace("[~~]", "_")
}
