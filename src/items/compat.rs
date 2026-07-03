// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

//! GTK-agnostic menu item representations for ksni tray support.
//!
//! These types provide a platform-independent way to represent menu items
//! that can be consumed by the ksni DBus tray implementation.
//!
//! These types are intentionally Send+Sync safe for use across threads
//! (e.g., ksni's DBus thread).

use std::sync::Arc;

use arc_swap::ArcSwap;

/// Thread-safe about metadata for ksni tray About dialogs.
///
/// This is a Send+Sync safe subset of `AboutMetadata` without the `Icon` field.
#[derive(Debug, Clone, Default)]
pub struct CompatAboutMetadata {
    /// The application name.
    pub name: Option<String>,
    /// The application version.
    pub version: Option<String>,
    /// The short version.
    pub short_version: Option<String>,
    /// The application authors.
    pub authors: Option<Vec<String>>,
    /// Application comments.
    pub comments: Option<String>,
    /// The copyright.
    pub copyright: Option<String>,
    /// The license text.
    pub license: Option<String>,
    /// The website URL.
    pub website: Option<String>,
    /// The website label.
    pub website_label: Option<String>,
    /// Credits text.
    pub credits: Option<String>,
}

impl CompatAboutMetadata {
    /// Creates a CompatAboutMetadata from an AboutMetadata reference.
    pub fn from_about_metadata(meta: &crate::AboutMetadata) -> Self {
        Self {
            name: meta.name.clone(),
            version: meta.version.clone(),
            short_version: meta.short_version.clone(),
            authors: meta.authors.clone(),
            comments: meta.comments.clone(),
            copyright: meta.copyright.clone(),
            license: meta.license.clone(),
            website: meta.website.clone(),
            website_label: meta.website_label.clone(),
            credits: meta.credits.clone(),
        }
    }
}

/// A standard menu item with an optional icon and predefined behavior.
///
/// This type is Send+Sync safe for cross-thread use.
#[derive(Debug, Clone)]
pub struct CompatStandardItem {
    /// Unique identifier for this menu item.
    pub id: String,
    /// Display label (mnemonics already stripped).
    pub label: String,
    /// Whether the item is enabled/clickable.
    pub enabled: bool,
    /// Optional icon as PNG bytes.
    pub icon: Option<Vec<u8>>,
    /// If this is a predefined menu item, a string identifier for its kind.
    /// Examples: "separator", "copy", "cut", "paste", "quit", "about", etc.
    /// None for regular menu items.
    pub predefined_item_id: Option<String>,
    /// About metadata for "about" predefined items.
    /// Only populated when predefined_item_id is Some("about").
    pub about_metadata: Option<CompatAboutMetadata>,
}

/// A checkmark/toggle menu item.
#[derive(Debug, Clone)]
pub struct CompatCheckmarkItem {
    /// Unique identifier for this menu item.
    pub id: String,
    /// Display label (mnemonics already stripped).
    pub label: String,
    /// Whether the item is enabled/clickable.
    pub enabled: bool,
    /// Whether the item is currently checked.
    pub checked: bool,
}

/// A submenu containing child items.
#[derive(Debug, Clone)]
pub struct CompatSubMenuItem {
    /// Display label (mnemonics already stripped).
    pub label: String,
    /// Whether the submenu is enabled.
    pub enabled: bool,
    /// Child menu items.
    pub submenu: Vec<Arc<ArcSwap<CompatMenuItem>>>,
}

/// A menu item that can be one of several types.
#[derive(Debug, Clone)]
pub enum CompatMenuItem {
    /// A standard clickable menu item.
    Standard(CompatStandardItem),
    /// A checkmark/toggle menu item.
    Checkmark(CompatCheckmarkItem),
    /// A submenu containing child items.
    SubMenu(CompatSubMenuItem),
    /// A separator line.
    Separator,
}

impl From<CompatStandardItem> for CompatMenuItem {
    fn from(item: CompatStandardItem) -> Self {
        CompatMenuItem::Standard(item)
    }
}

impl From<CompatCheckmarkItem> for CompatMenuItem {
    fn from(item: CompatCheckmarkItem) -> Self {
        CompatMenuItem::Checkmark(item)
    }
}

impl From<CompatSubMenuItem> for CompatMenuItem {
    fn from(item: CompatSubMenuItem) -> Self {
        CompatMenuItem::SubMenu(item)
    }
}

/// Removes mnemonic markers (&) from a label string.
///
/// - Single `&` is removed (e.g., "H&ello" -> "Hello")
/// - Double `&&` becomes a literal `&` (e.g., "Save && Exit" -> "Save & Exit")
///
/// This is used when populating compat labels for ksni, which doesn't
/// support GTK-style mnemonic markers.
pub fn strip_mnemonic(text: impl AsRef<str>) -> String {
    text.as_ref()
        .replace("&&", "\x00")  // Temporarily replace && with null
        .replace('&', "")       // Remove single &
        .replace('\x00', "&")   // Restore && as single &
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_mnemonic() {
        assert_eq!(strip_mnemonic("Hello"), "Hello");
        assert_eq!(strip_mnemonic("H&ello"), "Hello");
        assert_eq!(strip_mnemonic("&Hello"), "Hello");
        assert_eq!(strip_mnemonic("Hello&"), "Hello");
        assert_eq!(strip_mnemonic("Save && Exit"), "Save & Exit");
        assert_eq!(strip_mnemonic("&&Hello"), "&Hello");
        assert_eq!(strip_mnemonic("H&&ello"), "H&ello");
    }
}
