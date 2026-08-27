---
"muda": minor
---

Add `set_styled_text` to `MenuItem`, `IconMenuItem`, `CheckMenuItem`, and `Submenu`, plus a matching `styled_text` on each of their builders. It sets a label as a sequence of text parts carrying a semantic `TextStyle`, so `TextStyle::Secondary` can de-emphasize part of a label and make it read like Finder's `Preview (default)` in its "Open with" submenu. On macOS the secondary part renders in `NSColor.secondaryLabelColor`; on Windows and Linux the parts are concatenated and drawn as plain text.
