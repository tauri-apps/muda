---
"muda": minor
---

Add iOS support, showing the menu in the iPadOS 26 menu bar with the new `Menu::init_for_uiapp` and `Menu::remove_for_uiapp`. The first submenu fills the application menu, the others replace the system's menus, and predefined items reuse the system's own commands. Context menus are shown as edit menus on iOS 16 and later with the new `ContextMenu::show_context_menu_for_uiview`.
