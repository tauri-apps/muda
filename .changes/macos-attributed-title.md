---
"muda": minor
---

On macOS, Add `set_attributed_title` to `MenuItem`, `CheckMenuItem`, `IconMenuItem`, and `Submenu` to set a fully custom `NSAttributedString`. Passing `None` clears it and falls back to the plain text.
