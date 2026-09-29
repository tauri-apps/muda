---
"muda": minor
---

On macOS, Add `set_attributed_title` to `MenuItem`, `CheckMenuItem`, `IconMenuItem`, `Submenu` and `PredefinedMenuItem` to set a fully custom `NSAttributedString`. Passing `None` clears it and falls back to the plain text.
