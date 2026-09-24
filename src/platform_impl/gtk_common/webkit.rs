// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

//! Runs the predefined edit menu items on a focused `WebKitWebView`.
//!
//! A web view has none of the actions or key bindings GTK offers for text, and runs editing
//! commands through an API of its own, which is resolved at runtime so that WebKitGTK stays out
//! of the build.

use std::{
    ffi::{c_char, c_void},
    sync::OnceLock,
};

use crate::items::PredefinedMenuItemType;

/// Runs the editing command of an edit menu item on a `WebKitWebView`.
///
/// Returns whether the command ran: `item_type` has to be an edit command, and the application
/// has to be linked against WebKitGTK.
///
/// # Safety
///
/// `web_view` must point to a live `WebKitWebView`.
pub(crate) unsafe fn execute_editing_command(
    web_view: *mut c_void,
    item_type: &PredefinedMenuItemType,
) -> bool {
    let command = match item_type {
        PredefinedMenuItemType::Copy => c"Copy",
        PredefinedMenuItemType::Cut => c"Cut",
        PredefinedMenuItemType::Paste => c"Paste",
        PredefinedMenuItemType::SelectAll => c"SelectAll",
        PredefinedMenuItemType::Undo => c"Undo",
        PredefinedMenuItemType::Redo => c"Redo",
        _ => return false, // Not an edit command supported by WebKitWebView
    };

    /// `webkit_web_view_execute_editing_command`, as resolved in the running process.
    type ExecuteEditingCommand = unsafe extern "C" fn(*mut c_void, *const c_char);

    static SYMBOL: OnceLock<Option<ExecuteEditingCommand>> = OnceLock::new();

    let execute = SYMBOL.get_or_init(|| {
        // SAFETY: the symbol is looked up in the images the process has already loaded, and is
        // called below only through the signature WebKitGTK declares for it.
        unsafe {
            let symbol = libc::dlsym(
                libc::RTLD_DEFAULT,
                c"webkit_web_view_execute_editing_command".as_ptr(),
            );
            (!symbol.is_null()).then(|| std::mem::transmute::<*mut c_void, _>(symbol))
        }
    });

    match execute {
        // SAFETY: the caller guarantees the pointer, and the command outlives the call.
        Some(execute) => unsafe {
            execute(web_view, command.as_ptr());
            true
        },
        None => false,
    }
}
