// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

//! Emulates the predefined edit menu items, which GTK has no action for, on the widget the user
//! was editing.
//!
//! GTK 4 has no way to hand a widget a key event, so this activates the standard clipboard and
//! selection actions instead. A `WebKitWebView` has neither, and runs editing commands through
//! an API of its own, which is resolved at runtime so that WebKitGTK stays out of the build.

use std::ffi::c_void;

use gtk::{glib, prelude::*};

use crate::{items::PredefinedMenuItemType, platform_impl::gtk_common::webkit};

/// Where the widget that last held the focus outside of a menu is remembered, on its window.
const EDIT_FOCUS_DATA_KEY: &str = "mudaEditFocus";

/// Where the handler remembering that focus is kept, on its window, so that a window is only
/// connected to once and can be disconnected from again.
const EDIT_FOCUS_HANDLER_DATA_KEY: &str = "mudaEditFocusHandler";

/// Activates an edit command on the widget the user was editing in `window`.
pub(crate) fn send(window: &gtk::Window, item_type: &PredefinedMenuItemType) {
    let action = match item_type {
        PredefinedMenuItemType::Copy => "clipboard.copy",
        PredefinedMenuItemType::Cut => "clipboard.cut",
        PredefinedMenuItemType::Paste => "clipboard.paste",
        PredefinedMenuItemType::SelectAll => "selection.select-all",
        PredefinedMenuItemType::Undo => "text.undo",
        PredefinedMenuItemType::Redo => "text.redo",
        _ => return,
    };

    if let Some(widget) = edited_widget(window) {
        // SAFETY: the widget was matched against the `WebKitWebView` type.
        let ran = web_view_ancestor(&widget).is_some_and(|web_view| unsafe {
            webkit::execute_editing_command(web_view.as_ptr() as *mut c_void, item_type)
        });

        if !ran {
            // Widgets that hold no text do not have these actions. A menu whose `Copy` is
            // activated while such a widget has the focus does nothing, as a shortcut would.
            let _ = widget.activate_action(action, None);
        }
    }
}

/// Remembers the focused widget of `window` whenever the focus is not inside a menu.
///
/// A menu takes the focus while it is open, so the window's focus at the time an item is
/// activated is the menu item rather than the widget the command has to act on.
pub(crate) fn track_focus(window: &impl IsA<gtk::Window>) {
    let window = window.as_ref();

    // SAFETY: the handler is written and read as the same type, on this thread.
    if unsafe { window.data::<glib::SignalHandlerId>(EDIT_FOCUS_HANDLER_DATA_KEY) }.is_some() {
        return;
    }

    let handler = window.connect_focus_widget_notify(|window| {
        let Some(focus) = GtkWindowExt::focus(window) else {
            return;
        };

        if is_menu_widget(&focus) {
            return;
        }

        // SAFETY: the weak reference is read back as the same type, on this thread.
        unsafe { window.set_data(EDIT_FOCUS_DATA_KEY, focus.downgrade()) };
    });

    // SAFETY: the handler is read back as the same type, on this thread.
    unsafe { window.set_data(EDIT_FOCUS_HANDLER_DATA_KEY, handler) };
}

/// Stops remembering the focus of `window`, for when its menu bar is removed.
///
/// A context menu can still be shown on the window afterwards, and tracking it again is what
/// [`track_focus`] does; whatever is left behind is released with the window either way.
pub(crate) fn untrack_focus(window: &impl IsA<gtk::Window>) {
    let window = window.as_ref();

    // SAFETY: both values are taken back as the types they were written as, on this thread.
    unsafe {
        if let Some(handler) =
            window.steal_data::<glib::SignalHandlerId>(EDIT_FOCUS_HANDLER_DATA_KEY)
        {
            window.disconnect(handler);
        }
        window.steal_data::<glib::WeakRef<gtk::Widget>>(EDIT_FOCUS_DATA_KEY);
    }
}

/// Returns the widget an edit command has to act on: the last one focused outside of a menu.
fn edited_widget(window: &gtk::Window) -> Option<gtk::Widget> {
    // SAFETY: the value is the weak reference `track_focus` stored on this window.
    let remembered = unsafe { window.data::<glib::WeakRef<gtk::Widget>>(EDIT_FOCUS_DATA_KEY) }
        .and_then(|focus| unsafe { focus.as_ref() }.upgrade());

    remembered.or_else(|| GtkWindowExt::focus(window))
}

/// Returns whether `widget` is part of a menu rather than of the window's own content.
fn is_menu_widget(widget: &gtk::Widget) -> bool {
    std::iter::successors(Some(widget.clone()), |widget| widget.parent()).any(|widget| {
        matches!(
            widget.type_().name(),
            "GtkPopoverMenuBar" | "GtkPopoverMenu" | "GtkPopover"
        )
    })
}

/// Returns `widget` or its closest ancestor that is a `WebKitWebView`.
///
/// The type is looked up by name because WebKitGTK is not a dependency: it is only registered
/// once the application has created a web view of its own.
fn web_view_ancestor(widget: &gtk::Widget) -> Option<gtk::Widget> {
    let web_view_type = glib::Type::from_name("WebKitWebView")?;

    std::iter::successors(Some(widget.clone()), |widget| widget.parent())
        .find(|widget| widget.type_().is_a(web_view_type))
}
