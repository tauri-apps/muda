---
"muda": patch
---

On macOS, clear the application's services/windows/help/main menu slots when the menu occupying them is removed or dropped, so `NSApplication` no longer holds on to a stale menu.
