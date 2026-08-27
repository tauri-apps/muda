// Copyright 2022-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

/// How one part of a menu item's label is rendered.
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

/// Applies a styled label to a menu child.
///
/// Every item type exposes the same `set_styled_text`, so the platform split lives here
/// instead of being repeated in each wrapper.
pub(crate) fn apply_styled_text<S: AsRef<str>>(
    inner: &mut crate::platform_impl::MenuChild,
    parts: impl IntoIterator<Item = (S, TextStyle)>,
) {
    #[cfg(target_os = "macos")]
    {
        let parts: Vec<(String, TextStyle)> = parts
            .into_iter()
            .map(|(text, style)| (text.as_ref().to_string(), style))
            .collect();
        inner.set_styled_text(&parts);
    }
    #[cfg(not(target_os = "macos"))]
    {
        let combined: String = parts
            .into_iter()
            .map(|(text, _)| text.as_ref().to_string())
            .collect();
        inner.set_text(&combined);
    }
}
