---
"muda": minor
---

Moved Windows- and macOS-specific APIs from inherent implementations and the shared `ContextMenu` trait into platform extension traits:

- Removed `Menu::init_for_hwnd`, use `MenuExtWindows::init_for_hwnd` instead.
- Removed `Menu::init_for_hwnd_with_theme`, use `MenuExtWindows::init_for_hwnd_with_theme` instead.
- Removed `Menu::set_theme_for_hwnd`, use `MenuExtWindows::set_theme_for_hwnd` instead.
- Removed `Menu::haccel`, use `MenuExtWindows::haccel` instead.
- Removed `Menu::remove_for_hwnd`, use `MenuExtWindows::remove_for_hwnd` instead.
- Removed `Menu::hide_for_hwnd`, use `MenuExtWindows::hide_for_hwnd` instead.
- Removed `Menu::show_for_hwnd`, use `MenuExtWindows::show_for_hwnd` instead.
- Removed `Menu::is_visible_on_hwnd`, use `MenuExtWindows::is_visible_on_hwnd` instead.
- Removed `ContextMenu::hpopupmenu`, use `ContextMenuExtWindows::hpopupmenu` instead.
- Removed `ContextMenu::show_context_menu_for_hwnd`, use `ContextMenuExtWindows::show_context_menu_for_hwnd` instead.
- Removed `ContextMenu::attach_menu_subclass_for_hwnd`, use `ContextMenuExtWindows::attach_menu_subclass_for_hwnd` instead.
- Removed `ContextMenu::detach_menu_subclass_from_hwnd`, use `ContextMenuExtWindows::detach_menu_subclass_from_hwnd` instead.
- Removed `Menu::init_for_nsapp`, use `MenuExtMacOS::init_for_nsapp` instead.
- Removed `Menu::remove_for_nsapp`, use `MenuExtMacOS::remove_for_nsapp` instead.
- Removed `Submenu::set_as_windows_menu_for_nsapp`, use `SubmenuExtMacOS::set_as_windows_menu_for_nsapp` instead.
- Removed `Submenu::set_as_help_menu_for_nsapp`, use `SubmenuExtMacOS::set_as_help_menu_for_nsapp` instead.
- Removed `ContextMenu::show_context_menu_for_nsview`, use `ContextMenuExtMacOS::show_context_menu_for_nsview` instead.
- Removed `ContextMenu::ns_menu`, use `ContextMenuExtMacOS::ns_menu` instead.
