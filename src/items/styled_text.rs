// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

/// How one run of a menu item's label is rendered.
///
/// Styles are semantic, so each platform maps them to its own conventions instead of
/// the caller picking colors or fonts. That keeps labels correct in light and dark
/// modes, under increased contrast, and when the system menu font changes.
///
/// ## Platform-specific:
///
/// - **macOS**: [`TextStyle::Secondary`] renders in `NSColor.secondaryLabelColor`, the
///   same treatment Finder uses for the " (default)" suffix in its "Open with" submenu.
/// - **Windows / Linux**: every style renders as plain text for now.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum TextStyle {
    /// The platform's default menu label treatment.
    #[default]
    Normal,
    /// A de-emphasized treatment, for a part of the label that qualifies the rest:
    /// `Preview (default)`, `Speakers (current)`, `Folder (3 items selected)`.
    Secondary,
}
