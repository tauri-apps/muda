---
"muda": minor
---

**Breaking change** Remove the `TextStyle` enum, `MenuItem/IconMenuItem/CheckMenuItem/Submenu::set_styled_text` and `MenuItemBuilder/IconMenuItemBuilder/CheckMenuItemBuilder/SubmenuBuilder::styled_text`. Use `set_attributed_title` on macOS to style a label; the styled parts rendered as plain text on the other platforms anyway.
