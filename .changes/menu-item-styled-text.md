---
"muda": minor
---

Add `MenuItem/IconMenuItem/CheckMenuItem/Submenu::set_styled_text` and `MenuItemBuilder/IconMenuItemBuilder/CheckMenuItemBuilder/SubmenuBuilder::styled_text` to set a label as a sequence of text parts carrying a semantic `TextStyle`, so `TextStyle::Secondary` can de-emphasize part of a label. Supported only on macOS for now.
