---
"muda": patch
---

On Windows, fix `CheckMenuItem::is_checked` returning the item's `enabled` state instead of its checked state when the item isn't attached to a menu yet.
