---
"muda": minor
---

Add `MenuItem::set_text_with_secondary` and `IconMenuItem::set_text_with_secondary` (macOS) for rendering Finder-style "primary (secondary)" labels where the secondary part uses `NSColor.secondaryLabelColor`. Useful for "default" markers, "currently selected" indicators, etc. On other platforms, falls back to plain text concatenation.
