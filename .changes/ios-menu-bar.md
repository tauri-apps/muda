---
"muda": patch
---

Add iOS support, showing the menu in the iPadOS 26 menu bar with the new `Menu::init_for_uiapp` and `Menu::remove_for_uiapp`. Earlier versions use it for key commands and the keyboard shortcuts list. The first submenu fills the application menu, the others replace the system's menus, predefined items reuse the system's own commands, and `NativeIcon`s map to SF Symbols. Context menus are shown as edit menus on iOS 16 and later with the new `ContextMenu::show_context_menu_for_uiview`, where the last touch landed when no position is given.
