---
"muda": minor
---

Add `MenuItem::set_styled_text` and `IconMenuItem::set_styled_text`, which set a label as a sequence of runs carrying a semantic `TextStyle`. `TextStyle::Secondary` de-emphasizes a run, so labels can read like Finder's `Preview (default)` in its "Open with" submenu. On macOS the secondary run renders in `NSColor.secondaryLabelColor`; on Windows and Linux the runs are concatenated and drawn as plain text.
