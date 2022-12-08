// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use std::{slice, str};

use cocoa::{
    base::{id, nil},
    foundation::NSString,
};

/// Strips single `&` characters from the string.
///
/// `&` can be escaped as `&&` to prevent stripping, in which case a single `&` will be output.
pub fn strip_mnemonic<S: AsRef<str>>(string: S) -> String {
    string
        .as_ref()
        .replace("&&", "[~~]")
        .replace('&', "")
        .replace("[~~]", "&")
}

/// Copies the contents of the NSString into a `String` which gets returned.
pub(crate) unsafe fn ns_string_to_rust(ns_string: id) -> String {
    let slice = slice::from_raw_parts(ns_string.UTF8String() as *mut u8, ns_string.len());
    let string = str::from_utf8_unchecked(slice);
    string.to_owned()
}

/// Gets the app's name from the `localizedName` property of `NSRunningApplication`.
pub(crate) unsafe fn app_name() -> Option<id> {
    let app_class = class!(NSRunningApplication);
    let app: id = msg_send![app_class, currentApplication];
    let app_name: id = msg_send![app, localizedName];
    if app_name != nil {
        Some(app_name)
    } else {
        None
    }
}

/// Gets the app's name as a `String` from the `localizedName` property of `NSRunningApplication`.
pub(crate) unsafe fn app_name_string() -> Option<String> {
    app_name().map(|name| ns_string_to_rust(name))
}
