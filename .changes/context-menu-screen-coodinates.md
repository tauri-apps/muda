---
"muda": "patch"
---

**Breaking Change**: `ContextMenu::show_context_menu_for_hwnd`, `ContextMenu::show_context_menu_for_gtk_window` and `ContextMenu::show_context_menu_for_nsview` has been changed to take `x` and `y` relative to the screen top-left corner instead of relative to the window top-left corner.
