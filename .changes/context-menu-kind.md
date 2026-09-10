---
"muda": minor
---

Added `ContextMenu::kind()`, which returns the new `MenuKind` enum, and moved concrete menu downcasts onto that enum:

- Removed `ContextMenu::as_menu`, use `context_menu.kind().as_menu()` instead.
- Removed `ContextMenu::as_menu_unchecked`, use `context_menu.kind().as_menu_unchecked()` instead.
- Removed `ContextMenu::as_submenu`, use `context_menu.kind().as_submenu()` instead.
- Removed `ContextMenu::as_submenu_unchecked`, use `context_menu.kind().as_submenu_unchecked()` instead.
