---
"muda": patch
---

On Windows, fixed a `dangling` pointer crash when `Menu::remove_for_hwnd` is called before dropping that `Menu` and then attached a new `Menu`
