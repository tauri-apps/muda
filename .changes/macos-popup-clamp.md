---
"muda": patch
---

On macOS, context-menu popups no longer overflow off the bottom or right edge of the screen and fall back to scroll arrows. `show_context_menu_for_nsview` now nudges the popup anchor so the menu fits inside the active screen's visible frame, which accounts for the menu bar, the Dock, and multi-monitor setups.
