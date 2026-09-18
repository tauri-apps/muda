---
"muda": minor
---

**macOS**: Add `MenuItem::set_attributed_title` to set a fully custom `NSAttributedString` as a menu item's label. This is an escape hatch for layouts and colors the semantic `set_styled_text` API cannot express, such as a right-aligned trailing segment (via an `NSParagraphStyle` with a right `NSTextTab`, as the system battery menu does) or a custom foreground color to tint a whole row. Passing `None` clears it and falls back to the plain text.
